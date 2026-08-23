# Mines — Bevy recreation of GNOME Mines

This project recreates GNOME Mines 50.0 with Bevy 0.19 and Rust. It includes the
standard and custom board sizes, first-click safety, question flags, chording,
automatic clearing on flags, pausing, timing, persistent high scores, four
upstream themes, keyboard/touch/mouse controls, help, and the original game
credits. The UI is localized in Simplified Chinese and uses Bevy's system-font
fallback for Chinese glyph coverage.

The interface uses Bevy 0.19's native UI stack rather than an external immediate
mode GUI:

- `bsn!` and `bsn_list!` define the application shell and dynamic screens.
- A custom `SceneComponent` defines reusable difficulty cards.
- Feathers supplies buttons, checkboxes, and number inputs.
- `EditableText` powers the custom-board fields.
- `SettingsPlugin` and `SettingsGroup` persist reflected settings as TOML.
- Pointer observers handle left-click, right-click, and touch long-press.
- Delayed Commands synchronize newly spawned number fields.
- A high-contrast eight-color palette distinguishes adjacent-mine counts.
- `UiTransform` transitions animate screens, dialogs, pause state, and tile feedback.

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

Controls match GNOME Mines:

- Left click reveals a tile; click a revealed number to clear around it.
- Right click, touch-and-hold, or Ctrl+left click cycles a tile's mark.
- Arrow keys or `U J H K` move the keyboard cursor.
- Space reveals; Ctrl+Space marks.
- Ctrl+N changes difficulty, Ctrl+R restarts, and Pause pauses.

Bevy App Settings stores preferences and window state in the platform
preferences directory under `org.gnome.Mines/settings.toml`. Scores remain in
the compatible `bevy-gnome-mines/data.json` file.

## Licensing and artwork

The program is licensed under GPL-3.0-or-later; see [LICENSE](LICENSE).
Files under `assets/` are copied from
[GNOME Mines](https://github.com/GNOME/gnome-mines) and remain under their
upstream licenses and attribution. GNOME Mines credits Richard Hoelscher as the
artist. The GNOME name and application icon belong to their respective owners.
