// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod game;
mod storage;
mod ui;

use bevy::prelude::*;
use bevy::settings::SettingsPlugin;
use bevy::window::{ExitCondition, WindowResolution};
use storage::{Difficulty, PersistentData, Settings};
use ui::{
    MinesApp, NativeWindowState, animate_ui_transitions, apply_window_settings, close_and_save,
    feathers_plugins, initialize_number_inputs, keyboard_shortcuts, mines_theme,
    native_ui_observers, native_window_sync, rebuild_ui, schedule_settings_save, setup_assets,
    startup_scene, sync_setting_widgets, update_clock_text, use_system_ui_font, window_focus_pause,
    window_resize_layout,
};

fn main() {
    let cli_difficulty = parse_cli();
    let persistent = PersistentData::load();
    let legacy_settings = persistent.settings.clone();

    let mut app = App::new();
    app.register_type::<Difficulty>()
        .register_type::<Settings>()
        .insert_resource(legacy_settings)
        .insert_resource(ClearColor(Color::srgb(0.965, 0.961, 0.949)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "扫雷".to_owned(),
                name: Some("org.gnome.Mines".to_owned()),
                resolution: WindowResolution::new(600, 600),
                resizable: true,
                ..default()
            }),
            exit_condition: ExitCondition::DontExit,
            ..default()
        }))
        .add_plugins(SettingsPlugin::new("org.gnome.Mines"))
        .add_plugins(feathers_plugins())
        .add_plugins(native_ui_observers)
        .insert_resource(mines_theme());

    let mut settings = app.world().resource::<Settings>().clone();
    let mines = MinesApp::new(persistent, cli_difficulty, &mut settings);
    app.insert_resource(settings)
        .insert_resource(mines)
        .init_resource::<NativeWindowState>()
        .init_resource::<ui::UiRuntime>()
        .init_resource::<ui::TouchHold>()
        .add_systems(
            Startup,
            (startup_scene.spawn(), setup_assets, apply_window_settings).chain(),
        )
        .add_systems(
            Update,
            (
                keyboard_shortcuts,
                window_focus_pause,
                close_and_save,
                native_window_sync,
                window_resize_layout,
                rebuild_ui,
                use_system_ui_font,
                animate_ui_transitions,
                initialize_number_inputs,
                sync_setting_widgets,
                update_clock_text,
                schedule_settings_save,
            )
                .chain(),
        )
        .run();
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
