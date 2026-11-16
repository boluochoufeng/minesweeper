// SPDX-License-Identifier: GPL-3.0-or-later
//
// Settings and high-score persistence. Both `settings.toml` (managed by
// `bevy_settings`) and `data.json` (scores) live under the unified
// `org.gnome.Mines` preferences directory; legacy `bevy-gnome-mines/data.json`
// files are migrated automatically on first launch.

use crate::game::{Board, Cell, Mark, NEIGHBOURS, Outcome};
use bevy::{
    platform::dirs::preferences_dir,
    prelude::*,
    settings::{ReflectSettingsGroup, SettingsGroup},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Canonical reverse-DNS application identifier shared by windowing, settings
/// and score persistence.
pub const APP_ID: &str = "org.gnome.Mines";

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

/// Supported custom board width/height bounds.
pub const MIN_CUSTOM_DIM: usize = 4;
pub const MAX_CUSTOM_DIM: usize = 100;

/// Smallest logical window size the shell keeps; also enforced when a window
/// size arrives from settings or a programmatic resize.
pub const MIN_WINDOW_WIDTH: f32 = 700.0;
pub const MIN_WINDOW_HEIGHT: f32 = 600.0;

/// Upper bound accepted for persisted window sizes.
const MAX_WINDOW_SIZE: f32 = 16_384.0;

/// Cap restored elapsed durations at ~7 days so corrupt files cannot overflow.
const MAX_SAVED_ELAPSED_MS: u64 = 7 * 24 * 3_600 * 1_000;

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
                let width = settings.custom_width.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
                let height = settings.custom_height.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
                let max_mines = (width * height).saturating_sub(10).max(1);
                let mines = settings.custom_mines.clamp(1, max_mines);
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
    pub no_guessing: bool,
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
            no_guessing: false,
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
    settings.custom_width = settings.custom_width.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
    settings.custom_height = settings.custom_height.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Serialized snapshot of an active in-progress board and its elapsed clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedGame {
    pub difficulty: Difficulty,
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
    #[serde(default)]
    pub no_guessing: bool,
    pub elapsed_millis: u64,
    pub cells: Vec<Cell>,
}

/// Runtime resource holding a resumable saved game (loaded at startup or
/// checkpointed while playing).
#[derive(Resource, Debug, Default, Clone)]
pub struct SavedGameSlot(pub Option<SavedGame>);

impl SavedGame {
    /// Build a snapshot from an active, placed, unfinished board.
    pub fn from_board(difficulty: Difficulty, board: &Board, elapsed: Duration) -> Option<Self> {
        if !board.placed || board.revealed_count == 0 || board.outcome != Outcome::InProgress {
            return None;
        }
        let elapsed_millis = elapsed.as_millis().min(u128::from(MAX_SAVED_ELAPSED_MS)) as u64;
        Some(Self {
            difficulty,
            width: board.width,
            height: board.height,
            mine_count: board.mine_count,
            no_guessing: board.no_guessing,
            elapsed_millis,
            cells: board.cells.clone(),
        })
    }

    pub fn mines_left(&self) -> i64 {
        let flags = self
            .cells
            .iter()
            .filter(|cell| !cell.revealed && cell.mark == Mark::Flag)
            .count();
        self.mine_count as i64 - flags as i64
    }

    pub fn elapsed_secs(&self) -> u64 {
        self.elapsed_millis / 1_000
    }

    /// Validate bounds and invariants, recomputing derived neighbour counts so
    /// tampered snapshots are safely rejected before entering gameplay.
    pub fn validated(mut self) -> Option<Self> {
        if !(MIN_CUSTOM_DIM..=MAX_CUSTOM_DIM).contains(&self.width)
            || !(MIN_CUSTOM_DIM..=MAX_CUSTOM_DIM).contains(&self.height)
        {
            return None;
        }
        let total = self.width * self.height;
        if self.cells.len() != total || total <= 10 || !(1..=total - 10).contains(&self.mine_count)
        {
            return None;
        }
        let actual_mines = self.cells.iter().filter(|cell| cell.mine).count();
        if actual_mines != self.mine_count {
            return None;
        }
        if self.cells.iter().any(|cell| cell.revealed && cell.mine) {
            return None;
        }
        for cell in &mut self.cells {
            if cell.revealed {
                cell.mark = Mark::None;
            }
        }
        let revealed_count = self.cells.iter().filter(|cell| cell.revealed).count();
        if !(1..total - self.mine_count).contains(&revealed_count) {
            return None;
        }
        let mine_flags: Vec<bool> = self.cells.iter().map(|cell| cell.mine).collect();
        for idx in 0..total {
            let x = (idx % self.width) as i32;
            let y = (idx / self.width) as i32;
            let mut count = 0u8;
            for (dx, dy) in NEIGHBOURS {
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0 && ny >= 0 && (nx as usize) < self.width && (ny as usize) < self.height {
                    let n_idx = ny as usize * self.width + nx as usize;
                    if mine_flags[n_idx] {
                        count += 1;
                    }
                }
            }
            self.cells[idx].adjacent = count;
        }
        self.elapsed_millis = self.elapsed_millis.min(MAX_SAVED_ELAPSED_MS);
        Some(self)
    }

    /// Convert a validated snapshot back into a live [`Board`] and elapsed
    /// clock duration.
    pub fn into_restored(self) -> Option<(Difficulty, Board, Duration)> {
        let saved = self.validated()?;
        let revealed_count = saved.cells.iter().filter(|cell| cell.revealed).count();
        let flag_count = saved
            .cells
            .iter()
            .filter(|cell| !cell.revealed && cell.mark == Mark::Flag)
            .count();
        let board = Board {
            width: saved.width,
            height: saved.height,
            mine_count: saved.mine_count,
            cells: saved.cells,
            placed: true,
            no_guessing: saved.no_guessing,
            outcome: Outcome::InProgress,
            revealed_count,
            flag_count,
            exploded_cell: None,
        };
        Some((
            saved.difficulty,
            board,
            Duration::from_millis(saved.elapsed_millis),
        ))
    }
}

/// Persisted non-settings data stored in `org.gnome.Mines/data.json`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameData {
    pub scores: Vec<Score>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_game: Option<SavedGame>,
}

/// Legacy single-file store (`bevy-gnome-mines/data.json`) kept for automatic
/// upgrade migration.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct LegacyPersistentData {
    pub settings: Settings,
    pub scores: Vec<Score>,
}

/// Startup snapshot returned after loading modern storage or migrating legacy
/// `bevy-gnome-mines/data.json`.
#[derive(Debug, Default)]
pub struct LoadedPersistence {
    pub settings: Settings,
    pub scores: Vec<Score>,
    pub saved_game: Option<SavedGame>,
    pub migrate_settings: bool,
}

impl GameData {
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(ErrorKind::InvalidData, error))?;
        write_atomic(path, &text)
    }
}

/// Load scores, any resumable saved game (and legacy settings when migrating)
/// from the platform storage directories.
pub fn load_persistence() -> LoadedPersistence {
    load_persistence_at(
        data_path().as_deref(),
        legacy_data_path().as_deref(),
        settings_path().as_deref(),
    )
}

fn load_persistence_at(
    modern_path: Option<&Path>,
    legacy_path: Option<&Path>,
    settings_file: Option<&Path>,
) -> LoadedPersistence {
    if let Some(modern) = modern_path
        && modern.exists()
    {
        let data: GameData = load_json_from_path(modern);
        return LoadedPersistence {
            settings: Settings::default(),
            scores: data.scores,
            saved_game: data.saved_game.and_then(SavedGame::validated),
            migrate_settings: false,
        };
    }

    if let Some(legacy) = legacy_path
        && legacy.exists()
    {
        info!(
            "Migrating legacy game data from {} to unified {} directory",
            legacy.display(),
            APP_ID
        );
        let mut legacy_data: LegacyPersistentData = load_json_from_path(legacy);
        sanitize_settings(&mut legacy_data.settings);
        if let Some(modern) = modern_path {
            let migrated = GameData {
                scores: legacy_data.scores.clone(),
                saved_game: None,
            };
            if let Err(error) = migrated.save_to(modern) {
                warn!(
                    "Could not write migrated scores to {}: {error}",
                    modern.display()
                );
            }
        }
        let migrate_settings = settings_file.is_some_and(|path| !path.exists());
        return LoadedPersistence {
            settings: legacy_data.settings,
            scores: legacy_data.scores,
            saved_game: None,
            migrate_settings,
        };
    }

    LoadedPersistence::default()
}

fn load_json_from_path<T>(path: &Path) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            if error.kind() != ErrorKind::NotFound {
                warn!("Could not read game data {}: {error}", path.display());
            }
            return T::default();
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
            T::default()
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

/// Unified per-user application directory (`.../org.gnome.Mines`).
pub fn app_data_dir() -> Option<PathBuf> {
    preferences_dir().map(|dir| dir.join(APP_ID))
}

/// Modern score store (`.../org.gnome.Mines/data.json`).
pub fn data_path() -> Option<PathBuf> {
    app_data_dir().map(|dir| dir.join("data.json"))
}

/// Legacy score/settings store (`.../bevy-gnome-mines/data.json`) checked when
/// migrating older installations.
pub fn legacy_data_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("bevy-gnome-mines").join("data.json"))
}

/// Where `bevy_settings` stores `settings.toml` for this app.
pub fn settings_path() -> Option<PathBuf> {
    app_data_dir().map(|dir| dir.join("settings.toml"))
}

/// Persist the score table and optional in-progress game to `org.gnome.Mines/data.json`.
pub fn save_progress(scores: &[Score], saved_game: Option<&SavedGame>) {
    let Some(path) = data_path() else {
        return;
    };
    let data = GameData {
        scores: scores.to_vec(),
        saved_game: saved_game.cloned(),
    };
    if let Err(error) = data.save_to(&path) {
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

        let data: GameData = load_json_from_path(&path);
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
        let data: GameData = load_json_from_path(&dir.join("data.json"));
        assert!(data.scores.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_data_migrates_scores_and_settings_into_unified_dir() {
        let dir = scratch_dir("migrate");
        let legacy_path = dir.join("bevy-gnome-mines").join("data.json");
        let modern_dir = dir.join(APP_ID);
        let modern_data = modern_dir.join("data.json");
        let modern_settings = modern_dir.join("settings.toml");

        fs::create_dir_all(legacy_path.parent().unwrap()).expect("create legacy dir");
        let legacy_payload = LegacyPersistentData {
            settings: Settings {
                theme: "classic".to_owned(),
                custom_width: 24,
                ..Settings::default()
            },
            scores: vec![Score {
                width: 8,
                height: 8,
                mines: 10,
                seconds: 42,
                timestamp: 1_700_000_000,
            }],
        };
        fs::write(
            &legacy_path,
            serde_json::to_string_pretty(&legacy_payload).expect("serialize legacy"),
        )
        .expect("write legacy file");

        let loaded = load_persistence_at(
            Some(&modern_data),
            Some(&legacy_path),
            Some(&modern_settings),
        );
        assert!(loaded.migrate_settings);
        assert_eq!(loaded.settings.theme, "classic");
        assert_eq!(loaded.settings.custom_width, 24);
        assert_eq!(loaded.scores, legacy_payload.scores);
        assert!(
            modern_data.exists(),
            "migrated scores must be written to unified dir"
        );
        let written = fs::read_to_string(&modern_data).expect("read migrated json");
        assert!(
            !written.contains("\"settings\""),
            "modern data.json should only store non-settings game data"
        );

        // Subsequent launches read directly from `org.gnome.Mines/data.json` without re-migrating.
        let second = load_persistence_at(
            Some(&modern_data),
            Some(&legacy_path),
            Some(&modern_settings),
        );
        assert!(!second.migrate_settings);
        assert_eq!(second.scores, legacy_payload.scores);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saved_game_round_trips_and_rejects_tampered_boards() {
        use rand::{SeedableRng, rngs::StdRng};

        let dir = scratch_dir("saved_game");
        let data_file = dir.join("data.json");

        let mut rng = StdRng::seed_from_u64(1234);
        let mut board = Board::new(8, 8, 10);
        board.no_guessing = true;
        assert!(board.reveal(3, 3, &mut rng, false, false));

        // Mark one hidden cell with a flag so flag_count is also exercised.
        let flag_idx = board
            .cells
            .iter()
            .position(|cell| !cell.revealed)
            .expect("unfinished board must still have hidden cells");
        let (fx, fy) = board.coords(flag_idx);
        assert!(board.cycle_mark(fx, fy, false, false, false));

        let saved = SavedGame::from_board(Difficulty::Small, &board, Duration::from_millis(84_320))
            .expect("active placed board produces a snapshot");

        GameData {
            scores: Vec::new(),
            saved_game: Some(saved.clone()),
        }
        .save_to(&data_file)
        .expect("save data.json");

        let loaded = load_persistence_at(Some(&data_file), None, None);
        let restored_snap = loaded.saved_game.expect("saved game should load cleanly");
        let (difficulty, restored_board, elapsed) = restored_snap
            .into_restored()
            .expect("validated snapshot must restore");
        assert_eq!(difficulty, Difficulty::Small);
        assert_eq!(elapsed, Duration::from_millis(84_320));
        assert!(restored_board.placed);
        assert!(restored_board.no_guessing);
        assert_eq!(restored_board.width, board.width);
        assert_eq!(restored_board.height, board.height);
        assert_eq!(restored_board.mine_count, board.mine_count);
        assert_eq!(restored_board.revealed_count, board.revealed_count);
        assert_eq!(restored_board.flag_count, board.flag_count);
        assert_eq!(restored_board.cells, board.cells);

        // Tampered: mismatched mine_count.
        let mut bad_mines = saved.clone();
        bad_mines.mine_count += 1;
        assert!(bad_mines.into_restored().is_none());

        // Tampered: a revealed cell claims to be a mine.
        let mut bad_revealed_mine = saved.clone();
        let revealed_idx = bad_revealed_mine
            .cells
            .iter()
            .position(|cell| cell.revealed)
            .unwrap();
        let mine_idx = bad_revealed_mine
            .cells
            .iter()
            .position(|cell| cell.mine)
            .unwrap();
        bad_revealed_mine.cells[revealed_idx].mine = true;
        bad_revealed_mine.cells[mine_idx].mine = false;
        assert!(bad_revealed_mine.into_restored().is_none());

        // Tampered: already won (all safe cells revealed).
        let mut already_won = saved.clone();
        for cell in &mut already_won.cells {
            if !cell.mine {
                cell.revealed = true;
            }
        }
        assert!(already_won.into_restored().is_none());

        // Tampered: zero revealed cells.
        let mut unstarted = saved;
        for cell in &mut unstarted.cells {
            cell.revealed = false;
        }
        assert!(unstarted.into_restored().is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
