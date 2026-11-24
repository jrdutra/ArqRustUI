# ArqRustUI — library guide

*Português: [BIBLIOTECA.md](BIBLIOTECA.md). Overview in English: [README.en.md](README.en.md).*

## 1. Identity and purpose

- Directory: `ArqRustUI`; crate/import: `arqrustui`.
- Version: 0.1.0; edition 2021; declared minimum Rust: 1.80.
- Platform: native GUI for Windows 10/11. The crate uses `cfg(windows)`; compiling on another system does not validate the interface features.
- Purpose: provide windows, controls, layouts, drawing and interface integration reusable by any Rust application.

Its own implementation in Rust, `std` and hand-written FFI for native Windows APIs/DLLs. It does not use third-party GUI crates or bindings. The operating system APIs are a run-time requirement, not dependencies on external crates.

## 2. Implemented features

| Group | Public features |
|---|---|
| Application and windows | `Application`, `Window`, the library's own dialogs, multiple windows, DPI, focus and events. |
| Default window icon | The `icons::DEFAULT_WINDOW_ICON` logo applied automatically to every `Window`, including the library's own dialogs and without the optional `icons` feature. `WindowBuilder::icon` and `Window::set_icon` replace the default and keep precedence on DPI updates. |
| Controls | Buttons, labels, text fields, selection, calendars, lists, trees, tables, tabs (detachable), `DataGrid`, `PropertyGrid` and the drawn `ToggleSwitch`, `NumberBox`, `RatingControl`, `Card`, `Expander`, `BreadcrumbBar`, `Chart`, `PrintPreview`, `DockHost`/`DockPane` and `TitleBar` (frameless window). |
| Layout | Stack, Grid, Flow, Dock, Anchor, Split, `StatusBarLayout` and `ScrollViewport`. |
| Menus and commands | `MenuBar`, `MenuStrip`, `Menu`, context menus, shortcuts, `CommandBar`, `ToolBar` and `StatusBar`. |
| Drawing and themes | Canvas, fonts, colors, GDI/GDI+ (with floating-point vectors and gradients), SVG (`SvgImage`), Direct2D/DirectWrite, animations (`motion`) and configurable themes. |
| Formatted text | `RichTextBox` (with lists, tables and images), `TextFormat`, `StyledText`, RTF and `apply_highlights` for ranges without replacing the text or adding undo steps. |
| Button content | Icons, relative position and horizontal alignment through `content_alignment(TextAlign)`, with margin and truncation. |
| System | Clipboard (DIBV5, PNG, metafiles, monitoring), Shell, tray, notifications (with a COM activator), printing with a page range and preview, drag and drop, touch gestures, IME with the library's own candidate list, accessibility (UIA model for drawn controls) and internationalization (also of the library's own texts), as documented. |
| Asynchronous work | Dispatcher, timers and task helpers, respecting the UI thread. |
| Social network icons | 58 designs in `IconCategory::Social`, full catalog with 392 icons. The bevel batch replaces 22 designs and adds 18 platforms, with ICO, 512 px PNG and SVG; the other 18 earlier networks remain. Existing constants preserved; origin, credits and SVG limits in `assets/icons/social/ORIGEM.md` (Portuguese). |

The API map is in [AGENTS.en.md](AGENTS.en.md); the detailed list and the state of each feature are in [COMPONENTS.en.md](docs/COMPONENTS.en.md) and [FEATURE_MATRIX.en.md](docs/FEATURE_MATRIX.en.md). Do not read this summary as a guarantee that every feature is complete in every environment.

## 3. How to use it

In the `Cargo.toml` of a consumer in a sibling directory:

```toml
[dependencies]
arqrustui = { path = "../ArqRustUI" }
```

Example with the public API:

```rust
#![windows_subsystem = "windows"]
use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("ArqRustUI example").size(640, 360).build()?;
    let button = Button::builder().text("Open").icon(&icons::FOLDER_OPEN)
        .content_alignment(TextAlign::Left).parent(&window).build()?;
    let layout = StackLayout::vertical().padding(Thickness::all(16));
    layout.add_with(&button, LayoutParams::new().height(48));
    window.set_layout(layout);
    window.show();
    app.run()
}
```

The path is relative to the consumer's manifest; there is no automatic dependency discovery.

## 4. Dependencies and organization

**Crate dependencies: none from third parties and none from other ArqRust libraries.** The only entry in `dependencies` is the library's own `arqrustui-icons` crate (optional, `icons` feature), which lives inside this folder, in `crates/`, together with the eight data crates `arqrustui-icons-data1` to `-data8`: they are the bytes of ArqRustUI's own icon pack, split into parts of up to 9 MB only to fit crates.io's 10 MiB per-package limit. They are not another library of the suite nor third-party code, have no code besides `include_bytes!` and depend on nothing. `dev-dependencies` and `build-dependencies` are empty; `std` does not need to be declared in the manifest. For this, `ArqRustUI/Cargo.toml` is also a Cargo workspace with the `crates/*` members (the suite root is still not a workspace).

If an evolution really needs another library of the suite, it must be in a sibling folder and declared only through `path = "../ArqRustX"`. The example is an organization rule, not a dependency to add now. Using crates.io, other registries, Git, absolute paths, nested dependencies or third-party code copied into a local folder is forbidden. The restriction covers optional, platform-specific, test/build and transitive dependencies. Do not create cycles.

## 5. Internal structure

```text
ArqRustUI/
├── Cargo.toml
├── BIBLIOTECA.md
├── README.md
├── AGENTS.md
├── rustfmt.toml
├── src/
│   ├── lib.rs             API, modules and prelude
│   ├── core/              application, windows, events, handles and errors
│   ├── controls/          components and generic composition
│   ├── ffi/               native Windows declarations
│   ├── layout/            layout algorithms and compositions
│   ├── graphics/          drawing, fonts and images
│   ├── menus/             native and drawn menus
│   └── other modules      dialogs, themes, system, input, accessibility, i18n, icons
├── tests/                 unit/native integration tests
├── examples/              standalone programs
├── docs/                  architecture, components, events, compatibility and safety
├── crates/                the icon pack's own crates (facade + 7 data crates)
└── assets/icons/          origin, processing and preview sheet of the icon pack
```

`BIBLIOTECA.md` is the standard guide. `README.md` complements the presentation and the Cargo manifest points to this guide. Each of these documents has an English version next to it (`*.en.md`).

## 6. Errors, limits and compatibility

- Requires Windows to run and check the interface. Visual components belong to the UI thread and are not `Send`/`Sync`.
- `Application` must exist before controls are created. Use builders, `Clone` handles, callbacks and `GuiResult` through the public API.
- Failures are explicit; unavailable features may return `GuiError::Unsupported`. Check the limitations per Windows version and per environment.
- `apply_highlights` takes RichEdit UTF-16 ranges: a CRLF pair counts as one line break, and characters outside the BMP use two units. The suite's parsers return UTF-8 bytes; the consumer must convert the positions correctly.
- The feature does not define grammars or language palettes; it receives text, ranges and styles provided by the consumer.
- Dark theme, notifications, drag and drop, accessibility and other features have validation and platform caveats. See [COMPATIBILITY.en.md](docs/COMPATIBILITY.en.md) and the matrix, without inferring full approval from a successful build.
- GUI tests may depend on a graphical environment, APIs and system associations. Record the results that were run, including failures; historical counts in the documentation do not replace a current run.

## 7. Validation

Run inside `ArqRustUI/`:

```powershell
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline
cargo build --offline --release
cargo metadata --offline --format-version 1
```

The commands validate this library on its own. Also check the manifests and the origin of the packages: being in the cache/offline does not make a third-party crate allowed. Do not declare tests as passing without running them; record environment limitations and real failures.

## 8. Guidance for AI and maintenance

- Rustdoc is in English: modules, APIs, `Safety` contracts, macros and published
  examples. Guides retain Portuguese and English versions.
- Read the [local AGENTS.md](AGENTS.md) in full ([English version](AGENTS.en.md)) before changing or consuming the interface API.
- Keep it generic: no parser, format, name, flow or rule specific to a consuming application inside this library.
- Do not add GUI frameworks or bindings. Native integration is implemented in Rust with hand-written FFI; keep `unsafe` isolated and justified, ownership/RAII, error handling and documented public APIs.
- Use only `std` and, if there is a demonstrated need, public APIs of one of the suite's own libraries in a sibling directory. Do not add fictitious dependencies or create cycles.
- When extending generic features, update this guide, AGENTS.md, the component/matrix documentation (Portuguese and `.en.md` versions), examples and standalone tests.
- Validate the library on its own on Windows. Report changes and results separately from any consuming application.
