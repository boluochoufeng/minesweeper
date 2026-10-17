// SPDX-License-Identifier: GPL-3.0-or-later
//
// Custom board editor. Width/height/mine-density are real Feathers number
// inputs with hard limits, so invalid values cannot be entered.

use crate::assets::GameAssets;
use crate::state::{PendingAction, Screen};
use crate::storage::{Difficulty, Settings};
use crate::theme::tokens;
use crate::ui::actions::UiAction;
use crate::ui::menu;
use crate::ui::style::{self, FONT_BODY, FONT_CAPTION, FONT_TITLE};
use bevy::feathers::controls::{
    ButtonVariant, FeathersNumberInput, HardLimit, NumberInputRange, NumberInputValue, SoftLimit,
};
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText};
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui::{px, rem};
use bevy::ui_widgets::ValueChange;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CustomField {
    #[default]
    Width,
    Height,
    Percentage,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CustomFieldTarget {
    pub field: CustomField,
}

#[derive(Component, Default, Clone)]
pub struct SummaryText;

pub struct CustomBoardScreenPlugin;

impl Plugin for CustomBoardScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::CustomBoard), spawn_custom_board)
            .add_systems(Update, update_summary)
            .add_observer(custom_value_changed);
    }
}

fn spawn_custom_board(
    mut commands: Commands,
    assets: Res<GameAssets>,
    settings: Res<Settings>,
) {
    let mut entity = commands.spawn_scene(custom_screen(&assets, &settings));
    entity.insert(DespawnOnExit(Screen::CustomBoard));
}

fn custom_screen(assets: &GameAssets, settings: &Settings) -> impl Scene {
    let back = style::icon_button(
        assets.icons.back.clone(),
        UiAction::Pending(PendingAction::ShowNewGame),
    );
    let header = menu::header(
        "自定义棋盘".to_owned(),
        None,
        vec![back],
        Vec::new(),
        assets,
    );
    let percentage = if settings.custom_width * settings.custom_height > 0 {
        (100.0 * settings.custom_mines as f64
            / (settings.custom_width * settings.custom_height) as f64)
            .round() as i32
    } else {
        15
    };
    let width = settings.custom_width as i32;
    let height = settings.custom_height as i32;
    let summary = format!(
        "{} × {} · {} 个雷",
        settings.custom_width, settings.custom_height, settings.custom_mines
    );
    let rows: Vec<Box<dyn SceneList>> = vec![
        number_row("宽度", CustomField::Width, width, 8, 30),
        number_row("高度", CustomField::Height, height, 8, 30),
        number_row("雷区占比", CustomField::Percentage, percentage, 1, 99),
    ];
    let start = style::text_button(
        "开始游戏",
        UiAction::Pending(PendingAction::Start(Difficulty::Custom)),
        ButtonVariant::Primary,
    );
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
                padding: rem(1.6),
            }
            Children [
                Node {
                    width: percent(100),
                    max_width: rem(26),
                    flex_direction: FlexDirection::Column,
                    row_gap: rem(1.0),
                    padding: rem(1.4),
                    border: UiRect::all(px(1.0)),
                    border_radius: BorderRadius::all(rem(style::RADIUS_CARD)),
                }
                ThemeBackgroundColor(tokens::SURFACE_BG)
                ThemeBorderColor(tokens::SURFACE_BORDER)
                Children [
                    Text("棋盘尺寸")
                    TextFont { font_size: FontSize::Rem(FONT_TITLE) }
                    ThemeTextColor(tokens::TEXT_MAIN) ThemedText
                    --
                    {rows}
                    --
                    Node { height: px(1.0), width: percent(100) }
                    ThemeBackgroundColor(tokens::SURFACE_BORDER)
                    --
                    SummaryText
                    Text({summary})
                    TextFont { font_size: FontSize::Rem(FONT_CAPTION) }
                    ThemeTextColor(tokens::TEXT_SECONDARY) ThemedText
                    --
                    {start}
                ]
            ]
        ]
    }
}

fn number_row(
    label: &str,
    field: CustomField,
    value: i32,
    min: i32,
    max: i32,
) -> Box<dyn SceneList> {
    let label = label.to_owned();
    Box::new(bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            column_gap: rem(1.0),
        }
        Children [
            Text({label})
            TextFont { font_size: FontSize::Rem({FONT_BODY}) }
            ThemeTextColor(tokens::TEXT_MAIN) ThemedText
            Node { width: rem(5.0) }
            --
            @FeathersNumberInput
            CustomFieldTarget { field: {field} }
            NumberInputValue::I32({value})
            Node { flex_grow: 1.0 }
            on(custom_value_changed)
            HardLimit(NumberInputRange::I32({min}..={max}))
            SoftLimit(NumberInputRange::I32({min}..={max}))
        ]
    })
}

fn custom_value_changed(
    change: On<ValueChange<i32>>,
    fields: Query<&CustomFieldTarget>,
    mut settings: ResMut<Settings>,
) {
    if !change.is_final {
        return;
    }
    let Ok(target) = fields.get(change.source) else {
        return;
    };
    let old_size = settings.custom_width * settings.custom_height;
    let old_percentage = if old_size > 0 {
        100.0 * settings.custom_mines as f64 / old_size as f64
    } else {
        15.0
    };
    match target.field {
        CustomField::Width => settings.custom_width = change.value.clamp(8, 30) as usize,
        CustomField::Height => settings.custom_height = change.value.clamp(8, 30) as usize,
        CustomField::Percentage => {
            let size = settings.custom_width * settings.custom_height;
            let mines = ((change.value.clamp(1, 99) as f64 * size as f64 / 100.0).round() as usize)
                .clamp(1, size.saturating_sub(10).max(1));
            settings.custom_mines = mines;
        }
    }
    if !matches!(target.field, CustomField::Percentage) {
        let size = settings.custom_width * settings.custom_height;
        settings.custom_mines = ((old_percentage * size as f64 / 100.0).round() as usize)
            .clamp(1, size.saturating_sub(10).max(1));
    }
}

fn update_summary(settings: Res<Settings>, mut texts: Query<&mut Text, With<SummaryText>>) {
    if !settings.is_changed() {
        return;
    }
    let summary = format!(
        "{} × {} · {} 个雷",
        settings.custom_width, settings.custom_height, settings.custom_mines
    );
    for mut text in &mut texts {
        if text.0 != summary {
            text.0.clone_from(&summary);
        }
    }
}
