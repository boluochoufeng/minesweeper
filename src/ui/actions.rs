// SPDX-License-Identifier: GPL-3.0-or-later
//
// Declarative UI commands. Widgets carry an `ActionTarget`; one observer handles
// all of them so screens and dialogs stay free of game-flow logic.

use crate::state::{DialogKind, GameSession, PendingAction, Screen};
use crate::storage::{save_progress, Scores, Settings};
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
    /// Open the custom board editor.
    ShowCustom,
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

fn handle_ui_action(
    activate: On<Activate>,
    targets: Query<&ActionTarget>,
    mut session: ResMut<GameSession>,
    mut settings: ResMut<Settings>,
    scores: Res<Scores>,
    mut next: ResMut<NextState<Screen>>,
    mut commands: Commands,
) {
    let Ok(target) = targets.get(activate.event_target()) else {
        return;
    };
    match target.action {
        UiAction::None => {}
        UiAction::Pending(action) => session.request(action, &mut settings, &mut next),
        UiAction::ShowCustom => session.show_custom_board(&mut next),
        UiAction::TogglePause => session.toggle_pause(),
        UiAction::Open(kind) => {
            if matches!(kind, DialogKind::Victory) && session.dialog.is_none() {
                // Victory dialogs are opened by the outcome system, not by widgets.
            }
            session.dialog = Some(kind);
        }
        UiAction::CloseDialog => session.close_dialog(),
        UiAction::AcceptConfirm => session.accept_confirm(&mut settings, &mut next),
        UiAction::HelpPrevious => {
            session.help_page = session.help_page.saturating_sub(1);
        }
        UiAction::HelpNext => {
            session.help_page = (session.help_page + 1).min(crate::ui::dialogs::HELP_PAGE_COUNT - 1);
        }
        UiAction::Quit => {
            save_progress(&settings, &scores.0);
            commands.queue(SaveSettingsSync::IfChanged);
            commands.write_message(AppExit::Success);
        }
    }
}
