// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::{
    prelude::*,
    settings::{ReflectSettingsGroup, SettingsGroup},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
}

#[derive(Resource, SettingsGroup, Reflect, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "game")]
#[serde(default)]
pub struct Settings {
    pub theme: String,
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
            theme: "bgcolors".to_owned(),
            use_question_marks: true,
            use_autoflag: false,
            auto_clear_on_flags: false,
            custom_width: 16,
            custom_height: 16,
            custom_mines: 40,
            difficulty: Difficulty::Small,
            window_width: 600.0,
            window_height: 600.0,
            window_maximized: false,
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
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = data_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, text);
        }
    }

    pub fn add_score(&mut self, width: usize, height: usize, mines: usize, seconds: u64) {
        self.scores.push(Score {
            width,
            height,
            mines,
            seconds,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        self.scores.sort_by_key(|score| {
            (
                score.width,
                score.height,
                score.mines,
                score.seconds,
                score.timestamp,
            )
        });

        let mut seen = std::collections::HashMap::new();
        self.scores.retain(|score| {
            let count = seen
                .entry((score.width, score.height, score.mines))
                .or_insert(0usize);
            *count += 1;
            *count <= 10
        });
        self.save();
    }
}

fn data_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("bevy-gnome-mines").join("data.json"))
}
