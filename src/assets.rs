// SPDX-License-Identifier: GPL-3.0-or-later
//
// Runtime asset preparation: the GNOME Mines board glyphs are vector art that is
// rasterized at startup for the classic board styles, while the modern UI and
// board icons are defined inline as white SVGs that get tinted per theme.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Glyph slots inside each classic theme.
pub const GLYPH_MINE: usize = 0;
pub const GLYPH_EXPLODED: usize = 1;
pub const GLYPH_FLAG: usize = 2;
pub const GLYPH_INCORRECT: usize = 3;
pub const GLYPH_MAYBE: usize = 4;

/// Board art style selected by [`crate::storage::Settings::theme`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoardStyle {
    /// Modern flat UI cells with themed colors (default).
    #[default]
    ModernUi,
    /// GNOME Mines "background colors" art.
    BgColors,
    /// GNOME Mines "classic" art.
    Classic,
    /// GNOME Mines "default" art.
    Default,
    /// GNOME Mines "modern" art.
    GnomeModern,
}

impl BoardStyle {
    pub fn parse(value: &str) -> Self {
        match value {
            "bgcolors" => Self::BgColors,
            "classic" => Self::Classic,
            "default" => Self::Default,
            "modern" => Self::GnomeModern,
            _ => Self::ModernUi,
        }
    }

    pub fn is_modern(self) -> bool {
        self == Self::ModernUi
    }

    /// Index into the loaded classic glyph sets.
    pub fn glyph_set(self) -> usize {
        match self {
            Self::ModernUi | Self::Default => 2,
            Self::BgColors => 0,
            Self::Classic => 1,
            Self::GnomeModern => 3,
        }
    }
}

/// Legacy flat palette of the GNOME Mines look, used by the classic styles.
pub mod legacy {
    use super::BoardStyle;
    use crate::game::{Board, Cell, Mark, Outcome};
    use bevy::prelude::Color;

    pub const HIDDEN: Color = Color::srgb_u8(0xba, 0xbd, 0xb6);
    pub const HIDDEN_HOVER: Color = Color::srgb_u8(0xd3, 0xd7, 0xcf);
    pub const HIDDEN_PRESSED: Color = Color::srgb_u8(0x88, 0x8a, 0x85);
    pub const REVEALED: Color = Color::srgb_u8(0xde, 0xde, 0xdc);
    pub const BORDER_HIDDEN: Color = Color::srgb_u8(0x88, 0x8a, 0x85);
    pub const BORDER_REVEALED: Color = Color::srgb_u8(0xba, 0xbd, 0xb6);
    pub const CURSOR: Color = Color::srgb(0.13, 0.45, 0.86);
    pub const MONOCHROME_NUMBER: Color = Color::srgb_u8(0x2e, 0x34, 0x36);
    pub const WARNING_BG: Color = Color::srgb_u8(0xf8, 0xd4, 0xd4);
    pub const WARNING_BORDER: Color = Color::srgb_u8(0xcc, 0x00, 0x00);

    const EXPLODED_HIDDEN: Color = Color::srgb_u8(0x88, 0x8a, 0x85);
    const EXPLODED_HIDDEN_MODERN: Color = Color::srgb_u8(0x9a, 0x99, 0x96);
    const EXPLODED_REVEALED: Color = Color::srgb_u8(0xd3, 0xd7, 0xcf);
    const EXPLODED_REVEALED_MODERN: Color = Color::srgb_u8(0xc0, 0xbf, 0xbc);
    const EXPLODED_CELL_MODERN: Color = Color::srgb_u8(0xf5, 0xc2, 0x11);
    const INCORRECT_BG: Color = Color::srgb_u8(0xcc, 0x00, 0x00);

    /// Heatmap backgrounds from GNOME Mines `bgcolors/theme.css`.
    pub const BGCOLORS_TILES: [Color; 8] = [
        Color::srgb_u8(0xdd, 0xfa, 0xc3),
        Color::srgb_u8(0xec, 0xed, 0xbf),
        Color::srgb_u8(0xed, 0xda, 0xb4),
        Color::srgb_u8(0xed, 0xc3, 0x8a),
        Color::srgb_u8(0xf7, 0xa1, 0xa2),
        Color::srgb_u8(0xfe, 0xa7, 0x85),
        Color::srgb_u8(0xff, 0x7d, 0x60),
        Color::srgb_u8(0xff, 0x32, 0x3c),
    ];

    pub const NUMBER_COLORS: [(u8, u8, u8); 8] = [
        (0x1a, 0x5f, 0xb4),
        (0x21, 0x8b, 0x55),
        (0xc0, 0x1c, 0x28),
        (0x61, 0x35, 0x83),
        (0xa3, 0x47, 0x00),
        (0x08, 0x79, 0x90),
        (0x9c, 0x2b, 0x75),
        (0x3d, 0x38, 0x46),
    ];

    pub fn number_color(style: BoardStyle, number: u8) -> Color {
        match style {
            BoardStyle::BgColors | BoardStyle::Default => MONOCHROME_NUMBER,
            _ => {
                let (red, green, blue) = NUMBER_COLORS[(number.clamp(1, 8) - 1) as usize];
                Color::srgb_u8(red, green, blue)
            }
        }
    }

    pub fn bgcolor_tile(number: u8) -> Color {
        BGCOLORS_TILES[(number.clamp(1, 8) - 1) as usize]
    }

    pub fn cell_background(style: BoardStyle, cell: Cell, board: &Board, index: usize) -> Color {
        if board.outcome == Outcome::Lost {
            if board.exploded_cell == Some(index) {
                return if style == BoardStyle::GnomeModern {
                    EXPLODED_CELL_MODERN
                } else {
                    EXPLODED_REVEALED
                };
            }
            if !cell.revealed && cell.mark == Mark::Flag && !cell.mine {
                return INCORRECT_BG;
            }
            if !cell.revealed {
                return if style == BoardStyle::GnomeModern {
                    EXPLODED_HIDDEN_MODERN
                } else {
                    EXPLODED_HIDDEN
                };
            }
            if style == BoardStyle::BgColors && !cell.mine && cell.adjacent > 0 {
                return bgcolor_tile(cell.adjacent);
            }
            return if style == BoardStyle::GnomeModern {
                EXPLODED_REVEALED_MODERN
            } else {
                EXPLODED_REVEALED
            };
        }

        if cell.revealed {
            if style == BoardStyle::BgColors && !cell.mine && cell.adjacent > 0 {
                bgcolor_tile(cell.adjacent)
            } else {
                REVEALED
            }
        } else {
            HIDDEN
        }
    }
}

const TILE_SVGS: [[&[u8]; 5]; 4] = [
    [
        include_bytes!("../assets/themes/bgcolors/mine.svg"),
        include_bytes!("../assets/themes/bgcolors/exploded.svg"),
        include_bytes!("../assets/themes/bgcolors/flag.svg"),
        include_bytes!("../assets/themes/bgcolors/incorrect.svg"),
        include_bytes!("../assets/themes/bgcolors/maybe.svg"),
    ],
    [
        include_bytes!("../assets/themes/classic/mine.svg"),
        include_bytes!("../assets/themes/classic/exploded.svg"),
        include_bytes!("../assets/themes/classic/flag.svg"),
        include_bytes!("../assets/themes/classic/incorrect.svg"),
        include_bytes!("../assets/themes/classic/maybe.svg"),
    ],
    [
        include_bytes!("../assets/themes/default/mine.svg"),
        include_bytes!("../assets/themes/default/exploded.svg"),
        include_bytes!("../assets/themes/default/flag.svg"),
        include_bytes!("../assets/themes/default/incorrect.svg"),
        include_bytes!("../assets/themes/default/maybe.svg"),
    ],
    [
        include_bytes!("../assets/themes/modern/mine.svg"),
        include_bytes!("../assets/themes/modern/exploded.svg"),
        include_bytes!("../assets/themes/modern/flag.svg"),
        include_bytes!("../assets/themes/modern/incorrect.svg"),
        include_bytes!("../assets/themes/modern/maybe.svg"),
    ],
];

const GNOME_MODERN_NUMBER_SVGS: [&[u8]; 8] = [
    include_bytes!("../assets/themes/modern/1mines.svg"),
    include_bytes!("../assets/themes/modern/2mines.svg"),
    include_bytes!("../assets/themes/modern/3mines.svg"),
    include_bytes!("../assets/themes/modern/4mines.svg"),
    include_bytes!("../assets/themes/modern/5mines.svg"),
    include_bytes!("../assets/themes/modern/6mines.svg"),
    include_bytes!("../assets/themes/modern/7mines.svg"),
    include_bytes!("../assets/themes/modern/8mines.svg"),
];

/// Chrome and board icons, drawn as white shapes so they can be tinted per theme.
mod raw_icons {
    pub const MENU: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round"><path d="M4 7h16"/><path d="M4 12h16"/><path d="M4 17h16"/></g></svg>"##;
    pub const BACK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="#ffffff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 5l-7 7 7 7"/></g></svg>"##;
    pub const RESTART: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="#ffffff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 12a8 8 0 1 1-2.4-5.7"/><path d="M20 4v4.5h-4.5"/></g></svg>"##;
    pub const PAUSE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="#ffffff" stroke-width="3" stroke-linecap="round"><path d="M9 5v14"/><path d="M15 5v14"/></g></svg>"##;
    pub const PLAY: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="#ffffff" d="M8 4.8 20 12 8 19.2Z"/></svg>"##;

    // Modern board glyphs, drawn as white shapes for per-theme tinting.
    pub const MINE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g stroke="#ffffff" stroke-width="1.9" stroke-linecap="round"><path d="M12 3v18"/><path d="M3 12h18"/><path d="M5.6 5.6l12.8 12.8"/><path d="M18.4 5.6L5.6 18.4"/></g><circle cx="12" cy="12" r="4.6" fill="#ffffff"/></svg>"##;
    pub const FLAG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M7.5 21V3.5" fill="none" stroke="#ffffff" stroke-width="2.4" stroke-linecap="round"/><path d="M8.6 4h10l-2.7 4.3L18.6 12h-10z" fill="#ffffff"/></svg>"##;
    pub const EXPLODED: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g stroke="#ffffff" stroke-width="1.9" stroke-linecap="round"><path d="M12 2.5v19"/><path d="M2.5 12h19"/><path d="M5.2 5.2l13.6 13.6"/><path d="M18.8 5.2L5.2 18.8"/></g><circle cx="12" cy="12" r="4.2" fill="#ffffff"/></svg>"##;
    pub const INCORRECT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M6.5 21V3.5" fill="none" stroke="#ffffff" stroke-width="2.2" stroke-linecap="round"/><path d="M7.5 4h9l-2.4 3.9L16.5 12h-9z" fill="#ffffff" opacity="0.55"/><g stroke="#ffffff" stroke-width="2.4" stroke-linecap="round"><path d="M12.5 12.5l7 7"/><path d="M19.5 12.5l-7 7"/></g></svg>"##;
    pub const MAYBE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="#ffffff" stroke-width="2.4" stroke-linecap="round"><path d="M8.4 9.1a3.7 3.7 0 1 1 5.2 3.4c-1 .45-1.4 1.3-1.4 2.4"/></g><circle cx="12.2" cy="19" r="1.5" fill="#ffffff"/></svg>"##;
    pub const STOPWATCH: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="#ffffff" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="13.5" r="7.2"/><path d="M12 13.5v-4"/><path d="M9.2 2.6h5.6"/><path d="M12 2.6v2.2"/></g></svg>"##;
}

#[derive(Clone)]
pub struct UiIcons {
    pub menu: Handle<Image>,
    pub back: Handle<Image>,
    pub restart: Handle<Image>,
    pub pause: Handle<Image>,
    pub play: Handle<Image>,
    pub stopwatch: Handle<Image>,
}

/// Themed modern board glyphs.
#[derive(Clone)]
pub struct ModernGlyphs {
    pub mine: Handle<Image>,
    pub exploded: Handle<Image>,
    pub flag: Handle<Image>,
    pub incorrect: Handle<Image>,
    pub maybe: Handle<Image>,
}

/// All textures prepared at startup.
#[derive(Resource, Clone)]
pub struct GameAssets {
    /// Classic board glyphs per theme, indexed by [`BoardStyle::glyph_set`].
    pub classic: Vec<Vec<Handle<Image>>>,
    /// Vector gradient number glyphs (`1..=8`) for the GNOME Modern board style.
    pub gnome_modern_numbers: [Handle<Image>; 8],
    pub modern: ModernGlyphs,
    pub icons: UiIcons,
}

impl GameAssets {
    pub fn classic_glyph(&self, style: BoardStyle, glyph: usize) -> Handle<Image> {
        self.classic[style.glyph_set()][glyph].clone()
    }

    pub fn gnome_modern_number(&self, number: u8) -> Handle<Image> {
        self.gnome_modern_numbers[(number.clamp(1, 8) - 1) as usize].clone()
    }

    pub fn modern_glyph(&self, glyph: usize) -> Handle<Image> {
        match glyph {
            GLYPH_MINE => self.modern.mine.clone(),
            GLYPH_EXPLODED => self.modern.exploded.clone(),
            GLYPH_FLAG => self.modern.flag.clone(),
            GLYPH_INCORRECT => self.modern.incorrect.clone(),
            _ => self.modern.maybe.clone(),
        }
    }
}

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        // The initial `OnEnter` state transition runs before `PreStartup`, so the
        // textures have to exist by the time the first screen is spawned.
        let assets = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            prepare_assets(&mut images)
        };
        app.insert_resource(assets);
    }
}

fn prepare_assets(images: &mut Assets<Image>) -> GameAssets {
    // Every embedded board glyph and chrome icon is pure path/shape geometry,
    // so SVG parsing needs no system font scan on the startup thread.
    let options = resvg::usvg::Options::default();

    let classic: Vec<Vec<Handle<Image>>> = TILE_SVGS
        .iter()
        .map(|theme| {
            theme
                .iter()
                .map(|bytes| images.add(svg_image(bytes, 128, &options)))
                .collect()
        })
        .collect();

    let gnome_modern_numbers =
        GNOME_MODERN_NUMBER_SVGS.map(|bytes| images.add(svg_image(bytes, 128, &options)));

    let modern = ModernGlyphs {
        mine: images.add(icon_image(raw_icons::MINE, &options)),
        exploded: images.add(icon_image(raw_icons::EXPLODED, &options)),
        flag: images.add(icon_image(raw_icons::FLAG, &options)),
        incorrect: images.add(icon_image(raw_icons::INCORRECT, &options)),
        maybe: images.add(icon_image(raw_icons::MAYBE, &options)),
    };

    let icons = UiIcons {
        menu: images.add(icon_image(raw_icons::MENU, &options)),
        back: images.add(icon_image(raw_icons::BACK, &options)),
        restart: images.add(icon_image(raw_icons::RESTART, &options)),
        pause: images.add(icon_image(raw_icons::PAUSE, &options)),
        play: images.add(icon_image(raw_icons::PLAY, &options)),
        stopwatch: images.add(icon_image(raw_icons::STOPWATCH, &options)),
    };

    GameAssets {
        classic,
        gnome_modern_numbers,
        modern,
        icons,
    }
}

fn icon_image(source: &str, options: &resvg::usvg::Options<'_>) -> Image {
    let tree = resvg::usvg::Tree::from_str(source, options).expect("inline icon SVG must be valid");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(64, 64).expect("icon allocation failed");
    let tree_size = tree.size();
    let transform = resvg::tiny_skia::Transform::from_scale(
        64.0 / tree_size.width(),
        64.0 / tree_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Image::new(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixmap.take(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_rendered(images: &Assets<Image>, handle: &Handle<Image>, expected_size: u32) {
        let image = images.get(handle).expect("asset handle must resolve");
        assert_eq!(image.width(), expected_size);
        assert_eq!(image.height(), expected_size);
        let bytes = image.data.as_ref().expect("image pixel buffer must exist");
        assert!(
            bytes.as_chunks::<4>().0.iter().any(|rgba| rgba[3] > 0),
            "rasterized SVG should contain visible non-transparent pixels"
        );
    }

    #[test]
    fn all_svg_assets_rasterize_without_system_fonts() {
        let mut images = Assets::<Image>::default();
        let assets = prepare_assets(&mut images);

        for theme in &assets.classic {
            assert_eq!(theme.len(), 5);
            for handle in theme {
                assert_rendered(&images, handle, 128);
            }
        }
        for handle in &assets.gnome_modern_numbers {
            assert_rendered(&images, handle, 128);
        }
        for handle in [
            &assets.modern.mine,
            &assets.modern.exploded,
            &assets.modern.flag,
            &assets.modern.incorrect,
            &assets.modern.maybe,
        ] {
            assert_rendered(&images, handle, 64);
        }
        for handle in [
            &assets.icons.menu,
            &assets.icons.back,
            &assets.icons.restart,
            &assets.icons.pause,
            &assets.icons.play,
            &assets.icons.stopwatch,
        ] {
            assert_rendered(&images, handle, 64);
        }
    }
}
