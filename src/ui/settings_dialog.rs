// SPDX-License-Identifier: GPL-3.0-or-later
//
// Settings dialog body: tabbed panels with Feathers controls bound to the
// `Settings` resource.

use crate::storage::{BOARD_STYLES, Settings, UI_THEMES};
use crate::theme::{ThemePalette, tokens};
use crate::ui::style::{FONT_BODY, FONT_SMALL};
use bevy::feathers::controls::{
    FeathersCheckbox, FeathersSelect, OptionIndex, FeathersSlider, list_rows_from_strings,
};
use bevy::feathers::theme::{ThemeBorderColor, ThemeTextColor, ThemeToken, ThemedText};
use bevy::prelude::*;
use bevy::ui::{Checked, Overflow, OverflowAxis, ScrollPosition, rem};
use bevy::ui_widgets::{SelectedTab, Tab, TabList, tablist_self_update};
use bevy::ui_widgets::{SliderStep, SliderValue, slider_self_update};
use bevy::ui_widgets::{ScrollArea, ValueChange, checkbox_self_update};

/// Currently selected settings tab.
#[derive(Resource, Default, Debug)]
pub struct SettingsTabState {
    pub index: usize,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingsTab(pub usize);

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingsPanelRoot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingToggle {
    QuestionMarks,
    AutoClear,
    AutoFlag,
    ReducedMotion,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingCheckbox(pub SettingToggle);

impl Default for SettingToggle {
    fn default() -> Self {
        Self::QuestionMarks
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingSelectKind {
    UiTheme,
    BoardStyle,
}

impl Default for SettingSelectKind {
    fn default() -> Self {
        Self::UiTheme
    }
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingSelect(pub SettingSelectKind);

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ScaleSlider;

pub struct SettingsDialogPlugin;

impl Plugin for SettingsDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsTabState>()
            .add_systems(Update, (sync_settings_panel, style_selected_tabs))
            .add_observer(on_checkbox_changed)
            .add_observer(on_select_changed)
            .add_observer(on_scale_changed)
            .add_observer(on_tab_changed);
    }
}

/// Dialog body for the settings dialog.
pub fn contents(
    settings: &Settings,
    _palette: &ThemePalette,
    tab_index: usize,
) -> Box<dyn SceneList> {
    let tabs: Vec<Box<dyn SceneList>> = vec![
        tab_label("游戏", 0),
        tab_label("外观", 1),
        tab_label("操作", 2),
    ];
    let panel = panel_content(tab_index, settings);
    Box::new(bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: rem(0.8) }
        Children [
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: rem(0.2),
                border: UiRect::bottom(px(1)),
                padding: UiRect::bottom(rem(0.3)),
            }
            ThemeBorderColor(tokens::SURFACE_BORDER)
            TabList
            SelectedTab(None)
            on(tablist_self_update)
            Children [{tabs}]
            --
            Node {
                width: percent(100),
                min_height: rem(11),
                max_height: rem(15),
                padding: UiRect::right(rem(0.3)),
                overflow: Overflow {
                    x: OverflowAxis::Clip,
                    y: OverflowAxis::Scroll,
                },
            }
            ScrollPosition(Vec2::ZERO)
            ScrollArea
            SettingsPanelRoot
            Children [{panel}]
        ]
    })
}

fn tab_label(label: &str, index: usize) -> Box<dyn SceneList> {
    let label = label.to_owned();
    Box::new(bsn! {
        Tab
        SettingsTab(index)
        Node {
            padding: UiRect::axes(rem(0.8), rem(0.3)),
            border_radius: BorderRadius::all(rem(0.4)),
        }
        Children [
            Text({label})
            TextFont { font_size: FontSize::Rem(FONT_BODY) }
            ThemeTextColor(tokens::TEXT_MAIN) ThemedText
        ]
    })
}

fn panel_content(index: usize, settings: &Settings) -> Box<dyn SceneList> {
    match index {
        1 => appearance_panel(settings),
        2 => controls_panel(),
        _ => game_panel(settings),
    }
}

fn game_panel(settings: &Settings) -> Box<dyn SceneList> {
    let rows: Vec<Box<dyn SceneList>> = vec![
        checkbox_row(
            "使用问号标记",
            SettingToggle::QuestionMarks,
            settings.use_question_marks,
        ),
        checkbox_row(
            "插旗后自动清除周围",
            SettingToggle::AutoClear,
            settings.auto_clear_on_flags,
        ),
        checkbox_row("自动插旗", SettingToggle::AutoFlag, settings.use_autoflag),
    ];
    column(rows)
}

fn appearance_panel(settings: &Settings) -> Box<dyn SceneList> {
    let ui_theme_labels: Vec<&str> = UI_THEMES.iter().map(|(_, label)| *label).collect();
    let ui_theme_selected = UI_THEMES
        .iter()
        .position(|(key, _)| *key == settings.ui_theme)
        .unwrap_or(0);
    let board_labels: Vec<&str> = BOARD_STYLES.iter().map(|(_, label)| *label).collect();
    let board_selected = BOARD_STYLES
        .iter()
        .position(|(key, _)| *key == settings.theme)
        .unwrap_or(0);
    let rows: Vec<Box<dyn SceneList>> = vec![
        select_row(
            "界面主题",
            ui_theme_labels,
            ui_theme_selected,
            SettingSelectKind::UiTheme,
        ),
        select_row(
            "棋盘风格",
            board_labels,
            board_selected,
            SettingSelectKind::BoardStyle,
        ),
        slider_row("界面缩放", settings.ui_scale_factor()),
        checkbox_row(
            "减弱动画",
            SettingToggle::ReducedMotion,
            settings.reduced_motion,
        ),
    ];
    column(rows)
}

fn controls_panel() -> Box<dyn SceneList> {
    column(vec![text_line(
        "左键翻开 / 数字周围清除",
        FONT_BODY,
        tokens::TEXT_MAIN,
    ), text_line(
        "右键或长按插旗，问号标记可循环切换",
        FONT_BODY,
        tokens::TEXT_SECONDARY,
    ), text_line(
        "方向键或 U J H K 移动，空格翻开，Ctrl+空格插旗",
        FONT_BODY,
        tokens::TEXT_SECONDARY,
    ), text_line(
        "Ctrl+N 新游戏 · Ctrl+R 重新开始 · F1 帮助",
        FONT_SMALL,
        tokens::TEXT_SECONDARY,
    )])
}

fn column(rows: Vec<Box<dyn SceneList>>) -> Box<dyn SceneList> {
    Box::new(bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: rem(0.55) }
        Children [{rows}]
    })
}

fn checkbox_row(label: &str, toggle: SettingToggle, checked: bool) -> Box<dyn SceneList> {
    let label = label.to_owned();
    let caption = bsn! {
        Text({label})
        TextFont { font_size: FontSize::Rem(FONT_BODY) }
        ThemedText
    };
    if checked {
        Box::new(bsn! {
            @FeathersCheckbox { @caption: {caption} }
            Checked
            SettingCheckbox({toggle})
            on(checkbox_self_update)
        })
    } else {
        Box::new(bsn! {
            @FeathersCheckbox { @caption: {caption} }
            SettingCheckbox({toggle})
            on(checkbox_self_update)
        })
    }
}

fn select_row(
    label: &str,
    options: Vec<&str>,
    selected: usize,
    kind: SettingSelectKind,
) -> Box<dyn SceneList> {
    let label = label.to_owned();
    let rows = list_rows_from_strings(options, Some(selected));
    Box::new(bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: rem(0.8),
        }
        Children [
            Text({label})
            TextFont { font_size: FontSize::Rem(FONT_BODY) }
            ThemeTextColor(tokens::TEXT_MAIN) ThemedText
            --
            @FeathersSelect {
                @options: {rows},
                @max_visible: 4,
            }
            SettingSelect({kind})
            Node { width: rem(10.0) }
        ]
    })
}

fn slider_row(label: &str, value: f32) -> Box<dyn SceneList> {
    let label = label.to_owned();
    Box::new(bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: rem(0.8),
        }
        Children [
            Text({label})
            TextFont { font_size: FontSize::Rem(FONT_BODY) }
            ThemeTextColor(tokens::TEXT_MAIN) ThemedText
            --
            @FeathersSlider {
                @min: 0.85,
                @max: 1.5,
            }
            SliderValue({value})
            SliderStep(0.05)
            ScaleSlider
            Node { width: rem(10.0) }
            on(slider_self_update)
        ]
    })
}

fn text_line(content: &str, size: f32, token: ThemeToken) -> Box<dyn SceneList> {
    let content = content.to_owned();
    Box::new(bsn! {
        Text({content})
        TextFont { font_size: FontSize::Rem(size) }
        ThemeTextColor({token}) ThemedText
    })
}

fn on_checkbox_changed(
    change: On<ValueChange<bool>>,
    checkboxes: Query<&SettingCheckbox>,
    mut settings: ResMut<Settings>,
) {
    let Ok(target) = checkboxes.get(change.source) else {
        return;
    };
    match target.0 {
        SettingToggle::QuestionMarks => settings.use_question_marks = change.value,
        SettingToggle::AutoClear => settings.auto_clear_on_flags = change.value,
        SettingToggle::AutoFlag => settings.use_autoflag = change.value,
        SettingToggle::ReducedMotion => settings.reduced_motion = change.value,
    }
}

fn on_select_changed(
    change: On<ValueChange<Entity>>,
    selects: Query<&SettingSelect>,
    indices: Query<&OptionIndex>,
    mut settings: ResMut<Settings>,
) {
    let Ok(target) = selects.get(change.source) else {
        return;
    };
    let Ok(index) = indices.get(change.value) else {
        return;
    };
    match target.0 {
        SettingSelectKind::UiTheme => {
            if let Some((key, _)) = UI_THEMES.get(index.0) {
                settings.ui_theme = (*key).to_owned();
            }
        }
        SettingSelectKind::BoardStyle => {
            if let Some((key, _)) = BOARD_STYLES.get(index.0) {
                settings.theme = (*key).to_owned();
            }
        }
    }
}

fn on_scale_changed(
    change: On<ValueChange<f32>>,
    sliders: Query<&ScaleSlider>,
    mut settings: ResMut<Settings>,
) {
    if sliders.get(change.source).is_err() {
        return;
    }
    if change.value.is_finite() {
        settings.ui_scale = change.value.clamp(0.85, 1.5);
    }
}

fn on_tab_changed(
    change: On<ValueChange<Option<Entity>>>,
    tabs: Query<&SettingsTab>,
    mut state: ResMut<SettingsTabState>,
) {
    let Some(entity) = change.value else {
        return;
    };
    if let Ok(tab) = tabs.get(entity)
        && state.index != tab.0
    {
        state.index = tab.0;
    }
}

fn sync_settings_panel(
    mut commands: Commands,
    state: Res<SettingsTabState>,
    settings: Res<Settings>,
    panels: Query<Entity, With<SettingsPanelRoot>>,
) {
    if !state.is_changed() {
        return;
    }
    for panel in &panels {
        commands.entity(panel).despawn_children();
        let content = panel_content(state.index, &settings);
        commands
            .entity(panel)
            .queue_spawn_related_scenes::<Children>(content);
    }
}

fn style_selected_tabs(
    tabs: Query<(Entity, Has<bevy::ui::Selected>), With<SettingsTab>>,
    mut commands: Commands,
) {
    let _ = (&tabs, &mut commands);
}
