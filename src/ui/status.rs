// SPDX-License-Identifier: GPL-3.0-or-later
//
// Live status bar: remaining mines and elapsed time.

use crate::assets::GameAssets;
use crate::state::GameSession;
use crate::theme::{ThemePalette, tokens};
use crate::ui::style::{self, FONT_BODY, RADIUS_PILL, STATUS_HEIGHT};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText};
use bevy::prelude::*;
use bevy::ui::rem;

#[derive(Component, Default, Clone)]
pub struct FlagText;

#[derive(Component, Default, Clone)]
pub struct ClockText;

pub struct StatusPlugin;

impl Plugin for StatusPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_status);
    }
}

/// The pill-shaped counters shown under the board.
pub fn status_bar(
    assets: &GameAssets,
    palette: &ThemePalette,
    session: &GameSession,
) -> Box<dyn SceneList> {
    style::boxed_list(bsn_list! { @status_bar_scene(assets, palette, session) })
}

fn status_bar_scene(
    assets: &GameAssets,
    palette: &ThemePalette,
    session: &GameSession,
) -> impl Scene {
    let flag_icon = assets.modern.flag.clone();
    let clock_icon = assets.icons.stopwatch.clone();
    let accent = palette.0.accent;
    let muted = palette.0.text_secondary;
    let flags = format!("{}", session.mines_left().max(0));
    let elapsed = format_time(session.clock.elapsed().as_secs());
    bsn! {
        Node {
            height: rem(STATUS_HEIGHT),
            padding: UiRect::axes(rem(1.1), rem(0.3)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: rem(1.4),
            border: px(1),
            border_radius: BorderRadius::all(rem(RADIUS_PILL)),
        }
        ThemeBackgroundColor(tokens::PILL_BG)
        ThemeBorderColor(tokens::PILL_BORDER)
        Children [
            Node { align_items: AlignItems::Center, column_gap: rem(0.35) }
            Children [
                @icon_fragment(flag_icon, accent, 1.0)
                --
                Text({flags})
                TextFont { font_size: FontSize::Rem(FONT_BODY) }
                ThemeTextColor(tokens::TEXT_MAIN) ThemedText
                FlagText
            ]
            --
            Node { align_items: AlignItems::Center, column_gap: rem(0.35) }
            Children [
                @icon_fragment(clock_icon, muted, 1.0)
                --
                ClockText
                Text({elapsed})
                TextFont { font_size: FontSize::Rem(FONT_BODY) }
                ThemeTextColor(tokens::TEXT_MAIN) ThemedText
            ]
        ]
    }
}

fn icon_fragment(handle: Handle<Image>, tint: Color, size: f32) -> impl Scene {
    bsn! {
        ImageNode { image: {handle}, color: {tint} }
        Node { width: rem(size), height: rem(size) }
        Pickable::IGNORE
    }
}

fn update_status(
    session: Res<GameSession>,
    mut flags: Query<&mut Text, (With<FlagText>, Without<ClockText>)>,
    mut clock: Query<&mut Text, (With<ClockText>, Without<FlagText>)>,
) {
    let remaining = format!("{}", session.mines_left().max(0));
    for mut text in &mut flags {
        if text.0 != remaining {
            text.0.clone_from(&remaining);
        }
    }
    let elapsed = format_time(session.clock.elapsed().as_secs());
    for mut text in &mut clock {
        if text.0 != elapsed {
            text.0.clone_from(&elapsed);
        }
    }
}

pub fn format_time(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
