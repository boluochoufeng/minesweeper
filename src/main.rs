// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(dead_code)]

#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod assets;
mod game;
mod input;
mod state;
mod storage;
mod theme;
mod ui;

use bevy::feathers::FeathersPlugins;
use bevy::prelude::*;
use bevy::settings::SettingsPlugin;
use bevy::window::{ExitCondition, WindowResolution};
use assets::AssetsPlugin;
use input::InputPlugin;
use state::{GameSession, Screen};
use storage::{Difficulty, PersistentData, Scores, Settings};
use theme::ThemePlugin;
use ui::MinesUiPlugin;

/// Difficulty requested on the command line, applied after startup.
#[derive(Resource, Default)]
struct CliDifficulty(Option<Difficulty>);

fn main() {
    let persistent = PersistentData::load();
    let legacy_settings = persistent.settings.clone();
    let cli_difficulty = parse_cli();

    App::new()
        .register_type::<Difficulty>()
        .register_type::<Settings>()
        .insert_resource(legacy_settings)
        .insert_resource(Scores(persistent.scores.clone()))
        .insert_resource(CliDifficulty(cli_difficulty))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "扫雷".to_owned(),
                name: Some("org.gnome.Mines".to_owned()),
                resolution: WindowResolution::new(720, 760),
                resizable: true,
                ..default()
            }),
            exit_condition: ExitCondition::DontExit,
            ..default()
        }))
        .add_plugins(SettingsPlugin::new("org.gnome.Mines"))
        .add_plugins(FeathersPlugins)
        .init_state::<Screen>()
        .init_resource::<state::GameSession>()
        .add_plugins((AssetsPlugin, ThemePlugin, MinesUiPlugin, InputPlugin))
        .add_systems(Startup, (spawn_camera, apply_cli_difficulty).chain())
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn apply_cli_difficulty(
    cli: Res<CliDifficulty>,
    mut session: ResMut<GameSession>,
    mut settings: ResMut<Settings>,
    mut next: ResMut<NextState<Screen>>,
) {
    if let Some(difficulty) = cli.0 {
        session.start_game(difficulty, &mut settings, &mut next);
    }
}

fn parse_cli() -> Option<Difficulty> {
    let mut result = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-v" | "--version" => {
                println!("gnome-mines-bevy {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--small" => result = Some(Difficulty::Small),
            "--medium" => result = Some(Difficulty::Medium),
            "--big" | "--large" => result = Some(Difficulty::Large),
            "-h" | "--help" => {
                println!(
                    "Mines\n\nUsage: minesweeper [OPTION]\n\n  --small    Start a small game\n  --medium   Start a medium game\n  --big      Start a large game\n  -v, --version\n  -h, --help"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    result
}
