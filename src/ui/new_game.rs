// SPDX-License-Identifier: GPL-3.0-or-later
//
// Difficulty picker shown on launch.

use crate::assets::GameAssets;
use crate::state::{PendingAction, Screen};
use crate::storage::Difficulty;
use crate::theme::tokens;
use crate::ui::actions::{ActionTarget, UiAction};
use crate::ui::menu;
use crate::ui::style::{FONT_CAPTION, FONT_DISPLAY, FONT_SUBTITLE};
use bevy::feathers::controls::{ButtonVariant, FeathersButton};
use bevy::feathers::theme::{
    ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText,
};
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui::rem;

pub struct NewGameScreenPlugin;

impl Plugin for NewGameScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::NewGame), spawn_new_game);
    }
}

fn spawn_new_game(mut commands: Commands, assets: Res<GameAssets>) {
    let mut entity = commands.spawn_scene(new_game_screen(&assets));
    entity.insert(DespawnOnExit(Screen::NewGame));
}

fn new_game_screen(assets: &GameAssets) -> impl Scene {
    let header = menu::header("扫雷".to_owned(), None, Vec::new(), Vec::new(), assets);
    let cards: Vec<Box<dyn SceneList>> = vec![
        difficulty_card(
            "8 × 8",
            "10 个雷",
            UiAction::Pending(PendingAction::Start(Difficulty::Small)),
        ),
        difficulty_card(
            "16 × 16",
            "40 个雷",
            UiAction::Pending(PendingAction::Start(Difficulty::Medium)),
        ),
        difficulty_card(
            "30 × 16",
            "99 个雷",
            UiAction::Pending(PendingAction::Start(Difficulty::Large)),
        ),
        difficulty_card("自定义", "自选尺寸与雷数", UiAction::ShowCustom),
    ];
    bsn! {
        Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            {header}
            --
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: rem(1.4),
                padding: rem(1.4),
            }
            Children [
                Text("选择难度")
                TextFont { font_size: FontSize::Rem(FONT_DISPLAY) }
                ThemeTextColor(tokens::TEXT_MAIN) ThemedText
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

fn difficulty_card(title: &str, subtitle: &str, action: UiAction) -> Box<dyn SceneList> {
    let title = title.to_owned();
    let subtitle = subtitle.to_owned();
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
        Node {
            width: rem(13.0),
            height: rem(7.0),
            margin: rem(0.3),
            border: px(1),
        }
        ThemeBorderColor(tokens::CONTROL_BORDER)
    })
}
