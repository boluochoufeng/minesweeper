// SPDX-License-Identifier: GPL-3.0-or-later
//
// The Mines UI, split into small plugins. The shell owns app-wide concerns
// (scaling, fonts, window state, persistence); each screen and dialog module
// owns its own scene and interactions.

pub mod actions;
pub mod animation;
pub mod board;
pub mod custom_board;
pub mod dialogs;
pub mod menu;
pub mod new_game;
pub mod scores;
pub mod settings_dialog;
pub mod shell;
pub mod status;
pub mod style;

use bevy::prelude::*;

pub struct MinesUiPlugin;

impl Plugin for MinesUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            actions::ActionsPlugin,
            animation::AnimationPlugin,
            shell::ShellPlugin,
            new_game::NewGameScreenPlugin,
            custom_board::CustomBoardScreenPlugin,
            board::BoardScreenPlugin,
            status::StatusPlugin,
            dialogs::DialogsPlugin,
            settings_dialog::SettingsDialogPlugin,
        ));
    }
}
