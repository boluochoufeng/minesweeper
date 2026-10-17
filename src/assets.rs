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
    use bevy::prelude::Color;

    pub const HIDDEN: Color = Color::srgb(0.82, 0.82, 0.82);
    pub const REVEALED: Color = Color::srgb(0.96, 0.96, 0.95);
    pub const BORDER_HIDDEN: Color = Color::srgb(0.66, 0.66, 0.66);
    pub const BORDER_REVEALED: Color = Color::srgb(0.78, 0.78, 0.78);
    pub const CURSOR: Color = Color::srgb(0.13, 0.45, 0.86);
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

    pub fn number_color(number: u8) -> Color {
        let (red, green, blue) = NUMBER_COLORS[(number.clamp(1, 8) - 1) as usize];
        Color::srgb(
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
        )
    }
}

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
    pub modern: ModernGlyphs,
    pub icons: UiIcons,
}

impl GameAssets {
    pub fn classic_glyph(&self, style: BoardStyle, glyph: usize) -> Handle<Image> {
        self.classic[style.glyph_set()][glyph].clone()
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
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();

    let classic: Vec<Vec<Handle<Image>>> = TILE_SVGS
        .iter()
        .map(|theme| {
            theme[8..13]
                .iter()
                .map(|bytes| images.add(svg_image(bytes, 128, &options)))
                .collect()
        })
        .collect();

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
