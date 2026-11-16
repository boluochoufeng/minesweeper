// SPDX-License-Identifier: GPL-3.0-or-later
//
// Live status bar: remaining mines and elapsed time.

use crate::assets::GameAssets;
use crate::state::GameSession;
use crate::theme::tokens;
use crate::ui::style::{self, FONT_BODY, RADIUS_PILL, STATUS_HEIGHT};
use bevy::feathers::display::ThemedIcon;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText};
use bevy::prelude::*;
use bevy::ui::rem;

#[derive(Component, Default, Clone)]
pub struct FlagText;

#[derive(Component, Default, Clone)]
pub struct FlagStatusIcon;

#[derive(Component, Default, Clone, Copy)]
struct FlagOverLimit(bool);

#[derive(Component, Default, Clone)]
pub struct ClockText;

pub struct StatusPlugin;

impl Plugin for StatusPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_status);
    }
}

/// The pill-shaped counters shown under the board.
pub fn status_bar(assets: &GameAssets, session: &GameSession) -> Box<dyn SceneList> {
    style::boxed_list(bsn_list! { @status_bar_scene(assets, session) })
}

fn status_bar_scene(assets: &GameAssets, session: &GameSession) -> impl Scene {
    let flag_icon = assets.modern.flag.clone();
    let clock_icon = assets.icons.stopwatch.clone();
    let mines_left = session.mines_left();
    let over_limit = mines_left < 0;
    let flags = mines_left.to_string();
    let flag_icon_token = if over_limit {
        tokens::TEXT_DANGER
    } else {
        tokens::TEXT_ACCENT
    };
    let flag_text_token = if over_limit {
        tokens::TEXT_DANGER
    } else {
        tokens::TEXT_MAIN
    };
    let elapsed = format_time(session.elapsed_secs());
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
                @flag_icon_fragment(flag_icon, flag_icon_token, over_limit, 1.0)
                --
                Text({flags})
                TextFont { font_size: FontSize::Rem(FONT_BODY) }
                ThemeTextColor({flag_text_token}) ThemedText
                FlagText
                FlagOverLimit({over_limit})
            ]
            --
            Node { align_items: AlignItems::Center, column_gap: rem(0.35) }
            Children [
                @icon_fragment(clock_icon, tokens::TEXT_SECONDARY, 1.0)
                --
                ClockText
                Text({elapsed})
                TextFont { font_size: FontSize::Rem(FONT_BODY) }
                ThemeTextColor(tokens::TEXT_MAIN) ThemedText
            ]
        ]
    }
}

fn flag_icon_fragment(
    handle: Handle<Image>,
    token: bevy::feathers::theme::ThemeToken,
    over_limit: bool,
    size: f32,
) -> impl Scene {
    bsn! {
        ImageNode { image: {handle} }
        Node { width: rem(size), height: rem(size) }
        ThemedIcon
        TextColor(Color::WHITE)
        ThemeTextColor({token}) ThemedText
        FlagStatusIcon
        FlagOverLimit({over_limit})
        Pickable::IGNORE
    }
}

fn icon_fragment(
    handle: Handle<Image>,
    token: bevy::feathers::theme::ThemeToken,
    size: f32,
) -> impl Scene {
    bsn! {
        ImageNode { image: {handle} }
        Node { width: rem(size), height: rem(size) }
        ThemedIcon
        TextColor(Color::WHITE)
        ThemeTextColor({token}) ThemedText
        Pickable::IGNORE
    }
}

#[allow(clippy::type_complexity)]
fn update_status(
    mut commands: Commands,
    session: Res<GameSession>,
    mut flags: Query<(Entity, &mut Text, &mut FlagOverLimit), (With<FlagText>, Without<ClockText>)>,
    mut flag_icons: Query<(Entity, &mut FlagOverLimit), (With<FlagStatusIcon>, Without<FlagText>)>,
    mut clock: Query<&mut Text, (With<ClockText>, Without<FlagText>)>,
) {
    let mines_left = session.mines_left();
    let over_limit = mines_left < 0;
    let remaining = mines_left.to_string();
    for (entity, mut text, mut state) in &mut flags {
        if text.0 != remaining {
            text.0.clone_from(&remaining);
        }
        if state.0 != over_limit {
            state.0 = over_limit;
            let token = if over_limit {
                tokens::TEXT_DANGER
            } else {
                tokens::TEXT_MAIN
            };
            commands.entity(entity).insert(ThemeTextColor(token));
        }
    }
    for (entity, mut state) in &mut flag_icons {
        if state.0 != over_limit {
            state.0 = over_limit;
            let token = if over_limit {
                tokens::TEXT_DANGER
            } else {
                tokens::TEXT_ACCENT
            };
            commands.entity(entity).insert(ThemeTextColor(token));
        }
    }
    let elapsed = format_time(session.elapsed_secs());
    for mut text in &mut clock {
        if text.0 != elapsed {
            text.0.clone_from(&elapsed);
        }
    }
}

pub fn format_time(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
