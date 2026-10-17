# Mines — Bevy recreation of GNOME Mines

This project recreates GNOME Mines 50.0 with Bevy 0.20 and Rust. It includes the
standard and custom board sizes, first-click safety, question flags, chording,
automatic clearing on flags, pausing, timing, persistent high scores, five board
styles, four UI themes, keyboard/touch/mouse controls, help, and the original
game credits. The UI is localized in Simplified Chinese and uses Bevy's
system-font fallback for Chinese glyph coverage.

## Bevy 0.20 features in use

The interface is built entirely with Bevy's native UI stack rather than an
external immediate-mode GUI:

- `bsn!` and `bsn_list!` scenes define the application shell and the dynamic
  screens; `Commands::spawn_scene` and `despawn_children` swap them at runtime.
- Custom `SceneComponent`s define reusable difficulty cards and dialog layouts.
- Feathers (now shipped with Bevy as `bevy_feathers`) supplies buttons,
  checkboxes, selects, tab lists, sliders, number inputs, and modal dialogs.
- Feathers design tokens (`ThemeProps` → semantic token) drive every color, so
  the four UI themes only remap tokens instead of hard-coding palette values.
- `bevy_settings` persists reflected preferences as TOML in the platform
  preferences directory.
- Pointer observers (`On<Pointer<Press>>`, drag observers) handle left click,
  right click, drag, and touch long-press; 0.20 `EditableText` powers the
  custom-board number fields.
- `UiTransform` transitions animate screens, dialogs, the pause state, and tile
  feedback.
- The window icon is rasterized from the upstream SVG with `resvg`;
  `WindowResizeConstraints` plus a runtime clamp keep the layout usable, and
  window geometry/maximized state are restored on launch.

## UI overview

- Default "modern" board style: flat, theme-tinted tiles with no bitmap
  artwork, drawn entirely by UI nodes.
- Five board styles: `现代` (the default flat UI tile set) plus the four classic
  GNOME glyph tile sets `背景颜色`, `经典`, `默认`, and `GNOME 现代`.
- Four UI themes: `跟随系统`, `浅色`, `深色`, and `高对比度`; adjustable UI scale
  (85%–150%); a reduced-motion toggle.
- A modern shell: header bar with menu, restart, and pause buttons, a summary
  subtitle for the current board, a floating flag/time status pill, and modal
  dialogs for settings, scores, help (with page pager), keyboard shortcuts,
  about, new game, and the custom board.
- Minimum window size is 700×600; larger windows simply center the board and
  scale the tile grid up to fit.

## Run

```sh
cargo run --release
```

Optional command-line modes:

```text
--small    8 × 8, 10 mines
--medium   16 × 16, 40 mines
--big      30 × 16, 99 mines
--version  print version
```

## Controls

- Left click reveals a tile; click a revealed number to clear around it
  (chording).
- Right click, touch-and-hold, or Ctrl+left click cycles a tile's mark; the
  setting panel can enable question marks and auto-flagging.
- Arrow keys or `H J K L` move the keyboard cursor; Space reveals;
  Ctrl+Space marks.
- `P` (or the Pause key) toggles pause; the clock auto-pauses when the window
  loses focus.
- `Ctrl+N` opens the new-game screen, `Ctrl+R` restarts, `Ctrl+Q`/`Ctrl+W`
  quits, and digits `1`–`3` start a preset difficulty while `4` opens the
  custom-board dialog.
- `Esc` closes the current dialog or toggles pause on the board.
- `F1` help, `F2` settings, `F3` high scores, `F4` about, `F5` keyboard
  shortcuts, `F6` custom board. In debug builds `F9` reveals the safe cells so
  the victory flow can be tested.

Bevy App Settings stores preferences and window state in the platform
preferences directory under `org.gnome.Mines/settings.toml`. Scores remain in
the compatible `bevy-gnome-mines/data.json` file.

## Licensing and artwork

The program is licensed under GPL-3.0-or-later; see [LICENSE](LICENSE).
Files under `assets/` are copied from
[GNOME Mines](https://github.com/GNOME/gnome-mines) and remain under their
upstream licenses and attribution. GNOME Mines credits Richard Hoelscher as the
artist. The GNOME name and application icon belong to their respective owners.
