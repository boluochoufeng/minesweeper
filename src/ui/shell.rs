// SPDX-License-Identifier: GPL-3.0-or-later
//
// App-wide UI concerns: UI scale/fonts, window state and persistence.

use crate::game::Outcome;
use crate::state::{EscapeGuard, GameSession, Screen, resolve_saved_game};
use crate::storage::{
    MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, SavedGameSlot, Scores, Settings, save_progress,
};
use crate::theme::{ThemePalette, ui_font};
use crate::ui::animation::UiTransition;
use crate::ui::board::BoardPointerState;
use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::input::touch::TouchInput;
use bevy::prelude::*;
use bevy::settings::{SaveSettingsDeferred, SaveSettingsSync};
use bevy::text::{FontSource, RemSize};
use bevy::time::Real;
use bevy::window::{
    PrimaryWindow, RequestRedraw, WindowCloseRequested, WindowFocused, WindowResizeConstraints,
    WindowResized,
};
use bevy::winit::{UpdateMode, WinitSettings};
use std::time::Duration;
use winit::window::Icon;

const IDLE_FOCUSED_WAIT: Duration = Duration::from_secs(5);
const CLOCK_TICK_WAIT: Duration = Duration::from_secs(1);

/// Scales used by the shell when laying out screens and the board.
#[derive(Resource, Debug, Clone, Copy)]
pub struct UiMetrics {
    pub scale: f32,
}

impl Default for UiMetrics {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl UiMetrics {
    pub fn header_height(&self) -> f32 {
        crate::ui::style::HEADER_HEIGHT * 20.0 * self.scale
    }

    pub fn status_height(&self) -> f32 {
        crate::ui::style::STATUS_HEIGHT * 20.0 * self.scale
    }

    pub fn content_pad(&self) -> f32 {
        crate::ui::style::CONTENT_PAD * 20.0 * self.scale
    }
}

#[derive(Resource, Default)]
pub struct WindowSyncState {
    initialized: bool,
    save_elapsed: f32,
}

pub struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiMetrics>()
            .init_resource::<WindowSyncState>()
            .insert_resource(RemSize(20.0))
            .add_systems(Startup, apply_window_settings)
            .add_systems(
                Update,
                (
                    sync_ui_scale,
                    apply_ui_font,
                    window_focus_pause,
                    schedule_settings_save,
                    persist_progress,
                    close_and_save,
                    native_window_sync,
                ),
            )
            .add_systems(PostUpdate, (sync_power_mode, request_reactive_redraw));
    }
}

fn sync_ui_scale(
    settings: Res<Settings>,
    mut metrics: ResMut<UiMetrics>,
    mut rem: ResMut<RemSize>,
) {
    let desired = settings.ui_scale_factor();
    if (metrics.scale - desired).abs() > f32::EPSILON {
        metrics.scale = desired;
        rem.0 = 20.0 * desired;
    }
}

fn apply_ui_font(mut fonts: Query<&mut TextFont, Added<TextFont>>) {
    for mut font in &mut fonts {
        if matches!(&font.font, FontSource::Handle(_)) {
            font.font = ui_font();
        }
    }
}

pub fn window_focus_pause(
    mut focus_events: MessageReader<WindowFocused>,
    mut session: ResMut<GameSession>,
    settings: Res<Settings>,
    scores: Res<Scores>,
    mut saved_slot: ResMut<SavedGameSlot>,
) {
    for event in focus_events.read() {
        session.set_window_focused(event.focused);
        if !event.focused
            && session
                .board
                .as_ref()
                .is_some_and(|board| board.placed && board.outcome == Outcome::InProgress)
        {
            let saved = resolve_saved_game(&session, &settings, &mut saved_slot);
            save_progress(&scores.0, saved);
        }
    }
}

fn schedule_settings_save(settings: Res<Settings>, mut commands: Commands) {
    if settings.is_changed() {
        commands.queue(SaveSettingsDeferred(Duration::from_millis(400)));
    }
}

fn persist_progress(
    session: Res<GameSession>,
    settings: Res<Settings>,
    scores: Res<Scores>,
    mut saved_slot: ResMut<SavedGameSlot>,
) {
    let finished_placed = session
        .board
        .as_ref()
        .is_some_and(|board| board.placed && board.outcome != Outcome::InProgress);
    if scores.is_changed() || (finished_placed && saved_slot.0.is_some()) {
        let saved = resolve_saved_game(&session, &settings, &mut saved_slot);
        save_progress(&scores.0, saved);
    }
}

fn close_and_save(
    mut close: MessageReader<WindowCloseRequested>,
    session: Res<GameSession>,
    settings: Res<Settings>,
    scores: Res<Scores>,
    mut saved_slot: ResMut<SavedGameSlot>,
    mut commands: Commands,
) {
    if close.read().next().is_some() {
        let saved = resolve_saved_game(&session, &settings, &mut saved_slot);
        save_progress(&scores.0, saved);
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(bevy::app::AppExit::Success);
    }
}

pub fn apply_window_settings(mut window: Single<&mut Window>, settings: Res<Settings>) {
    window.resolution.set(
        settings.window_width.max(MIN_WINDOW_WIDTH),
        settings.window_height.max(MIN_WINDOW_HEIGHT),
    );
    // Clamp live drags as well: below this the header and the board cannot keep
    // their minimum cell size.
    window.resize_constraints = WindowResizeConstraints {
        min_width: MIN_WINDOW_WIDTH,
        min_height: MIN_WINDOW_HEIGHT,
        ..default()
    };
}

/// Keep the native window icon and maximized state in sync and remember the
/// window geometry in the settings.
pub fn native_window_sync(world: &mut World) {
    let delta = world.resource::<Time<Real>>().delta_secs();
    let resized_recently = world
        .get_resource::<Messages<WindowResized>>()
        .is_some_and(|messages| !messages.is_empty());
    let (needs_initialization, should_save) = {
        let mut state = world.resource_mut::<WindowSyncState>();
        state.save_elapsed += delta;
        let should_save = resized_recently || state.save_elapsed >= 1.0;
        if should_save {
            state.save_elapsed = 0.0;
        }
        (!state.initialized, should_save)
    };

    let Some(window_entity) = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(world)
        .next()
    else {
        return;
    };

    if needs_initialization {
        let start_maximized = world.resource::<Settings>().window_maximized;
        let icon = make_window_icon();
        let initialized = bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
            let Some(window) = windows.get_window(window_entity) else {
                return false;
            };
            if let Some(icon) = icon {
                window.set_window_icon(Some(icon));
            }
            window.set_maximized(start_maximized);
            true
        });
        if initialized {
            world.resource_mut::<WindowSyncState>().initialized = true;
        }
    }

    if !should_save {
        return;
    }
    let Some(window) = world.get::<Window>(window_entity) else {
        return;
    };
    let width = window.resolution.width();
    let height = window.resolution.height();
    // Native constraints are only honoured by interactive resizing, so clamp
    // programmatic resizes here as well: below this the difficulty picker and
    // the large board get clipped. Minimized windows report a near-zero size,
    // which must not be "corrected".
    let smaller_than_minimum =
        width > 1.0 && height > 1.0 && (width < MIN_WINDOW_WIDTH || height < MIN_WINDOW_HEIGHT);
    if smaller_than_minimum {
        if let Some(mut window) = world.get_mut::<Window>(window_entity) {
            window
                .resolution
                .set(width.max(MIN_WINDOW_WIDTH), height.max(MIN_WINDOW_HEIGHT));
        }
        return;
    }
    let maximized = bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
        windows
            .get_window(window_entity)
            .map(|window| window.is_maximized())
    });
    let (cur_w, cur_h, cur_max) = {
        let settings = world.resource::<Settings>();
        (
            settings.window_width,
            settings.window_height,
            settings.window_maximized,
        )
    };
    let size_changed = (width - cur_w).abs() > 0.5 || (height - cur_h).abs() > 0.5;
    let max_changed = maximized.is_some_and(|m| m != cur_max);
    if size_changed || max_changed {
        let mut settings = world.resource_mut::<Settings>();
        if size_changed {
            settings.window_width = width;
            settings.window_height = height;
        }
        if let Some(maximized) = maximized {
            settings.window_maximized = maximized;
        }
    }
}

/// Compute the reactive wait interval when focused: 5s when the game clock is
/// stopped, or aligned 1-second wakeups while the clock is actively ticking.
fn compute_focused_wait(
    clock_running: bool,
    elapsed: Duration,
    align: &mut Option<(u64, Duration)>,
) -> Duration {
    if !clock_running {
        *align = None;
        return IDLE_FOCUSED_WAIT;
    }

    let secs = elapsed.as_secs();
    let subsec_ms = elapsed.subsec_millis();

    if let Some((target_sec, wait)) = *align {
        if secs < target_sec {
            return wait;
        }
        *align = None;
    }

    // When resuming mid-second (or after OS scheduling jitter), schedule a
    // single catch-up tick right past the next whole-second boundary, then
    // settle onto a steady 1-second cadence so `UpdateMode` stops changing.
    if subsec_ms >= 100 {
        let catch_up = Duration::from_millis(u64::from(1000 - subsec_ms + 8));
        *align = Some((secs + 1, catch_up));
        return catch_up;
    }

    CLOCK_TICK_WAIT
}

fn sync_power_mode(
    session: Res<GameSession>,
    winit: Option<ResMut<WinitSettings>>,
    mut align: Local<Option<(u64, Duration)>>,
) {
    let Some(mut winit) = winit else {
        return;
    };
    if matches!(winit.focused_mode, UpdateMode::Continuous) {
        return;
    }
    let desired = UpdateMode::reactive_low_power(compute_focused_wait(
        session.clock.is_running(),
        session.clock.elapsed(),
        &mut align,
    ));
    if winit.focused_mode != desired {
        winit.focused_mode = desired;
    }
}

#[derive(SystemParam)]
struct DiscreteInputs<'w, 's> {
    keys: MessageReader<'w, 's, KeyboardInput>,
    mouse: MessageReader<'w, 's, MouseButtonInput>,
    wheel: MessageReader<'w, 's, MouseWheel>,
    touch: MessageReader<'w, 's, TouchInput>,
}

impl DiscreteInputs<'_, '_> {
    fn any_received(&mut self) -> bool {
        let had_key = self.keys.read().next().is_some();
        let had_mouse = self.mouse.read().next().is_some();
        let had_wheel = self.wheel.read().next().is_some();
        let had_touch = self.touch.read().next().is_some();
        had_key || had_mouse || had_wheel || had_touch
    }
}

#[derive(SystemParam)]
struct WatchedResources<'w> {
    session: Res<'w, GameSession>,
    settings: Res<'w, Settings>,
    scores: Res<'w, Scores>,
    saved_slot: Res<'w, SavedGameSlot>,
    palette: Res<'w, ThemePalette>,
    screen: Res<'w, State<Screen>>,
    next_screen: Res<'w, NextState<Screen>>,
}

impl WatchedResources<'_> {
    fn any_changed(&self) -> bool {
        self.session.is_changed()
            || self.settings.is_changed()
            || self.scores.is_changed()
            || self.saved_slot.is_changed()
            || self.palette.is_changed()
            || self.screen.is_changed()
    }

    fn pending_screen(&self) -> bool {
        matches!(*self.next_screen, NextState::Pending(_))
    }
}

#[allow(clippy::too_many_arguments)]
fn request_reactive_redraw(
    winit: Option<Res<WinitSettings>>,
    watched: WatchedResources,
    escape_guard: Res<EscapeGuard>,
    window_sync: Res<WindowSyncState>,
    board_pointer: Option<Res<BoardPointerState>>,
    transitions: Query<(), With<UiTransition>>,
    added_nodes: Query<(), Added<Node>>,
    mut inputs: DiscreteInputs,
    mut settle_frames: Local<u8>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    let Some(winit) = winit else {
        return;
    };
    if matches!(winit.focused_mode, UpdateMode::Continuous) {
        return;
    }

    let state_mutated = inputs.any_received() || watched.any_changed() || !added_nodes.is_empty();

    if state_mutated {
        *settle_frames = (*settle_frames).max(2);
    } else if board_pointer.as_ref().is_some_and(|p| p.is_changed()) {
        *settle_frames = (*settle_frames).max(1);
    }

    let needs_immediate = !transitions.is_empty()
        || watched.pending_screen()
        || escape_guard.0 > 0
        || !window_sync.initialized;

    if needs_immediate {
        redraw.write(RequestRedraw);
    } else if *settle_frames > 0 {
        *settle_frames -= 1;
        redraw.write(RequestRedraw);
    }
}

fn make_window_icon() -> Option<Icon> {
    const SIZE: u32 = 128;
    let tree = resvg::usvg::Tree::from_data(
        include_bytes!("../../assets/icons/org.gnome.Mines.svg"),
        &resvg::usvg::Options::default(),
    )
    .ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(SIZE, SIZE)?;
    let tree_size = tree.size();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            SIZE as f32 / tree_size.width(),
            SIZE as f32 / tree_size.height(),
        ),
        &mut pixmap.as_mut(),
    );
    Icon::from_rgba(pixmap.take(), SIZE, SIZE).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_wait_sleeps_when_idle_and_aligns_clock_ticks() {
        let mut align = None;

        // Idle / paused / menus sleep for 5s.
        assert_eq!(
            compute_focused_wait(false, Duration::ZERO, &mut align),
            IDLE_FOCUSED_WAIT
        );
        assert!(align.is_none());

        // Fresh clock start at 0.000s locks straight onto 1s wakeups.
        assert_eq!(
            compute_focused_wait(true, Duration::from_millis(2), &mut align),
            CLOCK_TICK_WAIT
        );
        assert!(align.is_none());

        // Resuming mid-second (3.700s) schedules one stable catch-up interval
        // (308ms) and holds it steady across intermediate redraws until 4.000s.
        let catch_up = compute_focused_wait(true, Duration::from_millis(3_700), &mut align);
        assert_eq!(catch_up, Duration::from_millis(308));
        assert_eq!(
            compute_focused_wait(true, Duration::from_millis(3_850), &mut align),
            catch_up
        );

        // Once the whole-second rollover arrives, locks back onto 1s wakeups.
        assert_eq!(
            compute_focused_wait(true, Duration::from_millis(4_008), &mut align),
            CLOCK_TICK_WAIT
        );
        assert!(align.is_none());
    }
}
