// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod game;
mod storage;
mod ui;

use bevy::prelude::*;
use bevy::window::{ExitCondition, WindowResolution};
use bevy_egui::EguiPlugin;
use storage::{Difficulty, PersistentData};
use ui::{MinesApp, NativeWindowState, mines_ui, native_window_sync, window_focus_pause};

fn main() {
    let cli_difficulty = parse_cli();
    let persistent = PersistentData::load();
    let width = persistent.settings.window_width.max(480.0);
    let height = persistent.settings.window_height.max(400.0);

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.965, 0.961, 0.949)))
        .insert_resource(MinesApp::new(persistent, cli_difficulty))
        .init_resource::<NativeWindowState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Mines".to_owned(),
                name: Some("org.gnome.Mines".to_owned()),
                resolution: WindowResolution::new(width as u32, height as u32),
                resizable: true,
                ..default()
            }),
            exit_condition: ExitCondition::OnPrimaryClosed,
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(bevy_egui::EguiPrimaryContextPass, mines_ui)
        .add_systems(Update, (window_focus_pause, native_window_sync))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
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
