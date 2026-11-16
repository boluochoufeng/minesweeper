// SPDX-License-Identifier: GPL-3.0-or-later
//
// Application state: which screen is active, the live game session and which
// dialog overlays it. All game-flow rules live here so the UI modules stay
// declarative.

use crate::game::{Board, GameClock, Outcome};
use crate::storage::{Difficulty, SavedGame, SavedGameSlot, Scores, Settings};
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
    ShowCustomBoard,
    Start(Difficulty),
    Restart,
}

/// Details of a confirmation dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmState {
    pub action: PendingAction,
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

/// Prevents the Escape key that closed a dialog from also reaching the game
/// shortcuts in the same frame.
#[derive(Resource, Default)]
pub struct EscapeGuard(pub u8);

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
    /// Start a fresh game on the given difficulty and switch to the board.
    pub fn start_game(
        &mut self,
        difficulty: Difficulty,
        settings: &mut Settings,
        next: &mut NextState<Screen>,
    ) {
        settings.difficulty = difficulty;
        let (width, height, mines) = difficulty.dimensions(settings);
        let mut board = Board::new(width, height, mines);
        board.no_guessing = settings.no_guessing;
        self.board = Some(board);
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

    /// Restore a previously saved in-progress game and switch to the playing
    /// screen. Returns `false` if the snapshot failed validation.
    pub fn resume_saved(
        &mut self,
        saved: SavedGame,
        settings: &mut Settings,
        next: &mut NextState<Screen>,
    ) -> bool {
        let Some((difficulty, board, elapsed)) = saved.into_restored() else {
            return false;
        };
        settings.difficulty = difficulty;
        if difficulty == Difficulty::Custom {
            settings.custom_width = board.width;
            settings.custom_height = board.height;
            settings.custom_mines = board.mine_count;
        }
        self.board = Some(board);
        self.clock = GameClock::from_elapsed(elapsed);
        self.paused = false;
        self.pause_requested = false;
        self.auto_paused = false;
        self.keyboard_cursor = 0;
        self.keyboard_active = false;
        self.dialog = None;
        self.help_page = 0;
        self.sync_pause_state();
        next.set(Screen::Playing);
        true
    }

    /// Capture a serializable snapshot when an active, already-placed board
    /// is still in progress.
    pub fn snapshot_saved_game(&self, settings: &Settings) -> Option<SavedGame> {
        let board = self.board.as_ref()?;
        SavedGame::from_board(settings.difficulty, board, self.clock.elapsed())
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
        self.auto_paused = false;
        self.keyboard_cursor = 0;
        self.keyboard_active = false;
        self.dialog = None;
        self.help_page = 0;
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
            PendingAction::ShowCustomBoard => self.show_custom_board(next),
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
            self.open_dialog(DialogKind::Confirm(ConfirmState { action }));
        } else {
            self.execute(action, settings, next);
        }
    }

    /// Reconcile `paused` and the game clock from manual pause, window focus
    /// loss and modal dialog state.
    pub fn sync_pause_state(&mut self) {
        let in_progress = self
            .board
            .as_ref()
            .is_some_and(|board| board.outcome == Outcome::InProgress);
        if !in_progress {
            self.paused = false;
            return;
        }
        let should_pause = self.clock.has_started()
            && (self.pause_requested || self.auto_paused || self.dialog.is_some());
        self.paused = should_pause;
        if should_pause {
            if self.clock.is_running() {
                self.clock.pause();
            }
        } else if !self.clock.is_running() {
            self.clock.resume();
        }
    }

    pub fn toggle_pause(&mut self) {
        let in_progress = self
            .board
            .as_ref()
            .is_some_and(|board| board.outcome == Outcome::InProgress);
        if in_progress && self.clock.has_started() {
            let next_paused = !self.paused;
            self.pause_requested = next_paused;
            if !next_paused {
                self.auto_paused = false;
            }
            self.sync_pause_state();
        }
    }

    /// Update automatic pause state when the primary window gains or loses focus.
    pub fn set_window_focused(&mut self, focused: bool) {
        if focused {
            if self.auto_paused {
                self.auto_paused = false;
                self.sync_pause_state();
            }
        } else {
            let in_progress = self
                .board
                .as_ref()
                .is_some_and(|board| board.outcome == Outcome::InProgress);
            if in_progress && self.clock.has_started() {
                self.auto_paused = true;
                self.sync_pause_state();
            }
        }
    }

    /// Open a modal dialog overlay, automatically pausing an active game clock.
    pub fn open_dialog(&mut self, kind: DialogKind) {
        if matches!(kind, DialogKind::Help) {
            self.help_page = 0;
        }
        self.dialog = Some(kind);
        self.sync_pause_state();
    }

    /// Close the current dialog and resume the clock if nothing else holds it paused.
    pub fn close_dialog(&mut self) {
        if self.dialog.take().is_some() {
            self.sync_pause_state();
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
        if self.paused || self.dialog.is_some() {
            return false;
        }
        let Some(board) = self.board.as_mut() else {
            return false;
        };
        let changed = match action {
            CellAction::Reveal(x, y) => {
                if !board.placed {
                    board.no_guessing = settings.no_guessing;
                }
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
        if changed
            && matches!(
                self.board.as_ref().map(|board| board.outcome),
                Some(Outcome::Lost | Outcome::Won)
            )
        {
            self.clock.pause();
            self.pause_requested = false;
            self.auto_paused = false;
            self.paused = false;
        }
        changed
    }

    /// Whole seconds elapsed on the current game clock, used consistently across
    /// the status bar, victory summary and high-score records.
    pub fn elapsed_secs(&self) -> u64 {
        self.clock.elapsed().as_secs()
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
        let (width, height, mines) = (board.width, board.height, board.mine_count);
        let seconds = self.elapsed_secs();
        scores.add(width, height, mines, seconds);
        self.open_dialog(DialogKind::Victory);
        true
    }

    pub fn mines_left(&self) -> i64 {
        let Some(board) = self.board.as_ref() else {
            return 0;
        };
        board.mine_count as i64 - board.flag_count as i64
    }

    /// Activate or move the keyboard cursor with 4-edge modulo wrap.
    /// The first directional press reveals the cursor in place so the player can
    /// see where focus is before moving or revealing.
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let Some(board) = self.board.as_ref() else {
            return;
        };
        if board.outcome != Outcome::InProgress {
            return;
        }
        let total = board.width * board.height;
        let current = self.keyboard_cursor.min(total.saturating_sub(1));
        self.keyboard_cursor = current;
        if !self.keyboard_active {
            self.keyboard_active = true;
            return;
        }
        let (x, y) = board.coords(current);
        let nx = (x as i32 + dx).rem_euclid(board.width as i32) as usize;
        let ny = (y as i32 + dy).rem_euclid(board.height as i32) as usize;
        self.keyboard_cursor = board.index(nx, ny);
    }
}

/// Synchronize `slot` with the active session (snapshot an in-progress placed
/// board, clear when finished, or preserve an untouched menu slot) and return
/// the value to persist.
pub fn resolve_saved_game<'a>(
    session: &GameSession,
    settings: &Settings,
    slot: &'a mut SavedGameSlot,
) -> Option<&'a SavedGame> {
    if let Some(board) = session.board.as_ref()
        && board.placed
    {
        if board.outcome == Outcome::InProgress && board.revealed_count > 0 {
            slot.0 = session.snapshot_saved_game(settings);
        } else {
            slot.0 = None;
        }
    }
    slot.0.as_ref()
}

/// Consume the stored snapshot from `slot`, resume into `session`, and clear
/// the on-disk saved slot so a resumed game won't linger if discarded.
pub fn resume_from_slot(
    session: &mut GameSession,
    slot: &mut SavedGameSlot,
    settings: &mut Settings,
    scores: &Scores,
    next: &mut NextState<Screen>,
) -> bool {
    let Some(saved) = slot.0.take() else {
        return false;
    };
    let resumed = session.resume_saved(saved, settings, next);
    crate::storage::save_progress(&scores.0, None);
    resumed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_cursor_activates_in_place_then_wraps_edges() {
        let mut session = GameSession {
            board: Some(Board::new(8, 8, 10)),
            keyboard_cursor: 0,
            keyboard_active: false,
            ..Default::default()
        };

        // First directional key lights up (0, 0) without jumping away.
        session.move_cursor(-1, 0);
        assert!(session.keyboard_active);
        assert_eq!(session.keyboard_cursor, 0);

        // Moving left from x=0 wraps to x=7; moving up from y=0 wraps to y=7.
        session.move_cursor(-1, 0);
        assert_eq!(session.keyboard_cursor, 7);
        session.move_cursor(0, -1);
        assert_eq!(session.keyboard_cursor, 7 * 8 + 7);

        // Moving right/down from (7, 7) wraps back to (0, 0).
        session.move_cursor(1, 1);
        assert_eq!(session.keyboard_cursor, 0);
    }

    #[test]
    fn request_guards_in_progress_board_and_executes_after_confirmation() {
        let mut session = GameSession::default();
        let mut settings = Settings::default();
        let mut next = NextState::<Screen>::default();

        // Starting when no cells are revealed executes immediately without a dialog.
        session.request(
            PendingAction::Start(Difficulty::Small),
            &mut settings,
            &mut next,
        );
        assert!(session.dialog.is_none());
        assert!(matches!(next, NextState::Pending(Screen::Playing)));

        // Reveal the first cell so the game is actively in progress.
        next = NextState::Unchanged;
        assert!(session.apply_cell_action(CellAction::Reveal(0, 0), &settings));
        assert!(session.clock.is_running());

        // Switching to CustomBoard mid-game opens a confirmation modal and pauses the clock.
        session.request(PendingAction::ShowCustomBoard, &mut settings, &mut next);
        assert_eq!(
            session.dialog,
            Some(DialogKind::Confirm(ConfirmState {
                action: PendingAction::ShowCustomBoard,
            }))
        );
        assert!(session.paused);
        assert!(!session.clock.is_running());
        assert!(matches!(next, NextState::Unchanged));

        // Cancelling the dialog resumes the clock and keeps the board intact.
        session.close_dialog();
        assert!(session.dialog.is_none());
        assert!(!session.paused);
        assert!(session.clock.is_running());
        assert!(session.board.is_some());

        // Requesting again and accepting executes the pending navigation.
        session.request(PendingAction::ShowCustomBoard, &mut settings, &mut next);
        session.accept_confirm(&mut settings, &mut next);
        assert!(session.dialog.is_none());
        assert!(session.board.is_none());
        assert!(matches!(next, NextState::Pending(Screen::CustomBoard)));
    }

    #[test]
    fn pause_reconciles_manual_focus_and_modal_overlays() {
        let mut session = GameSession::default();
        let mut settings = Settings::default();
        let mut next = NextState::<Screen>::default();
        session.start_game(Difficulty::Small, &mut settings, &mut next);

        // Before the first reveal, pause/focus toggles do not enter paused mode.
        session.toggle_pause();
        assert!(!session.paused);
        session.set_window_focused(false);
        assert!(!session.paused);
        session.set_window_focused(true);

        // Start the clock with the first reveal.
        assert!(session.apply_cell_action(CellAction::Reveal(2, 2), &settings));
        assert!(session.clock.is_running());

        // Opening a modal pauses the clock; cycling focus while the modal stays open
        // must NOT prematurely resume the clock until the modal closes.
        session.open_dialog(DialogKind::Settings);
        assert!(session.paused);
        assert!(!session.clock.is_running());
        session.set_window_focused(false);
        session.set_window_focused(true);
        assert!(session.paused);
        assert!(!session.clock.is_running());

        session.close_dialog();
        assert!(!session.paused);
        assert!(session.clock.is_running());

        // Manual pause survives window blur + refocus until explicitly unpaused.
        session.toggle_pause();
        assert!(session.paused);
        session.set_window_focused(false);
        session.set_window_focused(true);
        assert!(session.paused);
        session.toggle_pause();
        assert!(!session.paused);
        assert!(session.clock.is_running());
    }

    #[test]
    fn process_outcome_records_score_once_and_stops_clock() {
        let mut session = GameSession::default();
        let mut settings = Settings {
            use_question_marks: false,
            ..Settings::default()
        };
        let mut next = NextState::<Screen>::default();
        let mut scores = Scores::default();
        session.start_game(Difficulty::Small, &mut settings, &mut next);
        assert!(session.apply_cell_action(CellAction::Reveal(0, 0), &settings));

        // Over-flagging is reflected as a negative remaining mine count when question marks are off.
        for idx in 0..8 {
            let x = idx % 8;
            let y = 2 + idx / 8;
            let _ = session.apply_cell_action(CellAction::Mark(x, y), &settings);
        }
        assert_eq!(
            session.mines_left(),
            10 - session.board.as_ref().unwrap().flag_count as i64
        );

        // Reveal all safe cells to win, freeze the clock, and settle the outcome.
        session.board.as_mut().unwrap().debug_reveal_all();
        session.clock.pause();
        session.sync_pause_state();
        assert!(session.process_outcome(&mut scores));
        assert_eq!(session.dialog, Some(DialogKind::Victory));
        assert_eq!(scores.0.len(), 1);
        assert_eq!(scores.0[0].seconds, session.elapsed_secs());

        // Calling `process_outcome` again while Victory is open is idempotent.
        assert!(!session.process_outcome(&mut scores));
        assert_eq!(scores.0.len(), 1);
    }

    #[test]
    fn snapshot_and_resume_restores_in_progress_session() {
        let mut session = GameSession::default();
        let mut settings = Settings::default();
        let mut next = NextState::<Screen>::default();

        session.start_game(Difficulty::Medium, &mut settings, &mut next);
        assert!(
            session.snapshot_saved_game(&settings).is_none(),
            "unplaced board must not produce a saved snapshot"
        );

        assert!(session.apply_cell_action(CellAction::Reveal(0, 0), &settings));
        let board = session.board.as_ref().unwrap();
        let hidden_idx = board.cells.iter().position(|c| !c.revealed).unwrap();
        let (hx, hy) = board.coords(hidden_idx);
        assert!(session.apply_cell_action(CellAction::Mark(hx, hy), &settings));
        let expected_left = session.mines_left();

        let saved = session
            .snapshot_saved_game(&settings)
            .expect("active board must snapshot cleanly");

        let mut resumed = GameSession::default();
        let mut resumed_settings = Settings::default();
        let mut resumed_next = NextState::<Screen>::default();
        assert!(resumed.resume_saved(saved, &mut resumed_settings, &mut resumed_next));
        assert!(matches!(resumed_next, NextState::Pending(Screen::Playing)));
        assert_eq!(resumed_settings.difficulty, Difficulty::Medium);
        assert!(resumed.clock.is_running());
        assert_eq!(resumed.mines_left(), expected_left);
        assert!(resumed.snapshot_saved_game(&resumed_settings).is_some());

        resumed.board.as_mut().unwrap().debug_reveal_all();
        assert!(
            resumed.snapshot_saved_game(&resumed_settings).is_none(),
            "finished board must not be saved as resumable"
        );
    }
}
