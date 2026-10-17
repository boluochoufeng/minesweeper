// SPDX-License-Identifier: GPL-3.0-or-later
//
// Settings and high-score persistence. Settings are stored by `bevy_settings`
// as TOML in the platform preferences directory; scores stay in the compatible
// `bevy-gnome-mines/data.json` file.

use bevy::{
    prelude::*,
    settings::{ReflectSettingsGroup, SettingsGroup},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
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
    pub fn load() -> Self {
        let Some(path) = data_path() else {
            return Self::default();
        };
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
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
        fs::write(path, text)
    }

}

pub fn data_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("bevy-gnome-mines").join("data.json"))
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
