// SPDX-License-Identifier: GPL-3.0-or-later
//
// Modal dialogs built on FeathersDialog. A single system mirrors
// `GameSession::dialog` into entities, so opening/closing is just state.

use crate::assets::GameAssets;
use crate::state::{DialogKind, GameSession};
use crate::storage::{Scores, Settings};
use crate::theme::{ThemePalette, tokens};
use crate::ui::actions::UiAction;
use crate::ui::animation::UiTransition;
use crate::ui::scores as scores_ui;
use crate::ui::settings_dialog::{self, SettingsTabState};
use crate::ui::status::format_time;
use crate::ui::style::{self, FONT_BODY, FONT_CAPTION, FONT_SUBTITLE, FONT_TITLE};
use bevy::feathers::controls::{
    ButtonVariant, FeathersDialog, FeathersDialogBody, FeathersDialogClose, FeathersDialogFooter,
    FeathersDialogHeader,
};
use bevy::feathers::theme::{ThemeTextColor, ThemedText, ThemeToken};
use bevy::prelude::*;
use bevy::ui::rem;
use bevy::ui_widgets::{ModalDialogBarrier, RequestClose};

/// Number of help pages.
pub const HELP_PAGE_COUNT: usize = 4;

const HELP_PAGES: [(&str, &str); HELP_PAGE_COUNT] = [
    (
        "目标",
        "在不触雷的情况下清空雷区。格子中的数字表示相邻八格内的地雷数量。",
    ),
    (
        "揭开格子",
        "选择未揭开的格子进行探索。揭开空白区域时，会自动清除与它相连的安全格子。",
    ),
    (
        "插旗与快速清除",
        "右键单击或长按可标记疑似地雷。当数字周围的旗帜数量与数字相同时，选择该数字即可清除周围格子。",
    ),
    (
        "操作",
        "可以使用鼠标、触摸或键盘操作。首次揭开的格子及其相邻格子始终安全。",
    ),
];

const SHORTCUTS: &str = "\
新游戏                 Ctrl+N
重新开始               Ctrl+R
暂停 / 继续            Pause
帮助                   F1
退出                   Ctrl+Q / Ctrl+W

小型 / 中型 / 大型     1 / 2 / 3
自定义                 4
移动                   方向键或 H J K L
揭开 / 清除周围        空格
插旗                   Ctrl+空格";

pub struct DialogsPlugin;

impl Plugin for DialogsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveDialog>()
            .add_systems(Update, (sync_dialog, style_dialog_barrier))
            .add_observer(on_request_close);
    }
}

/// Marker for the root entity of the currently open dialog.
#[derive(Component, Default, Clone)]
pub struct DialogRoot;

#[derive(Resource, Default)]
pub struct ActiveDialog {
    kind: Option<DialogKind>,
    page: usize,
    entity: Option<Entity>,
}

#[allow(clippy::too_many_arguments)]
fn sync_dialog(
    mut commands: Commands,
    session: Res<GameSession>,
    settings: Res<Settings>,
    scores: Res<Scores>,
    assets: Res<GameAssets>,
    palette: Res<ThemePalette>,
    tab: Res<SettingsTabState>,
    mut active: ResMut<ActiveDialog>,
) {
    let kind = session.dialog;
    let page = session.help_page;
    if active.kind == kind && active.page == page {
        return;
    }
    if let Some(entity) = active.entity.take() {
        commands.entity(entity).despawn();
    }
    active.kind = kind;
    active.page = page;
    let Some(kind) = kind else {
        return;
    };
    let scene = dialog_scene(
        kind, &settings, &scores, &assets, &palette, &session, tab.index,
    );
    let entity = commands.spawn_scene(scene).insert(DialogRoot).id();
    active.entity = Some(entity);
}

fn style_dialog_barrier(
    palette: Res<ThemePalette>,
    all: Query<Entity, With<ModalDialogBarrier>>,
    added: Query<Entity, Added<ModalDialogBarrier>>,
    mut commands: Commands,
) {
    // Refresh every barrier when the palette changes (live theme switch), and
    // style each barrier as it spawns.
    if palette.is_changed() {
        for entity in &all {
            commands
                .entity(entity)
                .insert(BackgroundColor(palette.0.scrim));
        }
    } else {
        for entity in &added {
            commands
                .entity(entity)
                .insert(BackgroundColor(palette.0.scrim));
        }
    }
}

fn on_request_close(
    _close: On<RequestClose>,
    mut session: ResMut<GameSession>,
    mut guard: ResMut<crate::state::EscapeGuard>,
) {
    if session.dialog.is_some() {
        guard.0 = 3;
        session.close_dialog();
    }
}

#[allow(clippy::too_many_arguments)]
fn dialog_scene(
    kind: DialogKind,
    settings: &Settings,
    scores: &Scores,
    assets: &GameAssets,
    palette: &ThemePalette,
    session: &GameSession,
    tab: usize,
) -> impl Scene {
    let contents = dialog_contents(kind, settings, scores, assets, palette, session, tab);
    let width = match kind {
        DialogKind::Settings => 600.0,
        DialogKind::Help => 620.0,
        DialogKind::Scores => 560.0,
        DialogKind::Shortcuts => 500.0,
        _ => 440.0,
    };
    let transition = UiTransition::dialog(!settings.reduced_motion);
    bsn! {
        @FeathersDialog {
            @width: px(width),
            @contents: bsn_list! { {contents} },
        }
        UiTransform::IDENTITY
        UiTransition {
            elapsed: {transition.elapsed},
            delay: {transition.delay},
            duration: {transition.duration},
            from_scale: {transition.from_scale},
            from_y: {transition.from_y},
            easing: {transition.easing},
        }
        DialogRoot
    }
}

#[allow(clippy::too_many_arguments)]
fn dialog_contents(
    kind: DialogKind,
    settings: &Settings,
    scores: &Scores,
    assets: &GameAssets,
    palette: &ThemePalette,
    session: &GameSession,
    tab: usize,
) -> Box<dyn SceneList> {
    let _ = (assets, palette);
    match kind {
        DialogKind::Confirm(_) => frame(
            "开始新游戏？",
            Box::new(bsn! {
                Node { flex_direction: FlexDirection::Column, row_gap: rem(0.4) }
                Children [
                    {line("当前游戏进度将会丢失。", FONT_BODY, tokens::TEXT_SECONDARY)}
                ]
            }),
            footer_buttons(vec![
                button("取消", UiAction::CloseDialog, false),
                button("离开游戏", UiAction::AcceptConfirm, true),
            ]),
        ),
        DialogKind::Victory => {
            let seconds = session.clock.elapsed().as_secs_f64().round() as u64;
            let mut rows: Vec<Box<dyn SceneList>> = vec![line(
                format!("你用 {} 清空了雷区。", format_time(seconds)),
                FONT_BODY,
                tokens::TEXT_SECONDARY,
            )];
            if let Some(board) = session.board.as_ref()
                && let Some(best) = scores.best(board.width, board.height, board.mine_count)
            {
                rows.push(line(
                    format!("最佳成绩：{}", format_time(best)),
                    FONT_CAPTION,
                    tokens::TEXT_SECONDARY,
                ));
            }
            frame(
                "胜利！",
                Box::new(bsn! {
                    Node { flex_direction: FlexDirection::Column, row_gap: rem(0.4) }
                    Children [{rows}]
                }),
                footer_buttons(vec![
                    button(
                        "新游戏",
                        UiAction::Pending(crate::state::PendingAction::ShowNewGame),
                        false,
                    ),
                    button(
                        "再玩一次",
                        UiAction::Pending(crate::state::PendingAction::Restart),
                        true,
                    ),
                ]),
            )
        }
        DialogKind::Scores => frame(
            "成绩",
            scores_ui::contents(scores, palette),
            footer_buttons(vec![button("关闭", UiAction::CloseDialog, true)]),
        ),
        DialogKind::Settings => frame(
            "设置",
            settings_dialog::contents(settings, palette, tab),
            footer_buttons(vec![button("完成", UiAction::CloseDialog, true)]),
        ),
        DialogKind::Help => {
            let page = session.help_page.min(HELP_PAGE_COUNT - 1);
            let (title, text) = HELP_PAGES[page];
            let counter = format!("第 {} 页，共 {} 页", page + 1, HELP_PAGE_COUNT);
            let mut pager = Vec::new();
            if page == 0 {
                pager.push(style::disabled_button("上一页"));
            } else {
                pager.push(button("上一页", UiAction::HelpPrevious, false));
            }
            if page + 1 >= HELP_PAGE_COUNT {
                pager.push(style::disabled_button("下一页"));
            } else {
                pager.push(button("下一页", UiAction::HelpNext, false));
            }
            pager.push(button("关闭", UiAction::CloseDialog, true));
            frame(
                "扫雷帮助",
                Box::new(bsn! {
                    Node { flex_direction: FlexDirection::Column, row_gap: rem(0.5) }
                    Children [
                        {line(title, FONT_SUBTITLE, tokens::TEXT_MAIN)}
                        --
                        {line(text, FONT_BODY, tokens::TEXT_SECONDARY)}
                        --
                        {line(counter, FONT_CAPTION, tokens::TEXT_SECONDARY)}
                    ]
                }),
                footer_buttons(pager),
            )
        }
        DialogKind::Shortcuts => frame(
            "键盘快捷键",
            Box::new(bsn! {
                Node { flex_direction: FlexDirection::Column, row_gap: rem(0.3) }
                Children [
                    {line(SHORTCUTS, FONT_CAPTION, tokens::TEXT_SECONDARY)}
                ]
            }),
            footer_buttons(vec![button("关闭", UiAction::CloseDialog, true)]),
        ),
        DialogKind::About => frame(
            "关于扫雷",
            Box::new(bsn! {
                Node { flex_direction: FlexDirection::Column, row_gap: rem(0.4) }
                Children [
                    {line("扫雷 50.0", FONT_TITLE, tokens::TEXT_MAIN)}
                    --
                    {line(
                        "使用 Bevy 0.20 原生 UI、BSN 场景、Feathers 控件和应用设置完整复刻 GNOME 扫雷。",
                        FONT_BODY,
                        tokens::TEXT_SECONDARY,
                    )}
                    --
                    {line(
                        "美术资源来自 GNOME Mines，采用 GPL-3.0-or-later 许可证。",
                        FONT_CAPTION,
                        tokens::TEXT_SECONDARY,
                    )}
                ]
            }),
            footer_buttons(vec![button("关闭", UiAction::CloseDialog, true)]),
        ),
    }
}

fn frame(
    title: impl Into<String>,
    body: Box<dyn SceneList>,
    footer: Box<dyn SceneList>,
) -> Box<dyn SceneList> {
    let title = title.into();
    Box::new(bsn_list! {
        @FeathersDialogHeader
        Children [
            Text({title})
            TextFont { font_size: FontSize::Rem(FONT_TITLE) }
            ThemeTextColor(tokens::TEXT_MAIN) ThemedText
            --
            @FeathersDialogClose
        ]
        --
        @FeathersDialogBody
        Children [ {body} ]
        --
        @FeathersDialogFooter
        Children [ {footer} ]
    })
}

fn footer_buttons(items: Vec<Box<dyn SceneList>>) -> Box<dyn SceneList> {
    Box::new(bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::End,
            column_gap: rem(0.4),
            flex_wrap: FlexWrap::Wrap,
        }
        Children [{items}]
    })
}

fn button(label: &str, action: UiAction, primary: bool) -> Box<dyn SceneList> {
    let variant = if primary {
        ButtonVariant::Primary
    } else {
        ButtonVariant::Normal
    };
    style::text_button(label, action, variant)
}

fn line(content: impl Into<String>, size: f32, token: ThemeToken) -> Box<dyn SceneList> {
    let content = content.into();
    Box::new(bsn! {
        Text({content})
        TextFont { font_size: FontSize::Rem(size) }
        ThemeTextColor({token}) ThemedText
    })
}
