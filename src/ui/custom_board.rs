// SPDX-License-Identifier: GPL-3.0-or-later
//
// Custom board editor. Width/height/mine-density are real Feathers number
// inputs with hard limits, so invalid values cannot be entered.

use crate::assets::GameAssets;
use crate::state::{PendingAction, Screen};
use crate::storage::{Difficulty, MAX_CUSTOM_DIM, MIN_CUSTOM_DIM, Settings};
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
    Mines,
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
            .add_systems(Update, (sync_custom_inputs, update_summary))
            .add_observer(custom_value_changed);
    }
}

fn spawn_custom_board(mut commands: Commands, assets: Res<GameAssets>, settings: Res<Settings>) {
    let mut entity = commands.spawn_scene(custom_screen(&assets, &settings));
    entity.insert(DespawnOnExit(Screen::CustomBoard));
}

fn max_mines_for(width: usize, height: usize) -> usize {
    (width * height).saturating_sub(10).max(1)
}

fn max_percentage_for(width: usize, height: usize) -> i32 {
    let size = (width * height).max(1);
    let max_mines = max_mines_for(width, height);
    ((100.0 * max_mines as f64 / size as f64).floor() as i32).clamp(1, 99)
}

fn percentage_for(width: usize, height: usize, mines: usize) -> i32 {
    let size = width * height;
    if size > 0 {
        (100.0 * mines as f64 / size as f64)
            .round()
            .clamp(1.0, max_percentage_for(width, height) as f64) as i32
    } else {
        15
    }
}

fn format_summary(settings: &Settings) -> String {
    let width = settings.custom_width.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
    let height = settings.custom_height.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
    let mines = settings.custom_mines.clamp(1, max_mines_for(width, height));
    let density = 100.0 * mines as f64 / (width * height).max(1) as f64;
    format!("{width} × {height} · {mines} 个雷 ({density:.1}%)")
}

fn custom_screen(assets: &GameAssets, settings: &Settings) -> impl Scene {
    let back = style::icon_button(
        assets.icons.back.clone(),
        UiAction::Pending(PendingAction::ShowNewGame),
        "返回",
    );
    let header = menu::header(
        "自定义棋盘".to_owned(),
        None,
        vec![back],
        Vec::new(),
        assets,
    );
    let width_usize = settings.custom_width.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
    let height_usize = settings.custom_height.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
    let max_mines = max_mines_for(width_usize, height_usize);
    let mines = settings.custom_mines.clamp(1, max_mines) as i32;
    let percentage = percentage_for(width_usize, height_usize, mines as usize);
    let max_pct = max_percentage_for(width_usize, height_usize);
    let width = width_usize as i32;
    let height = height_usize as i32;
    let min_dim = MIN_CUSTOM_DIM as i32;
    let max_dim = MAX_CUSTOM_DIM as i32;
    let summary = format_summary(settings);
    let rows: Vec<Box<dyn SceneList>> = vec![
        number_row("宽度", CustomField::Width, width, min_dim, max_dim),
        number_row("高度", CustomField::Height, height, min_dim, max_dim),
        number_row("地雷数量", CustomField::Mines, mines, 1, max_mines as i32),
        number_row(
            "雷区占比 (%)",
            CustomField::Percentage,
            percentage,
            1,
            max_pct,
        ),
    ];
    let cancel = style::text_button(
        "取消",
        UiAction::Pending(PendingAction::ShowNewGame),
        ButtonVariant::Normal,
    );
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
                    Node {
                        width: percent(100),
                        justify_content: JustifyContent::End,
                        column_gap: rem(0.5),
                    }
                    Children [
                        {cancel}
                        --
                        {start}
                    ]
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
            Node { width: rem(6.5) }
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
        CustomField::Width => {
            settings.custom_width = (change.value as usize).clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
            let max_mines = max_mines_for(settings.custom_width, settings.custom_height);
            let size = settings.custom_width * settings.custom_height;
            settings.custom_mines =
                ((old_percentage * size as f64 / 100.0).round() as usize).clamp(1, max_mines);
        }
        CustomField::Height => {
            settings.custom_height = (change.value as usize).clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
            let max_mines = max_mines_for(settings.custom_width, settings.custom_height);
            let size = settings.custom_width * settings.custom_height;
            settings.custom_mines =
                ((old_percentage * size as f64 / 100.0).round() as usize).clamp(1, max_mines);
        }
        CustomField::Mines => {
            let max_mines = max_mines_for(settings.custom_width, settings.custom_height);
            settings.custom_mines = (change.value.max(1) as usize).clamp(1, max_mines);
        }
        CustomField::Percentage => {
            let max_mines = max_mines_for(settings.custom_width, settings.custom_height);
            let max_pct = max_percentage_for(settings.custom_width, settings.custom_height);
            let size = settings.custom_width * settings.custom_height;
            let mines = ((change.value.clamp(1, max_pct) as f64 * size as f64 / 100.0).round()
                as usize)
                .clamp(1, max_mines);
            settings.custom_mines = mines;
        }
    }
}

fn sync_custom_inputs(
    mut commands: Commands,
    settings: Res<Settings>,
    inputs: Query<(
        Entity,
        &CustomFieldTarget,
        &NumberInputValue,
        &HardLimit,
        &SoftLimit,
    )>,
) {
    if !settings.is_changed() {
        return;
    }
    let width = settings.custom_width.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
    let height = settings.custom_height.clamp(MIN_CUSTOM_DIM, MAX_CUSTOM_DIM);
    let max_mines = max_mines_for(width, height) as i32;
    let mines = settings.custom_mines.clamp(1, max_mines as usize) as i32;
    let max_pct = max_percentage_for(width, height);
    let pct = percentage_for(width, height, mines as usize);
    let min_dim = MIN_CUSTOM_DIM as i32;
    let max_dim = MAX_CUSTOM_DIM as i32;

    for (entity, target, val, hard, soft) in &inputs {
        let (want_val, want_range) = match target.field {
            CustomField::Width => (width as i32, min_dim..=max_dim),
            CustomField::Height => (height as i32, min_dim..=max_dim),
            CustomField::Mines => (mines, 1..=max_mines),
            CustomField::Percentage => (pct, 1..=max_pct),
        };
        let range = NumberInputRange::I32(want_range);
        if hard.0 != range {
            commands.entity(entity).insert(HardLimit(range.clone()));
        }
        if soft.0 != range {
            commands.entity(entity).insert(SoftLimit(range));
        }
        if *val != NumberInputValue::I32(want_val) {
            commands
                .entity(entity)
                .insert(NumberInputValue::I32(want_val));
        }
    }
}

fn update_summary(settings: Res<Settings>, mut texts: Query<&mut Text, With<SummaryText>>) {
    if !settings.is_changed() {
        return;
    }
    let summary = format_summary(&settings);
    for mut text in &mut texts {
        if text.0 != summary {
            text.0.clone_from(&summary);
        }
    }
}
