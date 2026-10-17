// SPDX-License-Identifier: GPL-3.0-or-later
//
// Custom design tokens for the Mines UI. Each pair associates a design token
// (used by `ThemeBackgroundColor` / `ThemeTextColor` components) with the
// semantic token that carries its color, following the Bevy Feathers two-stage
// theming model.

use bevy::feathers::theme::{SemanticToken, ThemeToken};

// Re-export the Feathers design tokens so UI code can use one import path.
pub use bevy::feathers::tokens::*;

macro_rules! token_pair {
    ($theme:ident, $semantic:ident, $key:literal, $doc:literal) => {
        #[doc = $doc]
        pub const $theme: ThemeToken = ThemeToken::new_static($key);
        /// Semantic token carrying the color of the paired design token.
        pub const $semantic: SemanticToken = SemanticToken::new_static($key);
    };
}

token_pair!(
    HEADER_BG,
    HEADER_BG_SEM,
    "mines.header.bg",
    "App header bar background."
);
token_pair!(
    HEADER_BORDER,
    HEADER_BORDER_SEM,
    "mines.header.border",
    "App header bottom border."
);
token_pair!(
    SURFACE_BG,
    SURFACE_BG_SEM,
    "mines.surface.bg",
    "Raised surface: board frame, cards."
);
token_pair!(
    SURFACE_BORDER,
    SURFACE_BORDER_SEM,
    "mines.surface.border",
    "Raised surface border."
);
token_pair!(
    SURFACE_ALT,
    SURFACE_ALT_SEM,
    "mines.surface.alt",
    "Subtle secondary surface."
);
token_pair!(
    PILL_BG,
    PILL_BG_SEM,
    "mines.pill.bg",
    "Status pill background."
);
token_pair!(
    PILL_BORDER,
    PILL_BORDER_SEM,
    "mines.pill.border",
    "Status pill border."
);
token_pair!(
    TEXT_SECONDARY,
    TEXT_SECONDARY_SEM,
    "mines.text.secondary",
    "Secondary label text."
);
token_pair!(
    TEXT_ACCENT,
    TEXT_ACCENT_SEM,
    "mines.text.accent",
    "Accent-colored text."
);
token_pair!(
    TEXT_DANGER,
    TEXT_DANGER_SEM,
    "mines.text.danger",
    "Danger-colored text."
);
token_pair!(
    TEXT_SUCCESS,
    TEXT_SUCCESS_SEM,
    "mines.text.success",
    "Success-colored text."
);
token_pair!(
    SCRIM_BG,
    SCRIM_BG_SEM,
    "mines.scrim.bg",
    "Dialog backdrop scrim."
);
token_pair!(
    CONTROL_BORDER,
    CONTROL_BORDER_SEM,
    "mines.control.border",
    "Border of input fields, selects and checkboxes."
);
token_pair!(
    TAB_BG_ACTIVE,
    TAB_BG_ACTIVE_SEM,
    "mines.tab.bg.active",
    "Background of the selected tab."
);
token_pair!(
    TAB_TEXT_ACTIVE,
    TAB_TEXT_ACTIVE_SEM,
    "mines.tab.text.active",
    "Label color of the selected tab."
);

// Feathers paints sliders as a full-row gradient: the value section uses
// `SLIDER_BAR`, the remaining track uses `SLIDER_BG`. The stock assignments map
// those to field/accent fills which are nearly invisible on our dialog
// surfaces, so they are remapped to the two tokens below.

/// Unfilled remainder of a slider track.
pub const SLIDER_TRACK_SEM: SemanticToken = SemanticToken::new_static("mines.slider.track");
/// Filled value portion of a slider.
pub const SLIDER_FILL_SEM: SemanticToken = SemanticToken::new_static("mines.slider.fill");
/// Filled value portion of a slider while hovered.
pub const SLIDER_FILL_HOVER_SEM: SemanticToken = SemanticToken::new_static("mines.slider.fill.hover");
