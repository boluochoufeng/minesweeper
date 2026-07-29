// SPDX-License-Identifier: GPL-3.0-or-later

use crate::game::{Board, GameClock, Mark, Outcome};
use crate::storage::{Difficulty, PersistentData};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowFocused};
use bevy_egui::{EguiContexts, egui};
use rand::rng;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::Duration;
use winit::window::Icon;

const THEMES: [(&str, &str); 4] = [
    ("bgcolors", "Background Colors"),
    ("classic", "Classic"),
    ("default", "Default"),
    ("modern", "Modern"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    NewGame,
    Custom,
    Game,
}

#[derive(Debug, Clone, Copy)]
enum PendingAction {
    ShowNewGame,
    Start(Difficulty),
    Restart,
}

#[derive(Debug, Clone, Copy)]
struct ConfirmState {
    action: PendingAction,
    was_paused: bool,
    was_pause_requested: bool,
}

#[derive(Debug, Clone, Copy)]
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
}

#[derive(Resource, Default)]
pub struct NativeWindowState {
    initialized: bool,
    save_elapsed: f32,
}

impl MinesApp {
    pub fn new(persistent: PersistentData, cli_difficulty: Option<Difficulty>) -> Self {
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
        };
        if let Some(difficulty) = cli_difficulty {
            app.start_game(difficulty);
        }
        app
    }

    fn start_game(&mut self, difficulty: Difficulty) {
        self.persistent.settings.difficulty = difficulty;
        let (width, height, mines) = difficulty.dimensions(&self.persistent.settings);
        self.board = Some(Board::new(width, height, mines));
        self.clock = GameClock::default();
        self.paused = false;
        self.pause_requested = false;
        self.auto_paused = false;
        self.keyboard_cursor = 0;
        self.keyboard_active = false;
        self.screen = Screen::Game;
        self.dialog = None;
        self.persistent.save();
    }

    fn execute(&mut self, action: PendingAction) {
        match action {
            PendingAction::ShowNewGame => {
                self.board = None;
                self.clock.pause();
                self.clock = GameClock::default();
                self.screen = Screen::NewGame;
                self.paused = false;
                self.pause_requested = false;
                self.dialog = None;
            }
            PendingAction::Start(difficulty) => self.start_game(difficulty),
            PendingAction::Restart => {
                let difficulty = self.persistent.settings.difficulty;
                self.start_game(difficulty);
            }
        }
    }

    fn request(&mut self, action: PendingAction) {
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
        } else {
            self.execute(action);
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
    }

    fn toggle_pause(&mut self) {
        if !self.clock.has_started() {
            return;
        }
        self.set_paused(!self.paused, true);
    }

    fn apply_cell_action(&mut self, action: CellAction) {
        if self.paused {
            return;
        }
        let Some(board) = self.board.as_mut() else {
            return;
        };
        let settings = &self.persistent.settings;
        match action {
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
            }
            CellAction::Mark(x, y) => {
                board.cycle_mark(
                    x,
                    y,
                    settings.use_question_marks,
                    settings.auto_clear_on_flags,
                    settings.use_autoflag,
                );
            }
            CellAction::Chord(x, y) => {
                board.chord(x, y, settings.auto_clear_on_flags, settings.use_autoflag);
            }
        }
        self.process_outcome();
    }

    fn process_outcome(&mut self) {
        let Some(board) = self.board.as_ref() else {
            return;
        };
        match board.outcome {
            Outcome::InProgress => {}
            Outcome::Lost => self.clock.pause(),
            Outcome::Won => {
                if matches!(self.dialog, Some(Dialog::Victory)) {
                    return;
                }
                self.clock.pause();
                let seconds = self.clock.elapsed().as_secs_f64().round() as u64;
                self.persistent
                    .add_score(board.width, board.height, board.mine_count, seconds);
                self.dialog = Some(Dialog::Victory);
            }
        }
    }

    fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
    }
}

pub fn mines_ui(
    mut contexts: EguiContexts,
    mut app: ResMut<MinesApp>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    egui_extras::install_image_loaders(ctx);
    configure_style(ctx);
    ctx.request_repaint_after(Duration::from_millis(200));

    handle_keyboard(ctx, &keyboard, &mut app, &mut exit);
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    render_header(&mut viewport_ui, &mut app, &mut exit);

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(246, 245, 244))
                .inner_margin(18.0),
        )
        .show(&mut viewport_ui, |ui| match app.screen {
            Screen::NewGame => render_new_game(ui, &mut app),
            Screen::Custom => render_custom_game(ui, &mut app),
            Screen::Game => render_game(ui, &mut app),
        });

    render_dialog(ctx, &mut app);
    Ok(())
}

fn configure_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    let mut style = (*ctx.style_of(egui::Theme::Light)).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.visuals = egui::Visuals::light();
    style.visuals.panel_fill = egui::Color32::from_rgb(246, 245, 244);
    style.visuals.window_fill = egui::Color32::from_rgb(250, 249, 247);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(24.0, egui::FontFamily::Proportional),
    );
    ctx.set_style_of(egui::Theme::Light, style);
}

fn render_header(
    viewport_ui: &mut egui::Ui,
    app: &mut MinesApp,
    exit: &mut MessageWriter<AppExit>,
) {
    egui::Panel::top("header")
        .exact_size(54.0)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(250, 249, 247))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(218, 216, 212),
                )),
        )
        .show(viewport_ui, |ui| {
            ui.horizontal(|ui| {
                if app.screen == Screen::Game {
                    if ui
                        .button(egui::RichText::new("<").size(20.0).strong())
                        .on_hover_text("Change Difficulty")
                        .clicked()
                    {
                        app.request(PendingAction::ShowNewGame);
                    }
                    if ui
                        .button(egui::RichText::new("↻").size(20.0))
                        .on_hover_text(
                            if app
                                .board
                                .as_ref()
                                .is_some_and(|b| b.outcome != Outcome::InProgress)
                            {
                                "Play Again"
                            } else {
                                "Start Over"
                            },
                        )
                        .clicked()
                    {
                        app.request(PendingAction::Restart);
                    }
                }

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Mines").strong().size(18.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button(egui::RichText::new("☰").size(20.0), |ui| {
                        if ui.button("Scores").clicked() {
                            app.open_dialog(Dialog::Scores);
                            ui.close();
                        }
                        if ui.button("Appearance").clicked() {
                            app.open_dialog(Dialog::Appearance);
                            ui.close();
                        }
                        ui.separator();
                        let question_changed = ui
                            .checkbox(
                                &mut app.persistent.settings.use_question_marks,
                                "Use Question Flags",
                            )
                            .changed();
                        let auto_changed = ui
                            .checkbox(
                                &mut app.persistent.settings.auto_clear_on_flags,
                                "Automatically clear on flags",
                            )
                            .changed();
                        if question_changed || auto_changed {
                            app.persistent.save();
                        }
                        ui.separator();
                        if ui.button("Keyboard Shortcuts").clicked() {
                            app.open_dialog(Dialog::Shortcuts);
                            ui.close();
                        }
                        if ui.button("Help").clicked() {
                            app.open_dialog(Dialog::Help);
                            ui.close();
                        }
                        if ui.button("About Mines").clicked() {
                            app.open_dialog(Dialog::About);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            app.persistent.save();
                            exit.write(AppExit::Success);
                        }
                    });

                    if app.screen == Screen::Game && app.clock.has_started() {
                        let symbol = if app.paused { "▶" } else { "⏸" };
                        if ui
                            .button(egui::RichText::new(symbol).size(18.0))
                            .on_hover_text(if app.paused { "Resume" } else { "Pause" })
                            .clicked()
                        {
                            app.toggle_pause();
                        }
                    }
                });
            });
        });
}

fn render_new_game(ui: &mut egui::Ui, app: &mut MinesApp) {
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() * 0.08).max(8.0));
        ui.heading("Choose Difficulty");
        ui.add_space(18.0);

        egui::Grid::new("difficulty_grid")
            .num_columns(2)
            .spacing(egui::vec2(18.0, 18.0))
            .show(ui, |ui| {
                difficulty_card(ui, app, Difficulty::Small, "8 × 8", "10 mines");
                difficulty_card(ui, app, Difficulty::Medium, "16 × 16", "40 mines");
                ui.end_row();
                difficulty_card(ui, app, Difficulty::Large, "30 × 16", "99 mines");
                if large_card(ui, "?", "Custom").clicked() {
                    app.screen = Screen::Custom;
                }
                ui.end_row();
            });
    });
}

fn difficulty_card(
    ui: &mut egui::Ui,
    app: &mut MinesApp,
    difficulty: Difficulty,
    size: &str,
    mines: &str,
) {
    if large_card(ui, size, mines).clicked() {
        app.request(PendingAction::Start(difficulty));
    }
}

fn large_card(ui: &mut egui::Ui, title: &str, subtitle: &str) -> egui::Response {
    ui.add_sized(
        egui::vec2(190.0, 116.0),
        egui::Button::new(
            egui::RichText::new(format!("{title}\n{subtitle}"))
                .size(22.0)
                .strong()
                .line_height(Some(32.0)),
        ),
    )
}

fn render_custom_game(ui: &mut egui::Ui, app: &mut MinesApp) {
    let mut width = app.persistent.settings.custom_width;
    let mut height = app.persistent.settings.custom_height;
    let old_size = (width, height);
    let old_mines = app.persistent.settings.custom_mines;
    let mut percentage = 100.0 * old_mines as f64 / (width * height) as f64;
    let mut percentage_changed = false;
    let mut start = false;
    let mut cancel = false;
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() * 0.12).max(12.0));
        ui.heading("Custom Game");
        ui.add_space(18.0);

        egui::Grid::new("custom_grid")
            .num_columns(2)
            .spacing(egui::vec2(16.0, 10.0))
            .show(ui, |ui| {
                ui.label("Width");
                ui.add(egui::DragValue::new(&mut width).range(4..=100));
                ui.end_row();
                ui.label("Height");
                ui.add(egui::DragValue::new(&mut height).range(4..=100));
                ui.end_row();
                ui.label("Percent mines");
                percentage_changed = ui
                    .add(
                        egui::DragValue::new(&mut percentage)
                            .range(0.01..=99.0)
                            .suffix("%")
                            .speed(1.0),
                    )
                    .changed();
                ui.end_row();
            });

        ui.add_space(18.0);
        if ui
            .add_sized(
                egui::vec2(220.0, 40.0),
                egui::Button::new(egui::RichText::new("Play Game").strong()),
            )
            .clicked()
        {
            start = true;
        }
        if ui
            .add_sized(egui::vec2(220.0, 36.0), egui::Button::new("Cancel"))
            .clicked()
        {
            cancel = true;
        }
    });

    let size = width * height;
    let mines = if old_size != (width, height) && !percentage_changed {
        old_mines.clamp(1, size - 10)
    } else {
        ((percentage * size as f64 / 100.0).round() as usize).clamp(1, size - 10)
    };
    let settings = &mut app.persistent.settings;
    if (
        settings.custom_width,
        settings.custom_height,
        settings.custom_mines,
    ) != (width, height, mines)
    {
        settings.custom_width = width;
        settings.custom_height = height;
        settings.custom_mines = mines;
        app.persistent.save();
    }
    if start {
        app.request(PendingAction::Start(Difficulty::Custom));
    } else if cancel {
        app.screen = Screen::NewGame;
    }
}

fn render_game(ui: &mut egui::Ui, app: &mut MinesApp) {
    let landscape = ui.available_width() > ui.available_height() * 1.15;
    if landscape {
        ui.horizontal(|ui| {
            let status_width = 118.0;
            ui.allocate_ui_with_layout(
                egui::vec2(
                    (ui.available_width() - status_width).max(120.0),
                    ui.available_height(),
                ),
                egui::Layout::top_down(egui::Align::Center),
                |ui| render_board(ui, app),
            );
            ui.separator();
            ui.allocate_ui_with_layout(
                egui::vec2(status_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Center).with_main_align(egui::Align::Center),
                |ui| render_status(ui, app),
            );
        });
    } else {
        let status_height = 74.0;
        ui.allocate_ui_with_layout(
            egui::vec2(
                ui.available_width(),
                (ui.available_height() - status_height).max(120.0),
            ),
            egui::Layout::top_down(egui::Align::Center),
            |ui| render_board(ui, app),
        );
        ui.separator();
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), status_height),
            egui::Layout::left_to_right(egui::Align::Center).with_main_align(egui::Align::Center),
            |ui| render_status(ui, app),
        );
    }
}

fn render_status(ui: &mut egui::Ui, app: &MinesApp) {
    let Some(board) = app.board.as_ref() else {
        return;
    };
    let seconds = app.clock.elapsed().as_secs_f64().round() as u64;
    let time_text = if seconds >= 3600 {
        format!(
            "{:02}∶{:02}∶{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    } else {
        format!("{:02}∶{:02}", seconds / 60, seconds % 60)
    };
    let draw = |ui: &mut egui::Ui, icon: egui::ImageSource<'static>, value: String| {
        ui.allocate_ui_with_layout(
            egui::vec2(104.0, 68.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(32.0, 32.0)));
                ui.label(egui::RichText::new(value).size(17.0).monospace());
            },
        );
    };
    draw(
        ui,
        egui::include_image!("../assets/icons/flag-symbolic.svg"),
        format!("{}/{}", board.flag_count, board.mine_count),
    );
    ui.add_space(if ui.layout().main_dir().is_vertical() {
        18.0
    } else {
        30.0
    });
    draw(
        ui,
        egui::include_image!("../assets/icons/stopwatch-symbolic.svg"),
        time_text,
    );
}

fn render_board(ui: &mut egui::Ui, app: &mut MinesApp) {
    let Some(board) = app.board.as_ref() else {
        return;
    };
    let width = board.width;
    let height = board.height;
    let available = ui.available_rect_before_wrap();
    let tile_size = (available.width() / width as f32)
        .min(available.height() / height as f32)
        .clamp(3.0, 64.0);
    let board_size = egui::vec2(tile_size * width as f32, tile_size * height as f32);
    let origin = available.center() - board_size * 0.5;
    let board_rect = egui::Rect::from_min_size(origin, board_size);
    ui.allocate_rect(available, egui::Sense::hover());
    let painter = ui.painter_at(board_rect);
    let mut action = None;

    for y in 0..height {
        for x in 0..width {
            let index = board.index(x, y);
            let cell = board.cells[index];
            let min = origin + egui::vec2(x as f32 * tile_size, y as f32 * tile_size);
            let rect = egui::Rect::from_min_size(min, egui::vec2(tile_size, tile_size));
            let response = ui.interact(
                rect,
                ui.id().with(("mine", index)),
                if app.paused {
                    egui::Sense::hover()
                } else {
                    egui::Sense::click()
                },
            );

            let keyboard_cursor =
                app.keyboard_active && app.keyboard_cursor == index && !app.paused;
            let fill = tile_fill(
                &app.persistent.settings.theme,
                cell,
                board.outcome,
                keyboard_cursor,
                response.hovered(),
                response.is_pointer_button_down_on(),
            );
            painter.rect_filled(rect.shrink(0.5), egui::CornerRadius::same(3), fill);
            painter.rect_stroke(
                rect.shrink(0.5),
                egui::CornerRadius::same(3),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(246, 245, 244)),
                egui::StrokeKind::Inside,
            );

            if let Some(icon) = cell_icon(cell, board.outcome) {
                let icon_rect = rect.shrink((tile_size * 0.13).clamp(0.5, 6.0));
                ui.put(
                    icon_rect,
                    egui::Image::new(theme_image_source(&app.persistent.settings.theme, icon))
                        .fit_to_exact_size(icon_rect.size()),
                );
            }

            let ctrl = ui.ctx().input(|input| input.modifiers.ctrl);
            if response.secondary_clicked()
                || (ctrl && response.clicked_by(egui::PointerButton::Primary))
            {
                action = Some(CellAction::Mark(x, y));
            } else if response.clicked_by(egui::PointerButton::Primary) {
                action = Some(if cell.revealed {
                    CellAction::Chord(x, y)
                } else {
                    CellAction::Reveal(x, y)
                });
            }
        }
    }

    if app.paused {
        painter.rect_filled(
            board_rect,
            egui::CornerRadius::same(4),
            egui::Color32::from_rgba_unmultiplied(186, 189, 182, 242),
        );
        painter.text(
            board_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Paused",
            egui::FontId::proportional(32.0),
            egui::Color32::WHITE,
        );
        let response = ui.interact(board_rect, ui.id().with("paused"), egui::Sense::click());
        if response.clicked() {
            app.set_paused(false, true);
        }
    }

    if let Some(action) = action {
        app.keyboard_active = false;
        app.apply_cell_action(action);
    }
}

fn tile_fill(
    theme: &str,
    cell: crate::game::Cell,
    outcome: Outcome,
    cursor: bool,
    hovered: bool,
    pressed: bool,
) -> egui::Color32 {
    if cursor {
        return egui::Color32::from_rgb(233, 185, 110);
    }
    if pressed {
        return egui::Color32::from_rgb(136, 138, 133);
    }
    if cell.revealed {
        if outcome == Outcome::Lost && cell.mine {
            return if theme == "modern" {
                egui::Color32::from_rgb(245, 194, 17)
            } else {
                egui::Color32::from_rgb(211, 215, 207)
            };
        }
        if theme == "bgcolors" && cell.adjacent > 0 {
            const COLORS: [(u8, u8, u8); 8] = [
                (221, 250, 195),
                (236, 237, 191),
                (237, 218, 180),
                (237, 195, 138),
                (247, 161, 162),
                (254, 167, 133),
                (255, 125, 96),
                (255, 50, 60),
            ];
            let (r, g, b) = COLORS[cell.adjacent as usize - 1];
            return egui::Color32::from_rgb(r, g, b);
        }
        if outcome == Outcome::Lost && theme == "modern" {
            egui::Color32::from_rgb(192, 191, 188)
        } else {
            egui::Color32::from_rgb(222, 222, 220)
        }
    } else if outcome == Outcome::Lost {
        if theme == "modern" {
            egui::Color32::from_rgb(154, 153, 150)
        } else {
            egui::Color32::from_rgb(136, 138, 133)
        }
    } else if hovered {
        egui::Color32::from_rgb(211, 215, 207)
    } else {
        egui::Color32::from_rgb(186, 189, 182)
    }
}

fn cell_icon(cell: crate::game::Cell, outcome: Outcome) -> Option<&'static str> {
    if cell.revealed {
        if cell.mine {
            Some("exploded")
        } else if cell.adjacent > 0 {
            Some(match cell.adjacent {
                1 => "1mines",
                2 => "2mines",
                3 => "3mines",
                4 => "4mines",
                5 => "5mines",
                6 => "6mines",
                7 => "7mines",
                _ => "8mines",
            })
        } else {
            None
        }
    } else if outcome == Outcome::Lost && cell.mark == Mark::Flag && !cell.mine {
        Some("incorrect")
    } else if outcome == Outcome::Lost && cell.mine && cell.mark != Mark::Flag {
        Some("mine")
    } else {
        match cell.mark {
            Mark::None => None,
            Mark::Flag => Some("flag"),
            Mark::Maybe => Some("maybe"),
        }
    }
}

macro_rules! included_svg {
    ($theme:literal, $icon:literal) => {
        egui::ImageSource::Bytes {
            uri: Cow::Borrowed(concat!("bytes://themes/", $theme, "/", $icon, ".svg")),
            bytes: egui::load::Bytes::Static(include_bytes!(concat!(
                "../assets/themes/",
                $theme,
                "/",
                $icon,
                ".svg"
            ))),
        }
    };
}

fn theme_image_source(theme: &str, icon: &str) -> egui::ImageSource<'static> {
    match (theme, icon) {
        ("bgcolors", "1mines") => included_svg!("bgcolors", "1mines"),
        ("bgcolors", "2mines") => included_svg!("bgcolors", "2mines"),
        ("bgcolors", "3mines") => included_svg!("bgcolors", "3mines"),
        ("bgcolors", "4mines") => included_svg!("bgcolors", "4mines"),
        ("bgcolors", "5mines") => included_svg!("bgcolors", "5mines"),
        ("bgcolors", "6mines") => included_svg!("bgcolors", "6mines"),
        ("bgcolors", "7mines") => included_svg!("bgcolors", "7mines"),
        ("bgcolors", "8mines") => included_svg!("bgcolors", "8mines"),
        ("bgcolors", "flag") => included_svg!("bgcolors", "flag"),
        ("bgcolors", "maybe") => included_svg!("bgcolors", "maybe"),
        ("bgcolors", "mine") => included_svg!("bgcolors", "mine"),
        ("bgcolors", "incorrect") => included_svg!("bgcolors", "incorrect"),
        ("bgcolors", "exploded") => included_svg!("bgcolors", "exploded"),
        ("classic", "1mines") => included_svg!("classic", "1mines"),
        ("classic", "2mines") => included_svg!("classic", "2mines"),
        ("classic", "3mines") => included_svg!("classic", "3mines"),
        ("classic", "4mines") => included_svg!("classic", "4mines"),
        ("classic", "5mines") => included_svg!("classic", "5mines"),
        ("classic", "6mines") => included_svg!("classic", "6mines"),
        ("classic", "7mines") => included_svg!("classic", "7mines"),
        ("classic", "8mines") => included_svg!("classic", "8mines"),
        ("classic", "flag") => included_svg!("classic", "flag"),
        ("classic", "maybe") => included_svg!("classic", "maybe"),
        ("classic", "mine") => included_svg!("classic", "mine"),
        ("classic", "incorrect") => included_svg!("classic", "incorrect"),
        ("classic", "exploded") => included_svg!("classic", "exploded"),
        ("default", "1mines") => included_svg!("default", "1mines"),
        ("default", "2mines") => included_svg!("default", "2mines"),
        ("default", "3mines") => included_svg!("default", "3mines"),
        ("default", "4mines") => included_svg!("default", "4mines"),
        ("default", "5mines") => included_svg!("default", "5mines"),
        ("default", "6mines") => included_svg!("default", "6mines"),
        ("default", "7mines") => included_svg!("default", "7mines"),
        ("default", "8mines") => included_svg!("default", "8mines"),
        ("default", "flag") => included_svg!("default", "flag"),
        ("default", "maybe") => included_svg!("default", "maybe"),
        ("default", "mine") => included_svg!("default", "mine"),
        ("default", "incorrect") => included_svg!("default", "incorrect"),
        ("default", "exploded") => included_svg!("default", "exploded"),
        ("modern", "1mines") => included_svg!("modern", "1mines"),
        ("modern", "2mines") => included_svg!("modern", "2mines"),
        ("modern", "3mines") => included_svg!("modern", "3mines"),
        ("modern", "4mines") => included_svg!("modern", "4mines"),
        ("modern", "5mines") => included_svg!("modern", "5mines"),
        ("modern", "6mines") => included_svg!("modern", "6mines"),
        ("modern", "7mines") => included_svg!("modern", "7mines"),
        ("modern", "8mines") => included_svg!("modern", "8mines"),
        ("modern", "flag") => included_svg!("modern", "flag"),
        ("modern", "maybe") => included_svg!("modern", "maybe"),
        ("modern", "mine") => included_svg!("modern", "mine"),
        ("modern", "incorrect") => included_svg!("modern", "incorrect"),
        ("modern", "exploded") => included_svg!("modern", "exploded"),
        (_, icon) => theme_image_source("bgcolors", icon),
    }
}

fn render_dialog(ctx: &egui::Context, app: &mut MinesApp) {
    let Some(dialog) = app.dialog else {
        return;
    };
    match dialog {
        Dialog::Menu => render_menu(ctx, app),
        Dialog::Confirm(state) => render_confirm(ctx, app, state),
        Dialog::Scores => render_scores(ctx, app),
        Dialog::Appearance => render_appearance(ctx, app),
        Dialog::Shortcuts => render_shortcuts(ctx, app),
        Dialog::Help => render_help(ctx, app),
        Dialog::About => render_about(ctx, app),
        Dialog::Victory => render_victory(ctx, app),
    }
}

fn render_menu(ctx: &egui::Context, app: &mut MinesApp) {
    let mut open = true;
    egui::Window::new("Menu")
        .open(&mut open)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 62.0))
        .collapsible(false)
        .resizable(false)
        .default_width(290.0)
        .show(ctx, |ui| {
            if ui.button("Scores").clicked() {
                app.dialog = Some(Dialog::Scores);
            }
            if ui.button("Appearance").clicked() {
                app.dialog = Some(Dialog::Appearance);
            }
            ui.separator();
            let question_changed = ui
                .checkbox(
                    &mut app.persistent.settings.use_question_marks,
                    "Use Question Flags",
                )
                .changed();
            let auto_changed = ui
                .checkbox(
                    &mut app.persistent.settings.auto_clear_on_flags,
                    "Automatically clear on flags",
                )
                .changed();
            if question_changed || auto_changed {
                app.persistent.save();
            }
            ui.separator();
            if ui.button("Keyboard Shortcuts").clicked() {
                app.dialog = Some(Dialog::Shortcuts);
            }
            if ui.button("Help").clicked() {
                app.dialog = Some(Dialog::Help);
            }
            if ui.button("About Mines").clicked() {
                app.dialog = Some(Dialog::About);
            }
        });
    if !open {
        app.dialog = None;
    }
}

fn modal(title: &str) -> egui::Window<'_> {
    egui::Window::new(title)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .collapsible(false)
        .resizable(false)
}

fn render_confirm(ctx: &egui::Context, app: &mut MinesApp, state: ConfirmState) {
    modal("Do you want to start a new game?")
        .default_width(390.0)
        .min_width(390.0)
        .max_width(390.0)
        .show(ctx, |ui| {
            ui.label("If you start a new game, your current progress will be lost.");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Keep Current Game").clicked() {
                    app.dialog = None;
                    app.paused = state.was_paused;
                    app.pause_requested = state.was_pause_requested;
                    if !app.paused {
                        app.clock.resume();
                    }
                }
                if ui
                    .add(
                        egui::Button::new("Start New Game")
                            .fill(egui::Color32::from_rgb(192, 28, 40)),
                    )
                    .clicked()
                {
                    app.execute(state.action);
                }
            });
        });
}

fn render_scores(ctx: &egui::Context, app: &mut MinesApp) {
    let mut open = true;
    egui::Window::new("Scores")
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .collapsible(false)
        .resizable(true)
        .default_size(egui::vec2(430.0, 420.0))
        .show(ctx, |ui| {
            let mut groups: BTreeMap<(usize, usize, usize), Vec<_>> = BTreeMap::new();
            for score in &app.persistent.scores {
                groups
                    .entry((score.width, score.height, score.mines))
                    .or_default()
                    .push(score);
            }
            if groups.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.label("No scores yet.");
                    ui.label("Complete a minefield to record your time.");
                });
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for ((width, height, mines), scores) in groups {
                        ui.collapsing(format!("{width} × {height}, {mines} mines"), |ui| {
                            egui::Grid::new(("scores", width, height, mines))
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.strong("Rank");
                                    ui.strong("Time");
                                    ui.end_row();
                                    for (rank, score) in scores.iter().enumerate() {
                                        ui.label(format!("{}.", rank + 1));
                                        ui.monospace(format_duration(score.seconds));
                                        ui.end_row();
                                    }
                                });
                        });
                    }
                });
            }
        });
    if !open {
        app.dialog = None;
    }
}

fn render_appearance(ctx: &egui::Context, app: &mut MinesApp) {
    let mut open = true;
    egui::Window::new("Select Theme")
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(430.0, 390.0))
        .show(ctx, |ui| {
            ui.label("Choose a tile appearance:");
            ui.add_space(8.0);
            for (theme, name) in THEMES {
                let selected = app.persistent.settings.theme == theme;
                let frame = egui::Frame::group(ui.style()).fill(if selected {
                    egui::Color32::from_rgb(220, 234, 249)
                } else {
                    ui.visuals().faint_bg_color
                });
                let response = frame
                    .show(ui, |ui| {
                        ui.set_min_height(70.0);
                        ui.horizontal(|ui| {
                            for icon in ["1mines", "3mines", "flag", "mine"] {
                                ui.add(
                                    egui::Image::new(theme_image_source(theme, icon))
                                        .fit_to_exact_size(egui::vec2(44.0, 44.0)),
                                );
                            }
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new(name).strong());
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if response.clicked() {
                    app.persistent.settings.theme = theme.to_owned();
                    app.persistent.save();
                }
            }
        });
    if !open {
        app.dialog = None;
    }
}

fn render_shortcuts(ctx: &egui::Context, app: &mut MinesApp) {
    let mut open = true;
    egui::Window::new("Keyboard Shortcuts")
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .collapsible(false)
        .default_size(egui::vec2(480.0, 520.0))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                shortcut_section(
                    ui,
                    "Window and game",
                    &[
                        ("Ctrl+N", "Start a new game"),
                        ("Ctrl+R", "Restart with current settings"),
                        ("Pause", "Pause or resume the game"),
                        ("F1", "Show Help"),
                        ("F10", "Open Menu"),
                        ("Ctrl+?", "Show Keyboard Shortcuts"),
                        ("Ctrl+Q / Ctrl+W", "Quit"),
                    ],
                );
                shortcut_section(
                    ui,
                    "Game size selection",
                    &[
                        ("1", "Game size: small"),
                        ("2", "Game size: medium"),
                        ("3", "Game size: large"),
                        ("4", "Game size: custom"),
                    ],
                );
                shortcut_section(
                    ui,
                    "In-game",
                    &[
                        ("Arrows / U J H K", "Move the keyboard cursor"),
                        ("Space", "Reveal field"),
                        ("Ctrl+Space", "Toggle field flag state"),
                    ],
                );
            });
        });
    if !open {
        app.dialog = None;
    }
}

fn shortcut_section(ui: &mut egui::Ui, heading: &str, rows: &[(&str, &str)]) {
    ui.heading(heading);
    egui::Grid::new(heading).striped(true).show(ui, |ui| {
        for (keys, action) in rows {
            ui.monospace(*keys);
            ui.label(*action);
            ui.end_row();
        }
    });
    ui.add_space(14.0);
}

fn render_help(ctx: &egui::Context, app: &mut MinesApp) {
    const PAGES: [(&str, &str); 5] = [
        (
            "Game rules",
            "Clear every tile that does not contain a mine. A revealed number tells you how many mines are in the eight surrounding tiles. Revealing a mine ends the game. Your first revealed tile and all of its neighbours are always safe.",
        ),
        (
            "Using flags",
            "Right-click a hidden tile, Ctrl-click it, or press Ctrl+Space with the keyboard cursor to mark a suspected mine. With question flags enabled, the next mark is a question mark. Right-click again to clear it.",
        ),
        (
            "Clearing around a number",
            "Click a revealed number after placing the matching number of flags around it to reveal all other neighbouring tiles. Incorrect flags can make this action reveal a mine.",
        ),
        (
            "Board size",
            "Choose Small (8×8, 10 mines), Medium (16×16, 40 mines), Large (30×16, 99 mines), or configure a custom board from 4×4 up to 100×100.",
        ),
        (
            "High scores",
            "A completed game records its rounded completion time. Scores are grouped by exact board dimensions and mine count, with lower times ranked first.",
        ),
    ];
    let mut open = true;
    egui::Window::new("Mines Help")
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .collapsible(false)
        .default_size(egui::vec2(520.0, 360.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    for (index, (title, _)) in PAGES.iter().enumerate() {
                        if ui
                            .selectable_label(app.help_page == index, *title)
                            .clicked()
                        {
                            app.help_page = index;
                        }
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.heading(PAGES[app.help_page].0);
                    ui.add_space(8.0);
                    ui.label(PAGES[app.help_page].1);
                    ui.add_space(18.0);
                    ui.hyperlink_to(
                        "Open the complete GNOME Mines manual",
                        "https://help.gnome.org/users/gnome-mines/stable/",
                    );
                });
            });
        });
    if !open {
        app.dialog = None;
    }
}

fn render_about(ctx: &egui::Context, app: &mut MinesApp) {
    let mut open = true;
    egui::Window::new("About Mines")
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .collapsible(false)
        .resizable(false)
        .default_width(430.0)
        .min_width(430.0)
        .max_width(430.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../assets/icons/org.gnome.Mines.svg"))
                        .fit_to_exact_size(egui::vec2(96.0, 96.0)),
                );
                ui.heading("Mines");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("Clear explosive mines off the board");
                ui.add_space(10.0);
                ui.label("The GNOME Project");
                ui.label("Bevy recreation");
                ui.add_space(10.0);
                ui.hyperlink_to("GNOME Mines source", "https://github.com/GNOME/gnome-mines");
                ui.label("GNU General Public License v3.0 or later");
                ui.small("Artwork by Richard Hoelscher and GNOME Mines contributors.");
            });
        });
    if !open {
        app.dialog = None;
    }
}

fn render_victory(ctx: &egui::Context, app: &mut MinesApp) {
    let Some(board) = app.board.as_ref() else {
        app.dialog = None;
        return;
    };
    let elapsed = app.clock.elapsed().as_secs_f64().round() as u64;
    let description = format!(
        "{} × {}, {} mines",
        board.width, board.height, board.mine_count
    );
    modal("Minefield Cleared!")
        .default_width(390.0)
        .min_width(390.0)
        .max_width(390.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("✓")
                        .size(48.0)
                        .color(egui::Color32::from_rgb(38, 162, 105)),
                );
                ui.heading(format_duration(elapsed));
                ui.label(description);
                ui.label("Your score has been saved.");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Scores").clicked() {
                        app.dialog = Some(Dialog::Scores);
                    }
                    if ui
                        .add(
                            egui::Button::new("Play Again")
                                .fill(egui::Color32::from_rgb(53, 132, 228)),
                        )
                        .clicked()
                    {
                        app.execute(PendingAction::Restart);
                    }
                });
            });
        });
}

fn format_duration(seconds: u64) -> String {
    if seconds >= 3600 {
        format!(
            "{:02}∶{:02}∶{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    } else {
        format!("{:02}∶{:02}", seconds / 60, seconds % 60)
    }
}

fn handle_keyboard(
    ctx: &egui::Context,
    keyboard: &ButtonInput<KeyCode>,
    app: &mut MinesApp,
    exit: &mut MessageWriter<AppExit>,
) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if ctrl && keyboard.just_pressed(KeyCode::KeyQ) || ctrl && keyboard.just_pressed(KeyCode::KeyW)
    {
        app.persistent.save();
        exit.write(AppExit::Success);
        return;
    }
    if ctrl && keyboard.just_pressed(KeyCode::KeyN) {
        app.request(PendingAction::ShowNewGame);
    }
    if ctrl && keyboard.just_pressed(KeyCode::KeyR) {
        app.request(PendingAction::Restart);
    }
    if keyboard.just_pressed(KeyCode::Pause) {
        app.toggle_pause();
    }
    if keyboard.just_pressed(KeyCode::F1) {
        app.open_dialog(Dialog::Help);
    }
    if keyboard.just_pressed(KeyCode::F10) {
        app.open_dialog(Dialog::Menu);
    }
    if ctrl && shift && keyboard.just_pressed(KeyCode::Slash) {
        app.open_dialog(Dialog::Shortcuts);
    }
    if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
        app.request(PendingAction::Start(Difficulty::Small));
    }
    if keyboard.just_pressed(KeyCode::Digit2) || keyboard.just_pressed(KeyCode::Numpad2) {
        app.request(PendingAction::Start(Difficulty::Medium));
    }
    if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Numpad3) {
        app.request(PendingAction::Start(Difficulty::Large));
    }
    if keyboard.just_pressed(KeyCode::Digit4) || keyboard.just_pressed(KeyCode::Numpad4) {
        if app
            .board
            .as_ref()
            .is_some_and(|board| board.revealed_count > 0 && board.outcome == Outcome::InProgress)
        {
            app.request(PendingAction::ShowNewGame);
        }
        app.screen = Screen::Custom;
    }

    if keyboard.just_pressed(KeyCode::Escape)
        && app.dialog.is_none()
        && app
            .board
            .as_ref()
            .is_none_or(|board| board.revealed_count == 0)
    {
        app.execute(PendingAction::ShowNewGame);
    }

    if app.screen != Screen::Game
        || app.dialog.is_some()
        || app.paused
        || ctx.egui_wants_keyboard_input()
    {
        return;
    }
    let Some(board) = app.board.as_ref() else {
        return;
    };
    if board.outcome != Outcome::InProgress {
        return;
    }
    let (mut x, mut y) = board.coords(app.keyboard_cursor);
    let mut moved = false;
    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyH) {
        x = (x + board.width - 1) % board.width;
        moved = true;
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyK) {
        x = (x + 1) % board.width;
        moved = true;
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyU) {
        y = (y + board.height - 1) % board.height;
        moved = true;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyJ) {
        y = (y + 1) % board.height;
        moved = true;
    }
    if moved {
        app.keyboard_cursor = y * board.width + x;
        app.keyboard_active = true;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        app.keyboard_active = true;
        let action = if ctrl {
            CellAction::Mark(x, y)
        } else if board.cells[board.index(x, y)].revealed {
            CellAction::Chord(x, y)
        } else {
            CellAction::Reveal(x, y)
        };
        app.apply_cell_action(action);
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
        let start_maximized = world
            .resource::<MinesApp>()
            .persistent
            .settings
            .window_maximized;
        let Some(icon) = make_window_icon() else {
            return;
        };
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
    let mut app = world.resource_mut::<MinesApp>();
    let settings = &mut app.persistent.settings;
    let mut changed = false;
    if (width - settings.window_width).abs() > 0.5 || (height - settings.window_height).abs() > 0.5
    {
        settings.window_width = width;
        settings.window_height = height;
        changed = true;
    }
    if let Some(maximized) = maximized
        && maximized != settings.window_maximized
    {
        settings.window_maximized = maximized;
        changed = true;
    }
    if changed {
        app.persistent.save();
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
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Icon::from_rgba(pixmap.take(), SIZE, SIZE).ok()
}
