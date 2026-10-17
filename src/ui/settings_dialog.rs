// SPDX-License-Identifier: GPL-3.0-or-later
//
// Settings dialog body: tabbed panels with Feathers controls bound to the
// `Settings` resource.

use crate::storage::{BOARD_STYLES, Settings, UI_THEMES};
use crate::theme::{ThemePalette, tokens};
use crate::ui::style::{FONT_BODY, FONT_SMALL};
use bevy::feathers::controls::{
    FeathersCheckbox, FeathersSelect, FeathersSlider, OptionIndex, list_rows_from_strings,
};
use bevy::feathers::theme::{
    ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemeToken, ThemedText,
};
use bevy::prelude::*;
use bevy::ui::{BorderColor, Checked, Overflow, OverflowAxis, ScrollPosition, Selected, rem};
use bevy::ui_widgets::{
    ScrollArea, SelectedTab, SliderStep, SliderValue, Tab, TabList, ValueChange,
    checkbox_self_update, slider_self_update, tablist_self_update,
};

/// Currently selected settings tab.
#[derive(Resource, Default, Debug)]
pub struct SettingsTabState {
    pub index: usize,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingsTab(pub usize);

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingsPanelRoot;

/// The selection state a tab header was last painted for, so the styling system
/// only touches components when something actually changed.
#[derive(Component, Debug, Clone, Copy, Default)]
struct TabStyling(bool);

/// Right-aligned percentage label next to the UI-scale slider.
#[derive(Component, Debug, Clone, Copy, Default)]
struct ScaleValueText;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingToggle {
    #[default]
    QuestionMarks,
    AutoClear,
    AutoFlag,
    ReducedMotion,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingCheckbox(pub SettingToggle);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingSelectKind {
    #[default]
    UiTheme,
    BoardStyle,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingSelect(pub SettingSelectKind);

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ScaleSlider;

pub struct SettingsDialogPlugin;

impl Plugin for SettingsDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsTabState>()
            .add_systems(
                Update,
                (
                    init_tab_selection,
                    sync_settings_panel,
                    style_selected_tabs,
                    hide_builtin_slider_text,
                    update_scale_label,
                ),
            )
            .add_observer(on_checkbox_changed)
            .add_observer(on_select_changed)
            .add_observer(on_scale_changed)
            .add_observer(on_tab_changed);
    }
}

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
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: rem(0.9) }
        Children [
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: rem(0.2),
                border: UiRect::bottom(px(1)),
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
            padding: UiRect::axes(rem(0.8), rem(0.35)),
            border: UiRect::bottom(px(2)),
        }
        BackgroundColor(Color::NONE)
        BorderColor::all(Color::NONE)
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
    column(vec![
        text_line("左键翻开 / 数字周围清除", FONT_BODY, tokens::TEXT_MAIN),
        text_line(
            "右键或长按插旗，问号标记可循环切换",
            FONT_BODY,
            tokens::TEXT_SECONDARY,
        ),
        text_line(
            "方向键或 U J H K 移动，空格翻开，Ctrl+空格插旗",
            FONT_BODY,
            tokens::TEXT_SECONDARY,
        ),
        text_line(
            "Ctrl+N 新游戏 · Ctrl+R 重新开始 · F1 帮助",
            FONT_SMALL,
            tokens::TEXT_SECONDARY,
        ),
    ])
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
            height: rem(2.6),
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
            Node { width: rem(11.0) }
        ]
    })
}

fn slider_row(label: &str, value: f32) -> Box<dyn SceneList> {
    let label = label.to_owned();
    let value_label = format!("{:.0}%", (value * 100.0).round());
    Box::new(bsn! {
        Node {
            width: percent(100),
            height: rem(2.6),
            align_items: AlignItems::Center,
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
            on(slider_self_update)
            --
            Node { width: rem(3.2), justify_content: JustifyContent::End }
            Children [
                ScaleValueText
                Text({value_label})
                TextFont { font_size: FontSize::Rem(FONT_BODY) }
                ThemeTextColor(tokens::TEXT_SECONDARY) ThemedText
            ]
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

/// Apply the initial tab selection once a settings tab strip has been spawned.
fn init_tab_selection(
    mut lists: Query<(&Children, &mut SelectedTab), Added<TabList>>,
    state: Res<SettingsTabState>,
) {
    for (children, mut selected) in &mut lists {
        if selected.0.is_some() {
            continue;
        }
        if let Some(tab) = children.get(state.index) {
            selected.0 = Some(*tab);
        }
    }
}

/// Paint the selected tab with a tinted background and an accent underline.
#[allow(clippy::type_complexity)]
fn style_selected_tabs(
    mut commands: Commands,
    mut tabs: Query<
        (
            Entity,
            Has<Selected>,
            &Children,
            Option<&TabStyling>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<SettingsTab>,
    >,
    texts: Query<(), With<Text>>,
) {
    for (entity, selected, children, styling, mut background, mut border) in &mut tabs {
        if styling.map(|styling| styling.0) == Some(selected) {
            continue;
        }
        if selected {
            commands.entity(entity).insert((
                ThemeBackgroundColor(tokens::TAB_BG_ACTIVE),
                ThemeBorderColor(tokens::TEXT_ACCENT),
                TabStyling(true),
            ));
        } else {
            commands.entity(entity).remove::<ThemeBackgroundColor>();
            commands.entity(entity).remove::<ThemeBorderColor>();
            commands.entity(entity).insert(TabStyling(false));
            background.0 = Color::NONE;
            border.top = Color::NONE;
            border.right = Color::NONE;
            border.bottom = Color::NONE;
            border.left = Color::NONE;
        }
        for child in children.iter() {
            if texts.get(child).is_err() {
                continue;
            }
            let token = if selected {
                tokens::TAB_TEXT_ACTIVE
            } else {
                tokens::TEXT_MAIN
            };
            commands.entity(child).insert(ThemeTextColor(token));
        }
    }
}

/// Feathers draws the slider value centered inside the track, which collides
/// with the filled portion. Hide it and rely on the dedicated label instead.
fn hide_builtin_slider_text(
    mut commands: Commands,
    sliders: Query<&Children, Added<FeathersSlider>>,
    children: Query<&Children>,
    texts: Query<Entity, With<Text>>,
) {
    for slider_children in &sliders {
        let mut stack: Vec<Entity> = slider_children.iter().collect();
        while let Some(entity) = stack.pop() {
            if texts.get(entity).is_ok() {
                commands.entity(entity).insert(Visibility::Hidden);
            }
            if let Ok(kids) = children.get(entity) {
                stack.extend(kids.iter());
            }
        }
    }
}

fn update_scale_label(settings: Res<Settings>, mut labels: Query<&mut Text, With<ScaleValueText>>) {
    if !settings.is_changed() {
        return;
    }
    let label = format!("{:.0}%", settings.ui_scale_factor() * 100.0);
    for mut text in &mut labels {
        if text.0 != label {
            text.0 = label.clone();
        }
    }
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
