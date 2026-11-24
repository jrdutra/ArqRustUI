# ArqRustUI

*Extended English guide. For the repository overview, screenshots, current status and getting started, see [README.md](README.md). The API reference (rustdoc) and paired guides are available in English. The Portuguese library guide is [BIBLIOTECA.md](BIBLIOTECA.md).*

ArqRustUI is a **native GUI library for Microsoft Windows written entirely in Rust**.
It drives the real Windows controls (Win32, Common Controls v6) and adds a set of
modern controls drawn by the library itself, behind a high-level API of builders,
closures, `Result` and RAII — you never have to touch an `HWND`, a `WPARAM` or a
window procedure.

- **Zero third-party dependencies.** Only the standard library and hand-written FFI
  for the DLLs that ship with Windows (Win32, Common Controls, GDI/GDI+,
  Direct2D/DirectWrite, COM, DWM, UI Automation). The only crates in `Cargo.lock`
  are this library and its own icon-pack crates.
- **Real native controls**: no HTML, no WebView, no custom rendering engine.
- **Modern controls drawn by the library**, in the window's theme, with keyboard
  support and UI Automation: toggle switch, number box with expressions, rating,
  cards, expander, breadcrumb bar, charts, print preview, dockable panes, detachable
  tabs, a frameless window with its own title bar, a command bar, an owner-drawn
  menu strip, info bars, toasts and web-style modal dialogs.
- **A 392-icon pack** (`.ico` 16–256 px, `.png`, some `.svg`), including 66 file-type
  icons resolved by extension.
- **Per-monitor DPI, light/dark/high-contrast themes, accessibility, IME, touch
  gestures, drag and drop, clipboard, printing, system notifications and tray
  icons.**
- **General purpose**: the library knows nothing about the applications that use it.

```rust
#![windows_subsystem = "windows"] // no console window in GUI builds
use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("My application").size(800, 600).centered().build()?;

    let button = Button::builder()
        .text("Click me")
        .icon(&icons::CHECK)
        .position(20, 20)
        .size(150, 40)
        .parent(&window)
        .build()?;
    button.on_click(|| MessageBox::info("Message", "Hello, Windows!"));

    window.show();
    app.run()
}
```

## Status

Development version **0.1.0**. See [Project status](README.md#project-status) and the [feature matrix](docs/FEATURE_MATRIX.en.md) for dated results and limitations. The recorded full Windows run on 2026-11-16 had 356 passes, 32 failures and one excluded printing-dialog test; do not interpret historical Wine results as a fully passing Windows suite. This guide describes APIs, not a completed release certification.

## Requirements

| Item | Requirement |
|---|---|
| Operating system | Windows 10 or 11 |
| Architectures | x64 (`x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`); x86 (`i686`) builds and passes the suite under Wine; ARM64 compiles but has never run |
| Rust | 1.80 or newer (MSRV) |
| Application manifest | not needed: visual styles and per-monitor DPI are enabled at run time |

## Installation

```toml
[dependencies]
arqrustui = { path = "../ArqRustUI" }   # local copy
# arqrustui = "0.1"                     # from crates.io, once published
```

The icon pack is the `icons` feature, on by default. Turn it off with
`default-features = false` if you do not use it; with LTO (the release profile of
this repository) only the icons you actually use end up in the executable.

## Core concepts

**One UI thread.** All widgets belong to the thread that created the `Application`;
visual types are neither `Send` nor `Sync`, so using them from another thread does
not compile. Other threads talk to the UI through a dispatcher or background tasks:

```rust
let ui = app.dispatcher();                    // Send + Sync + Clone
std::thread::spawn(move || {
    let total = expensive_work();
    let _ = ui.post(move || println!("{total}")); // runs on the UI thread
});

// `work` runs on another thread; `done` runs on the UI thread and may capture widgets.
spawn_background(|| read_big_file(), move |data| label.set_text(&data))?;
// Futures without a runtime: spawn_local(async move { .. });
```

**Widgets are cheap handles.** Cloning a control does not duplicate it; dropping a
handle does not destroy it. A native window lives until it is destroyed (explicitly
or together with its parent); calls on a destroyed widget return an error instead of
crashing.

**Logical units.** Positions and sizes are in 96-DPI logical units and are scaled to
each monitor's DPI automatically, including when a window moves between monitors.

**Builders and errors.** Every control is created with
`Control::builder()...parent(&container).build()?`, and fallible operations return
`GuiResult<T>` with a `GuiError`. Controls share a default height
(`DEFAULT_CONTROL_HEIGHT`, 28) so forms line up without manual tuning.

**Layouts** position children and follow resizes: `StackLayout`, `GridLayout`
(`Track::Fixed`, `Auto`, `Star`), `FlowLayout`, `DockLayout`, `AnchorLayout`,
`SplitLayout` and `AbsoluteLayout`, combined with `LayoutParams` (size, margins,
weight, alignment) and nested freely:

```rust
let form = StackLayout::vertical().spacing(8).padding(Thickness::all(12));
form.add_with(&name_box, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
form.add_with(&notes, LayoutParams::new().weight(1.0));
window.set_layout(form);
```

**Themes.** `Theme::light()`, `Theme::dark()` and `Theme::system()` apply to a window
and everything in it (`Theme::dark().apply(&window)`), including the library-drawn
controls and menus; high contrast is respected.

**Panics never cross into Windows.** Every function Windows calls directly (window
procedures, COM methods, callbacks) catches panics; the panic is re-raised by
`Application::run()` / `process_pending()`, outside the system callback.

## Events

Handlers are closures registered with `on_<something>`. The name tells **when** the
event happens:

| Form | When | Callback | Examples |
|---|---|---|---|
| present-tense verb | a direct user action, as it happens | returns nothing | `on_click`, `on_double_click`, `on_key_down`, `on_item_click`, `on_link_click` |
| past participle | a notification **after** something changed | returns nothing | `on_value_changed`, `on_selection_changed`, `on_toggled`, `on_closed`, `on_row_activated` |
| `_request` | a request **before** something happens, which may be refused | returns `bool` (`false` cancels) | `on_close_request`, `on_row_reorder_request`, `on_item_drop_request` |

```rust
window.on_close_request(|| MessageBox::confirm("Quit", "Close the window?"));
slider.on_value_changed(|value: i32| println!("{value}"));
list.on_row_reorder_request(|from, to| to != 0); // keep the first row in place
```

Nouns follow each control's vocabulary: *item* in `ListBox` and `TreeView`, *row* in
`ListView` and `DataGrid`. Older names that broke the convention still work as
`#[deprecated]` aliases (the compiler points to the new name) and will be removed
before 1.0. Handlers that return a `HandlerId` can be removed with
`widget.remove_handler(id)`.

## Components at a glance

| Area | Types |
|---|---|
| Windows | `Window` (builder, owner/modal, frameless, DWM corners and backdrops, shortcuts), `Dialog`, `TitleBar` |
| Basic controls | `Button`, `Label`, `TextBox`, `PasswordBox`, `CheckBox`, `RadioButton`, `ToggleButton`, `ComboBox`, `ListBox`, `Slider`, `ScrollBar`, `SpinButton`, `ProgressBar`, `PictureBox`, `GroupBox`, `Panel`, `Hyperlink` |
| Data | `ListView` (columns, icons, check boxes), `DataGrid` (millions of virtual rows, editing, filters, grouping, styles, CSV/HTML export), `TreeView`, `PropertyGrid`, `Chart` |
| Text | `RichTextBox` (RTF, lists, tables, images, range highlighting), `StyledText` |
| Modern controls | `ToggleSwitch`, `NumberBox`, `RatingControl`, `Card`, `Expander`, `BreadcrumbBar`, `DockHost`/`DockPane`, detachable `TabControl` tabs, `PrintPreview` |
| Commands | `MenuBar` (native) and `MenuStrip` (owner-drawn) built from the same `Menu`s, context menus, `CommandBar` (buttons, toggles, menus, search, overflow, customization), `ToolBar`, `StatusBar` |
| Notifications | `InfoBar`, `Toast`, `SystemNotification` (Windows notifications), `TrayIcon` |
| Dialogs | `OpenFileDialog`, `SaveFileDialog`, `FolderPicker`, `ColorDialog`, `FontDialog`, `PrintDialog`, `PageSetupDialog`, `FindDialog`, `MessageBox`, `TaskDialog`, web-style `Modal` |
| Drawing | `Canvas` on `on_paint` (GDI), GDI+ vectors and images, Direct2D/DirectWrite, an SVG subset, animations (`motion`) |
| System | clipboard (text, HTML, images, files, custom formats), drag and drop, printing, monitors, cursors, shell folders, IME, touch gestures |
| Accessibility | UI Automation for native and library-drawn controls (Invoke, Toggle, Value, RangeValue, Selection, Grid, Text, Scroll...), live-region announcements |

The full reference with every method is in [`docs/COMPONENTS.en.md`](docs/COMPONENTS.en.md)
and in the English rustdoc (`cargo doc --open`). The guides also have English versions (`*.en.md`).

### Web-style modal

```rust
let modal = Modal::builder()
    .title("Delete file?")
    .message("The file will be permanently deleted.")
    .button("Cancel", DialogResult::Cancel)
    .primary_button("Delete", DialogResult::Ok)
    .build(&window)?;
if modal.show()? == DialogResult::Ok { /* delete */ }
```

A dimmed backdrop covers the owner window and a centred card holds the title,
content and buttons, with animation, Esc/Enter handling, focus trapping and
validation before closing (`on_close_request`). Forms go in `modal.content()`.

### Icons

```rust
let save = Button::builder().text("Save").icon(&icons::SAVE).parent(&window).build()?;
let icon = icons::for_path("report.pdf").unwrap_or(&icons::FILE);   // PDF_FILE
let found = icons::find("folder-open");                              // by name or alias
```

392 icons in 22 categories (`icons::in_category(IconCategory::FileTypes)`), each in
`.ico` 16–256 px and `.png`; 39 also in `.svg`. Any API that shows an icon accepts
`&impl IconSource`, so your own icons (`IconAsset::new(..)` with `include_bytes!`)
work the same way. A sheet with every icon: [`assets/icons/preview.png`](assets/icons/preview.png).

### Library texts in English

The few texts the library shows by itself (Yes/No in the property grid, "More
options", close buttons, print preview...) are Portuguese by default. Switch them to
English once at start-up:

```rust
arqrustui::i18n::set_library_language("en");
```

Or follow the Windows display language (Portuguese on a Portuguese Windows, English
everywhere else):

```rust
arqrustui::i18n::use_system_library_language();
```

Individual texts can be overridden with `i18n::set_library_text`, or taken from your
own translation catalogue with `i18n::apply_library_translations`. Icon category
names (`IconCategory::label`) follow the same setting.

Error messages (`GuiError`'s `Display`, `InvalidArgument`/`Unsupported` reasons) and
deprecation notes are in English regardless of this setting.

## Building, testing and examples

```powershell
cargo build --release
cargo test                               # unit + integration tests (create real windows)
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo run --example demo                 # tour of every component
```

There are 32 examples (`cargo run --example <name>`), from `hello_world`, `login`,
`calculator` and `text_editor` to `listview_columns`, `command_bar`, `menu_strip`,
`notifications`, `drag_drop`, `charts`, `modern_controls`, `modal`, `icons` (an
icon-pack browser) and `demo`.

## Repository layout

Windows use `icons::DEFAULT_WINDOW_ICON` when no icon is explicitly selected,
including builds with `default-features = false`. `WindowBuilder::icon` and
`Window::set_icon` override it and keep that choice across DPI updates.
Custom `Dialog` windows follow the same rule. The executable file's icon is a
separate application build resource.

```text
ArqRustUI/
├── src/                  the library (core, controls, layout, menus, dialogs, graphics,
│                         themes, system, input, accessibility, i18n, icons, ffi)
├── crates/
│   ├── arqrustui-icons/        facade re-exporting the icon data crates
│   └── arqrustui-icons-data1…8 icon bytes, ≤ 9 MB each (crates.io's 10 MiB limit)
├── tests/                integration tests (real windows)
├── examples/             runnable examples
├── docs/                 architecture, components, events, compatibility, safety
└── assets/icons/         icon-pack origin, processing notes and preview sheet
```

The ten crates share one version and are published together (data crates, facade,
then `arqrustui`) — see [`docs/RELEASING.md`](docs/RELEASING.md).

The social category contains 58 designs. The bevel pack replaces 22 existing
designs and adds 18 platforms, with original ICO, PNG 512 and SVG files; the
remaining 18 designs stay available. Existing constants and aliases are preserved;
`facebook_messenger` finds `MESSENGER`. Artwork includes Font Awesome Free under
CC BY 4.0; attribution accompanies the affected data crates. See
`assets/icons/social/ORIGEM.md` and `docs/ICON_LICENSES.md`. The SVG renderer does
not draw text used by BeReal, KakaoTalk and Lemon8; their ICO/PNG versions are complete.

## Documentation map

In English (each one has a Portuguese original next to it, without `.en`):

| Document | Contents |
|---|---|
| [`BIBLIOTECA.en.md`](BIBLIOTECA.en.md) | Standard library guide (purpose, usage, structure, maintenance) |
| [`AGENTS.en.md`](AGENTS.en.md) | Guide for AI agents that use or extend the library |
| [`docs/COMPONENTS.en.md`](docs/COMPONENTS.en.md) | Component guide and API summary |
| [`docs/EVENTS.en.md`](docs/EVENTS.en.md) | Events, message loop, threads and `async` |
| [`docs/ARCHITECTURE.en.md`](docs/ARCHITECTURE.en.md) | Internal architecture |
| [`docs/MEMORY_AND_SAFETY.en.md`](docs/MEMORY_AND_SAFETY.en.md) | Handle ownership, `unsafe` policy, panic safety |
| [`docs/CUSTOM_CONTROLS.en.md`](docs/CUSTOM_CONTROLS.en.md) | Writing custom controls |
| [`docs/COMPATIBILITY.en.md`](docs/COMPATIBILITY.en.md) | Windows versions, known limitations, validation status |
| [`docs/FEATURE_MATRIX.en.md`](docs/FEATURE_MATRIX.en.md) | State of every feature |
| [`docs/RELEASING.en.md`](docs/RELEASING.en.md) | How to publish the crates |
| [`CHANGELOG.en.md`](CHANGELOG.en.md) | Changes per version |

In Portuguese only: [`README.md`](README.md) (the Portuguese version of this page), `docs/COMPOSITION.md`,
`docs/ICONE_PADRAO.md` (default window icon), `docs/ICON_LICENSES.md`, `docs/REDES_SOCIAIS.md` and
`docs/REDES_SOCIAIS_BEVEL.md` (social network icons) and `assets/icons/README.md` (icon pack origin).

The texts and comments of the programs in `examples/` are in English.

## License

MIT — see [`LICENSE`](LICENSE).
