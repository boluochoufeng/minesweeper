// SPDX-License-Identifier: GPL-3.0-or-later
//
// The game screen. The board is spawned once per game and then only the cells
// that actually changed are updated, so clicking never rebuilds the UI tree.

use crate::assets::{
    BoardStyle, GLYPH_EXPLODED, GLYPH_FLAG, GLYPH_INCORRECT, GLYPH_MAYBE, GLYPH_MINE, GameAssets,
    legacy,
};
use crate::game::{Board, Cell, Mark, Outcome};
use crate::state::{CellAction, GameSession, PendingAction, Screen};
use crate::storage::{Scores, Settings};
use crate::theme::{tokens, ThemePalette};
use crate::ui::actions::{ActionTarget, UiAction};
use crate::ui::animation::UiTransition;
use crate::ui::menu;
use crate::ui::shell::UiMetrics;
use crate::ui::status;
use crate::ui::style::{self, FONT_DISPLAY, RADIUS_CELL};
use bevy::feathers::controls::{ButtonVariant, FeathersButton};
use bevy::feathers::display::ThemedIcon;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText};
use bevy::picking::events::{PointerClick, PointerPress, PointerRelease};
use bevy::prelude::*;
use bevy::text::LineHeight;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui::rem;
use bevy::window::PrimaryWindow;
use std::time::{Duration, Instant};

const LONG_PRESS: Duration = Duration::from_millis(500);

/// Marker + index for a clickable cell.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CellIndex {
    pub index: usize,
}

/// Marker for the grid container that holds the cells.
#[derive(Component, Default, Clone)]
pub struct BoardGrid;

/// Entity handles and change-tracking state for the live board.
#[derive(Resource)]
pub struct BoardView {
    pub grid: Entity,
    pub cells: Vec<Entity>,
    pub last: Vec<Cell>,
    pub last_cursor: Option<usize>,
    pub last_cell_size: f32,
    pub last_outcome: Outcome,
    pub style: BoardStyle,
}

/// The pause overlay entity, when shown.
#[derive(Resource, Default)]
pub struct PauseOverlay(pub Option<Entity>);

/// Marks the header button glyph so it can flip between pause and play.
#[derive(Component, Default, Clone)]
pub struct PauseHeaderIcon;

/// The root entity of the game screen, used to rebuild the board on style change.
#[derive(Resource, Default)]
pub struct BoardScreenRoot(pub Option<Entity>);

/// Active touch long-press tracking.
#[derive(Resource, Default)]
pub struct TouchHold(pub Option<(u64, usize, Instant)>);

pub struct BoardScreenPlugin;

impl Plugin for BoardScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PauseOverlay>()
            .init_resource::<BoardScreenRoot>()
            .init_resource::<TouchHold>()
            .add_systems(OnEnter(Screen::Playing), spawn_game_screen)
            .add_systems(OnExit(Screen::Playing), clear_board_state)
            .add_systems(
                Update,
                (
                    setup_board,
                    rebuild_for_style_change,
                    fit_board,
                    sync_board,
                    sync_pause_overlay,
                    sync_pause_icon,
                )
                    .chain(),
            )
            .add_observer(cell_clicked)
            .add_observer(cell_pressed)
            .add_observer(cell_released);
    }
}

fn clear_board_state(mut commands: Commands, mut overlay: ResMut<PauseOverlay>, mut root: ResMut<BoardScreenRoot>) {
    commands.remove_resource::<BoardView>();
    root.0 = None;
    overlay.0 = None;
}

fn spawn_game_screen(
    mut commands: Commands,
    assets: Res<GameAssets>,
    settings: Res<Settings>,
    session: Res<GameSession>,
    metrics: Res<UiMetrics>,
    mut root: ResMut<BoardScreenRoot>,
) {
    let Some(board) = session.board.as_ref() else {
        return;
    };
    let style = BoardStyle::parse(&settings.theme);
    let entity = commands
        .spawn_scene(game_screen(&assets, &settings, &session, board, style, &metrics))
        .insert(DespawnOnExit(Screen::Playing))
        .id();
    root.0 = Some(entity);
}

#[allow(clippy::too_many_arguments)]
fn game_screen(
    assets: &GameAssets,
    settings: &Settings,
    session: &GameSession,
    board: &Board,
    _style: BoardStyle,
    _metrics: &UiMetrics,
) -> impl Scene {
    let paused = session.paused;
    let pause_action = UiAction::TogglePause;
    let subtitle = format!(
        "{} · {} × {} · {} 雷",
        settings.difficulty.label(),
        board.width,
        board.height,
        board.mine_count
    );
    let back = style::icon_button(
        assets.icons.back.clone(),
        UiAction::Pending(PendingAction::ShowNewGame),
    );
    let restart = style::icon_button(
        assets.icons.restart.clone(),
        UiAction::Pending(PendingAction::Restart),
    );
    let pause_icon = if paused {
        assets.icons.play.clone()
    } else {
        assets.icons.pause.clone()
    };
    let pause = pause_header_button(pause_icon, pause_action);
    let header = menu::header(
        "扫雷".to_owned(),
        Some(subtitle),
        vec![back],
        vec![restart, pause],
        assets,
    );
    let status = status::status_bar(assets, session);
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
                row_gap: rem(0.9),
                padding: rem(1.0),
            }
            Children [
                Node {
                    padding: rem(0.35),
                    border: px(1),
                    border_radius: BorderRadius::all(rem(0.7)),
                }
                ThemeBackgroundColor(tokens::SURFACE_BG)
                ThemeBorderColor(tokens::SURFACE_BORDER)
                Children [
                    Node {
                        display: Display::Grid,
                        column_gap: px(0),
                        row_gap: px(0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                    }
                    BoardGrid
                ]
                --
                {status}
            ]
        ]
    }
}

fn pause_header_button(icon: Handle<Image>, action: UiAction) -> Box<dyn SceneList> {
    style::boxed_list(bsn_list! { @pause_header_button_scene(icon, action) })
}

fn pause_header_button_scene(icon: Handle<Image>, action: UiAction) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! {
                ImageNode { image: {icon} }
                Node { width: rem(1.15), height: rem(1.15) }
                ThemedIcon
                PauseHeaderIcon
            },
            @variant: ButtonVariant::Plain,
        }
        ActionTarget { action: {action} }
    }
}

fn setup_board(
    mut commands: Commands,
    grids: Query<Entity, Added<BoardGrid>>,
    session: Res<GameSession>,
    settings: Res<Settings>,
    assets: Res<GameAssets>,
    palette: Res<ThemePalette>,
) {
    let Some(board) = session.board.as_ref() else {
        return;
    };
    let style = BoardStyle::parse(&settings.theme);
    for grid in &grids {
        let mut cells = Vec::with_capacity(board.cells.len());
        for index in 0..board.cells.len() {
            let entity = spawn_cell(
                &mut commands,
                board,
                index,
                style,
                &palette.0,
                &assets,
            );
            cells.push(entity);
        }
        commands.entity(grid).add_children(&cells);
        commands
            .entity(grid)
            .insert(BackgroundColor(grid_background(style, &palette.0)));
        commands.insert_resource(BoardView {
            grid,
            cells,
            last: board.cells.clone(),
            last_cursor: None,
            last_cell_size: 0.0,
            last_outcome: board.outcome,
            style,
        });
    }
}

fn grid_background(
    style: BoardStyle,
    palette: &crate::theme::palette::Palette,
) -> Color {
    // Fills the tiny gaps left between rounded cell corners so the grid reads as
    // a continuous lattice instead of exposing the board surface underneath.
    if style.is_modern() {
        palette.cell_border
    } else {
        legacy::BORDER_HIDDEN
    }
}

fn spawn_cell(
    commands: &mut Commands,
    board: &Board,
    index: usize,
    style: BoardStyle,
    palette: &crate::theme::palette::Palette,
    assets: &GameAssets,
) -> Entity {
    let cell = board.cells[index];
    let (background, border, width) = cell_colors(cell, board, index, style, palette, false);
    let radius = if style.is_modern() {
        rem(RADIUS_CELL)
    } else {
        px(0.0)
    };
    let content = cell_content(cell, board, index, style, palette, assets);
    let mut entity = commands.spawn((
        CellIndex { index },
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(width)),
            border_radius: BorderRadius::all(radius),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(border),
    ));
    entity.queue_spawn_related_scenes::<Children>(content);
    entity.id()
}

fn cell_glyph(cell: Cell, board: &Board, index: usize) -> Option<usize> {
    if board.outcome == Outcome::Lost {
        if board.exploded_cell == Some(index) {
            return Some(GLYPH_EXPLODED);
        }
        if cell.mark == Mark::Flag && !cell.mine {
            return Some(GLYPH_INCORRECT);
        }
        if cell.mine && cell.mark != Mark::Flag {
            return Some(GLYPH_MINE);
        }
    }
    if cell.revealed {
        if cell.mine {
            Some(GLYPH_MINE)
        } else {
            None
        }
    } else {
        match cell.mark {
            Mark::None => None,
            Mark::Flag => Some(GLYPH_FLAG),
            Mark::Maybe => Some(GLYPH_MAYBE),
        }
    }
}

fn glyph_tint(
    glyph: usize,
    style: BoardStyle,
    palette: &crate::theme::palette::Palette,
) -> Color {
    if !style.is_modern() {
        return Color::WHITE;
    }
    match glyph {
        GLYPH_FLAG => palette.accent,
        GLYPH_INCORRECT => palette.danger,
        GLYPH_MAYBE => palette.text_secondary,
        GLYPH_EXPLODED => palette.danger,
        _ => palette.text_primary,
    }
}

fn cell_content(
    cell: Cell,
    board: &Board,
    index: usize,
    style: BoardStyle,
    palette: &crate::theme::palette::Palette,
    assets: &GameAssets,
) -> Box<dyn SceneList> {
    if let Some(glyph) = cell_glyph(cell, board, index) {
        let handle = if style.is_modern() {
            assets.modern_glyph(glyph)
        } else {
            assets.classic_glyph(style, glyph)
        };
        let tint = glyph_tint(glyph, style, palette);
        return Box::new(bsn_list! {
            ImageNode { image: {handle}, color: {tint} }
            Node { width: percent(68), height: percent(68) }
            Pickable::IGNORE
        });
    }
    if cell.revealed && !cell.mine && cell.adjacent > 0 {
        let number = cell.adjacent.to_string();
        let color = if style.is_modern() {
            palette.number_colors[(cell.adjacent - 1) as usize]
        } else {
            legacy::number_color(cell.adjacent)
        };
        let max_dim = board.width.max(board.height).max(1);
        let font_size = FontSize::VMin(46.0 / max_dim as f32);
        return Box::new(bsn_list! {
            Text({number})
            TextFont { font_size: {font_size} }
            LineHeight::RelativeToFont(0.85)
            TextColor({color})
            Pickable::IGNORE
        });
    }
    Box::new(bsn_list! {})
}

fn cell_colors(
    cell: Cell,
    board: &Board,
    index: usize,
    style: BoardStyle,
    palette: &crate::theme::palette::Palette,
    selected: bool,
) -> (Color, Color, f32) {
    if style.is_modern() {
        let background = if board.outcome == Outcome::Lost && board.exploded_cell == Some(index) {
            palette.cell_exploded_bg
        } else if board.outcome == Outcome::Lost && cell.mark == Mark::Flag && !cell.mine {
            palette.cell_wrong_bg
        } else if cell.revealed {
            palette.cell_revealed
        } else {
            palette.cell_hidden
        };
        let border = if selected {
            palette.cell_cursor
        } else {
            palette.cell_border
        };
        let width = if selected {
            2.5
        } else if palette.high_contrast {
            1.5
        } else {
            1.0
        };
        (background, border, width)
    } else {
        let background = if cell.revealed {
            legacy::REVEALED
        } else {
            legacy::HIDDEN
        };
        let border = if selected {
            legacy::CURSOR
        } else if cell.revealed {
            legacy::BORDER_REVEALED
        } else {
            legacy::BORDER_HIDDEN
        };
        let width = if selected { 2.5 } else { 0.75 };
        (background, border, width)
    }
}

fn fit_board(
    session: Res<GameSession>,
    view: Option<ResMut<BoardView>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    metrics: Res<UiMetrics>,
    mut grids: Query<&mut Node, With<BoardGrid>>,
) {
    let Some(mut view) = view else {
        return;
    };
    let Some(board) = session.board.as_ref() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.scale_factor();
    let available_width =
        (window.width() - 2.0 * metrics.content_pad()).max(120.0);
    let available_height = (window.height()
        - metrics.header_height()
        - metrics.status_height()
        - 3.0 * metrics.content_pad())
    .max(120.0);
    let cell = ((available_width / board.width as f32)
        .min(available_height / board.height as f32)
        * scale)
        .floor()
        .max(6.0)
        / scale;
    if (cell - view.last_cell_size).abs() < 0.5 {
        return;
    }
    view.last_cell_size = cell;
    let width = cell * board.width as f32;
    let height = cell * board.height as f32;
    if let Ok(mut node) = grids.get_mut(view.grid) {
        node.width = px(width);
        node.height = px(height);
        node.grid_template_columns =
            RepeatedGridTrack::px::<Vec<RepeatedGridTrack>>(board.width as u16, cell);
        node.grid_template_rows =
            RepeatedGridTrack::px::<Vec<RepeatedGridTrack>>(board.height as u16, cell);
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_board(
    session: Res<GameSession>,
    settings: Res<Settings>,
    palette: Res<ThemePalette>,
    assets: Res<GameAssets>,
    view: Option<ResMut<BoardView>>,
    mut commands: Commands,
    mut cells: Query<
        (&CellIndex, &mut BackgroundColor, &mut BorderColor, &mut Node),
        Without<BoardGrid>,
    >,
    mut grids: Query<&mut BackgroundColor, (With<BoardGrid>, Without<CellIndex>)>,
) {
    let Some(mut view) = view else {
        return;
    };
    let Some(board) = session.board.as_ref() else {
        return;
    };
    if board.cells.len() != view.cells.len() {
        return;
    }
    let reduced_motion = settings.reduced_motion;
    let palette_changed = palette.is_changed();
    // The end of a game repaints every cell (mines surface, wrong flags are
    // marked), even though most cells keep their own state.
    let outcome_changed = board.outcome != view.last_outcome;
    let cursor = if session.keyboard_active {
        Some(session.keyboard_cursor.min(board.cells.len().saturating_sub(1)))
    } else {
        None
    };
    if palette_changed
        && let Ok(mut grid_bg) = grids.get_mut(view.grid)
    {
        grid_bg.0 = grid_background(view.style, &palette.0);
    }

    for index in 0..board.cells.len() {
        let cell = board.cells[index];
        let changed = palette_changed || outcome_changed || cell != view.last[index] || {
            let was_selected = view.last_cursor == Some(index);
            let is_selected = cursor == Some(index);
            was_selected != is_selected
        };
        if !changed {
            continue;
        }
        let selected = cursor == Some(index);
        let (background, border, width) =
            cell_colors(cell, board, index, view.style, &palette.0, selected);
        if let Ok((_, mut background_color, mut border_color, mut node)) =
            cells.get_mut(view.cells[index])
        {
            if background_color.0 != background {
                background_color.0 = background;
            }
            if border_color.top != border {
                *border_color = BorderColor::all(border);
            }
            if node.border.top != px(width) {
                node.border = UiRect::all(px(width));
            }
        }
        let was_changed_state = cell != view.last[index] || outcome_changed || palette_changed;
        if was_changed_state {
            let content = cell_content(cell, board, index, view.style, &palette.0, &assets);
            commands.entity(view.cells[index]).despawn_children();
            commands
                .entity(view.cells[index])
                .queue_spawn_related_scenes::<Children>(content);
            if !reduced_motion {
                let exploded = board.exploded_cell == Some(index);
                commands
                    .entity(view.cells[index])
                    .insert(UiTransition::cell(true, exploded));
            }
        }
    }

    view.last = board.cells.clone();
    view.last_cursor = cursor;
    view.last_outcome = board.outcome;
}

fn sync_pause_overlay(
    mut commands: Commands,
    session: Res<GameSession>,
    assets: Res<GameAssets>,
    palette: Res<ThemePalette>,
    mut overlay: ResMut<PauseOverlay>,
) {
    let should_show = session.dialog.is_none()
        && session.paused
        && session
            .board
            .as_ref()
            .is_some_and(|board| board.outcome == Outcome::InProgress);
    if should_show && overlay.0.is_none() {
        let entity = commands
            .spawn_scene(pause_overlay(&assets, &palette))
            .insert((DespawnOnExit(Screen::Playing), GlobalZIndex(50)))
            .id();
        overlay.0 = Some(entity);
    } else if !should_show
        && let Some(entity) = overlay.0.take()
    {
        commands.entity(entity).despawn();
    }
}

/// Keep the header button glyph in sync with the pause state.
fn sync_pause_icon(
    session: Res<GameSession>,
    assets: Res<GameAssets>,
    mut icons: Query<&mut ImageNode, With<PauseHeaderIcon>>,
) {
    let want = if session.paused {
        assets.icons.play.clone()
    } else {
        assets.icons.pause.clone()
    };
    for mut node in &mut icons {
        if node.image != want {
            node.image = want.clone();
        }
    }
}

fn pause_overlay(_assets: &GameAssets, palette: &ThemePalette) -> impl Scene {
    let resume = style::text_button("继续游戏", UiAction::TogglePause, ButtonVariant::Primary);
        bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        BackgroundColor({palette.0.scrim})
        Children [
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: rem(0.8),
                padding: UiRect::axes(rem(2.4), rem(1.6)),
                border: px(1),
                border_radius: BorderRadius::all(rem(0.9)),
            }
            ThemeBackgroundColor(tokens::SURFACE_BG)
            ThemeBorderColor(tokens::SURFACE_BORDER)
            Children [
                Text("已暂停")
                TextFont { font_size: FontSize::Rem(FONT_DISPLAY) }
                ThemeTextColor(tokens::TEXT_MAIN) ThemedText
                --
                {resume}
            ]
        ]
    }
}

fn cell_clicked(
    mut click: On<PointerClick>,
    cells: Query<&CellIndex>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<GameSession>,
    settings: Res<Settings>,
    mut scores: ResMut<Scores>,
) {
    if click.pointer.id.is_touch() {
        return;
    }
    let Ok(target) = cells.get(click.entity) else {
        return;
    };
    click.propagate(false);
    session.keyboard_active = false;
    let Some(board) = session.board.as_ref() else {
        return;
    };
    let (x, y) = board.coords(target.index);
    let ctrl_pressed =
        keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let action = match click.button {
        PointerButton::Secondary => CellAction::Mark(x, y),
        PointerButton::Primary if ctrl_pressed => CellAction::Mark(x, y),
        PointerButton::Primary if board.cells[target.index].revealed => CellAction::Chord(x, y),
        PointerButton::Primary => CellAction::Reveal(x, y),
        _ => return,
    };
    session.apply_cell_action(action, &settings);
    session.process_outcome(&mut scores);
}

fn cell_pressed(
    press: On<PointerPress>,
    cells: Query<&CellIndex>,
    mut hold: ResMut<TouchHold>,
) {
    let Some(touch_id) = press.pointer.id.get_touch_id() else {
        return;
    };
    let Ok(target) = cells.get(press.entity) else {
        return;
    };
    hold.0 = Some((touch_id, target.index, Instant::now()));
}

fn cell_released(
    mut release: On<PointerRelease>,
    cells: Query<&CellIndex>,
    mut hold: ResMut<TouchHold>,
    mut session: ResMut<GameSession>,
    settings: Res<Settings>,
    mut scores: ResMut<Scores>,
) {
    let Some(touch_id) = release.pointer.id.get_touch_id() else {
        return;
    };
    let Ok(target) = cells.get(release.entity) else {
        return;
    };
    let Some((held_id, held_index, started)) = hold.0.take() else {
        return;
    };
    if held_id != touch_id || held_index != target.index {
        return;
    }
    release.propagate(false);
    session.keyboard_active = false;
    let Some(board) = session.board.as_ref() else {
        return;
    };
    let (x, y) = board.coords(target.index);
    let action = if started.elapsed() >= LONG_PRESS {
        CellAction::Mark(x, y)
    } else if board.cells[target.index].revealed {
        CellAction::Chord(x, y)
    } else {
        CellAction::Reveal(x, y)
    };
    session.apply_cell_action(action, &settings);
    session.process_outcome(&mut scores);
}

#[allow(clippy::too_many_arguments)]
fn rebuild_for_style_change(
    mut commands: Commands,
    settings: Res<Settings>,
    session: Res<GameSession>,
    assets: Res<GameAssets>,
    metrics: Res<UiMetrics>,
    view: Option<Res<BoardView>>,
    mut root: ResMut<BoardScreenRoot>,
    mut overlay: ResMut<PauseOverlay>,
) {
    let Some(view) = view else {
        return;
    };
    if !settings.is_changed() {
        return;
    }
    let style = BoardStyle::parse(&settings.theme);
    if style == view.style {
        return;
    }
    let Some(board) = session.board.as_ref() else {
        return;
    };
    if let Some(entity) = root.0.take() {
        commands.entity(entity).despawn();
    }
    if let Some(entity) = overlay.0.take() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<BoardView>();
    let entity = commands
        .spawn_scene(game_screen(
            &assets, &settings, &session, board, style, &metrics,
        ))
        .insert(DespawnOnExit(Screen::Playing))
        .id();
    root.0 = Some(entity);
}
