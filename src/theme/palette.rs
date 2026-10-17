// SPDX-License-Identifier: GPL-3.0-or-later
//
// Color palettes for the three UI themes. Every surface and state color used by
// the application is defined here so the UI code never hard-codes a color.

use bevy::prelude::Color;

/// Fully resolved set of colors for one UI theme.
#[derive(Debug, Clone)]
pub struct Palette {
    /// High-contrast palettes thicken borders and drop shadows.
    pub high_contrast: bool,

    pub app_bg: Color,
    pub header_bg: Color,
    pub header_border: Color,
    pub surface_bg: Color,
    pub surface_border: Color,
    pub surface_alt: Color,
    pub dialog_bg: Color,
    pub scrim: Color,
    pub shadow: Color,

    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,

    pub accent: Color,
    pub accent_hover: Color,
    pub accent_pressed: Color,
    pub on_accent: Color,

    pub danger: Color,
    pub danger_bg: Color,
    pub success: Color,
    pub success_bg: Color,
    pub warning: Color,

    pub control_bg: Color,
    pub control_bg_hover: Color,
    pub control_bg_pressed: Color,
    pub control_bg_disabled: Color,
    pub field_bg: Color,
    pub field_border: Color,
    pub focus_ring: Color,

    pub cell_hidden: Color,
    pub cell_hidden_hover: Color,
    pub cell_hidden_pressed: Color,
    pub cell_revealed: Color,
    pub cell_border: Color,
    pub cell_cursor: Color,
    pub cell_exploded_bg: Color,
    pub cell_wrong_bg: Color,
    pub number_colors: [Color; 8],

    pub pill_bg: Color,
    pub pill_border: Color,
}

impl Palette {
    pub fn light() -> Self {
        Self {
            high_contrast: false,
            app_bg: Color::srgb(0.961, 0.965, 0.973),
            header_bg: Color::WHITE,
            header_border: Color::srgb(0.886, 0.902, 0.925),
            surface_bg: Color::WHITE,
            surface_border: Color::srgb(0.886, 0.902, 0.925),
            surface_alt: Color::srgb(0.945, 0.953, 0.965),
            dialog_bg: Color::WHITE,
            scrim: Color::srgba(0.059, 0.067, 0.086, 0.45),
            shadow: Color::srgba(0.059, 0.090, 0.165, 0.10),

            text_primary: Color::srgb(0.078, 0.094, 0.129),
            text_secondary: Color::srgb(0.357, 0.392, 0.447),
            text_disabled: Color::srgba(0.357, 0.392, 0.447, 0.55),

            accent: Color::srgb(0.231, 0.510, 0.965),
            accent_hover: Color::srgb(0.145, 0.388, 0.922),
            accent_pressed: Color::srgb(0.114, 0.306, 0.847),
            on_accent: Color::WHITE,

            danger: Color::srgb(0.937, 0.267, 0.267),
            danger_bg: Color::srgb(0.992, 0.886, 0.882),
            success: Color::srgb(0.063, 0.725, 0.506),
            success_bg: Color::srgb(0.878, 0.973, 0.941),
            warning: Color::srgb(0.961, 0.620, 0.043),

            control_bg: Color::srgb(0.925, 0.937, 0.953),
            control_bg_hover: Color::srgb(0.878, 0.898, 0.925),
            control_bg_pressed: Color::srgb(0.824, 0.855, 0.894),
            control_bg_disabled: Color::srgba(0.925, 0.937, 0.953, 0.55),
            field_bg: Color::WHITE,
            field_border: Color::srgb(0.796, 0.824, 0.863),
            focus_ring: Color::srgba(0.231, 0.510, 0.965, 0.55),

            cell_hidden: Color::srgb(0.894, 0.918, 0.949),
            cell_hidden_hover: Color::srgb(0.839, 0.871, 0.914),
            cell_hidden_pressed: Color::srgb(0.776, 0.820, 0.878),
            cell_revealed: Color::srgb(0.984, 0.988, 0.996),
            cell_border: Color::srgb(0.843, 0.867, 0.902),
            cell_cursor: Color::srgb(0.231, 0.510, 0.965),
            cell_exploded_bg: Color::srgb(0.992, 0.886, 0.882),
            cell_wrong_bg: Color::srgb(0.992, 0.886, 0.882),
            number_colors: [
                Color::srgb(0.145, 0.388, 0.922),
                Color::srgb(0.020, 0.588, 0.412),
                Color::srgb(0.863, 0.149, 0.149),
                Color::srgb(0.486, 0.227, 0.929),
                Color::srgb(0.706, 0.325, 0.035),
                Color::srgb(0.031, 0.569, 0.698),
                Color::srgb(0.745, 0.094, 0.365),
                Color::srgb(0.216, 0.255, 0.318),
            ],
            pill_bg: Color::WHITE,
            pill_border: Color::srgb(0.886, 0.902, 0.925),
        }
    }

    pub fn dark() -> Self {
        Self {
            high_contrast: false,
            app_bg: Color::srgb(0.059, 0.067, 0.086),
            header_bg: Color::srgb(0.086, 0.102, 0.129),
            header_border: Color::srgb(0.165, 0.188, 0.251),
            surface_bg: Color::srgb(0.086, 0.102, 0.129),
            surface_border: Color::srgb(0.165, 0.188, 0.251),
            surface_alt: Color::srgb(0.106, 0.125, 0.165),
            dialog_bg: Color::srgb(0.106, 0.125, 0.165),
            scrim: Color::srgba(0.0, 0.0, 0.0, 0.60),
            shadow: Color::srgba(0.0, 0.0, 0.0, 0.50),

            text_primary: Color::srgb(0.902, 0.914, 0.937),
            text_secondary: Color::srgb(0.604, 0.639, 0.698),
            text_disabled: Color::srgba(0.604, 0.639, 0.698, 0.50),

            accent: Color::srgb(0.376, 0.647, 0.980),
            accent_hover: Color::srgb(0.231, 0.510, 0.965),
            accent_pressed: Color::srgb(0.145, 0.388, 0.922),
            on_accent: Color::srgb(0.059, 0.067, 0.086),

            danger: Color::srgb(0.973, 0.443, 0.443),
            danger_bg: Color::srgb(0.290, 0.114, 0.122),
            success: Color::srgb(0.204, 0.827, 0.600),
            success_bg: Color::srgb(0.071, 0.235, 0.188),
            warning: Color::srgb(0.984, 0.749, 0.141),

            control_bg: Color::srgb(0.149, 0.176, 0.220),
            control_bg_hover: Color::srgb(0.184, 0.216, 0.275),
            control_bg_pressed: Color::srgb(0.220, 0.255, 0.325),
            control_bg_disabled: Color::srgba(0.149, 0.176, 0.220, 0.55),
            field_bg: Color::srgb(0.118, 0.141, 0.180),
            field_border: Color::srgb(0.220, 0.251, 0.318),
            focus_ring: Color::srgba(0.376, 0.647, 0.980, 0.65),

            cell_hidden: Color::srgb(0.141, 0.165, 0.212),
            cell_hidden_hover: Color::srgb(0.180, 0.208, 0.275),
            cell_hidden_pressed: Color::srgb(0.220, 0.251, 0.333),
            cell_revealed: Color::srgb(0.106, 0.125, 0.165),
            cell_border: Color::srgb(0.200, 0.227, 0.294),
            cell_cursor: Color::srgb(0.376, 0.647, 0.980),
            cell_exploded_bg: Color::srgb(0.290, 0.114, 0.122),
            cell_wrong_bg: Color::srgb(0.290, 0.114, 0.122),
            number_colors: [
                Color::srgb(0.376, 0.647, 0.980),
                Color::srgb(0.204, 0.827, 0.600),
                Color::srgb(0.973, 0.443, 0.443),
                Color::srgb(0.655, 0.545, 0.980),
                Color::srgb(0.984, 0.749, 0.141),
                Color::srgb(0.133, 0.827, 0.933),
                Color::srgb(0.957, 0.447, 0.714),
                Color::srgb(0.820, 0.835, 0.859),
            ],
            pill_bg: Color::srgb(0.118, 0.141, 0.180),
            pill_border: Color::srgb(0.200, 0.227, 0.294),
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            high_contrast: true,
            app_bg: Color::WHITE,
            header_bg: Color::WHITE,
            header_border: Color::BLACK,
            surface_bg: Color::WHITE,
            surface_border: Color::BLACK,
            surface_alt: Color::WHITE,
            dialog_bg: Color::WHITE,
            scrim: Color::srgba(0.0, 0.0, 0.0, 0.75),
            shadow: Color::srgba(0.0, 0.0, 0.0, 0.0),

            text_primary: Color::BLACK,
            text_secondary: Color::BLACK,
            text_disabled: Color::srgb(0.35, 0.35, 0.35),

            accent: Color::srgb(0.0, 0.0, 0.933),
            accent_hover: Color::srgb(0.0, 0.0, 0.8),
            accent_pressed: Color::srgb(0.0, 0.0, 0.667),
            on_accent: Color::WHITE,

            danger: Color::srgb(0.690, 0.0, 0.125),
            danger_bg: Color::srgb(1.0, 0.898, 0.898),
            success: Color::srgb(0.0, 0.392, 0.0),
            success_bg: Color::srgb(0.898, 1.0, 0.898),
            warning: Color::srgb(0.6, 0.35, 0.0),

            control_bg: Color::WHITE,
            control_bg_hover: Color::srgb(0.9, 0.9, 0.9),
            control_bg_pressed: Color::srgb(0.8, 0.8, 0.8),
            control_bg_disabled: Color::WHITE,
            field_bg: Color::WHITE,
            field_border: Color::BLACK,
            focus_ring: Color::srgb(0.0, 0.0, 0.933),

            cell_hidden: Color::WHITE,
            cell_hidden_hover: Color::srgb(0.9, 0.9, 0.9),
            cell_hidden_pressed: Color::srgb(0.8, 0.8, 0.8),
            cell_revealed: Color::WHITE,
            cell_border: Color::BLACK,
            cell_cursor: Color::srgb(0.0, 0.0, 0.933),
            cell_exploded_bg: Color::srgb(1.0, 0.8, 0.8),
            cell_wrong_bg: Color::srgb(1.0, 0.8, 0.8),
            number_colors: [
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
                Color::BLACK,
            ],
            pill_bg: Color::WHITE,
            pill_border: Color::BLACK,
        }
    }
}
