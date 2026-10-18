// SPDX-License-Identifier: GPL-3.0-or-later
//
// Shared visual language: typographic scale, spacing and reusable scene
// fragments. Sizes are expressed in rem so the whole UI follows the user's UI
// scale setting.

use crate::ui::actions::{ActionTarget, UiAction};
use bevy::feathers::controls::{ButtonVariant, FeathersButton};
use bevy::feathers::display::ThemedIcon;
use bevy::feathers::theme::ThemedText;
use bevy::prelude::*;
use bevy::ui::rem;

// Typographic scale (rem).
pub const FONT_DISPLAY: f32 = 2.1;
pub const FONT_TITLE: f32 = 1.5;
pub const FONT_SUBTITLE: f32 = 1.2;
pub const FONT_BODY: f32 = 1.0;
pub const FONT_CAPTION: f32 = 0.85;
pub const FONT_SMALL: f32 = 0.75;

// Layout metrics (rem).
pub const HEADER_HEIGHT: f32 = 3.6;
pub const STATUS_HEIGHT: f32 = 2.6;
pub const CONTENT_PAD: f32 = 1.4;
pub const RADIUS_CARD: f32 = 0.9;
pub const RADIUS_CELL: f32 = 0.3;
pub const RADIUS_PILL: f32 = 99.0;

/// Box a freshly built scene list for storage in `Vec<Box<dyn SceneList>>`.
pub fn boxed_list(list: impl SceneList + 'static) -> Box<dyn SceneList> {
    Box::new(list)
}

/// An icon-only flat button that emits an [`UiAction`]. The glyph is tinted by
/// the button's inherited text color, so it follows the active theme. `label`
/// is exposed as the accessible name, which icon-only buttons otherwise lack.
pub fn icon_button(handle: Handle<Image>, action: UiAction, label: &str) -> Box<dyn SceneList> {
    let label = label.to_owned();
    boxed_list(bsn_list! { @icon_button_scene(handle, action, label) })
}

fn icon_button_scene(handle: Handle<Image>, action: UiAction, label: String) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! {
                ImageNode { image: {handle} }
                Node { width: rem(1.15), height: rem(1.15) }
                ThemedIcon
            },
            @variant: ButtonVariant::Plain,
        }
        ActionTarget { action: {action} }
        AccessibleLabel({label})
    }
}

/// A labelled button that emits an [`UiAction`].
pub fn text_button(label: &str, action: UiAction, variant: ButtonVariant) -> Box<dyn SceneList> {
    let caption = label.to_owned();
    let accessible = label.to_owned();
    boxed_list(bsn_list! { @text_button_scene(caption, accessible, action, variant) })
}

fn text_button_scene(
    label: String,
    accessible: String,
    action: UiAction,
    variant: ButtonVariant,
) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn! {
                Text({label})
                TextFont { font_size: FontSize::Rem(FONT_BODY) }
                ThemedText
            },
            @variant: {variant},
        }
        ActionTarget { action: {action} }
        AccessibleLabel({accessible})
    }
}
