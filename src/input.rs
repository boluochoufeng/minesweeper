// SPDX-License-Identifier: GPL-3.0-or-later
//
// Keyboard shortcuts for game flow and the keyboard cursor.

use crate::state::{CellAction, DialogKind, EscapeGuard, GameSession, PendingAction, Screen};
use crate::storage::{Scores, Settings, save_progress};
use bevy::app::AppExit;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::settings::SaveSettingsSync;
use bevy::text::EditableText;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, keyboard_shortcuts);
    }
}

#[allow(clippy::too_many_arguments)]
fn keyboard_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    editable: Query<(), With<EditableText>>,
    screen: Res<State<Screen>>,
    mut session: ResMut<GameSession>,
    mut settings: ResMut<Settings>,
    mut scores: ResMut<Scores>,
    mut next: ResMut<NextState<Screen>>,
    mut commands: Commands,
    mut escape_guard: ResMut<EscapeGuard>,
) {
    if focus.get().is_some_and(|entity| editable.contains(entity)) {
        return;
    }
    // Open dialogs own their input. Escape mirrors the modal barrier's close
    // gesture so dialogs can always be dismissed from the keyboard.
    if session.dialog.is_some() {
        if keyboard.just_pressed(KeyCode::Escape) {
            escape_guard.0 = 3;
            session.close_dialog();
        }
        return;
    }
    if escape_guard.0 > 0 {
        escape_guard.0 -= 1;
    }
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    if ctrl && (keyboard.just_pressed(KeyCode::KeyQ) || keyboard.just_pressed(KeyCode::KeyW)) {
        save_progress(&settings, &scores.0);
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
        return;
    }
    if keyboard.just_pressed(KeyCode::F1) {
        session.dialog = Some(DialogKind::Help);
        return;
    }
    if keyboard.just_pressed(KeyCode::F2) {
        session.dialog = Some(DialogKind::Settings);
        return;
    }
    if keyboard.just_pressed(KeyCode::F3) {
        session.dialog = Some(DialogKind::Scores);
        return;
    }
    if keyboard.just_pressed(KeyCode::F4) {
        session.dialog = Some(DialogKind::About);
        return;
    }
    if keyboard.just_pressed(KeyCode::F5) {
        session.dialog = Some(DialogKind::Shortcuts);
        return;
    }
    if keyboard.just_pressed(KeyCode::F6) {
        session.show_custom_board(&mut next);
        return;
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        if escape_guard.0 > 0 {
            escape_guard.0 = 0;
            return;
        }
        match *screen.get() {
            // Escape plays the pause toggle role on the board; starting a new
            // game stays behind Ctrl+N so a stray key cannot discard progress.
            Screen::Playing => session.toggle_pause(),
            Screen::CustomBoard => session.show_new_game(&mut next),
            Screen::NewGame => {}
        }
        return;
    }
    if ctrl && keyboard.just_pressed(KeyCode::KeyN) {
        session.request(PendingAction::ShowNewGame, &mut settings, &mut next);
    }
    if ctrl && keyboard.just_pressed(KeyCode::KeyR) {
        session.request(PendingAction::Restart, &mut settings, &mut next);
    }
    if keyboard.just_pressed(KeyCode::Pause) || keyboard.just_pressed(KeyCode::KeyP) {
        session.toggle_pause();
    }
    // Development helper: reveal every safe cell so the victory flow can be
    // exercised without playing a whole board. `cfg!(...)` would still compile
    // the body in release builds, where the method does not exist.
    #[cfg(debug_assertions)]
    if keyboard.just_pressed(KeyCode::F9) {
        if let Some(board) = session.board.as_mut() {
            board.debug_reveal_all();
        }
        session.process_outcome(&mut scores);
        // A real win freezes the clock; keep the shortcut faithful to it.
        if session
            .board
            .as_ref()
            .is_some_and(|board| board.outcome == crate::game::Outcome::Won)
        {
            session.clock.pause();
        }
        return;
    }
    for (key, difficulty) in [
        (KeyCode::Digit1, crate::storage::Difficulty::Small),
        (KeyCode::Digit2, crate::storage::Difficulty::Medium),
        (KeyCode::Digit3, crate::storage::Difficulty::Large),
    ] {
        if keyboard.just_pressed(key) {
            session.request(PendingAction::Start(difficulty), &mut settings, &mut next);
        }
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        session.show_custom_board(&mut next);
    }
    if session.dialog.is_some() {
        return;
    }
    if *screen.get() != Screen::Playing || session.paused {
        return;
    }

    // Cache the board geometry, then release the immutable borrow.
    let (width, height, mut x, mut y) = {
        let Some(board) = session.board.as_ref() else {
            return;
        };
        let index = session.keyboard_cursor.min(board.width * board.height - 1);
        let (x, y) = board.coords(index);
        (board.width, board.height, x, y)
    };
    let old = (x, y);
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyH) {
        x = x.saturating_sub(1);
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyL) {
        x = (x + 1).min(width - 1);
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyK) {
        y = y.saturating_sub(1);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyJ) {
        y = (y + 1).min(height - 1);
    }
    if (x, y) != old {
        session.keyboard_cursor = y * width + x;
        session.keyboard_active = true;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        session.keyboard_active = true;
        let revealed = session
            .board
            .as_ref()
            .is_some_and(|board| board.cells[y * width + x].revealed);
        let action = if ctrl {
            CellAction::Mark(x, y)
        } else if revealed {
            CellAction::Chord(x, y)
        } else {
            CellAction::Reveal(x, y)
        };
        session.apply_cell_action(action, &settings);
        session.process_outcome(&mut scores);
    }
}


