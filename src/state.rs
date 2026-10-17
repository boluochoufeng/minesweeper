// SPDX-License-Identifier: GPL-3.0-or-later
//
// Application state: which screen is active, the live game session and which
// dialog overlays it. All game-flow rules live here so the UI modules stay
// declarative.

use crate::game::{Board, GameClock, Outcome};
use crate::storage::{Difficulty, Scores, Settings};
use bevy::prelude::*;

/// Exclusive screens of the application.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    NewGame,
    CustomBoard,
    Playing,
}

/// A deferred action that may need confirmation when progress would be lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PendingAction {
    #[default]
    ShowNewGame,
    Start(Difficulty),
    Restart,
}

/// Snapshot of the pause state around a confirmation dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmState {
    pub action: PendingAction,
    pub was_paused: bool,
    pub was_pause_requested: bool,
}

/// Dialog overlays currently supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    Scores,
    Settings,
    Shortcuts,
    Help,
    About,
    Victory,
    Confirm(ConfirmState),
}

/// One cell interaction, resolved from pointer or keyboard input.
#[derive(Debug, Clone, Copy)]
pub enum CellAction {
    Reveal(usize, usize),
    Mark(usize, usize),
    Chord(usize, usize),
}

/// The live game: board, clock, pause state and the keyboard cursor.
#[derive(Resource, Default)]
pub struct GameSession {
    pub board: Option<Board>,
    pub clock: GameClock,
    pub paused: bool,
    pub pause_requested: bool,
    pub auto_paused: bool,
    pub keyboard_cursor: usize,
    pub keyboard_active: bool,
    pub dialog: Option<DialogKind>,
    pub help_page: usize,
}

impl GameSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a fresh game on the given difficulty and switch to the board.
    pub fn start_game(
        &mut self,
        difficulty: Difficulty,
        settings: &mut Settings,
        next: &mut NextState<Screen>,
    ) {
        settings.difficulty = difficulty;
        let (width, height, mines) = difficulty.dimensions(settings);
        self.board = Some(Board::new(width, height, mines));
        self.clock = GameClock::default();
        self.paused = false;
        self.pause_requested = false;
        self.auto_paused = false;
        self.keyboard_cursor = 0;
        self.keyboard_active = false;
        self.dialog = None;
        self.help_page = 0;
        next.set(Screen::Playing);
    }

    /// Go back to the difficulty picker.
    pub fn show_new_game(&mut self, next: &mut NextState<Screen>) {
        self.board = None;
        self.clock = GameClock::default();
        self.paused = false;
        self.pause_requested = false;
        self.auto_paused = false;
        self.keyboard_cursor = 0;
        self.keyboard_active = false;
        self.dialog = None;
        self.help_page = 0;
        next.set(Screen::NewGame);
    }

    /// Show the custom board editor.
    pub fn show_custom_board(&mut self, next: &mut NextState<Screen>) {
        self.board = None;
        self.clock = GameClock::default();
        self.paused = false;
        self.pause_requested = false;
        self.dialog = None;
        next.set(Screen::CustomBoard);
    }

    fn execute(
        &mut self,
        action: PendingAction,
        settings: &mut Settings,
        next: &mut NextState<Screen>,
    ) {
        match action {
            PendingAction::ShowNewGame => self.show_new_game(next),
            PendingAction::Start(difficulty) => self.start_game(difficulty, settings, next),
            PendingAction::Restart => self.start_game(settings.difficulty, settings, next),
        }
    }

    /// Request an action, asking for confirmation when progress would be lost.
    pub fn request(
        &mut self,
        action: PendingAction,
        settings: &mut Settings,
        next: &mut NextState<Screen>,
    ) {
        let needs_confirmation = self
            .board
            .as_ref()
            .is_some_and(|board| board.revealed_count > 0 && board.outcome == Outcome::InProgress);
        if needs_confirmation {
            let state = ConfirmState {
                action,
                was_paused: self.paused,
                was_pause_requested: self.pause_requested,
            };
            self.set_paused(true, false);
            self.dialog = Some(DialogKind::Confirm(state));
        } else {
            self.execute(action, settings, next);
        }
    }

    pub fn set_paused(&mut self, paused: bool, requested: bool) {
        if self
            .board
            .as_ref()
            .is_none_or(|board| board.outcome != Outcome::InProgress)
        {
            return;
        }
        self.paused = paused;
        if requested {
            self.pause_requested = paused;
        }
        if paused {
            self.clock.pause();
        } else {
            self.clock.resume();
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.clock.has_started() {
            self.set_paused(!self.paused, true);
        }
    }

    /// Close the current dialog. Confirmation dialogs restore the pause state
    /// they interrupted.
    pub fn close_dialog(&mut self) {
        if let Some(DialogKind::Confirm(state)) = self.dialog.take() {
            self.paused = state.was_paused;
            self.pause_requested = state.was_pause_requested;
            if self.paused {
                self.clock.pause();
            } else {
                self.clock.resume();
            }
        }
    }

    /// Accept the pending action of the open confirmation dialog.
    pub fn accept_confirm(&mut self, settings: &mut Settings, next: &mut NextState<Screen>) {
        if let Some(DialogKind::Confirm(state)) = self.dialog.take() {
            self.execute(state.action, settings, next);
        }
    }

    /// Apply one cell interaction. Returns `true` when the board changed.
    pub fn apply_cell_action(&mut self, action: CellAction, settings: &Settings) -> bool {
        if self.paused {
            return false;
        }
        let Some(board) = self.board.as_mut() else {
            return false;
        };
        let changed = match action {
            CellAction::Reveal(x, y) => {
                let changed = board.reveal(
                    x,
                    y,
                    &mut rand::rng(),
                    settings.auto_clear_on_flags,
                    settings.use_autoflag,
                );
                if changed && !self.clock.has_started() {
                    self.clock.start();
                }
                changed
            }
            CellAction::Mark(x, y) => board.cycle_mark(
                x,
                y,
                settings.use_question_marks,
                settings.auto_clear_on_flags,
                settings.use_autoflag,
            ),
            CellAction::Chord(x, y) => {
                board.chord(x, y, settings.auto_clear_on_flags, settings.use_autoflag)
            }
        };
        if changed {
            match self.board.as_ref().map(|board| board.outcome) {
                Some(Outcome::Lost) => {
                    self.clock.pause();
                }
                Some(Outcome::Won) => {
                    self.clock.pause();
                }
                _ => {}
            }
        }
        changed
    }

    /// Register the finished game in the score table and open the victory
    /// dialog. Returns `true` when the game was won in this call.
    pub fn process_outcome(&mut self, scores: &mut Scores) -> bool {
        let Some(board) = self.board.as_ref() else {
            return false;
        };
        if board.outcome != Outcome::Won || matches!(self.dialog, Some(DialogKind::Victory)) {
            return false;
        }
        let seconds = self.clock.elapsed().as_secs_f64().round() as u64;
        scores.add(board.width, board.height, board.mine_count, seconds);
        self.dialog = Some(DialogKind::Victory);
        true
    }

    /// Board progress in per-mille, used by the status bar.
    pub fn progress(&self) -> f32 {
        let Some(board) = self.board.as_ref() else {
            return 0.0;
        };
        let total = board.width * board.height;
        if total == 0 {
            return 0.0;
        }
        let safe = total.saturating_sub(board.mine_count);
        if safe == 0 {
            return 0.0;
        }
        (board.revealed_count.min(safe) as f32) / (safe as f32)
    }

    pub fn mines_left(&self) -> i64 {
        let Some(board) = self.board.as_ref() else {
            return 0;
        };
        board.mine_count as i64 - board.flag_count as i64
    }
}
