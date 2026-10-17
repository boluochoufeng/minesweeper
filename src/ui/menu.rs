// SPDX-License-Identifier: GPL-3.0-or-later
//
// Header bar and the application menu. The menu is a Feathers popover attached
// to an icon button, so it needs no separate dialog state.

use crate::state::{DialogKind, PendingAction};
use crate::theme::{tokens, ThemePalette};
use crate::ui::actions::{ActionTarget, UiAction};
use crate::ui::style::{self, FONT_BODY, FONT_CAPTION, FONT_TITLE, HEADER_HEIGHT};
use crate::assets::GameAssets;
use bevy::feathers::controls::{
    ButtonVariant, FeathersMenu, FeathersMenuButton, FeathersMenuDivider, FeathersMenuItem,
    FeathersMenuPopup,
};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText};
use bevy::prelude::*;
use bevy::ui::rem;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Header bar shared by every screen.
pub fn header(
    title: String,
    subtitle: Option<String>,
    leading: Vec<Box<dyn SceneList>>,
    trailing: Vec<Box<dyn SceneList>>,
    assets: &GameAssets,
    palette: &ThemePalette,
) -> Box<dyn SceneList> {
    let title_block = title_column(title, subtitle);
    let left = bsn_list! {
        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: rem(0.5) }
        Children [
            {leading}
            --
            {title_block}
        ]
    };
    let right = bsn_list! {
        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: rem(0.2) }
        Children [
            {trailing}
            --
            {menu_button(assets, palette)}
        ]
    };
    Box::new(bsn! {
        Node {
            width: percent(100),
            height: rem(HEADER_HEIGHT),
            flex_shrink: 0.0,
            padding: UiRect::axes(rem(0.9), rem(0.3)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect::bottom(px(1)),
        }
        ThemeBackgroundColor(tokens::HEADER_BG)
        ThemeBorderColor(tokens::HEADER_BORDER)
        Children [ {left} -- {right} ]
    })
}

fn title_column(title: String, subtitle: Option<String>) -> Box<dyn SceneList> {
    let subtitle_node: Box<dyn SceneList> = match subtitle {
        Some(subtitle) => Box::new(bsn! {
            Text({subtitle})
            TextFont { font_size: FontSize::Rem(FONT_CAPTION) }
            ThemeTextColor(tokens::TEXT_SECONDARY) ThemedText
        }),
        None => Box::new(bsn_list! {}),
    };
    Box::new(bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            row_gap: rem(0.02),
        }
        Children [
            Text({title})
            TextFont { font_size: FontSize::Rem(FONT_TITLE) }
            ThemeTextColor(tokens::TEXT_MAIN) ThemedText
            --
            {subtitle_node}
        ]
    })
}

/// The icon button that opens the application menu.
pub fn menu_button(assets: &GameAssets, palette: &ThemePalette) -> Box<dyn SceneList> {
    let icon = assets.icons.menu.clone();
    let tint = palette.0.text_primary;
    let popup = menu_popup();
    style::boxed_list(bsn_list! {
        @FeathersMenu
        Children [
            @FeathersMenuButton {
                @caption: bsn! {
                    ImageNode { image: {icon}, color: {tint} }
                    Node { width: rem(1.0), height: rem(1.0) }
                },
                @arrow: false,
            }
            --
            {popup}
        ]
    })

}

fn menu_popup() -> Box<dyn SceneList> {
    let new_game = menu_item("新游戏", UiAction::Pending(PendingAction::ShowNewGame));
    let restart = menu_item("重新开始", UiAction::Pending(PendingAction::Restart));
    let settings = menu_item("设置", UiAction::Open(DialogKind::Settings));
    let scores = menu_item("成绩", UiAction::Open(DialogKind::Scores));
    let help = menu_item("帮助", UiAction::Open(DialogKind::Help));
    let shortcuts = menu_item("键盘快捷键", UiAction::Open(DialogKind::Shortcuts));
    let about = menu_item("关于扫雷", UiAction::Open(DialogKind::About));
    let quit = menu_item("退出", UiAction::Quit);
    Box::new(bsn! {
        @FeathersMenuPopup
        Children [
            {new_game}
            --
            {restart}
            --
            {menu_divider()}
            --
            {settings}
            --
            {scores}
            --
            {menu_divider()}
            --
            {help}
            --
            {shortcuts}
            --
            {about}
            --
            {menu_divider()}
            --
            {quit}
        ]
    })
}

fn menu_item(label: &str, action: UiAction) -> Box<dyn SceneList> {
    let label = label.to_owned();
    Box::new(bsn! {
        @FeathersMenuItem {
            @caption: bsn! {
                Text({label})
                TextFont { font_size: FontSize::Rem(FONT_BODY) }
                ThemedText
            },
        }
        ActionTarget { action: {action} }
    })
}

fn menu_divider() -> Box<dyn SceneList> {
    Box::new(bsn! { @FeathersMenuDivider })
}

/// Small text button used in headers (plain variant).
pub fn header_text_button(label: &str, action: UiAction) -> Box<dyn SceneList> {
    style::text_button(label, action, ButtonVariant::Plain)
}
