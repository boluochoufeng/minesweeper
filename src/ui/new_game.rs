// SPDX-License-Identifier: GPL-3.0-or-later
//
// Difficulty picker shown on launch.

use crate::assets::GameAssets;
use crate::state::{PendingAction, Screen};
use crate::storage::{Difficulty, SavedGame, SavedGameSlot};
use crate::theme::tokens;
use crate::ui::actions::{ActionTarget, UiAction};
use crate::ui::menu;
use crate::ui::status::format_time;
use crate::ui::style::{FONT_BODY, FONT_CAPTION, FONT_DISPLAY, FONT_SUBTITLE};
use bevy::feathers::controls::{ButtonVariant, FeathersButton};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText};
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui::{Overflow, OverflowAxis, ScrollPosition, rem};
use bevy::ui_widgets::ScrollArea;

pub struct NewGameScreenPlugin;

impl Plugin for NewGameScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::NewGame), spawn_new_game);
    }
}

fn spawn_new_game(mut commands: Commands, assets: Res<GameAssets>, saved_slot: Res<SavedGameSlot>) {
    let mut entity = commands.spawn_scene(new_game_screen(&assets, saved_slot.0.as_ref()));
    entity.insert(DespawnOnExit(Screen::NewGame));
}

fn new_game_screen(assets: &GameAssets, saved: Option<&SavedGame>) -> impl Scene {
    let header = menu::header("扫雷".to_owned(), None, Vec::new(), Vec::new(), assets);
    let has_saved = saved.is_some();
    let resume_section: Box<dyn SceneList> = match saved {
        Some(saved) => resume_card(saved),
        None => Box::new(bsn_list! {}),
    };
    let card_height = if has_saved { 6.2 } else { 7.0 };
    let body_gap = if has_saved { 0.95 } else { 1.4 };
    let body_pad = if has_saved { 1.0 } else { 1.4 };
    let cards: Vec<Box<dyn SceneList>> = vec![
        difficulty_card(
            "8 × 8",
            "10 个雷",
            card_height,
            UiAction::Pending(PendingAction::Start(Difficulty::Small)),
        ),
        difficulty_card(
            "16 × 16",
            "40 个雷",
            card_height,
            UiAction::Pending(PendingAction::Start(Difficulty::Medium)),
        ),
        difficulty_card(
            "30 × 16",
            "99 个雷",
            card_height,
            UiAction::Pending(PendingAction::Start(Difficulty::Large)),
        ),
        difficulty_card(
            "自定义",
            "自选尺寸与雷数",
            card_height,
            UiAction::Pending(PendingAction::ShowCustomBoard),
        ),
    ];
    bsn! {
        Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            {header}
            --
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: rem(body_gap),
                padding: rem(body_pad),
                overflow: Overflow {
                    x: OverflowAxis::Clip,
                    y: OverflowAxis::Scroll,
                },
            }
            ScrollPosition(Vec2::ZERO)
            ScrollArea
            Children [
                Text("选择难度")
                TextFont { font_size: FontSize::Rem(FONT_DISPLAY) }
                ThemeTextColor(tokens::TEXT_MAIN) ThemedText
                --
                {resume_section}
                --
                Node {
                    width: percent(100),
                    max_width: rem(30),
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                Children [{cards}]
                --
                Text("首次点击永远安全 · 右键或长按插旗")
                TextFont { font_size: FontSize::Rem(FONT_CAPTION) }
                ThemeTextColor(tokens::TEXT_SECONDARY) ThemedText
            ]
        ]
    }
}

fn resume_card(saved: &SavedGame) -> Box<dyn SceneList> {
    let time = format_time(saved.elapsed_secs());
    let base = format!(
        "{} · {} × {} · 剩余 {} 雷 · {}",
        saved.difficulty.label(),
        saved.width,
        saved.height,
        saved.mines_left(),
        time,
    );
    let subtitle = if saved.no_guessing {
        format!("{base} · 无猜")
    } else {
        base
    };
    let accessible = format!("继续上一局 {subtitle}");
    Box::new(bsn! {
        @FeathersButton {
            @caption: bsn! {
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: rem(0.2),
                }
                Children [
                    Text("继续上一局")
                    TextFont { font_size: FontSize::Rem(FONT_BODY) }
                    ThemedText
                    --
                    Text({subtitle})
                    TextFont { font_size: FontSize::Rem(FONT_CAPTION) }
                    ThemedText
                ]
            },
            @variant: ButtonVariant::Primary,
        }
        ActionTarget { action: UiAction::ResumeSavedGame }
        AccessibleLabel({accessible})
        Node {
            width: rem(26.6),
            height: rem(4.2),
            margin: UiRect::horizontal(rem(0.3)),
            border: px(1),
        }
    })
}

fn difficulty_card(
    title: &str,
    subtitle: &str,
    height_rem: f32,
    action: UiAction,
) -> Box<dyn SceneList> {
    let title = title.to_owned();
    let subtitle = subtitle.to_owned();
    let accessible = format!("{title} {subtitle}");
    Box::new(bsn! {
        @FeathersButton {
            @caption: bsn! {
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: rem(0.25),
                }
                Children [
                    Text({title})
                    TextFont { font_size: FontSize::Rem(FONT_SUBTITLE) }
                    ThemeTextColor(tokens::TEXT_MAIN) ThemedText
                    --
                    Text({subtitle})
                    TextFont { font_size: FontSize::Rem(FONT_CAPTION) }
                    ThemeTextColor(tokens::TEXT_SECONDARY) ThemedText
                ]
            },
            @variant: ButtonVariant::Normal,
        }
        ActionTarget { action: {action} }
        AccessibleLabel({accessible})
        Node {
            width: rem(13.0),
            height: rem(height_rem),
            margin: rem(0.3),
            border: px(1),
        }
        ThemeBorderColor(tokens::CONTROL_BORDER)
    })
}
