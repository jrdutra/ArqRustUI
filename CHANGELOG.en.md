# Changelog

*Português: [CHANGELOG.md](CHANGELOG.md).*

All notable changes to ArqRustUI are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[semantic versioning](https://semver.org/): while the version is 0.x, a "minor"
release (0.**2**.0) may bring breaking changes, always listed under **Changed** or
**Removed** with what to do to upgrade.

The ten crates in the repository (`arqrustui`, the `arqrustui-icons` facade and the
eight `arqrustui-icons-dataN`) always share the same version and are published
together (see [`docs/RELEASING.md`](docs/RELEASING.md), in Portuguese).

Sections of each version: **Added**, **Changed**, **Deprecated**, **Removed**,
**Fixed**, **Security**.

## [Unreleased]

### Added

- Social network pack supplied on 2026-11-16: 40 designs available in the `Social`
  category, with 39 new assets and reuse of the identical Threads one. The catalog
  now has 374 icons. `GITHUB_SOCIAL`, `GITLAB_SOCIAL` and `THREADS_ALTERNATE` are
  separate variants; the earlier logos are still available. The ICOs received were
  kept as they were, including crops and fragments documented in
  `assets/icons/social/ORIGEM.md`.
- Eighth icon data crate of our own, keeping the parts below 9 MB.
- `i18n::system_library_language` and `i18n::use_system_library_language`: they pick
  the table of the library's own texts from the Windows display language (`"pt"` on
  a Portuguese Windows, `"en"` on all others). The default is still Portuguese.
- 22 `LibraryText::Category*` keys (`category_actions`…) with the icon category
  names in Portuguese and English, and `IconCategory::label_key`.
- English versions of the guides (`*.en.md`, next to each original): `BIBLIOTECA`,
  `AGENTS`, `CHANGELOG` and, in `docs/`, `ARCHITECTURE`, `COMPATIBILITY`, `COMPONENTS`,
  `CUSTOM_CONTROLS`, `EVENTS`, `FEATURE_MATRIX`, `MEMORY_AND_SAFETY` and `RELEASING`.

### Changed

- GitHub preparation: English root README with a local gallery of supplied
  screenshots, all 32 examples, usage, limitations and publication guidance.
  Main package `readme` metadata now uses `README.md`. Expanded cache/temporary
  exclusions; local work archives retained outside the repository. Source code,
  assets, tests and Git history are preserved.
- English rustdoc throughout modules, APIs, safety contracts, icon macros, data
  crates and published examples. Guides retain Portuguese and English versions;
  localized text remains in translation tables.
- Responsive layouts and a 780×520 minimum size in `modern_controls`.
  The number field retains its 140 logical-unit width.
- Fixed duplicate GDI-to-GDI+ scaling in `Canvas`, which clipped `ToggleSwitch`
  and `RatingControl` stars at 150%. Pixel regression covers 96, 144 and 192 DPI
  and restoration of subsequent GDI drawing.
- Removed redundant drag-image `map_or`, fixing strict Clippy's
  `map_or_identity` failure with `-D warnings`.
- Messages for developers are now in English: `GuiError`'s `Display`, the
  `InvalidArgument`/`Unsupported` reasons, the contexts of Win32 errors, internal
  `expect`s, SVG reader errors and the notes of the `#[deprecated]` aliases. Texts
  shown to end users remain in the `LibraryText` table. Code that compared an
  error's text should compare the `GuiError` variant instead.
- **Breaking:** `IconCategory::label` returns `String` (before: a `&'static str`
  fixed in Portuguese) and follows the language of `i18n::set_library_language`; it
  can be overridden with `i18n::set_library_text(LibraryText::CategoryActions, ..)`.
  To upgrade, replace `.unwrap_or("…")` with `.unwrap_or_else(|| "…".to_string())`
  where the result was combined with a `&str`.
- Descriptions in the ten `Cargo.toml` files (shown on crates.io) are in English.
- Texts and comments of the 32 examples (`examples/`) are in English. The ones that
  show the library's own texts call `i18n::set_library_language("en")`. In
  `validated_form`, the Brazilian CEP field became a 5-digit ZIP code.

- The New command in the `demo` and `menus` examples uses the document icon
  `icons::FILE`, at the resolution requested by the control, instead of the generic
  application icon. Both examples still compile without the optional icon catalog.
- Social networks updated from the `arqrustui-social-icons-bevel-3d.zip` pack:
  22 designs replaced and 18 new ones, keeping the earlier constants and the
  platforms without a replacement in the pack. There are now 392 icons, 58 social
  ones and 79 SVG sources. New PNGs at 512 px; `facebook_messenger` finds
  `MESSENGER`. Artwork, origin, Font Awesome/CC BY 4.0 credits and the text
  limitation in three SVGs are documented and included in the affected data
  subpackages.
- Windows without an explicit icon now use the library logo, available as
  `icons::DEFAULT_WINDOW_ICON` even without the optional `icons` feature. Icons set
  in the builder or with `set_icon` still take precedence, including on DPI
  updates. The original ICO ships with the library package.
- The aliases `21_github`, `22_gitlab` and `31_threads_alternate` find the matching
  designs of the social pack; the earlier canonical names still find their original
  designs.
- Library name and directory updated to ArqRustUI; crate/import to `arqrustui`. The
  eight icon subpackages use the `arqrustui-icons` prefix. Consumers must update
  their path dependencies and imports.

## [0.1.0] — not yet released

First version. Validated so far under Wine 9 (`x86_64-pc-windows-gnu` and
`i686-pc-windows-gnu`); the items that still need to be seen on real Windows are
marked with ⚠ in [`docs/FEATURE_MATRIX.en.md`](docs/FEATURE_MATRIX.en.md) and
described in [`docs/COMPATIBILITY.en.md`](docs/COMPATIBILITY.en.md).

### Added

- Native GUI library for Windows 10 and 11 (x64) in pure Rust: only `std` and
  hand-written FFI for Win32, Common Controls v6, GDI/GDI+, Direct2D/DirectWrite,
  COM, DWM and UI Automation; no third-party crates.
- Core: `Application`, windows with builders, events through closures
  (`on_<event>`), `GuiResult`, RAII for Windows resources, a message loop with
  `UiDispatcher` for other threads, timers, per-monitor DPI, visual styles and DPI
  enabled at run time (no manifest).
- Native controls (buttons, fields, lists, trees, `ListView`, virtual `DataGrid`,
  tabs, `RichTextBox` with highlights, date and time pickers, progress, scroll and
  status bars, among others) and controls drawn by the library: switch, number box
  with expressions, rating, cards, expander, breadcrumb trail, charts, print
  preview, dockable panes, detachable tabs and a window with its own title bar.
- Menus: Windows `MenuBar` and drawn `MenuStrip` with the same `Menu`s, context
  menus, `CommandBar` with buttons, toggles, menus, search and customization.
- Dialogs: the common system ones (open, save, folder, color, font, print, page
  setup, find), `MessageBox`, `TaskDialog`, the library's own `Dialog` and a
  web-style `Modal` (dimmed backdrop, centered card, animation, validation and
  trapped focus).
- Layouts (stack, grid, flow, dock, anchors, split, absolute), light, dark and high
  contrast themes, animations (`motion`), SVG, touch gestures, IME with a candidate
  window, drag and drop (files, text and reordering), clipboard, Windows
  notifications, system tray, accessibility through UI Automation and translatable
  library texts (Portuguese and English).
- Pack of 335 icons in 22 categories (`icons` feature, on by default): `.ico` from
  16 to 256 px, `.png` and 39 in `.svg`, with constants (`icons::FOLDER`), search by
  name and alias (`icons::find`), categories and icon by file type
  (`icons::for_extension("pdf")`, `icons::for_path(..)`, 66 types with their own
  design, 92 extensions recognized).
- Distribution ready for crates.io: the icon bytes (about 61 MB) live in seven data
  crates of up to 9 MB (`arqrustui-icons-data1` to `-data7`), gathered by the
  `arqrustui-icons` facade, which is the optional dependency of the `icons`
  feature; the `arqrustui` package is under 1 MB. The `arqrustui::icons::*` API is
  the same as before the split.
- docs.rs metadata (documentation built with the `x86_64-pc-windows-msvc` target,
  since the crate is `#![cfg(windows)]`).
- Continuous integration on GitHub Actions: formatting, clippy and documentation
  without warnings, tests on real Windows (MSVC and GNU x64, MSVC x86), minimum Rust
  version (1.80), documentation as on docs.rs, size of each package below the
  crates.io limit and an experimental ARM64 build.
- Support for 32-bit Windows (x86): the whole suite passes on 32-bit Wine.
- Complete guide in English (`README.en.md`).
- `ImeCandidates::new`, `with_selection` and `with_page`, to build a candidate list
  outside the library.
- Tests for what comes from Windows and was not simulated before
  (`tests/system_events.rs`): click and Enter on a link, system theme change
  (`WM_SETTINGCHANGE`), files dropped on the window (`WM_DROPFILES`), the tooltip
  showing when the pointer rests, the Windows and the drawn context menu
  (`WM_CONTEXTMENU`) and, on a DPI change, the window icon and the suggested
  rectangle.
- Automated tests of the system dialogs (open, save, folder, color, font, print,
  page setup, find, `MessageBox` and `TaskDialog`): a timer answers the modal dialog
  as a user would (`tests/dialogs.rs`).

### Changed

- The largest files were split into submodules by topic, without changing the API
  or the behavior: `DataGrid` (drawing, editing, export, filtering and grouping,
  reordering, styles), `CommandBar` (keyboard and accessibility, layout, drawing,
  the `»` panel), `WidgetCore` message dispatch (`core/widget/dispatch.rs`), the
  drawn menus (state machine, drawing, accessibility), `MenuStrip` (bar and window
  messages), the UI Automation tree (the provider's COM methods and calls for the
  tests) and notifications (WinRT/COM infrastructure). No file is longer than
  1,200 lines.
- Event names unified under one convention (present-tense verb for user actions,
  participle for a notice after the fact, `_request` for a request that can be
  refused; see "Event names" in `docs/EVENTS.en.md`). New names:
  `Window::on_close_request`, `InfoBar::on_closed`, `Toast::on_dismissed`,
  `CommandButton::on_toggled`, `Hyperlink::on_link_click`,
  `ListView::on_row_activated`, `ListView`/`DataGrid::on_row_reorder_request` and
  `on_rows_reorder_request`, `ListBox::on_item_reorder_request` and
  `on_items_reorder_request`, `TreeView::on_item_drop_request` and
  `Transition::on_finished`, `Menu::on_highlight_changed`,
  `Ime::on_candidates_changed` and `Ime::on_composition_changed`.
- The 13 public structs that the library produces and users only read became
  `#[non_exhaustive]`: `KeyEvent`, `MouseEvent`, `PointerEvent`, `GestureEvent`,
  `FindEvent`, `NotificationActivation`, `FontChoice`, `TaskDialogResult`,
  `DetachedTab`, `ImeCandidates`, `RawMessage`, `Monitor` and `WindowsVersion`.
  **When upgrading:** the fields are still public for reading, but these structs can
  no longer be built with a literal (`Type { .. }`) outside the library; for
  `ImeCandidates`, which is also an API input, use `ImeCandidates::new(items)` with
  `with_selection`/`with_page`. Value types (`Color`, `Rect`, `Thickness`…) and
  styles can still be built.
- 59 of the 63 public enums became `#[non_exhaustive]` (among them `GuiError`,
  `Event`/`EventKind`, `IconCategory` and `DialogResult`), so that new variants do
  not break library users. **When upgrading:** a `match` on these enums outside the
  library needs a `_ => ..` arm. Only the closed sets `Orientation`, `CheckState` and
  `TreeDropPosition` stay exhaustive (and `LibraryText`, which was already
  `#[non_exhaustive]`).
- Icon PNGs that were exact copies of the `.ico`'s 256 px frame (153 of the 335) are
  now read from inside the `.ico`: `IconAsset::png_bytes` returns the same bytes as
  before, without storing them twice.

### Fixed

- 32-bit: the library did not link, because 32-bit user32 does not export
  `GetWindowLongPtrW`/`SetWindowLongPtrW`; they now point to
  `GetWindowLongW`/`SetWindowLongW`, as in `windows.h`.
- 32-bit: `double` values in a `VARIANT` (UI Automation: `RangeValue.Value` and
  other numeric properties) lost their high half and reached screen readers as 0;
  `VARIANT::from_f64`/`as_f64` write and read the 8 bytes on every target.
- The clipboard tests (a system-wide resource) are now serialized; before, running
  in parallel, one of them failed now and then.

### Security

- No panic crosses the boundary with Windows anymore. Before, only the window
  procedures and some callbacks had `catch_unwind`; the COM methods of UI
  Automation, `IDataObject`, `IDropSource`/`IDropTarget` and the notification
  activator, and the `ListView` sort callbacks, RichEdit stream, monitor enumeration
  and the reordering subclass (123 functions) did not, and a panic in them was
  undefined behavior in Rust 1.80 (and would abort the process from 1.81 on). Now
  all of them run inside `core::ffi_guard`: Windows receives `E_UNEXPECTED` (COM) or
  a neutral value, and the panic is re-raised by the message loop. A test scans the
  code and fails if a new entry point is left unprotected. The MSRV is still 1.80.

### Deprecated

- The old names of the renamed events still work as `#[deprecated]` aliases (the
  compiler warns and points to the new name): `Window::on_close`,
  `InfoBar::on_close`, `Toast::on_dismiss`, `CommandButton::on_toggle`,
  `Hyperlink::on_link_clicked`, `ListView::on_item_activated`,
  `ListView`/`DataGrid::on_reorder` and `on_rows_reordered`, `ListBox::on_reorder`
  and `on_items_reordered`, `TreeView::on_item_drop`, `Transition::on_done`,
  `Menu::on_highlight`, `Ime::on_candidates` and `Ime::on_composition`. They will be
  removed before version 1.0.

### Known issues

- Almost nothing has been checked on real Windows: only two composition items were
  recorded as tested there, by earlier work. CI is the first place where the whole
  suite runs outside Wine.

[Unreleased]: #unreleased
[0.1.0]: #010--not-yet-released
