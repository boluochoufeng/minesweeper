// SPDX-License-Identifier: GPL-3.0-or-later
//
// UI theming: resolves the user's theme preference (including "follow the
// system") into a concrete palette, rebuilds the Feathers `UiTheme` resource and
// keeps the OS window chrome in sync.

pub mod palette;
pub mod tokens;

use crate::storage::Settings;
use bevy::color::{Alpha, Luminance};
use bevy::feathers::{
    dark_theme::create_dark_theme,
    theme::{InheritableThemeTextColor, ThemeProps, UiTheme},
    tokens::semantic,
};
use bevy::prelude::*;
use bevy::text::FontSource;
use bevy::window::{PrimaryWindow, WindowTheme};
use palette::Palette;

/// User preference for the UI color theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
    HighContrast,
}

impl ThemeMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "dark" => Self::Dark,
            "high-contrast" => Self::HighContrast,
            _ => Self::System,
        }
    }

    fn resolve(self, system_theme: Option<WindowTheme>) -> ResolvedTheme {
        match self {
            Self::System => match system_theme {
                Some(WindowTheme::Dark) => ResolvedTheme::Dark,
                _ => ResolvedTheme::Light,
            },
            Self::Light => ResolvedTheme::Light,
            Self::Dark => ResolvedTheme::Dark,
            Self::HighContrast => ResolvedTheme::HighContrast,
        }
    }
}

/// Concrete theme that is actually rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResolvedTheme {
    #[default]
    Light,
    Dark,
    HighContrast,
}

impl ResolvedTheme {
    fn palette(self) -> Palette {
        match self {
            Self::Light => Palette::light(),
            Self::Dark => Palette::dark(),
            Self::HighContrast => Palette::high_contrast(),
        }
    }
}

/// Current resolved palette, read by the board and other custom widgets.
#[derive(Resource, Clone)]
pub struct ThemePalette(pub Palette);

impl Default for ThemePalette {
    fn default() -> Self {
        Self(Palette::light())
    }
}

/// Tracked theme state, used to detect when a rebuild is needed.
#[derive(Resource, Default, Debug)]
pub struct ActiveTheme {
    pub mode: ThemeMode,
    pub resolved: ResolvedTheme,
    pub initialized: bool,
}

/// Font family chain used for all UI text. Covers Latin and CJK glyphs through
/// the system font database (`system_font_discovery`).
pub const UI_FONT_FAMILIES: &str = "Noto Sans SC, Noto Sans CJK SC, Microsoft YaHei UI, Microsoft YaHei, PingFang SC, Noto Sans, sans-serif";

/// Resolved UI font source.
pub fn ui_font() -> FontSource {
    FontSource::families(UI_FONT_FAMILIES)
}

/// Monotonic counter bumped every time the resolved theme is rebuilt. Systems
/// that must run exactly once per theme change compare this against a `Local`.
#[derive(Resource, Default, Debug)]
pub struct ThemeRevision(pub u64);

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        // Build the initial theme from the loaded settings so the first frame
        // already has colors instead of the Feathers placeholder theme.
        let palette = app
            .world()
            .get_resource::<Settings>()
            .map(|settings| ThemeMode::parse(&settings.ui_theme).resolve(None).palette())
            .unwrap_or_else(Palette::light);
        app.insert_resource(UiTheme(build_theme(&palette)))
            .insert_resource(ThemePalette(palette))
            .init_resource::<ActiveTheme>()
            .init_resource::<ThemeRevision>()
            .add_systems(Update, (sync_theme, repaint_inherited_text_colors).chain());
    }
}

/// Feathers widgets resolve [`InheritableThemeTextColor`] once, when the
/// component is inserted, and store the result in a propagated `TextColor`.
/// Re-insert the component whenever the theme changes so widget captions,
/// checkbox labels, slider values and themed icons are repainted with the new
/// palette instead of keeping the colors of the theme they were spawned under.
fn repaint_inherited_text_colors(
    mut commands: Commands,
    revision: Res<ThemeRevision>,
    mut seen: Local<u64>,
    inherited: Query<(Entity, &InheritableThemeTextColor)>,
) {
    if *seen == revision.0 {
        return;
    }
    *seen = revision.0;
    for (entity, InheritableThemeTextColor(token)) in &inherited {
        commands
            .entity(entity)
            .remove::<InheritableThemeTextColor>()
            .insert(InheritableThemeTextColor(token.clone()));
    }
}

/// Rebuild the palette and `UiTheme` when the preference or system theme
/// changes.
fn sync_theme(
    settings: Res<Settings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut active: ResMut<ActiveTheme>,
    mut palette: ResMut<ThemePalette>,
    mut ui_theme: ResMut<UiTheme>,
    mut revision: ResMut<ThemeRevision>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let mode = ThemeMode::parse(&settings.ui_theme);

    // Ask the platform to paint the title bar in the matching variant.
    let window_theme = match mode {
        ThemeMode::System => None,
        ThemeMode::Light => Some(WindowTheme::Light),
        ThemeMode::Dark | ThemeMode::HighContrast => Some(WindowTheme::Dark),
    };
    let system_theme = if window_theme.is_none() {
        window.window_theme
    } else {
        None
    };
    if window.window_theme != window_theme {
        window.window_theme = window_theme;
    }

    let resolved = mode.resolve(system_theme);
    if active.initialized && active.mode == mode && active.resolved == resolved {
        return;
    }
    active.mode = mode;
    active.resolved = resolved;
    active.initialized = true;

    let next = resolved.palette();
    ui_theme.0 = build_theme(&next);
    *palette = ThemePalette(next);
    revision.0 = revision.0.wrapping_add(1);
}

/// Build Feathers theme properties from a fully resolved palette.
pub fn build_theme(palette: &Palette) -> ThemeProps {
    let mut theme = create_dark_theme();
    // The base palette already encodes every surface level; drop the dark
    // theme's context-specific tuning so colors stay consistent.
    theme.semantic_overrides.clear();

    let base: [(_, Color); 24] = [
        (semantic::SURFACE_WINDOW, palette.app_bg),
        (semantic::SURFACE_PANE_BODY, palette.surface_bg),
        (semantic::SURFACE_PANE_HEADER, palette.header_bg),
        (semantic::SURFACE_SUBPANE_BODY, palette.surface_bg),
        (semantic::SURFACE_SUBPANE_HEADER, palette.surface_alt),
        (semantic::SURFACE_GROUP, palette.surface_alt),
        (semantic::SURFACE_DIALOG, palette.dialog_bg),
        (semantic::SURFACE_MENU, palette.dialog_bg),
        (semantic::BORDER_DEFAULT, palette.surface_border),
        (semantic::TEXT_DEFAULT, palette.text_primary),
        (semantic::TEXT_DIM, palette.text_secondary),
        (semantic::TEXT_DISABLED, palette.text_disabled),
        (semantic::TEXT_ON_ACCENT, palette.on_accent),
        (semantic::FILL_ACCENT_DEFAULT, palette.accent),
        (semantic::FILL_ACCENT_HOVER, palette.accent_hover),
        (semantic::FILL_ACCENT_PRESSED, palette.accent_pressed),
        (
            semantic::FILL_ACCENT_DISABLED,
            palette.accent.with_alpha(0.5),
        ),
        (semantic::FILL_SOLID_DEFAULT, palette.control_bg),
        (semantic::FILL_SOLID_HOVER, palette.control_bg_hover),
        (semantic::FILL_SOLID_PRESSED, palette.control_bg_pressed),
        (semantic::FILL_SOLID_DISABLED, palette.control_bg_disabled),
        (semantic::FILL_ITEM_HOVER, palette.surface_alt),
        (semantic::FILL_ITEM_PRESSED, palette.control_bg_pressed),
        (semantic::FOCUS_RING, palette.focus_ring),
    ];
    for (token, color) in base {
        theme.semantic_base.insert(token, color);
    }
    theme
        .semantic_base
        .insert(semantic::FILL_FIELD_DEFAULT, palette.field_bg);
    theme
        .semantic_base
        .insert(semantic::FILL_FIELD_HOVER, palette.field_bg.lighter(0.03));
    theme
        .semantic_base
        .insert(semantic::FILL_FIELD_PRESSED, palette.field_bg.lighter(0.06));
    theme
        .semantic_base
        .insert(semantic::FILL_ITEM_DEFAULT, Color::NONE);
    theme.semantic_base.insert(semantic::FILL_NONE, Color::NONE);

    // The Feathers dark theme assumes white-ish text on dark controls; the light
    // palettes need those text tokens mapped back to the default text color.
    let text_tokens: [(
        &bevy::feathers::theme::ThemeToken,
        &bevy::feathers::theme::SemanticToken,
    ); 5] = [
        (
            &bevy::feathers::tokens::BUTTON_TEXT,
            &semantic::TEXT_DEFAULT,
        ),
        (
            &bevy::feathers::tokens::MENUITEM_TEXT,
            &semantic::TEXT_DEFAULT,
        ),
        (
            &bevy::feathers::tokens::LISTROW_TEXT,
            &semantic::TEXT_DEFAULT,
        ),
        (
            &bevy::feathers::tokens::TEXT_INPUT_TEXT,
            &semantic::TEXT_DEFAULT,
        ),
        (
            &bevy::feathers::tokens::SLIDER_TEXT,
            &semantic::TEXT_DEFAULT,
        ),
    ];
    for (design, semantic_token) in text_tokens {
        theme
            .token_assignments
            .insert(design.clone(), semantic_token.clone());
    }
    for token in [
        bevy::feathers::tokens::BUTTON_TEXT_DISABLED,
        bevy::feathers::tokens::MENUITEM_TEXT_DISABLED,
        bevy::feathers::tokens::LISTROW_TEXT_DISABLED,
        bevy::feathers::tokens::TEXT_INPUT_TEXT_DISABLED,
        bevy::feathers::tokens::SLIDER_TEXT_DISABLED,
    ] {
        theme
            .token_assignments
            .insert(token, semantic::TEXT_DISABLED.clone());
    }

    // Custom Mines tokens.
    let custom: Vec<(bevy::feathers::theme::SemanticToken, Color)> = vec![
        (tokens::HEADER_BG_SEM, palette.header_bg),
        (tokens::HEADER_BORDER_SEM, palette.header_border),
        (tokens::SURFACE_BG_SEM, palette.surface_bg),
        (tokens::SURFACE_BORDER_SEM, palette.surface_border),
        (tokens::SURFACE_ALT_SEM, palette.surface_alt),
        (tokens::PILL_BG_SEM, palette.pill_bg),
        (tokens::PILL_BORDER_SEM, palette.pill_border),
        (tokens::TEXT_SECONDARY_SEM, palette.text_secondary),
        (tokens::TEXT_ACCENT_SEM, palette.accent),
        (tokens::TEXT_DANGER_SEM, palette.danger),
        (tokens::TEXT_SUCCESS_SEM, palette.success),
        (tokens::SCRIM_BG_SEM, palette.scrim),
        (tokens::CONTROL_BORDER_SEM, palette.field_border),
        (tokens::TAB_BG_ACTIVE_SEM, palette.accent.with_alpha(0.14)),
        (tokens::TAB_TEXT_ACTIVE_SEM, palette.accent),
        (tokens::SLIDER_TRACK_SEM, palette.field_border),
        (tokens::SLIDER_FILL_SEM, palette.accent),
        (tokens::SLIDER_FILL_HOVER_SEM, palette.accent_hover),
    ];
    for (token, color) in custom {
        theme.semantic_base.insert(token, color);
    }

    // Give the checkbox and select outlines enough contrast on light surfaces.
    theme
        .token_assignments
        .insert(tokens::CHECKBOX_BORDER, tokens::CONTROL_BORDER_SEM);
    theme
        .token_assignments
        .insert(tokens::CHECKBOX_BORDER_HOVER, tokens::CONTROL_BORDER_SEM);
    theme
        .token_assignments
        .insert(tokens::CHECKBOX_BG, semantic::FILL_SOLID_DEFAULT);
    theme
        .token_assignments
        .insert(tokens::CHECKBOX_BG_HOVER, semantic::FILL_SOLID_HOVER);
    theme
        .token_assignments
        .insert(tokens::CHECKBOX_TEXT, semantic::TEXT_DEFAULT);

    // Make slider tracks and value bars readable on the dialog surface.
    for design in [
        bevy::feathers::tokens::SLIDER_BG,
        bevy::feathers::tokens::SLIDER_BG_HOVER,
        bevy::feathers::tokens::SLIDER_BG_PRESSED,
        bevy::feathers::tokens::SLIDER_BG_DISABLED,
    ] {
        theme
            .token_assignments
            .insert(design, tokens::SLIDER_TRACK_SEM);
    }
    theme
        .token_assignments
        .insert(bevy::feathers::tokens::SLIDER_BAR, tokens::SLIDER_FILL_SEM);
    for design in [
        bevy::feathers::tokens::SLIDER_BAR_HOVER,
        bevy::feathers::tokens::SLIDER_BAR_PRESSED,
    ] {
        theme
            .token_assignments
            .insert(design, tokens::SLIDER_FILL_HOVER_SEM);
    }
    theme.token_assignments.insert(
        bevy::feathers::tokens::SLIDER_BAR_DISABLED,
        tokens::SLIDER_FILL_SEM,
    );

    let assignments: Vec<(
        bevy::feathers::theme::ThemeToken,
        bevy::feathers::theme::SemanticToken,
    )> = vec![
        (tokens::HEADER_BG, tokens::HEADER_BG_SEM),
        (tokens::HEADER_BORDER, tokens::HEADER_BORDER_SEM),
        (tokens::SURFACE_BG, tokens::SURFACE_BG_SEM),
        (tokens::SURFACE_BORDER, tokens::SURFACE_BORDER_SEM),
        (tokens::SURFACE_ALT, tokens::SURFACE_ALT_SEM),
        (tokens::PILL_BG, tokens::PILL_BG_SEM),
        (tokens::PILL_BORDER, tokens::PILL_BORDER_SEM),
        (tokens::TEXT_SECONDARY, tokens::TEXT_SECONDARY_SEM),
        (tokens::TEXT_ACCENT, tokens::TEXT_ACCENT_SEM),
        (tokens::TEXT_DANGER, tokens::TEXT_DANGER_SEM),
        (tokens::TEXT_SUCCESS, tokens::TEXT_SUCCESS_SEM),
        (tokens::SCRIM_BG, tokens::SCRIM_BG_SEM),
        (tokens::CONTROL_BORDER, tokens::CONTROL_BORDER_SEM),
        (tokens::TAB_BG_ACTIVE, tokens::TAB_BG_ACTIVE_SEM),
        (tokens::TAB_TEXT_ACTIVE, tokens::TAB_TEXT_ACTIVE_SEM),
    ];
    for (design, semantic_token) in assignments {
        theme.token_assignments.insert(design, semantic_token);
    }

    theme
}
