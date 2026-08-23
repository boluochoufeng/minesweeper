// SPDX-License-Identifier: GPL-3.0-or-later
//
// Native Bevy 0.19 UI. The interface is built with BSN scenes and Feathers
// scene components; the board rules remain in the engine-agnostic game module.

use crate::game::{Board, GameClock, Mark, Outcome};
use crate::storage::{Difficulty, PersistentData, Settings};
use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    feathers::{
        FeathersPlugins,
        controls::{
            ButtonVariant, FeathersButton, FeathersCheckbox, FeathersNumberInput, NumberFormat,
            NumberInputValue, UpdateNumberInput,
        },
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens,
    },
    input_focus::InputFocus,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    settings::{SaveSettingsDeferred, SaveSettingsSync},
    text::{EditableText, FontSource, TextEdit},
    ui::Checked,
    ui_widgets::{Activate, ValueChange, checkbox_self_update},
    window::{PrimaryWindow, WindowCloseRequested, WindowFocused, WindowResized},
};
use rand::rng;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use winit::window::Icon;

const THEMES: [(&str, &str); 4] = [
    ("bgcolors", "背景颜色"),
    ("classic", "经典"),
    ("default", "默认"),
    ("modern", "现代"),
];
const LONG_PRESS: Duration = Duration::from_millis(500);
const NUMBER_COLORS: [(&str, [u8; 3]); 8] = [
    ("#1a5fb4", [0x1a, 0x5f, 0xb4]),
    ("#218b55", [0x21, 0x8b, 0x55]),
    ("#c01c28", [0xc0, 0x1c, 0x28]),
    ("#613583", [0x61, 0x35, 0x83]),
    ("#a34700", [0xa3, 0x47, 0x00]),
    ("#087990", [0x08, 0x79, 0x90]),
    ("#9c2b75", [0x9c, 0x2b, 0x75]),
    ("#3d3846", [0x3d, 0x38, 0x46]),
];

const TILE_SVGS: [[&[u8]; 13]; 4] = [
    [
        include_bytes!("../assets/themes/bgcolors/1mines.svg"),
        include_bytes!("../assets/themes/bgcolors/2mines.svg"),
        include_bytes!("../assets/themes/bgcolors/3mines.svg"),
        include_bytes!("../assets/themes/bgcolors/4mines.svg"),
        include_bytes!("../assets/themes/bgcolors/5mines.svg"),
        include_bytes!("../assets/themes/bgcolors/6mines.svg"),
        include_bytes!("../assets/themes/bgcolors/7mines.svg"),
        include_bytes!("../assets/themes/bgcolors/8mines.svg"),
        include_bytes!("../assets/themes/bgcolors/mine.svg"),
        include_bytes!("../assets/themes/bgcolors/exploded.svg"),
        include_bytes!("../assets/themes/bgcolors/flag.svg"),
        include_bytes!("../assets/themes/bgcolors/incorrect.svg"),
        include_bytes!("../assets/themes/bgcolors/maybe.svg"),
    ],
    [
        include_bytes!("../assets/themes/classic/1mines.svg"),
        include_bytes!("../assets/themes/classic/2mines.svg"),
        include_bytes!("../assets/themes/classic/3mines.svg"),
        include_bytes!("../assets/themes/classic/4mines.svg"),
        include_bytes!("../assets/themes/classic/5mines.svg"),
        include_bytes!("../assets/themes/classic/6mines.svg"),
        include_bytes!("../assets/themes/classic/7mines.svg"),
        include_bytes!("../assets/themes/classic/8mines.svg"),
        include_bytes!("../assets/themes/classic/mine.svg"),
        include_bytes!("../assets/themes/classic/exploded.svg"),
        include_bytes!("../assets/themes/classic/flag.svg"),
        include_bytes!("../assets/themes/classic/incorrect.svg"),
        include_bytes!("../assets/themes/classic/maybe.svg"),
    ],
    [
        include_bytes!("../assets/themes/default/1mines.svg"),
        include_bytes!("../assets/themes/default/2mines.svg"),
        include_bytes!("../assets/themes/default/3mines.svg"),
        include_bytes!("../assets/themes/default/4mines.svg"),
        include_bytes!("../assets/themes/default/5mines.svg"),
        include_bytes!("../assets/themes/default/6mines.svg"),
        include_bytes!("../assets/themes/default/7mines.svg"),
        include_bytes!("../assets/themes/default/8mines.svg"),
        include_bytes!("../assets/themes/default/mine.svg"),
        include_bytes!("../assets/themes/default/exploded.svg"),
        include_bytes!("../assets/themes/default/flag.svg"),
        include_bytes!("../assets/themes/default/incorrect.svg"),
        include_bytes!("../assets/themes/default/maybe.svg"),
    ],
    [
        include_bytes!("../assets/themes/modern/1mines.svg"),
        include_bytes!("../assets/themes/modern/2mines.svg"),
        include_bytes!("../assets/themes/modern/3mines.svg"),
        include_bytes!("../assets/themes/modern/4mines.svg"),
        include_bytes!("../assets/themes/modern/5mines.svg"),
        include_bytes!("../assets/themes/modern/6mines.svg"),
        include_bytes!("../assets/themes/modern/7mines.svg"),
        include_bytes!("../assets/themes/modern/8mines.svg"),
        include_bytes!("../assets/themes/modern/mine.svg"),
        include_bytes!("../assets/themes/modern/exploded.svg"),
        include_bytes!("../assets/themes/modern/flag.svg"),
        include_bytes!("../assets/themes/modern/incorrect.svg"),
        include_bytes!("../assets/themes/modern/maybe.svg"),
    ],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Screen {
    #[default]
    NewGame,
    Custom,
    Game,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum PendingAction {
    #[default]
    ShowNewGame,
    Start(Difficulty),
    Restart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConfirmState {
    action: PendingAction,
    was_paused: bool,
    was_pause_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dialog {
    Menu,
    Confirm(ConfirmState),
    Scores,
    Appearance,
    Shortcuts,
    Help,
    About,
    Victory,
}

#[derive(Debug, Clone, Copy)]
enum CellAction {
    Reveal(usize, usize),
    Mark(usize, usize),
    Chord(usize, usize),
}

#[derive(Resource)]
pub struct MinesApp {
    persistent: PersistentData,
    screen: Screen,
    board: Option<Board>,
    clock: GameClock,
    paused: bool,
    pause_requested: bool,
    auto_paused: bool,
    keyboard_cursor: usize,
    keyboard_active: bool,
    dialog: Option<Dialog>,
    help_page: usize,
    layout_dirty: bool,
    screen_transition: bool,
    dialog_transition: bool,
    animated_cells: BTreeSet<usize>,
}

impl MinesApp {
    pub fn new(
        persistent: PersistentData,
        cli_difficulty: Option<Difficulty>,
        settings: &mut Settings,
    ) -> Self {
        let mut app = Self {
            persistent,
            screen: Screen::NewGame,
            board: None,
            clock: GameClock::default(),
            paused: false,
            pause_requested: false,
            auto_paused: false,
            keyboard_cursor: 0,
            keyboard_active: false,
            dialog: None,
            help_page: 0,
            layout_dirty: true,
            screen_transition: true,
            dialog_transition: false,
            animated_cells: BTreeSet::new(),
        };
        if let Some(difficulty) = cli_difficulty {
            app.start_game(difficulty, settings);
        }
        app
    }

    fn start_game(&mut self, difficulty: Difficulty, settings: &mut Settings) {
        settings.difficulty = difficulty;
        let (width, height, mines) = difficulty.dimensions(settings);
        self.board = Some(Board::new(width, height, mines));
        self.clock = GameClock::default();
        self.paused = false;
        self.pause_requested = false;
        self.auto_paused = false;
        self.keyboard_cursor = 0;
        self.keyboard_active = false;
        self.screen = Screen::Game;
        self.dialog = None;
        self.layout_dirty = true;
        self.screen_transition = true;
        self.dialog_transition = false;
        self.animated_cells.clear();
    }

    fn execute(&mut self, action: PendingAction, settings: &mut Settings) {
        match action {
            PendingAction::ShowNewGame => {
                self.board = None;
                self.clock.pause();
                self.clock = GameClock::default();
                self.screen = Screen::NewGame;
                self.paused = false;
                self.pause_requested = false;
                self.dialog = None;
                self.layout_dirty = true;
                self.screen_transition = true;
                self.dialog_transition = false;
                self.animated_cells.clear();
            }
            PendingAction::Start(difficulty) => self.start_game(difficulty, settings),
            PendingAction::Restart => self.start_game(settings.difficulty, settings),
        }
    }

    fn request(&mut self, action: PendingAction, settings: &mut Settings) {
        let needs_confirmation = self
            .board
            .as_ref()
            .is_some_and(|board| board.revealed_count > 0 && board.outcome == Outcome::InProgress);
        if needs_confirmation {
            let state = ConfirmState {
                action,
                was_paused: self.paused,
                was_pause_requested: self.pause_requested,
            };
            self.set_paused(true, false);
            self.dialog = Some(Dialog::Confirm(state));
            self.layout_dirty = true;
            self.dialog_transition = true;
        } else {
            self.execute(action, settings);
        }
    }

    fn set_paused(&mut self, paused: bool, requested: bool) {
        if self
            .board
            .as_ref()
            .is_none_or(|board| board.outcome != Outcome::InProgress)
        {
            return;
        }
        self.paused = paused;
        if requested {
            self.pause_requested = paused;
        }
        if paused {
            self.clock.pause();
        } else {
            self.clock.resume();
        }
        self.layout_dirty = true;
    }

    fn toggle_pause(&mut self) {
        if self.clock.has_started() {
            self.set_paused(!self.paused, true);
        }
    }

    fn apply_cell_action(&mut self, action: CellAction, settings: &Settings) {
        if self.paused {
            return;
        }
        let Some(board) = self.board.as_mut() else {
            return;
        };
        let previous_cells = board.cells.clone();
        let changed = match action {
            CellAction::Reveal(x, y) => {
                let changed = board.reveal(
                    x,
                    y,
                    &mut rng(),
                    settings.auto_clear_on_flags,
                    settings.use_autoflag,
                );
                if changed && !self.clock.has_started() {
                    self.clock.start();
                }
                changed
            }
            CellAction::Mark(x, y) => board.cycle_mark(
                x,
                y,
                settings.use_question_marks,
                settings.auto_clear_on_flags,
                settings.use_autoflag,
            ),
            CellAction::Chord(x, y) => {
                board.chord(x, y, settings.auto_clear_on_flags, settings.use_autoflag)
            }
        };
        if changed {
            self.layout_dirty = true;
            self.animated_cells = previous_cells
                .iter()
                .zip(&board.cells)
                .enumerate()
                .filter_map(|(index, (before, after))| {
                    (before.revealed != after.revealed || before.mark != after.mark)
                        .then_some(index)
                })
                .collect();
        }
        self.process_outcome(settings);
    }

    fn process_outcome(&mut self, settings: &Settings) {
        let Some(board) = self.board.as_ref() else {
            return;
        };
        match board.outcome {
            Outcome::InProgress => {}
            Outcome::Lost => {
                self.clock.pause();
                self.layout_dirty = true;
            }
            Outcome::Won => {
                if matches!(self.dialog, Some(Dialog::Victory)) {
                    return;
                }
                self.clock.pause();
                let seconds = self.clock.elapsed().as_secs_f64().round() as u64;
                self.persistent.settings = settings.clone();
                self.persistent
                    .add_score(board.width, board.height, board.mine_count, seconds);
                self.dialog = Some(Dialog::Victory);
                self.layout_dirty = true;
                self.dialog_transition = true;
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct NativeWindowState {
    initialized: bool,
    save_elapsed: f32,
}

#[derive(Resource, Default)]
pub struct UiRuntime {
    root: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct TouchHold {
    active: Option<(u64, usize, Instant)>,
}

#[derive(Resource, Clone)]
struct ThemeAssets {
    themes: Vec<Vec<Handle<Image>>>,
    flag: Handle<Image>,
    stopwatch: Handle<Image>,
}

#[derive(Component, Default, Clone)]
struct UiRoot;

#[derive(Component, Default, Clone)]
pub(crate) struct ClockText;

#[derive(Component, Default, Clone)]
struct ActionTarget {
    action: UiAction,
}

#[derive(Component, Default, Clone)]
struct CellTarget {
    index: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum TransitionEasing {
    #[default]
    CubicOut,
    BackOut,
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct UiTransition {
    elapsed: f32,
    delay: f32,
    duration: f32,
    from_scale: f32,
    from_y: f32,
    easing: TransitionEasing,
}

impl Default for UiTransition {
    fn default() -> Self {
        Self::none()
    }
}

impl UiTransition {
    const fn none() -> Self {
        Self {
            elapsed: 0.0,
            delay: 0.0,
            duration: 0.0,
            from_scale: 1.0,
            from_y: 0.0,
            easing: TransitionEasing::CubicOut,
        }
    }

    const fn screen(enabled: bool) -> Self {
        if enabled {
            Self {
                elapsed: 0.0,
                delay: 0.0,
                duration: 0.24,
                from_scale: 0.985,
                from_y: 16.0,
                easing: TransitionEasing::CubicOut,
            }
        } else {
            Self::none()
        }
    }

    const fn dialog(enabled: bool) -> Self {
        if enabled {
            Self {
                elapsed: 0.0,
                delay: 0.025,
                duration: 0.22,
                from_scale: 0.92,
                from_y: 12.0,
                easing: TransitionEasing::BackOut,
            }
        } else {
            Self::none()
        }
    }

    const fn cell(enabled: bool, exploded: bool) -> Self {
        if enabled {
            Self {
                elapsed: 0.0,
                delay: 0.0,
                duration: if exploded { 0.32 } else { 0.18 },
                from_scale: if exploded { 0.52 } else { 0.72 },
                from_y: 0.0,
                easing: TransitionEasing::BackOut,
            }
        } else {
            Self::none()
        }
    }

    const fn pause() -> Self {
        Self {
            elapsed: 0.0,
            delay: 0.0,
            duration: 0.2,
            from_scale: 0.82,
            from_y: 8.0,
            easing: TransitionEasing::BackOut,
        }
    }
}

#[derive(Component, Default, Clone)]
pub(crate) struct SettingTarget {
    key: SettingKey,
}

#[derive(Component, Default, Clone)]
pub(crate) struct CustomInput {
    field: CustomField,
}

#[derive(Debug, Clone, Copy, Default)]
enum UiAction {
    #[default]
    None,
    Request(PendingAction),
    ShowCustom,
    TogglePause,
    Open(DialogKind),
    CloseDialog,
    ConfirmAccept,
    ConfirmCancel,
    Theme(usize),
    HelpPrevious,
    HelpNext,
    Quit,
}

#[derive(Debug, Clone, Copy)]
enum DialogKind {
    Menu,
    Scores,
    Appearance,
    Shortcuts,
    Help,
    About,
}

#[derive(Debug, Clone, Copy, Default)]
enum SettingKey {
    #[default]
    QuestionMarks,
    AutoClear,
}

#[derive(Debug, Clone, Copy, Default)]
enum CustomField {
    #[default]
    Width,
    Height,
    Percentage,
}

#[derive(SceneComponent, Default, Clone)]
#[scene(DifficultyCardProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
struct DifficultyCard;

struct DifficultyCardProps {
    title: String,
    subtitle: String,
    action: UiAction,
}

impl Default for DifficultyCardProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            subtitle: String::new(),
            action: UiAction::None,
        }
    }
}

impl DifficultyCard {
    fn scene(props: DifficultyCardProps) -> impl Scene {
        let title = props.title;
        let subtitle = props.subtitle;
        let action = props.action;
        bsn! {
            @FeathersButton {
                @caption: bsn! {
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        row_gap: px(8),
                    }
                    Children [
                        (
                            Text({title})
                            TextFont { font_size: FontSize::Px(30.0) }
                            TextColor(Color::srgb(0.16, 0.16, 0.18))
                            ThemedText
                        ),
                        (
                            Text({subtitle})
                            TextFont { font_size: FontSize::Px(19.0) }
                            TextColor(Color::srgb(0.20, 0.20, 0.22))
                            ThemedText
                        )
                    ]
                }
            }
            ActionTarget { action: {action} }
            Node {
                width: px(250),
                height: px(150),
                margin: px(7),
                border_radius: BorderRadius::all(px(10)),
            }
            on(handle_action)
        }
    }
}

#[derive(Clone)]
struct UiSnapshot {
    screen: Screen,
    board: Option<Board>,
    paused: bool,
    keyboard_cursor: usize,
    keyboard_active: bool,
    dialog: Option<Dialog>,
    help_page: usize,
    elapsed: u64,
    viewport_size: Vec2,
    scale_factor: f32,
    settings: Settings,
    scores: Vec<crate::storage::Score>,
    screen_transition: bool,
    dialog_transition: bool,
    animated_cells: BTreeSet<usize>,
}

pub fn feathers_plugins() -> FeathersPlugins {
    FeathersPlugins
}

pub fn native_ui_observers(app: &mut App) {
    app.add_observer(cell_clicked)
        .add_observer(cell_pressed)
        .add_observer(cell_released);
}

pub fn mines_theme() -> UiTheme {
    let mut theme = UiTheme(create_dark_theme());
    let overrides = [
        ("feathers.window.bg", Color::srgb(0.965, 0.961, 0.949)),
        ("feathers.text.main", Color::srgb(0.12, 0.12, 0.13)),
        ("feathers.text.dim", Color::srgb(0.36, 0.36, 0.38)),
        ("feathers.button.bg", Color::srgb(0.88, 0.88, 0.88)),
        ("feathers.button.bg.hover", Color::srgb(0.82, 0.82, 0.82)),
        ("feathers.button.bg.pressed", Color::srgb(0.74, 0.74, 0.74)),
        ("feathers.button.bg.disabled", Color::srgb(0.91, 0.91, 0.91)),
        ("feathers.button.txt", Color::srgb(0.12, 0.12, 0.13)),
        ("feathers.button.primary.bg", Color::srgb(0.21, 0.52, 0.89)),
        (
            "feathers.button.primary.bg.hover",
            Color::srgb(0.16, 0.45, 0.82),
        ),
        (
            "feathers.button.primary.bg.pressed",
            Color::srgb(0.11, 0.37, 0.72),
        ),
        ("feathers.button.primary.txt", Color::WHITE),
        ("feathers.button.plain.bg", Color::NONE),
        (
            "feathers.button.plain.bg.hover",
            Color::srgb(0.88, 0.88, 0.88),
        ),
        (
            "feathers.button.plain.bg.pressed",
            Color::srgb(0.80, 0.80, 0.80),
        ),
        ("feathers.checkbox.text", Color::srgb(0.12, 0.12, 0.13)),
        ("feathers.checkbox.bg", Color::WHITE),
        ("feathers.checkbox.border", Color::srgb(0.55, 0.55, 0.57)),
        (
            "feathers.checkbox.bg.checked",
            Color::srgb(0.21, 0.52, 0.89),
        ),
        (
            "feathers.checkbox.border.checked",
            Color::srgb(0.21, 0.52, 0.89),
        ),
        ("feathers.textinput.bg", Color::WHITE),
        ("feathers.textinput.text", Color::srgb(0.12, 0.12, 0.13)),
        ("feathers.textinput.cursor", Color::srgb(0.12, 0.12, 0.13)),
    ];
    for (token, color) in overrides {
        theme.set_color(token, color);
    }
    theme
}

pub fn startup_scene() -> impl SceneList {
    bsn_list![Camera2d]
}

pub fn setup_assets(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut svg_options = resvg::usvg::Options::default();
    svg_options.fontdb_mut().load_system_fonts();
    let themes = TILE_SVGS
        .iter()
        .map(|theme| {
            theme
                .iter()
                .enumerate()
                .map(|(tile_index, bytes)| {
                    set_number_color(&mut svg_options, (tile_index < 8).then_some(tile_index));
                    images.add(svg_image(bytes, 128, &svg_options))
                })
                .collect()
        })
        .collect();
    set_number_color(&mut svg_options, None);
    let flag = images.add(svg_image(
        include_bytes!("../assets/icons/flag-symbolic.svg"),
        64,
        &svg_options,
    ));
    let stopwatch = images.add(svg_image(
        include_bytes!("../assets/icons/stopwatch-symbolic.svg"),
        64,
        &svg_options,
    ));
    commands.insert_resource(ThemeAssets {
        themes,
        flag,
        stopwatch,
    });
}

fn set_number_color(options: &mut resvg::usvg::Options<'_>, number_index: Option<usize>) {
    options.style_sheet = number_index.map(|index| {
        let color = NUMBER_COLORS[index].0;
        format!("text, tspan, path {{ fill: {color} !important; }}")
    });
}

fn svg_image(bytes: &[u8], size: u32, options: &resvg::usvg::Options<'_>) -> Image {
    let tree =
        resvg::usvg::Tree::from_data(bytes, options).expect("GNOME Mines SVG assets must be valid");
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(size, size).expect("theme texture allocation failed");
    let tree_size = tree.size();
    let transform = resvg::tiny_skia::Transform::from_scale(
        size as f32 / tree_size.width(),
        size as f32 / tree_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixmap.take(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

pub fn rebuild_ui(world: &mut World) {
    if !world.resource::<MinesApp>().layout_dirty || world.get_resource::<ThemeAssets>().is_none() {
        return;
    }

    if let Some(root) = world.resource_mut::<UiRuntime>().root.take()
        && world.get_entity(root).is_ok()
    {
        world.despawn(root);
    }

    let (viewport_size, scale_factor) = {
        let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
        windows
            .iter(world)
            .next()
            .map(|window| {
                (
                    Vec2::new(window.resolution.width(), window.resolution.height()),
                    window.resolution.scale_factor(),
                )
            })
            .unwrap_or((Vec2::new(600.0, 600.0), 1.0))
    };
    let snapshot = {
        let app = world.resource::<MinesApp>();
        UiSnapshot {
            screen: app.screen,
            board: app.board.clone(),
            paused: app.paused,
            keyboard_cursor: app.keyboard_cursor,
            keyboard_active: app.keyboard_active,
            dialog: app.dialog,
            help_page: app.help_page,
            elapsed: app.clock.elapsed().as_secs(),
            viewport_size,
            scale_factor,
            settings: world.resource::<Settings>().clone(),
            scores: app.persistent.scores.clone(),
            screen_transition: app.screen_transition,
            dialog_transition: app.dialog_transition,
            animated_cells: app.animated_cells.clone(),
        }
    };
    let assets = world.resource::<ThemeAssets>().clone();
    let root = world
        .spawn_scene(app_scene(snapshot, assets))
        .expect("native Mines UI scene should resolve")
        .id();
    world.resource_mut::<UiRuntime>().root = Some(root);
    let mut app = world.resource_mut::<MinesApp>();
    app.layout_dirty = false;
    app.screen_transition = false;
    app.dialog_transition = false;
    app.animated_cells.clear();
}

fn app_scene(snapshot: UiSnapshot, assets: ThemeAssets) -> impl Scene {
    let header = header_scene(&snapshot);
    let content = screen_scene(&snapshot, &assets);
    let dialog = dialog_scene(&snapshot, &assets);
    let transition = UiTransition::screen(snapshot.screen_transition);
    bsn! {
        UiRoot
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            {header},
            (
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    padding: UiRect::all(px(20)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                UiTransition {
                    elapsed: {transition.elapsed},
                    delay: {transition.delay},
                    duration: {transition.duration},
                    from_scale: {transition.from_scale},
                    from_y: {transition.from_y},
                    easing: {transition.easing},
                }
                Children [{content}]
            ),
            {dialog}
        ]
    }
}

fn header_scene(snapshot: &UiSnapshot) -> Box<dyn SceneList> {
    let right = boxed_button(
        "菜单",
        UiAction::Open(DialogKind::Menu),
        ButtonVariant::Normal,
    );
    if snapshot.screen == Screen::NewGame {
        return Box::new(bsn_list![
            (
                Node {
                    width: percent(100),
                    height: px(66),
                    flex_shrink: 0.0,
                    padding: UiRect::axes(px(18), px(10)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::bottom(px(1)),
                }
                BackgroundColor(Color::srgb(0.99, 0.99, 0.99))
                BorderColor::all(Color::srgb(0.74, 0.74, 0.74))
                Children [
                    (
                        Node {
                            min_width: px(190),
                        }
                        Pickable::IGNORE
                    ),
                    (
                        Text("扫雷")
                        TextFont { font_size: FontSize::Px(26.0) }
                        TextColor(Color::srgb(0.12, 0.12, 0.13))
                    ),
                    (
                        Node {
                            min_width: px(190),
                            justify_content: JustifyContent::End,
                        }
                        Children [{right}]
                    )
                ]
            )
        ]);
    }

    let mut left: Vec<Box<dyn SceneList>> = Vec::new();
    left.push(boxed_button(
        "返回",
        UiAction::Request(PendingAction::ShowNewGame),
        ButtonVariant::Plain,
    ));
    if snapshot.screen == Screen::Game {
        left.push(boxed_button(
            "重新开始",
            UiAction::Request(PendingAction::Restart),
            ButtonVariant::Plain,
        ));
        if snapshot
            .board
            .as_ref()
            .is_some_and(|board| board.outcome == Outcome::InProgress)
        {
            left.push(boxed_button(
                if snapshot.paused { "继续" } else { "暂停" },
                UiAction::TogglePause,
                ButtonVariant::Plain,
            ));
        }
    }
    Box::new(bsn_list![
        (
        Node {
            width: percent(100),
            height: px(66),
            flex_shrink: 0.0,
            padding: UiRect::axes(px(18), px(10)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect::bottom(px(1)),
        }
        BackgroundColor(Color::srgb(0.99, 0.99, 0.99))
        BorderColor::all(Color::srgb(0.74, 0.74, 0.74))
        Children [
            (
                Node {
                    min_width: px(190),
                    column_gap: px(6),
                    align_items: AlignItems::Center,
                }
                Children [{left}]
            ),
            (
                Text("扫雷")
                TextFont { font_size: FontSize::Px(26.0) }
                TextColor(Color::srgb(0.12, 0.12, 0.13))
            ),
            (
                Node {
                    min_width: px(190),
                    justify_content: JustifyContent::End,
                }
                Children [{right}]
            )
        ]
        )
    ])
}

fn screen_scene(snapshot: &UiSnapshot, assets: &ThemeAssets) -> Box<dyn SceneList> {
    match snapshot.screen {
        Screen::NewGame => Box::new(bsn_list![new_game_scene()]),
        Screen::Custom => Box::new(bsn_list![custom_scene(snapshot)]),
        Screen::Game => Box::new(bsn_list![game_scene(snapshot, assets)]),
    }
}

fn new_game_scene() -> impl Scene {
    let cards = vec![
        difficulty_card(
            "8 × 8",
            "10 个雷",
            UiAction::Request(PendingAction::Start(Difficulty::Small)),
        ),
        difficulty_card(
            "16 × 16",
            "40 个雷",
            UiAction::Request(PendingAction::Start(Difficulty::Medium)),
        ),
        difficulty_card(
            "30 × 16",
            "99 个雷",
            UiAction::Request(PendingAction::Start(Difficulty::Large)),
        ),
        difficulty_card("?", "自定义", UiAction::ShowCustom),
    ];
    bsn! {
        Node {
            width: percent(100),
            max_width: px(620),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(26),
        }
        Children [
            (
                Text("选择难度")
                TextFont { font_size: FontSize::Px(36.0) }
                TextColor(Color::srgb(0.32, 0.32, 0.34))
            ),
            (
                Node {
                    width: percent(100),
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                Children [{cards}]
            )
        ]
    }
}

fn difficulty_card(title: &str, subtitle: &str, action: UiAction) -> impl Scene {
    let title = title.to_owned();
    let subtitle = subtitle.to_owned();
    bsn! {
        @DifficultyCard {
            @title: {title},
            @subtitle: {subtitle},
            @action: {action},
        }
    }
}

fn custom_scene(snapshot: &UiSnapshot) -> impl Scene {
    let settings = &snapshot.settings;
    let percentage = (100.0 * settings.custom_mines as f64
        / (settings.custom_width * settings.custom_height) as f64)
        .round() as i32;
    let inputs = vec![
        number_input("宽度", CustomField::Width),
        number_input("高度", CustomField::Height),
        number_input("雷区占比（%）", CustomField::Percentage),
    ];
    let summary = format!(
        "{} × {}，{} 个雷（{}%）",
        settings.custom_width, settings.custom_height, settings.custom_mines, percentage
    );
    let start = bsn_list![action_button(
        "开始游戏",
        UiAction::Request(PendingAction::Start(Difficulty::Custom)),
        ButtonVariant::Primary,
    )];
    bsn! {
        Node {
            width: percent(100),
            max_width: px(520),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(18),
            padding: px(24),
        }
        Children [
            (
                Text("自定义游戏")
                TextFont { font_size: FontSize::Px(34.0) }
                TextColor(Color::srgb(0.22, 0.22, 0.24))
            ),
            {inputs},
            (
                Text({summary})
                TextFont { font_size: FontSize::Px(17.0) }
                TextColor(Color::srgb(0.36, 0.36, 0.38))
            ),
            {start}
        ]
    }
}

fn number_input(label: &str, field: CustomField) -> impl Scene {
    let label = label.to_owned();
    bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(16),
        }
        Children [
            (
                Text({label})
                TextFont { font_size: FontSize::Px(19.0) }
                TextColor(Color::srgb(0.18, 0.18, 0.20))
            ),
            (
                @FeathersNumberInput {
                    @number_format: NumberFormat::I32,
                }
                CustomInput { field: {field} }
                Node {
                    width: px(170),
                }
                on(custom_value_changed)
            )
        ]
    }
}

fn game_scene(snapshot: &UiSnapshot, assets: &ThemeAssets) -> impl Scene {
    let board = snapshot
        .board
        .as_ref()
        .expect("game screen always owns a board");
    let theme_index = theme_index(&snapshot.settings.theme);
    let textures = &assets.themes[theme_index];
    let cells: Vec<_> = (0..board.cells.len())
        .map(|index| cell_scene(snapshot, index, textures))
        .collect();
    let available_width = (snapshot.viewport_size.x - 40.0).clamp(1.0, 1100.0);
    let available_height = (snapshot.viewport_size.y - 170.0).clamp(1.0, 820.0);
    let cell_size = ((available_width / board.width as f32)
        .min(available_height / board.height as f32)
        * snapshot.scale_factor)
        .floor()
        .max(1.0)
        / snapshot.scale_factor;
    let board_width = cell_size * board.width as f32;
    let board_height = cell_size * board.height as f32;
    let status = bsn_list![status_scene(snapshot, assets)];
    let pause_transition = UiTransition::pause();
    let pause_overlay: Box<dyn SceneList> = if snapshot.paused {
        Box::new(bsn_list![
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(0),
                    top: percent(0),
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                ZIndex(20)
                BackgroundColor(Color::srgba(0.96, 0.96, 0.96, 0.86))
                Children [(
                    Text("已暂停")
                    TextFont { font_size: FontSize::Px(44.0) }
                    TextColor(Color::srgb(0.18, 0.18, 0.20))
                    UiTransition {
                        elapsed: {pause_transition.elapsed},
                        delay: {pause_transition.delay},
                        duration: {pause_transition.duration},
                        from_scale: {pause_transition.from_scale},
                        from_y: {pause_transition.from_y},
                        easing: {pause_transition.easing},
                    }
                )]
            )
        ])
    } else {
        Box::new(bsn_list![])
    };

    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(12),
            min_height: px(0),
        }
        Children [
            (
                Node {
                    position_type: PositionType::Relative,
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                Children [
                    (
                        Node {
                            display: Display::Grid,
                            width: {px(board_width)},
                            height: {px(board_height)},
                            grid_template_columns:
                                {RepeatedGridTrack::px::<Vec<RepeatedGridTrack>>(
                                    board.width as u16,
                                    cell_size,
                                )},
                            grid_template_rows:
                                {RepeatedGridTrack::px::<Vec<RepeatedGridTrack>>(
                                    board.height as u16,
                                    cell_size,
                                )},
                            column_gap: px(0),
                            row_gap: px(0),
                        }
                        BackgroundColor(Color::srgb(0.72, 0.72, 0.72))
                        Children [{cells}]
                    ),
                    {pause_overlay}
                ]
            ),
            {status}
        ]
    }
}

fn cell_scene(snapshot: &UiSnapshot, index: usize, textures: &[Handle<Image>]) -> impl Scene {
    let board = snapshot.board.as_ref().unwrap();
    let cell = board.cells[index];
    let selected = snapshot.keyboard_active && snapshot.keyboard_cursor == index;
    let background = if cell.revealed {
        Color::srgb(0.96, 0.96, 0.95)
    } else {
        Color::srgb(0.82, 0.82, 0.82)
    };
    let border_color = if selected {
        Color::srgb(0.13, 0.45, 0.86)
    } else if cell.revealed {
        Color::srgb(0.78, 0.78, 0.78)
    } else {
        Color::srgb(0.66, 0.66, 0.66)
    };
    let transition = UiTransition::cell(
        snapshot.animated_cells.contains(&index),
        board.exploded_cell == Some(index),
    );
    let visual: Box<dyn SceneList> = if cell.revealed && !cell.mine && cell.adjacent > 0 {
        let number = cell.adjacent.to_string();
        let color = number_color(cell.adjacent);
        let font_size = FontSize::VMin(60.0 / board.width.max(board.height).max(1) as f32);
        Box::new(bsn_list![
            (
                Text({number})
                TextFont { font_size: {font_size} }
                TextColor({color})
                Pickable::IGNORE
            )
        ])
    } else {
        tile_graphic(board, index).map_or_else(
            || Box::new(bsn_list![]) as Box<dyn SceneList>,
            |graphic| {
                let handle = textures[graphic].clone();
                Box::new(bsn_list![
                    (
                        ImageNode { image: {handle} }
                        Node {
                            width: percent(100),
                            height: percent(100),
                        }
                        Pickable::IGNORE
                    )
                ]) as Box<dyn SceneList>
            },
        )
    };

    bsn! {
        CellTarget { index }
        Node {
            width: percent(100),
            height: percent(100),
            min_width: px(0),
            min_height: px(0),
            border: {px(if selected { 2.5 } else { 0.75 })},
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            overflow: Overflow::clip(),
        }
        BackgroundColor({background})
        BorderColor::all(border_color)
        UiTransition {
            elapsed: {transition.elapsed},
            delay: {transition.delay},
            duration: {transition.duration},
            from_scale: {transition.from_scale},
            from_y: {transition.from_y},
            easing: {transition.easing},
        }
        Children [{visual}]
    }
}

fn number_color(number: u8) -> Color {
    let [red, green, blue] = NUMBER_COLORS[(number.clamp(1, 8) - 1) as usize].1;
    Color::srgb(
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
    )
}

fn tile_graphic(board: &Board, index: usize) -> Option<usize> {
    let cell = board.cells[index];
    if board.outcome == Outcome::Lost {
        if board.exploded_cell == Some(index) {
            return Some(9);
        }
        if cell.mark == Mark::Flag && !cell.mine {
            return Some(11);
        }
        if cell.mine && cell.mark != Mark::Flag {
            return Some(8);
        }
    }
    if cell.revealed {
        if cell.mine {
            Some(8)
        } else if cell.adjacent > 0 {
            Some(cell.adjacent as usize - 1)
        } else {
            None
        }
    } else {
        match cell.mark {
            Mark::None => None,
            Mark::Flag => Some(10),
            Mark::Maybe => Some(12),
        }
    }
}

fn status_scene(snapshot: &UiSnapshot, assets: &ThemeAssets) -> impl Scene {
    let board = snapshot.board.as_ref().unwrap();
    let flags = format!("{}/{}", board.flag_count, board.mine_count);
    let elapsed = format_time(snapshot.elapsed);
    let flag = assets.flag.clone();
    let stopwatch = assets.stopwatch.clone();
    bsn! {
        Node {
            height: px(52),
            padding: UiRect::axes(px(18), px(8)),
            column_gap: px(34),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(10)),
        }
        BackgroundColor(Color::srgb(0.91, 0.91, 0.90))
        Children [
            (
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                }
                Children [
                    (
                        ImageNode { image: {flag} }
                        Node { width: px(25), height: px(25) }
                    ),
                    (
                        Text({flags})
                        TextFont { font_size: FontSize::Px(20.0) }
                        TextColor(Color::srgb(0.12, 0.12, 0.13))
                    )
                ]
            ),
            (
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                }
                Children [
                    (
                        ImageNode { image: {stopwatch} }
                        Node { width: px(25), height: px(25) }
                    ),
                    (
                        ClockText
                        Text({elapsed})
                        TextFont { font_size: FontSize::Px(20.0) }
                        TextColor(Color::srgb(0.12, 0.12, 0.13))
                    )
                ]
            )
        ]
    }
}

fn dialog_scene(snapshot: &UiSnapshot, assets: &ThemeAssets) -> Box<dyn SceneList> {
    let Some(dialog) = snapshot.dialog else {
        return Box::new(bsn_list![]);
    };
    let animate = snapshot.dialog_transition;
    match dialog {
        Dialog::Menu => menu_dialog(snapshot, animate),
        Dialog::Confirm(_) => confirm_dialog(animate),
        Dialog::Scores => scores_dialog(snapshot, animate),
        Dialog::Appearance => appearance_dialog(snapshot, assets, animate),
        Dialog::Shortcuts => shortcuts_dialog(animate),
        Dialog::Help => help_dialog(snapshot, animate),
        Dialog::About => about_dialog(animate),
        Dialog::Victory => victory_dialog(snapshot, animate),
    }
}

fn modal(
    title: &str,
    body: Box<dyn SceneList>,
    actions: Vec<Box<dyn SceneList>>,
    animate: bool,
) -> impl Scene {
    let title = title.to_owned();
    let transition = UiTransition::dialog(animate);
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: percent(0),
            top: percent(0),
            width: percent(100),
            height: percent(100),
            padding: px(24),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        ZIndex(100)
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45))
        Children [(
            Node {
                width: percent(100),
                max_width: px(620),
                max_height: percent(92),
                flex_direction: FlexDirection::Column,
                row_gap: px(18),
                padding: px(24),
                border: px(1),
                border_radius: BorderRadius::all(px(12)),
                overflow: Overflow::clip(),
            }
            BackgroundColor(Color::srgb(0.98, 0.98, 0.98))
            BorderColor::all(Color::srgb(0.67, 0.67, 0.67))
            UiTransition {
                elapsed: {transition.elapsed},
                delay: {transition.delay},
                duration: {transition.duration},
                from_scale: {transition.from_scale},
                from_y: {transition.from_y},
                easing: {transition.easing},
            }
            Children [
                (
                    Text({title})
                    TextFont { font_size: FontSize::Px(30.0) }
                    TextColor(Color::srgb(0.16, 0.16, 0.18))
                ),
                (
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        overflow: Overflow::clip_y(),
                    }
                    Children [{body}]
                ),
                (
                    Node {
                        width: percent(100),
                        justify_content: JustifyContent::End,
                        align_items: AlignItems::Center,
                        column_gap: px(10),
                        flex_wrap: FlexWrap::Wrap,
                    }
                    Children [{actions}]
                )
            ]
        )]
    }
}

fn menu_dialog(snapshot: &UiSnapshot, animate: bool) -> Box<dyn SceneList> {
    let toggles = vec![
        setting_checkbox("使用问号标记", SettingKey::QuestionMarks),
        setting_checkbox("插旗后自动清除", SettingKey::AutoClear),
    ];
    let items = vec![
        boxed_button(
            "成绩",
            UiAction::Open(DialogKind::Scores),
            ButtonVariant::Normal,
        ),
        boxed_button(
            "外观",
            UiAction::Open(DialogKind::Appearance),
            ButtonVariant::Normal,
        ),
        boxed_button(
            "键盘快捷键",
            UiAction::Open(DialogKind::Shortcuts),
            ButtonVariant::Normal,
        ),
        boxed_button(
            "帮助",
            UiAction::Open(DialogKind::Help),
            ButtonVariant::Normal,
        ),
        boxed_button(
            "关于扫雷",
            UiAction::Open(DialogKind::About),
            ButtonVariant::Normal,
        ),
    ];
    let version = format!(
        "Bevy 0.19 原生界面 · {}主题",
        THEMES[theme_index(&snapshot.settings.theme)].1
    );
    let body = Box::new(bsn_list![
        (
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
            }
            Children [{toggles}]
        ),
        (
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
            }
            Children [{items}]
        ),
        (
            Text({version})
            TextFont { font_size: FontSize::Px(14.0) }
            TextColor(Color::srgb(0.42, 0.42, 0.44))
        )
    ]);
    let actions = vec![
        boxed_button("退出", UiAction::Quit, ButtonVariant::Plain),
        boxed_button("关闭", UiAction::CloseDialog, ButtonVariant::Primary),
    ];
    Box::new(bsn_list![modal("菜单", body, actions, animate)])
}

fn setting_checkbox(label: &str, key: SettingKey) -> impl Scene {
    let label = label.to_owned();
    bsn! {
        @FeathersCheckbox {
            @caption: bsn! { Text({label}) ThemedText }
        }
        SettingTarget { key: {key} }
        on(checkbox_self_update)
        on(setting_changed)
    }
}

fn confirm_dialog(animate: bool) -> Box<dyn SceneList> {
    let body = Box::new(bsn_list![
        (
            Text("当前游戏进度将会丢失。")
            TextFont { font_size: FontSize::Px(19.0) }
            TextColor(Color::srgb(0.22, 0.22, 0.24))
        )
    ]);
    let actions = vec![
        boxed_button("取消", UiAction::ConfirmCancel, ButtonVariant::Normal),
        boxed_button("离开游戏", UiAction::ConfirmAccept, ButtonVariant::Primary),
    ];
    Box::new(bsn_list![modal("开始新游戏？", body, actions, animate)])
}

fn scores_dialog(snapshot: &UiSnapshot, animate: bool) -> Box<dyn SceneList> {
    let mut groups: BTreeMap<(usize, usize, usize), Vec<_>> = BTreeMap::new();
    for score in &snapshot.scores {
        groups
            .entry((score.width, score.height, score.mines))
            .or_default()
            .push(score);
    }
    let mut text = String::new();
    if groups.is_empty() {
        text.push_str("暂无成绩。完成一局游戏即可记录首个成绩。");
    } else {
        for ((width, height, mines), scores) in groups {
            text.push_str(&format!("{width} × {height}，{mines} 个雷\n"));
            for (rank, score) in scores.iter().take(10).enumerate() {
                text.push_str(&format!(
                    "  {:>2}. {}\n",
                    rank + 1,
                    format_time(score.seconds)
                ));
            }
            text.push('\n');
        }
    }
    let body = Box::new(bsn_list![
        (
            Text({text})
            TextFont { font_size: FontSize::Px(17.0) }
            TextColor(Color::srgb(0.18, 0.18, 0.20))
        )
    ]);
    let actions = vec![boxed_button(
        "关闭",
        UiAction::CloseDialog,
        ButtonVariant::Primary,
    )];
    Box::new(bsn_list![modal("成绩", body, actions, animate)])
}

fn appearance_dialog(
    snapshot: &UiSnapshot,
    assets: &ThemeAssets,
    animate: bool,
) -> Box<dyn SceneList> {
    let cards: Vec<_> = THEMES
        .iter()
        .enumerate()
        .map(|(index, (_, label))| {
            theme_card(
                index,
                label,
                snapshot.settings.theme == THEMES[index].0,
                assets.themes[index][8].clone(),
            )
        })
        .collect();
    let body = Box::new(bsn_list![
        (
            Node {
                width: percent(100),
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
            }
            Children [{cards}]
        )
    ]);
    let actions = vec![boxed_button(
        "完成",
        UiAction::CloseDialog,
        ButtonVariant::Primary,
    )];
    Box::new(bsn_list![modal("外观", body, actions, animate)])
}

fn theme_card(index: usize, label: &str, selected: bool, image: Handle<Image>) -> impl Scene {
    let label = if selected {
        format!("✓ {label}")
    } else {
        label.to_owned()
    };
    bsn! {
        @FeathersButton {
            @caption: bsn! {
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(7),
                }
                Children [
                    (
                        ImageNode { image: {image} }
                        Node { width: px(68), height: px(68) }
                    ),
                    (
                        Text({label})
                        TextFont { font_size: FontSize::Px(16.0) }
                        ThemedText
                    )
                ]
            }
        }
        ActionTarget { action: UiAction::Theme(index) }
        Node {
            width: px(135),
            height: px(120),
            margin: px(5),
        }
        on(handle_action)
    }
}

fn shortcuts_dialog(animate: bool) -> Box<dyn SceneList> {
    let text = "\
新游戏                 Ctrl+N
重新开始               Ctrl+R
暂停 / 继续            Pause
帮助                   F1
菜单                   F10 / Ctrl+?
退出                   Ctrl+Q / Ctrl+W

小型 / 中型 / 大型     1 / 2 / 3
自定义                 4
移动                   方向键或 U J H K
揭开 / 清除周围        空格
插旗                   Ctrl+空格";
    let body = Box::new(bsn_list![
        (
            Text(text)
            TextFont { font_size: FontSize::Px(17.0) }
            TextColor(Color::srgb(0.18, 0.18, 0.20))
        )
    ]);
    let actions = vec![boxed_button(
        "关闭",
        UiAction::CloseDialog,
        ButtonVariant::Primary,
    )];
    Box::new(bsn_list![modal("键盘快捷键", body, actions, animate)])
}

const HELP_PAGES: [(&str, &str); 4] = [
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

fn help_dialog(snapshot: &UiSnapshot, animate: bool) -> Box<dyn SceneList> {
    let page = snapshot.help_page.min(HELP_PAGES.len() - 1);
    let (title, text) = HELP_PAGES[page];
    let counter = format!("第 {} 页，共 {} 页", page + 1, HELP_PAGES.len());
    let body = Box::new(bsn_list![
        (
            Text(title)
            TextFont { font_size: FontSize::Px(23.0) }
            TextColor(Color::srgb(0.16, 0.16, 0.18))
        ),
        (
            Text(text)
            TextFont { font_size: FontSize::Px(18.0) }
            TextColor(Color::srgb(0.24, 0.24, 0.26))
        ),
        (
            Text({counter})
            TextFont { font_size: FontSize::Px(14.0) }
            TextColor(Color::srgb(0.43, 0.43, 0.45))
        )
    ]);
    let actions = vec![
        boxed_button("上一页", UiAction::HelpPrevious, ButtonVariant::Normal),
        boxed_button("下一页", UiAction::HelpNext, ButtonVariant::Normal),
        boxed_button("关闭", UiAction::CloseDialog, ButtonVariant::Primary),
    ];
    Box::new(bsn_list![modal("扫雷帮助", body, actions, animate)])
}

fn about_dialog(animate: bool) -> Box<dyn SceneList> {
    let body = Box::new(bsn_list![
        (
            Text("扫雷 50.0")
            TextFont { font_size: FontSize::Px(25.0) }
            TextColor(Color::srgb(0.16, 0.16, 0.18))
        ),
        (
            Text("使用 Bevy 0.19 原生 UI、BSN 场景、Feathers 控件和应用设置完整复刻 GNOME 扫雷。")
            TextFont { font_size: FontSize::Px(17.0) }
            TextColor(Color::srgb(0.24, 0.24, 0.26))
        ),
        (
            Text("美术资源来自 GNOME Mines，采用 GPL-3.0-or-later 许可证。")
            TextFont { font_size: FontSize::Px(15.0) }
            TextColor(Color::srgb(0.40, 0.40, 0.42))
        )
    ]);
    let actions = vec![boxed_button(
        "关闭",
        UiAction::CloseDialog,
        ButtonVariant::Primary,
    )];
    Box::new(bsn_list![modal("关于扫雷", body, actions, animate)])
}

fn victory_dialog(snapshot: &UiSnapshot, animate: bool) -> Box<dyn SceneList> {
    let time = format!("你用 {} 清空了雷区。", format_time(snapshot.elapsed));
    let body = Box::new(bsn_list![
        (
            Text("干得漂亮！")
            TextFont { font_size: FontSize::Px(28.0) }
            TextColor(Color::srgb(0.12, 0.45, 0.20))
        ),
        (
            Text({time})
            TextFont { font_size: FontSize::Px(19.0) }
            TextColor(Color::srgb(0.20, 0.20, 0.22))
        )
    ]);
    let actions = vec![
        boxed_button(
            "新游戏",
            UiAction::Request(PendingAction::ShowNewGame),
            ButtonVariant::Normal,
        ),
        boxed_button(
            "再玩一次",
            UiAction::Request(PendingAction::Restart),
            ButtonVariant::Primary,
        ),
    ];
    Box::new(bsn_list![modal("胜利", body, actions, animate)])
}

fn action_button(label: &str, action: UiAction, variant: ButtonVariant) -> impl Scene {
    let label = label.to_owned();
    bsn! {
        @FeathersButton {
            @caption: bsn! { Text({label}) ThemedText },
            @variant: {variant},
        }
        ActionTarget { action: {action} }
        on(handle_action)
    }
}

fn boxed_button(label: &str, action: UiAction, variant: ButtonVariant) -> Box<dyn SceneList> {
    Box::new(bsn_list![action_button(label, action, variant)])
}

fn handle_action(
    activate: On<Activate>,
    targets: Query<&ActionTarget>,
    mut app: ResMut<MinesApp>,
    mut settings: ResMut<Settings>,
    mut commands: Commands,
) {
    let Ok(target) = targets.get(activate.event_target()) else {
        return;
    };
    match target.action {
        UiAction::None => {}
        UiAction::Request(action) => app.request(action, &mut settings),
        UiAction::ShowCustom => {
            app.screen = Screen::Custom;
            app.dialog = None;
            app.layout_dirty = true;
            app.screen_transition = true;
        }
        UiAction::TogglePause => app.toggle_pause(),
        UiAction::Open(kind) => {
            app.dialog = Some(match kind {
                DialogKind::Menu => Dialog::Menu,
                DialogKind::Scores => Dialog::Scores,
                DialogKind::Appearance => Dialog::Appearance,
                DialogKind::Shortcuts => Dialog::Shortcuts,
                DialogKind::Help => Dialog::Help,
                DialogKind::About => Dialog::About,
            });
            app.layout_dirty = true;
            app.dialog_transition = true;
        }
        UiAction::CloseDialog => {
            app.dialog = None;
            app.layout_dirty = true;
        }
        UiAction::ConfirmAccept => {
            if let Some(Dialog::Confirm(state)) = app.dialog {
                app.execute(state.action, &mut settings);
            }
        }
        UiAction::ConfirmCancel => {
            if let Some(Dialog::Confirm(state)) = app.dialog {
                app.paused = state.was_paused;
                app.pause_requested = state.was_pause_requested;
                if app.paused {
                    app.clock.pause();
                } else {
                    app.clock.resume();
                }
            }
            app.dialog = None;
            app.layout_dirty = true;
        }
        UiAction::Theme(index) => {
            if let Some((name, _)) = THEMES.get(index) {
                settings.theme = (*name).to_owned();
                app.layout_dirty = true;
            }
        }
        UiAction::HelpPrevious => {
            app.help_page = app.help_page.saturating_sub(1);
            app.layout_dirty = true;
            app.dialog_transition = true;
        }
        UiAction::HelpNext => {
            app.help_page = (app.help_page + 1).min(HELP_PAGES.len() - 1);
            app.layout_dirty = true;
            app.dialog_transition = true;
        }
        UiAction::Quit => {
            commands.queue(SaveSettingsSync::IfChanged);
            commands.write_message(AppExit::Success);
        }
    }
}

fn setting_changed(
    change: On<ValueChange<bool>>,
    targets: Query<&SettingTarget>,
    mut settings: ResMut<Settings>,
    mut app: ResMut<MinesApp>,
) {
    let Ok(target) = targets.get(change.event_target()) else {
        return;
    };
    match target.key {
        SettingKey::QuestionMarks => settings.use_question_marks = change.value,
        SettingKey::AutoClear => settings.auto_clear_on_flags = change.value,
    }
    app.layout_dirty = true;
}

pub fn sync_setting_widgets(
    settings: Res<Settings>,
    widgets: Query<(Entity, &SettingTarget, Has<Checked>)>,
    mut commands: Commands,
) {
    for (entity, target, checked) in &widgets {
        let expected = match target.key {
            SettingKey::QuestionMarks => settings.use_question_marks,
            SettingKey::AutoClear => settings.auto_clear_on_flags,
        };
        if expected && !checked {
            commands.entity(entity).insert(Checked);
        } else if !expected && checked {
            commands.entity(entity).remove::<Checked>();
        }
    }
}

fn custom_value_changed(
    change: On<ValueChange<i32>>,
    inputs: Query<&CustomInput>,
    mut settings: ResMut<Settings>,
    mut app: ResMut<MinesApp>,
) {
    if !change.is_final {
        return;
    }
    let Ok(input) = inputs.get(change.event_target()) else {
        return;
    };
    let old_size = settings.custom_width * settings.custom_height;
    let old_percentage = 100.0 * settings.custom_mines as f64 / old_size as f64;
    match input.field {
        CustomField::Width => settings.custom_width = change.value.clamp(4, 100) as usize,
        CustomField::Height => settings.custom_height = change.value.clamp(4, 100) as usize,
        CustomField::Percentage => {
            let size = settings.custom_width * settings.custom_height;
            settings.custom_mines = ((change.value.clamp(1, 99) as f64 * size as f64 / 100.0)
                .round() as usize)
                .clamp(1, size - 10);
        }
    }
    if !matches!(input.field, CustomField::Percentage) {
        let size = settings.custom_width * settings.custom_height;
        settings.custom_mines =
            ((old_percentage * size as f64 / 100.0).round() as usize).clamp(1, size - 10);
    }
    app.layout_dirty = true;
}

pub fn initialize_number_inputs(
    inputs: Query<(Entity, &CustomInput), Added<CustomInput>>,
    children: Query<&Children>,
    mut text_inputs: Query<&mut EditableText>,
    settings: Res<Settings>,
    mut commands: Commands,
) {
    for (entity, input) in &inputs {
        let value = match input.field {
            CustomField::Width => settings.custom_width as i32,
            CustomField::Height => settings.custom_height as i32,
            CustomField::Percentage => (100.0 * settings.custom_mines as f64
                / (settings.custom_width * settings.custom_height) as f64)
                .round() as i32,
        };
        if let Ok(children) = children.get(entity) {
            for child in children.iter() {
                if let Ok(mut text) = text_inputs.get_mut(child) {
                    text.queue_edit(TextEdit::SelectAll);
                    text.queue_edit(TextEdit::Insert(value.to_string().into()));
                    commands
                        .entity(child)
                        .insert(TextColor(Color::srgb(0.12, 0.12, 0.13)));
                    break;
                }
            }
        }
        commands.delayed().secs(0.05).trigger(UpdateNumberInput {
            entity,
            value: NumberInputValue::I32(value),
        });
    }
}

pub fn use_system_ui_font(mut fonts: Query<&mut TextFont, Added<TextFont>>) {
    for mut font in &mut fonts {
        font.font = FontSource::SystemUi;
    }
}

fn cell_clicked(
    mut click: On<Pointer<Click>>,
    cells: Query<&CellTarget>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app: ResMut<MinesApp>,
    settings: Res<Settings>,
) {
    if click.pointer_id.is_touch() {
        return;
    }
    let Ok(target) = cells.get(click.event_target()) else {
        return;
    };
    click.propagate(false);
    app.keyboard_active = false;
    let Some(board) = app.board.as_ref() else {
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
    app.apply_cell_action(action, &settings);
}

fn cell_pressed(press: On<Pointer<Press>>, cells: Query<&CellTarget>, mut hold: ResMut<TouchHold>) {
    let Some(touch_id) = press.pointer_id.get_touch_id() else {
        return;
    };
    let Ok(target) = cells.get(press.event_target()) else {
        return;
    };
    hold.active = Some((touch_id, target.index, Instant::now()));
}

fn cell_released(
    mut release: On<Pointer<Release>>,
    cells: Query<&CellTarget>,
    mut hold: ResMut<TouchHold>,
    mut app: ResMut<MinesApp>,
    settings: Res<Settings>,
) {
    let Some(touch_id) = release.pointer_id.get_touch_id() else {
        return;
    };
    let Ok(target) = cells.get(release.event_target()) else {
        return;
    };
    let Some((held_id, held_index, started)) = hold.active.take() else {
        return;
    };
    if held_id != touch_id || held_index != target.index {
        return;
    }
    release.propagate(false);
    app.keyboard_active = false;
    let Some(board) = app.board.as_ref() else {
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
    app.apply_cell_action(action, &settings);
}

pub fn keyboard_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    editable: Query<(), With<EditableText>>,
    mut app: ResMut<MinesApp>,
    mut settings: ResMut<Settings>,
    mut commands: Commands,
) {
    if focus.get().is_some_and(|entity| editable.contains(entity)) {
        return;
    }
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    if (ctrl && keyboard.just_pressed(KeyCode::KeyQ))
        || (ctrl && keyboard.just_pressed(KeyCode::KeyW))
    {
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
        return;
    }
    if keyboard.just_pressed(KeyCode::F1) {
        app.dialog = Some(Dialog::Help);
        app.layout_dirty = true;
        app.dialog_transition = true;
    }
    if keyboard.just_pressed(KeyCode::F10) || (ctrl && keyboard.just_pressed(KeyCode::Slash)) {
        app.dialog = Some(Dialog::Menu);
        app.layout_dirty = true;
        app.dialog_transition = true;
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Some(Dialog::Confirm(state)) = app.dialog {
            app.paused = state.was_paused;
            app.pause_requested = state.was_pause_requested;
            if app.paused {
                app.clock.pause();
            } else {
                app.clock.resume();
            }
            app.dialog = None;
        } else if app.dialog.take().is_none() && app.screen != Screen::NewGame {
            app.request(PendingAction::ShowNewGame, &mut settings);
        }
        app.layout_dirty = true;
    }
    if ctrl && keyboard.just_pressed(KeyCode::KeyN) {
        app.request(PendingAction::ShowNewGame, &mut settings);
    }
    if ctrl && keyboard.just_pressed(KeyCode::KeyR) {
        app.request(PendingAction::Restart, &mut settings);
    }
    if keyboard.just_pressed(KeyCode::Pause) {
        app.toggle_pause();
    }
    if keyboard.just_pressed(KeyCode::Digit1) {
        app.request(PendingAction::Start(Difficulty::Small), &mut settings);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        app.request(PendingAction::Start(Difficulty::Medium), &mut settings);
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        app.request(PendingAction::Start(Difficulty::Large), &mut settings);
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        app.screen = Screen::Custom;
        app.dialog = None;
        app.layout_dirty = true;
        app.screen_transition = true;
    }

    if app.dialog.is_some() || app.screen != Screen::Game || app.paused {
        return;
    }
    let (width, height, mut x, mut y) = {
        let Some(board) = app.board.as_ref() else {
            return;
        };
        let (x, y) = board.coords(app.keyboard_cursor.min(board.cells.len() - 1));
        (board.width, board.height, x, y)
    };
    let old = (x, y);
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyH) {
        x = x.saturating_sub(1);
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyL) {
        x = (x + 1).min(width - 1);
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyU) {
        y = y.saturating_sub(1);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyJ) {
        y = (y + 1).min(height - 1);
    }
    if (x, y) != old {
        app.keyboard_cursor = y * width + x;
        app.keyboard_active = true;
        app.layout_dirty = true;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        app.keyboard_active = true;
        let index = y * width + x;
        let revealed = app
            .board
            .as_ref()
            .is_some_and(|board| board.cells[index].revealed);
        let action = if ctrl {
            CellAction::Mark(x, y)
        } else if revealed {
            CellAction::Chord(x, y)
        } else {
            CellAction::Reveal(x, y)
        };
        app.apply_cell_action(action, &settings);
    }
}

pub fn animate_ui_transitions(
    time: Res<Time>,
    mut transitions: Query<(Entity, &mut UiTransition, &mut UiTransform)>,
    mut commands: Commands,
) {
    for (entity, mut transition, mut transform) in &mut transitions {
        if transition.duration <= 0.0 {
            *transform = UiTransform::IDENTITY;
            commands.entity(entity).remove::<UiTransition>();
            continue;
        }

        transition.elapsed += time.delta_secs();
        let linear =
            ((transition.elapsed - transition.delay) / transition.duration).clamp(0.0, 1.0);
        let eased = transition_easing(linear, transition.easing);
        let scale = transition.from_scale + (1.0 - transition.from_scale) * eased;
        let y = transition.from_y * (1.0 - eased);
        transform.scale = Vec2::splat(scale);
        transform.translation = Val2::px(0.0, y);

        if linear >= 1.0 {
            *transform = UiTransform::IDENTITY;
            commands.entity(entity).remove::<UiTransition>();
        }
    }
}

fn transition_easing(value: f32, easing: TransitionEasing) -> f32 {
    match easing {
        TransitionEasing::CubicOut => 1.0 - (1.0 - value).powi(3),
        TransitionEasing::BackOut => {
            const OVERSHOOT: f32 = 1.70158;
            let shifted = value - 1.0;
            1.0 + (OVERSHOOT + 1.0) * shifted.powi(3) + OVERSHOOT * shifted.powi(2)
        }
    }
}

pub fn update_clock_text(app: Res<MinesApp>, mut clock: Query<&mut Text, With<ClockText>>) {
    let value = format_time(app.clock.elapsed().as_secs());
    for mut text in &mut clock {
        if text.0 != value {
            text.0.clone_from(&value);
        }
    }
}

pub fn window_focus_pause(
    mut focus_events: MessageReader<WindowFocused>,
    mut app: ResMut<MinesApp>,
) {
    for event in focus_events.read() {
        if !event.focused && app.clock.is_running() {
            app.set_paused(true, false);
            app.auto_paused = true;
        } else if event.focused && app.auto_paused && !app.pause_requested && app.dialog.is_none() {
            app.auto_paused = false;
            app.set_paused(false, false);
        }
    }
}

pub fn window_resize_layout(
    mut resize_events: MessageReader<WindowResized>,
    mut app: ResMut<MinesApp>,
) {
    if resize_events.read().next().is_some() {
        app.layout_dirty = true;
    }
}

pub fn schedule_settings_save(settings: Res<Settings>, mut commands: Commands) {
    if settings.is_changed() {
        commands.queue(SaveSettingsDeferred(Duration::from_millis(400)));
    }
}

pub fn close_and_save(mut close: MessageReader<WindowCloseRequested>, mut commands: Commands) {
    if close.read().next().is_some() {
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
    }
}

pub fn apply_window_settings(mut window: Single<&mut Window>, settings: Res<Settings>) {
    window.resolution.set(
        settings.window_width.max(480.0),
        settings.window_height.max(400.0),
    );
}

pub fn native_window_sync(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let (needs_initialization, should_save) = {
        let mut state = world.resource_mut::<NativeWindowState>();
        state.save_elapsed += delta;
        let should_save = state.save_elapsed >= 1.0;
        if should_save {
            state.save_elapsed = 0.0;
        }
        (!state.initialized, should_save)
    };

    let Some(window_entity) = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(world)
        .next()
    else {
        return;
    };

    if needs_initialization {
        let start_maximized = world.resource::<Settings>().window_maximized;
        if let Some(icon) = make_window_icon() {
            let initialized = bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
                let Some(window) = windows.get_window(window_entity) else {
                    return false;
                };
                window.set_window_icon(Some(icon));
                window.set_maximized(start_maximized);
                true
            });
            if initialized {
                world.resource_mut::<NativeWindowState>().initialized = true;
            }
        }
    }

    if !should_save {
        return;
    }
    let Some(window) = world.get::<Window>(window_entity) else {
        return;
    };
    let width = window.resolution.width();
    let height = window.resolution.height();
    let maximized = bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
        windows
            .get_window(window_entity)
            .map(|window| window.is_maximized())
    });
    let mut settings = world.resource_mut::<Settings>();
    if (width - settings.window_width).abs() > 0.5 || (height - settings.window_height).abs() > 0.5
    {
        settings.window_width = width;
        settings.window_height = height;
    }
    if let Some(maximized) = maximized {
        settings.window_maximized = maximized;
    }
}

fn make_window_icon() -> Option<Icon> {
    const SIZE: u32 = 128;
    let tree = resvg::usvg::Tree::from_data(
        include_bytes!("../assets/icons/org.gnome.Mines.svg"),
        &resvg::usvg::Options::default(),
    )
    .ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(SIZE, SIZE)?;
    let tree_size = tree.size();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            SIZE as f32 / tree_size.width(),
            SIZE as f32 / tree_size.height(),
        ),
        &mut pixmap.as_mut(),
    );
    Icon::from_rgba(pixmap.take(), SIZE, SIZE).ok()
}

fn theme_index(name: &str) -> usize {
    THEMES
        .iter()
        .position(|(theme, _)| *theme == name)
        .unwrap_or_default()
}

fn format_time(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_number_svg_uses_the_accessible_palette() {
        let mut options = resvg::usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        for (theme_index, theme) in TILE_SVGS.iter().enumerate() {
            for (number_index, svg) in theme.iter().take(8).enumerate() {
                set_number_color(&mut options, Some(number_index));
                let image = svg_image(svg, 128, &options);
                let pixels = image.data.expect("raster image data should be retained");
                let expected = NUMBER_COLORS[number_index].1;

                assert!(
                    pixels
                        .chunks_exact(4)
                        .any(|pixel| { pixel[0..3] == expected && pixel[3] > 0 }),
                    "theme {theme_index} number {} did not use its palette color",
                    number_index + 1
                );
            }
        }
    }

    #[test]
    fn transition_easing_has_exact_endpoints() {
        for easing in [TransitionEasing::CubicOut, TransitionEasing::BackOut] {
            assert_eq!(transition_easing(0.0, easing), 0.0);
            assert!((transition_easing(1.0, easing) - 1.0).abs() < f32::EPSILON);
        }
    }
}
