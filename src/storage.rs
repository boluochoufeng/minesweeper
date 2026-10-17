// SPDX-License-Identifier: GPL-3.0-or-later
//
// Settings and high-score persistence. Settings are stored by `bevy_settings`
// as TOML in the platform preferences directory; scores stay in the compatible
// `bevy-gnome-mines/data.json` file.

use bevy::{
    platform::dirs::preferences_dir,
    prelude::*,
    settings::{ReflectSettingsGroup, SettingsGroup},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Values for [`Settings::theme`] that select the board art style.
pub const BOARD_STYLES: [(&str, &str); 5] = [
    ("ui", "现代"),
    ("bgcolors", "背景颜色"),
    ("classic", "经典"),
    ("default", "默认"),
    ("modern", "GNOME 现代"),
];

/// Values for [`Settings::ui_theme`].
pub const UI_THEMES: [(&str, &str); 4] = [
    ("system", "跟随系统"),
    ("light", "浅色"),
    ("dark", "深色"),
    ("high-contrast", "高对比度"),
];

/// Smallest logical window size the shell keeps; also enforced when a window
/// size arrives from settings or a programmatic resize.
pub const MIN_WINDOW_WIDTH: f32 = 700.0;
pub const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// Upper bound accepted for persisted window sizes.
const MAX_WINDOW_SIZE: f32 = 16_384.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default)]
pub enum Difficulty {
    #[default]
    Small,
    Medium,
    Large,
    Custom,
}

impl Difficulty {
    pub fn dimensions(self, settings: &Settings) -> (usize, usize, usize) {
        match self {
            Self::Small => (8, 8, 10),
            Self::Medium => (16, 16, 40),
            Self::Large => (30, 16, 99),
            Self::Custom => {
                let width = settings.custom_width.clamp(4, 100);
                let height = settings.custom_height.clamp(4, 100);
                let mines = settings.custom_mines.clamp(1, width * height - 10);
                (width, height, mines)
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "小型",
            Self::Medium => "中型",
            Self::Large => "大型",
            Self::Custom => "自定义",
        }
    }
}

#[derive(Resource, SettingsGroup, Reflect, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "game")]
#[serde(default)]
pub struct Settings {
    /// Board art style key, see [`BOARD_STYLES`].
    pub theme: String,
    /// UI color theme key, see [`UI_THEMES`].
    pub ui_theme: String,
    /// Global UI scale multiplier.
    pub ui_scale: f32,
    /// Disable non-essential animations.
    pub reduced_motion: bool,
    pub use_question_marks: bool,
    pub use_autoflag: bool,
    pub auto_clear_on_flags: bool,
    pub custom_width: usize,
    pub custom_height: usize,
    pub custom_mines: usize,
    pub difficulty: Difficulty,
    pub window_width: f32,
    pub window_height: f32,
    pub window_maximized: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "ui".to_owned(),
            ui_theme: "system".to_owned(),
            ui_scale: 1.0,
            reduced_motion: false,
            use_question_marks: true,
            use_autoflag: false,
            auto_clear_on_flags: false,
            custom_width: 16,
            custom_height: 16,
            custom_mines: 40,
            difficulty: Difficulty::Small,
            window_width: 720.0,
            window_height: 760.0,
            window_maximized: false,
        }
    }
}

impl Settings {
    /// UI scale clamped to the supported range.
    pub fn ui_scale_factor(&self) -> f32 {
        if self.ui_scale.is_finite() {
            self.ui_scale.clamp(0.85, 1.5)
        } else {
            1.0
        }
    }
}

/// Repairs values that can arrive from an old `data.json`, a hand-edited
/// `settings.toml` or a partial migration. Unknown keys fall back to the
/// current defaults and numbers are clamped to the ranges the UI can render.
pub fn sanitize_settings(settings: &mut Settings) {
    let theme = settings.theme.trim().to_owned();
    settings.theme = if BOARD_STYLES.iter().any(|(key, _)| *key == theme) {
        theme
    } else {
        BOARD_STYLES[0].0.to_owned()
    };

    let ui_theme = settings.ui_theme.trim().to_owned();
    settings.ui_theme = if UI_THEMES.iter().any(|(key, _)| *key == ui_theme) {
        ui_theme
    } else {
        UI_THEMES[0].0.to_owned()
    };

    settings.ui_scale = settings.ui_scale_factor();
    settings.custom_width = settings.custom_width.clamp(4, 100);
    settings.custom_height = settings.custom_height.clamp(4, 100);
    // Leave at least ten non-mine cells so a custom board stays solvable.
    let max_mines = (settings.custom_width * settings.custom_height).saturating_sub(10);
    settings.custom_mines = settings.custom_mines.clamp(1, max_mines.max(1));
    settings.window_width = sanitize_window_dimension(settings.window_width, MIN_WINDOW_WIDTH);
    settings.window_height = sanitize_window_dimension(settings.window_height, MIN_WINDOW_HEIGHT);
}

fn sanitize_window_dimension(value: f32, minimum: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, MAX_WINDOW_SIZE)
    } else {
        minimum
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub width: usize,
    pub height: usize,
    pub mines: usize,
    pub seconds: u64,
    pub timestamp: u64,
}

/// High scores, best first per board configuration.
#[derive(Resource, Debug, Default, Clone)]
pub struct Scores(pub Vec<Score>);

impl Scores {
    pub fn add(&mut self, width: usize, height: usize, mines: usize, seconds: u64) {
        self.0.push(Score {
            width,
            height,
            mines,
            seconds,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        self.0.sort_by_key(|score| {
            (
                score.width,
                score.height,
                score.mines,
                score.seconds,
                score.timestamp,
            )
        });

        let mut seen = std::collections::HashMap::new();
        self.0.retain(|score| {
            let count = seen
                .entry((score.width, score.height, score.mines))
                .or_insert(0usize);
            *count += 1;
            *count <= 10
        });
    }

    pub fn best(&self, width: usize, height: usize, mines: usize) -> Option<u64> {
        self.0
            .iter()
            .filter(|score| score.width == width && score.height == height && score.mines == mines)
            .map(|score| score.seconds)
            .min()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistentData {
    pub settings: Settings,
    pub scores: Vec<Score>,
}

impl PersistentData {
    /// Read the JSON store. A missing file is normal; an unreadable or corrupt
    /// file is reported and moved aside so the next save starts from a clean
    /// slate without silently discarding the player's scores.
    pub fn load() -> Self {
        data_path()
            .map(|path| load_from_path(&path))
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = data_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        write_atomic(&path, &text)
    }
}

fn load_from_path(path: &Path) -> PersistentData {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            if error.kind() != ErrorKind::NotFound {
                warn!("Could not read game data {}: {error}", path.display());
            }
            return PersistentData::default();
        }
    };
    match serde_json::from_str(&text) {
        Ok(data) => data,
        Err(error) => {
            warn!("Could not parse game data {}: {error}", path.display());
            let backup = corrupt_backup_path(path);
            match fs::rename(path, &backup) {
                Ok(()) => warn!("Unreadable game data moved to {}", backup.display()),
                Err(error) => {
                    warn!("Could not move unreadable game data aside: {error}");
                }
            }
            PersistentData::default()
        }
    }
}

/// Write through a temporary file so an interrupted save cannot truncate the
/// previous data. Windows `rename` refuses to replace an existing file, so the
/// fallback removes the target first.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, text)?;
    match fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            fs::rename(&temp, path)
        }
    }
}

fn corrupt_backup_path(path: &Path) -> PathBuf {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    path.with_file_name(format!("data.json.corrupt-{seconds}"))
}

pub fn data_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("bevy-gnome-mines").join("data.json"))
}

/// Where `bevy_settings` stores `settings.toml` for this app.
pub fn settings_path() -> Option<PathBuf> {
    preferences_dir().map(|dir| dir.join("org.gnome.Mines").join("settings.toml"))
}

/// True on the first launch after upgrading from the JSON-only releases: a
/// legacy `data.json` exists, but the modern settings file has not been written
/// yet.
pub fn needs_settings_migration() -> bool {
    let Some(settings) = settings_path() else {
        return false;
    };
    !settings.exists() && data_path().is_some_and(|path| path.exists())
}

/// Persist the score table together with a snapshot of the settings.
pub fn save_progress(settings: &Settings, scores: &[Score]) {
    let data = PersistentData {
        settings: settings.clone(),
        scores: scores.to_vec(),
    };
    if let Err(error) = data.save() {
        warn!("Could not save game data: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("minesweeper-storage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch directory");
        dir
    }

    #[test]
    fn sanitize_repairs_invalid_values() {
        let mut settings = Settings {
            theme: "nope".to_owned(),
            ui_theme: "  ".to_owned(),
            ui_scale: f32::NAN,
            custom_width: 1,
            custom_height: 500,
            custom_mines: 10_000,
            window_width: f32::INFINITY,
            window_height: 0.0,
            ..Settings::default()
        };
        sanitize_settings(&mut settings);
        assert_eq!(settings.theme, "ui");
        assert_eq!(settings.ui_theme, "system");
        assert_eq!(settings.ui_scale, 1.0);
        assert_eq!(settings.custom_width, 4);
        assert_eq!(settings.custom_height, 100);
        assert_eq!(settings.custom_mines, 4 * 100 - 10);
        assert_eq!(settings.window_width, MIN_WINDOW_WIDTH);
        assert_eq!(settings.window_height, MIN_WINDOW_HEIGHT);
    }

    #[test]
    fn sanitize_keeps_supported_values() {
        let mut settings = Settings {
            theme: "classic".to_owned(),
            ui_theme: "dark".to_owned(),
            ui_scale: 1.2,
            custom_width: 12,
            custom_height: 9,
            custom_mines: 20,
            window_width: 900.0,
            window_height: 800.0,
            ..Settings::default()
        };
        let expected = settings.clone();
        sanitize_settings(&mut settings);
        assert_eq!(settings, expected);
    }

    #[test]
    fn atomic_write_round_trips_and_replaces() {
        let dir = scratch_dir("atomic");
        let path = dir.join("data.json");
        write_atomic(&path, "{\"scores\":[]}").expect("initial write");
        assert_eq!(fs::read_to_string(&path).expect("read"), "{\"scores\":[]}");
        assert!(!path.with_extension("json.tmp").exists());

        write_atomic(&path, "{\"scores\":[1]}").expect("overwrite");
        assert_eq!(fs::read_to_string(&path).expect("read"), "{\"scores\":[1]}");
        assert!(!path.with_extension("json.tmp").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_load_is_backed_up() {
        let dir = scratch_dir("corrupt");
        let path = dir.join("data.json");
        fs::write(&path, "not json at all").expect("write corrupt file");

        let data = load_from_path(&path);
        assert_eq!(data.settings, Settings::default());
        assert!(data.scores.is_empty());
        assert!(!path.exists());

        let backups: Vec<PathBuf> = fs::read_dir(&dir)
            .expect("list scratch directory")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|entry| {
                entry
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("data.json.corrupt-"))
            })
            .collect();
        assert_eq!(backups.len(), 1, "one corrupt backup should exist");
        assert_eq!(
            fs::read_to_string(&backups[0]).expect("read backup"),
            "not json at all"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_loads_defaults() {
        let dir = scratch_dir("missing");
        let data = load_from_path(&dir.join("data.json"));
        assert_eq!(data.settings, Settings::default());
        assert!(data.scores.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
