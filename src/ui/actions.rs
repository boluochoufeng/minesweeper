// SPDX-License-Identifier: GPL-3.0-or-later
//
// Declarative UI commands. Widgets carry an `ActionTarget`; one observer handles
// all of them so screens and dialogs stay free of game-flow logic.

use crate::state::{
    DialogKind, GameSession, PendingAction, Screen, resolve_saved_game, resume_from_slot,
};
use crate::storage::{SavedGameSlot, Scores, Settings, save_progress};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::settings::SaveSettingsSync;
use bevy::ui_widgets::Activate;

/// What a widget does when activated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UiAction {
    #[default]
    None,
    /// Request a deferred action (may ask for confirmation).
    Pending(PendingAction),
    /// Resume the saved in-progress game from disk/slot.
    ResumeSavedGame,
    /// Pause or resume the clock.
    TogglePause,
    /// Open a dialog overlay.
    Open(DialogKind),
    /// Close the current dialog.
    CloseDialog,
    /// Accept the pending confirmation.
    AcceptConfirm,
    /// Show the previous help page.
    HelpPrevious,
    /// Show the next help page.
    HelpNext,
    /// Quit the application.
    Quit,
}

/// Component attached to any widget that emits a [`UiAction`].
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActionTarget {
    pub action: UiAction,
}

pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_ui_action);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_ui_action(
    activate: On<Activate>,
    targets: Query<&ActionTarget>,
    mut session: ResMut<GameSession>,
    mut settings: ResMut<Settings>,
    scores: Res<Scores>,
    mut saved_slot: ResMut<SavedGameSlot>,
    mut next: ResMut<NextState<Screen>>,
    mut commands: Commands,
) {
    let Ok(target) = targets.get(activate.event_target()) else {
        return;
    };
    match target.action {
        UiAction::None => {}
        UiAction::Pending(action) => session.request(action, &mut settings, &mut next),
        UiAction::ResumeSavedGame => {
            resume_from_slot(
                &mut session,
                &mut saved_slot,
                &mut settings,
                &scores,
                &mut next,
            );
        }
        UiAction::TogglePause => session.toggle_pause(),
        UiAction::Open(kind) => session.open_dialog(kind),
        UiAction::CloseDialog => session.close_dialog(),
        UiAction::AcceptConfirm => {
            session.accept_confirm(&mut settings, &mut next);
            if saved_slot.0.take().is_some() {
                save_progress(&scores.0, None);
            }
        }
        UiAction::HelpPrevious => {
            session.help_page = session.help_page.saturating_sub(1);
        }
        UiAction::HelpNext => {
            session.help_page =
                (session.help_page + 1).min(crate::ui::dialogs::HELP_PAGE_COUNT - 1);
        }
        UiAction::Quit => {
            let saved = resolve_saved_game(&session, &settings, &mut saved_slot);
            save_progress(&scores.0, saved);
            commands.queue(SaveSettingsSync::IfChanged);
            commands.write_message(AppExit::Success);
        }
    }
}
