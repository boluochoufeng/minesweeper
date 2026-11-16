// SPDX-License-Identifier: GPL-3.0-or-later
//
// Keyboard shortcuts for game flow and the keyboard cursor.

use crate::state::{
    CellAction, DialogKind, EscapeGuard, GameSession, PendingAction, Screen, resolve_saved_game,
    resume_from_slot,
};
use crate::storage::{SavedGameSlot, Scores, Settings, save_progress};
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

/// True when the platform primary shortcut modifier (`Ctrl` everywhere, plus
/// `Cmd` / `Super` on macOS) is currently held.
pub fn primary_modifier_pressed(keyboard: &ButtonInput<KeyCode>) -> bool {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    #[cfg(target_os = "macos")]
    {
        ctrl || keyboard.pressed(KeyCode::SuperLeft) || keyboard.pressed(KeyCode::SuperRight)
    }
    #[cfg(not(target_os = "macos"))]
    {
        ctrl
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
    mut saved_slot: ResMut<SavedGameSlot>,
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
    let primary_mod = primary_modifier_pressed(&keyboard);

    if primary_mod && (keyboard.just_pressed(KeyCode::KeyQ) || keyboard.just_pressed(KeyCode::KeyW))
    {
        let saved = resolve_saved_game(&session, &settings, &mut saved_slot);
        save_progress(&scores.0, saved);
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
        return;
    }
    if keyboard.just_pressed(KeyCode::F1) {
        session.open_dialog(DialogKind::Help);
        return;
    }
    if keyboard.just_pressed(KeyCode::F2) {
        session.open_dialog(DialogKind::Settings);
        return;
    }
    if keyboard.just_pressed(KeyCode::F3) {
        session.open_dialog(DialogKind::Scores);
        return;
    }
    if keyboard.just_pressed(KeyCode::F4) {
        session.open_dialog(DialogKind::About);
        return;
    }
    if keyboard.just_pressed(KeyCode::F5) {
        session.open_dialog(DialogKind::Shortcuts);
        return;
    }
    if keyboard.just_pressed(KeyCode::F6) {
        session.request(PendingAction::ShowCustomBoard, &mut settings, &mut next);
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
    if primary_mod && keyboard.just_pressed(KeyCode::KeyN) {
        session.request(PendingAction::ShowNewGame, &mut settings, &mut next);
    }
    if primary_mod && keyboard.just_pressed(KeyCode::KeyR) {
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
        // A real win freezes the clock before recording the score.
        if session
            .board
            .as_ref()
            .is_some_and(|board| board.outcome == crate::game::Outcome::Won)
        {
            session.clock.pause();
            session.sync_pause_state();
        }
        session.process_outcome(&mut scores);
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
        session.request(PendingAction::ShowCustomBoard, &mut settings, &mut next);
    }
    if *screen.get() == Screen::NewGame
        && focus.get().is_none()
        && (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter))
        && resume_from_slot(
            &mut session,
            &mut saved_slot,
            &mut settings,
            &scores,
            &mut next,
        )
    {
        return;
    }
    if session.dialog.is_some() {
        return;
    }
    if *screen.get() != Screen::Playing || session.paused {
        return;
    }

    let mut dx = 0i32;
    let mut dy = 0i32;
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyH) {
        dx -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyL) {
        dx += 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyK) {
        dy -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyJ) {
        dy += 1;
    }
    if dx != 0 || dy != 0 {
        session.move_cursor(dx, dy);
    }
    if keyboard.just_pressed(KeyCode::Space) {
        let Some((x, y, revealed)) = session.board.as_ref().map(|board| {
            let index = session.keyboard_cursor.min(board.width * board.height - 1);
            let (x, y) = board.coords(index);
            (x, y, board.cells[index].revealed)
        }) else {
            return;
        };
        if !session.keyboard_active {
            session.keyboard_active = true;
            return;
        }
        let action = if primary_mod {
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
