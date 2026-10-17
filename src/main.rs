// SPDX-License-Identifier: GPL-3.0-or-later

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

use assets::AssetsPlugin;
use bevy::feathers::FeathersPlugins;
use bevy::prelude::*;
use bevy::settings::{SaveSettingsSync, SettingsPlugin};
use bevy::window::{ExitCondition, WindowResolution};
use input::InputPlugin;
use state::{GameSession, Screen};
use storage::{Difficulty, PersistentData, Scores, Settings};
use theme::ThemePlugin;
use ui::MinesUiPlugin;

/// Difficulty requested on the command line, applied after startup.
#[derive(Resource, Default)]
struct CliDifficulty(Option<Difficulty>);

fn main() {
    let cli_difficulty = parse_cli();

    let mut app = App::new();
    // `DefaultPlugins` installs the log plugin, so the save file is read only
    // after this point: warnings about an unreadable or corrupt `data.json`
    // then reach the log instead of being silently dropped.
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "扫雷".to_owned(),
            name: Some("org.gnome.Mines".to_owned()),
            resolution: WindowResolution::new(720, 760),
            resizable: true,
            ..default()
        }),
        exit_condition: ExitCondition::DontExit,
        ..default()
    }));

    let mut persistent = PersistentData::load();
    storage::sanitize_settings(&mut persistent.settings);
    // `SettingsPlugin` loads `settings.toml` during `add_plugins` and leaves an
    // already-inserted `Settings` resource untouched when the file is missing.
    // The legacy `data.json` snapshot is therefore used as the first-run value
    // and is written out to TOML once the app is running.
    let migrate_settings = storage::needs_settings_migration();

    app.register_type::<Difficulty>()
        .register_type::<Settings>()
        .insert_resource(persistent.settings.clone())
        .insert_resource(Scores(persistent.scores.clone()))
        .insert_resource(CliDifficulty(cli_difficulty))
        .add_plugins(SettingsPlugin::new("org.gnome.Mines"))
        .add_plugins(FeathersPlugins)
        .init_state::<Screen>()
        .init_resource::<state::GameSession>()
        .init_resource::<state::EscapeGuard>()
        .add_plugins((AssetsPlugin, ThemePlugin, MinesUiPlugin, InputPlugin))
        .add_systems(Startup, (spawn_camera, apply_cli_difficulty).chain());

    if migrate_settings {
        info!("Migrating settings from data.json to settings.toml");
        app.add_systems(Startup, persist_migrated_settings);
    }

    // Opt-in diagnostics: `MINES_PERF=1` logs FPS and frame time once per
    // second, which makes release-build UI cost easy to measure.
    if std::env::var_os("MINES_PERF").is_some() {
        info!("Performance diagnostics enabled (MINES_PERF)");
        app.add_plugins((
            bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
            bevy::diagnostic::LogDiagnosticsPlugin::default(),
        ));
    }

    app.run();
}

/// Write the migrated settings to `settings.toml` exactly once, on the first
/// launch that finds a legacy `data.json` but no modern settings file.
fn persist_migrated_settings(mut commands: Commands) {
    commands.queue(SaveSettingsSync::Always);
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
