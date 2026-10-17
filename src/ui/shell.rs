// SPDX-License-Identifier: GPL-3.0-or-later
//
// App-wide UI concerns: UI scale/fonts, window state and persistence.

use crate::state::GameSession;
use crate::storage::{save_progress, Scores, Settings};
use crate::theme::ui_font;
use bevy::prelude::*;
use bevy::settings::{SaveSettingsDeferred, SaveSettingsSync};
use bevy::text::{FontSource, RemSize};
use bevy::window::{PrimaryWindow, WindowCloseRequested, WindowFocused};
use std::time::Duration;
use winit::window::Icon;

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
            );
    }
}

fn sync_ui_scale(settings: Res<Settings>, mut metrics: ResMut<UiMetrics>, mut rem: ResMut<RemSize>) {
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
) {
    for event in focus_events.read() {
        if !event.focused && session.clock.is_running() {
            session.set_paused(true, false);
            session.auto_paused = true;
        } else if event.focused
            && session.auto_paused
            && !session.pause_requested
            && session.dialog.is_none()
        {
            session.auto_paused = false;
            session.set_paused(false, false);
        }
    }
}

fn schedule_settings_save(settings: Res<Settings>, mut commands: Commands) {
    if settings.is_changed() {
        commands.queue(SaveSettingsDeferred(Duration::from_millis(400)));
    }
}

fn persist_progress(settings: Res<Settings>, scores: Res<Scores>) {
    if scores.is_changed() {
        save_progress(&settings, &scores.0);
    }
}

fn close_and_save(
    mut close: MessageReader<WindowCloseRequested>,
    settings: Res<Settings>,
    scores: Res<Scores>,
    mut commands: Commands,
) {
    if close.read().next().is_some() {
        save_progress(&settings, &scores.0);
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(bevy::app::AppExit::Success);
    }
}

pub fn apply_window_settings(mut window: Single<&mut Window>, settings: Res<Settings>) {
    window.resolution.set(
        settings.window_width.max(560.0),
        settings.window_height.max(520.0),
    );
}

/// Keep the native window icon and maximized state in sync and remember the
/// window geometry in the settings.
pub fn native_window_sync(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let (needs_initialization, should_save) = {
        let mut state = world.resource_mut::<WindowSyncState>();
        state.save_elapsed += delta;
        let should_save = state.save_elapsed >= 1.0;
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
        if let Some(icon) = make_window_icon() {
            let initialized = bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
                let Some(window) = windows.get_window(window_entity) else {
                    return false;
                };
                window.set_window_icon(Some(icon));
                window.set_maximized(start_maximized);
                true
            });
            if initialized {
                world.resource_mut::<WindowSyncState>().initialized = true;
            }
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
    let maximized = bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
        windows
            .get_window(window_entity)
            .map(|window| window.is_maximized())
    });
    let mut settings = world.resource_mut::<Settings>();
    if (width - settings.window_width).abs() > 0.5 || (height - settings.window_height).abs() > 0.5
    {
        settings.window_width = width;
        settings.window_height = height;
    }
    if let Some(maximized) = maximized {
        settings.window_maximized = maximized;
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
