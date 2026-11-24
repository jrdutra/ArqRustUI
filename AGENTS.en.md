# ArqRustUI — guide for AI agents

*Português: [AGENTS.md](AGENTS.md).*

This file is the source of truth for any AI agent that is going to **use**, **integrate** or **extend** ArqRustUI. Read it in full before generating code. It does not depend on any consuming project.

---

## 1. What it is

ArqRustUI is a **native Windows** GUI library in **pure Rust**:

- crate `arqrustui` (edition 2021, Rust ≥ 1.80), **with no third-party dependencies** — only `std` and hand-written FFI for the Windows DLLs; the icon pack bytes come from the library's own crates in `crates/` (`arqrustui-icons` and `arqrustui-icons-data1` to `-data8`);
- native system controls (Win32 / Common Controls v6), plus a few composites written in Rust;
- high-level API: builders, closures, `Result`, RAII;
- a **generic, self-contained** library: it does not and must not know any application that uses it.

It only works on Windows (`#![cfg(windows)]`; on other systems the crate compiles empty).

## 2. Integrating it into a project

```toml
# Cargo.toml of the consuming project
[dependencies]
arqrustui = { path = "../ArqRustUI" }   # local library in a sibling directory
```

```rust
#![windows_subsystem = "windows"]   // first line of main.rs: no console window
use arqrustui::prelude::*;            // imports everything a typical application needs
```

No manifest, `build.rs`, `.rc` resource or extra initialization is needed: `Application::new()` enables visual styles, per-monitor DPI and COM.

Every `Window` without an explicit icon uses the `icons::DEFAULT_WINDOW_ICON` logo, even
with `default-features = false`. `WindowBuilder::icon(&icon)` and
`window.set_icon(&icon)` replace the default, also after DPI changes.
The library's own dialogs created with `Dialog` follow the same rule. The logo is not part of
the optional catalog of 392 icons. The executable file's icon is a build resource of
the application, defined by the application itself.

## 3. Required skeleton of every application

```rust
#![windows_subsystem = "windows"]
use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;                       // 1. always first, once per UI thread
    let window = Window::builder().title("App").size(640, 400).centered().build()?; // 2. window (born hidden)

    let name = TextBox::builder().placeholder("Name").bounds(16, 16, 240, 24).parent(&window).build()?;
    let greet = Button::builder().text("&Greet").bounds(16, 52, 100, 28).default(true).parent(&window).build()?;
    let output = Label::builder().bounds(16, 92, 400, 20).parent(&window).build()?;

    // 3. events: clone the handles the closure uses
    let (input, target) = (name.clone(), output.clone());
    greet.on_click(move || target.set_text(&format!("Hello, {}!", input.text())));

    window.show();                                       // 4. show
    app.run()                                            // 5. event loop (blocks until the last window closes)
}
```

## 4. Mental model — rules that produce correct code

1. **`Application` first.** Any construction before it returns `GuiError::NotInitialized`. Keep `app` alive until the end of `main`.
2. **One UI thread.** Visual types are not `Send`/`Sync` (compile error when moving them to another thread). For background work use `spawn_local` + `run_blocking`, `spawn_background` or `app.dispatcher()` (section 9).
3. **Controls are `Clone` handles.** Cloning does not duplicate the control; dropping the handle does not destroy it. The control lives as long as the native window exists. There is no need to keep controls in structs to keep them alive.
4. **Every control needs `.parent(&something)`** — `Window`, `Panel`, `GroupBox`, a `TabControl` page, `dialog.window()`. Without a parent: `GuiError::MissingParent`.
5. **Builders return `GuiResult<T>`**: use `?`. Simple setters (`set_text`, `set_bounds`…) do not return `Result`.
6. **Logical units (96 DPI)** for sizes, control positions, mouse events and drawing. Never multiply by DPI by hand. Exception: the position of top-level windows is in screen pixels.
7. **`Window::builder().size(w, h)` is the client area size**, not the outer size.
8. **Event closures are `'static`**: capture clones (`let x = x.clone(); move || …`). Shared mutable state: `Rc<Cell<T>>` / `Rc<RefCell<T>>`.
9. **Events fire on user actions**, not on `set_*` (except `set_text` on edit fields, which raises `on_text_changed`).
10. **Never block the UI thread** (`thread::sleep`, long I/O, slow loops).
11. **Dialogs**: `Ok(None)` = the user cancelled; `Err(_)` = a real failure.
12. **Version-dependent features** return `GuiError::Unsupported` — treat them as optional (`let _ = window.set_corner_style(..);` with a comment, or `if let Err(e) = …`).
13. **Do not use `arqrustui::ffi` or `unsafe`** in application code, unless there is a real, documented need (section 12).

## 5. Quick map: "I need X" → use Y

| Need | API |
|---|---|
| Main window | `Window::builder()…build()?` |
| Custom modal dialog | `Dialog::new(&owner, title, w, h)?` + `show_modal()?` |
| Web-style modal (dimmed backdrop, centered card) | `Modal::builder().title(..).message(..).button(..).primary_button(..).build(&window)?.show()?`; form: controls in `modal.content()` + `on_close_request` to validate; shortcuts `Modal::confirm` / `Modal::alert` |
| Message / confirmation | `MessageBox::info/warning/error/confirm`, `MessageBox::show` |
| Button / label / text field | `Button`, `Label`, `TextBox` (`.multiline(true)`), `PasswordBox` |
| Formatted text | `RichTextBox` |
| Options | `CheckBox`, `RadioButton`, `ToggleButton`, `ComboBox`, `ListBox` |
| Number | `NumberBox` (limits, step, decimals, expressions), `SpinButton`, `Slider`, `ProgressBar`, `ScrollBar` |
| On/off | `ToggleSwitch::builder().text("Wi-Fi").on(true)` (+ `on_toggled`) |
| Star rating | `RatingControl::builder().maximum(5).allow_half(true)` |
| Collapsible section | `Expander::builder().header(..).content_height(n)` → children in `expander.content()` |
| Card | `Card::builder().title(..).subtitle(..).clickable(true)` → children in `card.content_bounds()` |
| Breadcrumb trail | `BreadcrumbBar::builder().items(&[..])` + `on_item_click` / `truncate` |
| Line/area/bar/pie chart | `Chart::builder().kind(ChartKind::Bar).categories(&[..]).series(ChartSeries::new(..))` |
| Date / time | `DatePicker`, `TimePicker`, `MonthCalendar` |
| Small table | `ListView` |
| Large, editable or virtual table | `DataGrid` |
| Tree | `TreeView` |
| Editable name/value pairs | `PropertyGrid` |
| Tabs | `TabControl` (`add_tab` returns the tab's `Panel`; `set_detachable(true)` to drag tabs into their own windows) |
| Dockable panes (IDE style) | `DockHost` + `add_pane(title, DockSide::Left)` → `DockPane::content()`; `float()`/`dock()` |
| Frameless window with its own bar | `TitleBar::attach(&window)?` (at the top of the layout) |
| Group / contain controls | `Panel`, `GroupBox`, `SplitContainer` |
| Menu bar / context menu | `MenuBar` + `Menu` (native) or `MenuStrip` + `Menu` (drawn, follows the theme), `ContextMenu` (`set_drawn(true)` for the same drawing) |
| Toolbar / status bar | `ToolBar::new(&window)?` (native), `StatusBar::new(&window)?` |
| Command bar with icons, `▾` menus, split button and search | `CommandBar::builder().parent(&window).build()?` + `add_button(CommandButton::new(..).icon(..).menu(&m))`, `add_separator`, `add_spacer`, `add_search(..)`, `add_widget(..)` |
| Automatic positioning | `StackLayout`, `GridLayout`, `DockLayout`, `FlowLayout`, `AnchorLayout`, `SplitLayout` |
| Open / save / folder | `OpenFileDialog`, `SaveFileDialog`, `FolderPicker` |
| Color / font / printing | `ColorDialog`, `FontDialog`, `PrintDialog` (`show_with(PrintOptions)` for a page range) |
| Multi-page document, preview | `PrintDocument::new(name, pages, \|i, canvas\| ..)` + `PrintPreviewDialog::new(doc).show(&window)?` or `printer.print_document(&doc)` |
| Question with custom buttons | `TaskDialog` |
| Custom drawing | `Panel::on_paint(\|canvas\| …)`; antialiasing: `canvas.with_gdiplus` (curves and gradients: `fill_figures`/`stroke_figures`); animation: `Direct2DSurface::attach` |
| SVG image | `SvgImage::parse(..)?` → `render(w, h)?` / `canvas.draw_svg(..)` / `PictureBox::set_svg`; icons: `icons::X.svg_image()` |
| Animate values, positions, show/hide | `motion::animate_value(from, to, Duration, Easing::EaseOut, \|v\| ..)?`, `motion::animate_bounds(&w, Rect, ..)`, `motion::show_animated(&w, ShowEffect::SlideDown, ..)` |
| Pinch, rotation, drag with inertia (touch) | `gestures::on_gesture(&w, \|g\| ..)?` (`g.scale`, `g.rotation`, `g.translation`, `g.phase`) |
| Themed IME candidate list | `Ime::use_candidate_window(&field)?` |
| Accessible drawn control | `accessibility::set_accessible_model(&panel, impl AccessibleModel)` |
| Light/dark theme | `Theme::dark().apply(&window)`, `Theme::system()`, `SystemTheme::current()` |
| Tray icon / notification | `TrayIcon` |
| Action Center notification (with buttons) | `SystemNotification::new("Title").body(..).button("Open", "open").on_activated(\|arg\| ..).show()?` |
| In-window notice | `InfoBar::builder().severity(Severity::Warning).title(..).message(..).parent(&w).build()?` (+ `add_action`, `open`/`close`) |
| Temporary notice in the corner | `Toast::new("Saved").message(..).severity(Severity::Success).show(&window)?` |
| Clipboard | `Clipboard::set_text` / `Clipboard::text`; changes: `ClipboardWatcher::new(\|\| ..)?`; vector drawing: `Metafile::record(..)` + `ClipboardContent::metafile`; several formats at once: `Clipboard::set(&ClipboardContent::new().text(..).html(..).image(&img)?)`; `Clipboard::{html, image, files, data("MyApp.Items")}` |
| Reorder by dragging | `list.set_reorderable(true)` (+ `on_item_reorder_request(\|from, to\| true)` on `ListBox`, `on_row_reorder_request` on `ListView`/`DataGrid`); `tree.set_drag_enabled(true)` (+ `on_item_drop_request`, `on_item_moved`) |
| Drag out | `list.on_drag_out(\|i\| Some(ClipboardContent::new().text(..)))`; from any control: `start_drag(&w, &content, &[DropEffect::Copy])?` / `start_drag_with_image` |
| Receive drops (files, text, images) | `accept_drops(&w, DropHandler::new().on_over(\|d\| ..).on_drop(\|d, effect\| ..))?`; `d.files()`, `d.text()`, `d.image()?`, `d.position()` |
| Open a file/URL in the system | `arqrustui::system::shell::open(..)` |
| Keyboard shortcut | `window.add_shortcut(Shortcut::ctrl(Key(b'S' as u16)), \|\| …)` |
| Timer | `Timer::interval(d, f)?` (keep the value) / `Timer::once(d, f)?` |
| Background work | `spawn_local(async { run_blocking(..).await })`, `spawn_background` |
| Regional dates/numbers | `Locale::user_default().format_date(..)` etc. |
| Translatable texts | `arqrustui::i18n::Translations`; the library's own texts: `i18n::set_library_language("en")`, `i18n::use_system_library_language()` (Windows language) or `apply_library_translations` |
| Ready-made icon (392 in the pack) | `icons::FOLDER`, `icons::SAVE`… (constants); `icons::find("name")`; sheet in `assets/icons/preview.png` |
| Social networks (58 designs) | `icons::YOUTUBE`, `WHATSAPP`, `X_TWITTER`, `BEREAL`, `KICK`, `KUAISHOU_KWAI`…; `IconCategory::Social`. Earlier names preserved; bevel batch in ICO/PNG 512/SVG, origin, attribution and limits in `assets/icons/social/ORIGEM.md` (Portuguese) |
| Icon by file type (file list, explorer) | `icons::for_path(&path).unwrap_or(&icons::FILE)` or `icons::for_extension("pdf")`; constants `icons::PDF_FILE`, `icons::XLSX_FILE`… (category `IconCategory::FileTypes`) |
| Icon on a button / label | `Button::builder().icon(&icons::SAVE)`, `Label::builder().icon(..)`; `set_icon`, `clear_icon`, `icon_position` |
| Icon in a tree / list / grid / tabs | `tree.set_item_icon`, `list.set_row_icon`, `grid.set_row_icon` / `grid.set_icon_provider`, `tabs.set_tab_icon` |
| Icon in a combo box / list box | `ComboBox::builder().with_icons(true)` + `add_item_with_icon` (same for `ListBox`) |
| Icon in a bar, menu, window, tray, dialog | `toolbar.add_button(t, Some(&icons::X), ..)`, `status.set_part_icon`, `item.set_icon`, `window.set_icon`, `TrayIcon::new(&icons::X, ..)`, `TaskDialog::custom_icon` |
| Large image/icon | `PictureBox::builder().icon(&icons::X)` (above 256 px it uses the PNG) |
| The application's own icon | `IconAsset::new("name", include_bytes!("a.ico"), include_bytes!("a.png"), None)` |

Full signatures: `docs/COMPONENTS.en.md` (summary) and rustdoc (`cargo doc --open`). The programs in `examples/` are ready-made templates — **look at the closest example before writing a new screen**.

## 6. Builder pattern (the same for every control)

```rust
Type::builder()
    .text("…")                          // initial text
    .bounds(x, y, width, height)        // or .position(x, y) and/or .size(w, h)
    .parent(&container)                 // required
    .enabled(true).visible(true).tab_stop(true)
    .font(Font::new("Segoe UI", 10.0).bold())
    .tooltip("hint")
    /* type-specific options */
    .build()?
```

Exceptions without a builder (direct constructor): `StatusBar::new(&w)?`, `ToolBar::new(&w)?`, `Rebar::new(&w)?`, `PropertyGrid::new(&parent, Rect)?`, `Dialog::new(..)?`, `Menu::new()?`, `MenuBar::new()?`, `MenuStrip::new()?`, `ContextMenu::new()?`, `TrayIcon::new(..)?`.

Methods common to every `Widget`: `text/set_text`, `bounds/set_bounds`, `set_position`, `set_size`, `client_size`, `show/hide/set_visible`, `set_enabled`, `focus`, `set_font`, `set_foreground/set_background`, `set_tooltip`, `set_context_menu`, `set_cursor`, `invalidate`, `destroy`, `is_alive`, `on_click`, `on_double_click`, `on_mouse_*`, `on_key_down/up`, `on_char`, `on_focus/on_blur`, `on_resize`, `on_destroy`, `on_context_menu`.

## 7. Layout — prefer layouts to fixed coordinates

```rust
let window = Window::builder().title("Layout").size(480, 320).min_size(360, 240).build()?;
let editor = TextBox::builder().multiline(true).parent(&window).build()?;
let ok = Button::builder().text("OK").default(true).parent(&window).build()?;
let cancel = Button::builder().text("Cancel").cancel(true).parent(&window).build()?;

let buttons = StackLayout::horizontal().spacing(8);
buttons.add_with(Spacer, LayoutParams::new().weight(1.0));          // pushes the buttons to the right
buttons.add_with(&ok, LayoutParams::new().width(90));
buttons.add_with(&cancel, LayoutParams::new().width(90));

let root = StackLayout::vertical().spacing(8).padding(Thickness::all(12));
root.add_with(&editor, LayoutParams::new().weight(1.0));            // takes the remaining space
root.add_with(buttons, LayoutParams::new().height(28));             // layouts nest
window.set_layout(root);                                            // reapplied on every resize
```

- Controls are passed by reference (`&ok`); layouts and `Spacer` are passed by value.
- Without `weight`, the item uses the `width`/`height` of `LayoutParams` or the control's current size.
- Form grid: `GridLayout::new(&[Track::Auto, Track::Star(1.0)], &[Track::Fixed(26), …]).gaps(8, 8)`; `grid.add(&label, row, 0)`.
- Window with a toolbar/status bar: reserve the height with padding — `DockLayout::new().padding(Thickness::new(0, toolbar.height(), 0, status.height()))`.
- Tab/panel content: `panel.set_layout(..)`.
- After adding items to a layout that is already attached: `window.relayout()`.

## 8. Events — canonical examples

```rust
button.on_click(|| { /* … */ });
text.on_text_changed({ let t = text.clone(); move || println!("{}", t.text()) });
check.on_value_changed(|checked: bool| { /* … */ });
combo.on_selection_changed(|index: Option<usize>| { /* … */ });
slider.on_value_changed(|value: i32| { /* … */ });
list_view.on_selection_changed(|rows: Vec<usize>| { /* … */ });
tree.on_selection_changed(|item: Option<TreeItem>| { /* … */ });
tabs.on_selection_changed(|index: usize| { /* … */ });
grid.on_cell_edited(|row, column, text: &str| { /* … */ });
window.on_close_request(|| true);         // return false to prevent closing
window.on_resize(|size: Size| { /* … */ });
window.on_drop_files(|files| { /* &[PathBuf]; requires .accept_files(true) */ });
list.on_row_reorder_request(|from, to| true); // ListView/DataGrid; return false to refuse
tree.on_item_drop_request(|item, position: TreeDropPosition| true);
panel.on_mouse_down(|e: &MouseEvent| { /* e.position, e.button */ });
panel.on_key_down(|e: &KeyEvent| { if e.key == Key::ESCAPE { /* … */ } });
```

State shared between handlers:

```rust
let total = Rc::new(Cell::new(0u32));
let (counter, label) = (total.clone(), label.clone());
button.on_click(move || {
    counter.set(counter.get() + 1);
    label.set_text(&counter.get().to_string());
});
```

Removal: `let id = w.on_click(..); w.remove_handler(id);`. Details: `docs/EVENTS.en.md`.

## 9. Threads and slow operations

```rust
// Recommended pattern: async without an external runtime.
let (label, progress) = (label.clone(), progress.clone());
spawn_local(async move {
    progress.set_marquee(true);
    let text = run_blocking(|| std::fs::read_to_string("large.txt")).await; // runs on another thread
    progress.set_marquee(false);
    match text {
        Ok(content) => label.set_text(&format!("{} bytes", content.len())),
        Err(error) => MessageBox::error("Error", &error.to_string()),
    }
})?;
```

- The `run_blocking` closure must be `Send`: **do not capture controls in it** — capture only data; use the controls after the `.await`.
- From any thread: `let ui = app.dispatcher(); ui.post(move || …)` (the closure runs on the UI; it cannot capture controls created outside it either — pass data and reach the controls some other way, or use `spawn_local`).
- Pause without blocking: `sleep(Duration::from_millis(100)).await`.

## 10. Common recipes

**Modal dialog with a result**

```rust
let dialog = Dialog::new(&window, "Rename", 320, 110)?;
let field = TextBox::builder().bounds(12, 12, 296, 24).parent(dialog.window()).build()?;
let ok = Button::builder().text("OK").bounds(212, 70, 96, 28).default(true).parent(dialog.window()).build()?;
let handle = dialog.clone();
ok.on_click(move || handle.close_with(DialogResult::Ok));
if dialog.show_modal()? == DialogResult::Ok { let _new_name = field.text(); }
```

**Menus with shortcuts**

```rust
let file = Menu::new()?;
file.add_item_with_shortcut("&Open...", Shortcut::ctrl(Key(b'O' as u16)), || { /* … */ })?;
file.add_separator()?;
let target = window.clone();
file.add_item("E&xit", move || target.close())?;
let bar = MenuBar::new()?;
bar.add_menu("&File", &file)?;
window.set_menu(&bar)?;            // build the menus BEFORE set_menu
```

**Drawn menu bar (follows the theme)**

```rust
// Same `Menu`s; the band lives in the non-client area (client area and layouts do not change).
file.add_radio_item("&List", true, || { /* … */ })?;          // group: consecutive radio items
edit.on_open({ let paste = paste.clone(); move |_| paste.set_enabled(has_text()) }); // before opening
let strip = MenuStrip::new()?;
strip.add_menu("&File", &file)?;
strip.add_menu_with_icon("&Tools", &icons::TOOLS, &tools)?;
window.set_menu_strip(&strip)?;    // item shortcuts work right away, including items added later
Theme::dark().apply(&window);      // the bar and the lists follow; or strip.set_style(Some(MenuStyle::dark()))
context.set_drawn(true);           // context menu with the same drawing
```

**Command bar (icons, menus, search)**

```rust
let bar = CommandBar::builder().parent(&window).build()?;            // ordinary control: reserve bar.height()
bar.add_button(CommandButton::new("Back").icon(&icons::BACK).icon_only().on_click(|| { /* … */ }))?;
bar.add_button(CommandButton::new("Home").icon(&icons::HOME).split(&pages).on_click(|| { /* … */ }))?;
bar.add_separator()?;
bar.add_button(CommandButton::new("Tools").icon(&icons::TOOLS).menu(&tools))?; // ▾ opens the Menu
bar.add_spacer()?;                                                   // whatever comes next goes to the right
let search = bar.add_search("Search", 220, |text| { /* Enter */ })?; // TextBox; Esc clears
let root = StackLayout::vertical();
root.add_with(&bar, LayoutParams::new().height(bar.height()));
```

**Open and save a file**

```rust
if let Some(path) = OpenFileDialog::new().filter("Text", "*.txt").filter("All", "*.*").show(&window)? {
    editor.set_text(&std::fs::read_to_string(&path).map_err(|e| GuiError::InvalidArgument(e.to_string()))?);
}
if let Some(path) = SaveFileDialog::new().default_extension("txt").filter("Text", "*.txt").show(&window)? { /* write */ }
```

**Table**

```rust
let list = ListView::builder().grid_lines(true).parent(&window).build()?;
list.add_column("Name", 200, ColumnAlign::Left);
list.add_column("Size", 90, ColumnAlign::Right);
list.add_row(&["a.txt", "12"]);
let sorter = list.clone();
list.on_column_click(move |column| sorter.sort_by_column(column, true));
```

**Large / editable grid**

```rust
let grid = DataGrid::builder().editable(true).parent(&window).build()?;
grid.add_column("Key", 160, ColumnAlign::Left);
grid.add_column("Value", 160, ColumnAlign::Left);
grid.set_rows(vec![vec!["a".into(), "1".into()]]);
// or, for millions of rows: grid.set_virtual_source(1_000_000, |row, column| format!("{row}:{column}"));
```

**Custom drawing**

```rust
let canvas_panel = Panel::builder().background(Color::WHITE).parent(&window).build()?;
canvas_panel.on_paint(|canvas| {
    let size = canvas.size();
    canvas.line(Point::new(0, 0), Point::new(size.width, size.height), &Pen::new(Color::RED, 2));
    canvas.text("Hello", Point::new(10, 10), &Font::system(), Color::BLACK);
});
// To redraw after changing the state: canvas_panel.invalidate();
```

**Theme**

```rust
Theme::system().apply(&window);                     // follows light/dark/high contrast
let target = window.clone();
window.on_theme_changed(move || Theme::system().apply(&target));
```

**Tray application**

```rust
app.set_quit_on_last_window_closed(false);
let tray = TrayIcon::new(&Icon::system(SystemIcon::Application)?, "My app")?;   // keep `tray` alive
let menu = ContextMenu::new()?;
menu.add_item("Exit", Application::quit)?;
tray.set_menu(&menu);
```

**Chart, SVG and animation**

```rust
let chart = Chart::builder()
    .kind(ChartKind::Bar)
    .categories(&["Q1", "Q2", "Q3"])
    .series(ChartSeries::new("Sales", &[12.0, 18.0, 9.0]))
    .parent(&window)
    .build()?;
chart.set_values(0, &[14.0, 20.0, 11.0]);                        // animated transition
let logo = PictureBox::builder().size(96, 96).parent(&window).build()?;
logo.set_svg(SvgImage::parse(r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="teal"/></svg>"#)?);
motion::animate_bounds(&logo, Rect::new(200, 40, 96, 96), Duration::from_millis(250), Easing::EaseOut)?;
```

**Modern controls and frameless window**

```rust
let bar = TitleBar::attach(&window)?;                              // removes the system frame
let host = DockHost::builder().parent(&window).build()?;
let root = StackLayout::vertical();
root.add_with(&bar, LayoutParams::new().height(bar.height()));
root.add_with(&host, LayoutParams::new().weight(1.0));
window.set_layout(root);
let files = host.add_pane("Files", DockSide::Left)?;               // children in files.content()
let wifi = ToggleSwitch::builder().text("Wi-Fi").parent(&host.center()).build()?;
wifi.on_toggled(|on| println!("{on}"));
let quantity = NumberBox::builder().range(0.0, 99.0).parent(&host.center()).build()?;
```

**Icons**

```rust
// Constants of `arqrustui::icons` (`icons` feature, on by default): only the icons used go into the
// executable in release with LTO. Every method that shows an icon accepts `&impl IconSource`
// (an `IconAsset` from the pack or an `Icon`) and loads the right resolution for the control's DPI.
let save = Button::builder().text("&Save").icon(&icons::SAVE).parent(&window).build()?;
let folder = tree.add_root("Projects")?;
tree.set_item_icons(folder, &icons::FOLDER, &icons::FOLDER_OPEN)?;   // closed / selected
list.set_row_icon(0, &icons::JSON_FILE)?;
grid.set_icon_provider(Some(|row: usize| (row % 2 == 0).then_some(&icons::FILE)))?; // virtual source
let kinds = ComboBox::builder().with_icons(true).parent(&window).build()?;
kinds.add_item_with_icon("Python", &icons::PYTHON)?;
canvas.draw_icon(&icons::WARNING, Point::new(8, 8), 32);              // inside on_paint
```

### Text formatted by runs
Use StyledText::new(font, points), push(text, TextFormat) and RichTextBox::set_rtf(document.to_rtf()). Unicode serialization and RTF tables belong to the library. Complete example with native dialogs: examples/styled_text.rs. Do not build RTF syntax by hand in the application.

### Reusable composition
Use StatusBarLayout to reserve the real status bar and fit_height after showing/hiding content. Use ScrollViewport for scrollable vertical content: viewport in the layout, controls in content, measurement in layout_content. Use StyledText::push_mapped to highlight by Unicode position. Reference: docs/COMPOSITION.md (Portuguese); standalone example: examples/composition.rs. Messages, icons and highlight criteria belong to the application.

`StatusBarLayout::fit_height` only resizes windows in the normal state. Maximized, minimized or full screen, it preserves the geometry and updates limits/layout; after restoring, call it again to adjust the height. Maximize and footer regressions: `tests/window_maximize.rs`, run on Windows at 144 DPI.

### Visual highlighting in editors
Use `RichTextBox::apply_highlights(&base, &[(utf16_range, TextFormat)])` to apply styles without replacing the content, moving the selection or adding undo steps. CRLF counts as one position in RichEdit; characters outside the BMP take two UTF-16 units. The application is responsible for analyzing the text, for the palette and for discarding stale results of asynchronous tasks. The library knows no data formats. Example: `examples/rich_highlight.rs`; native test: `tests/rich_highlight.rs`.

### Button alignment
Use `Button::builder().content_alignment(TextAlign::Right)` to align the icon/text group to the right. `icon_position` only controls the relative order, not the group's alignment. The usual minimum inner margin is 8 logical units and the text gets an ellipsis when it does not fit. The default stays centered. Example: `examples/button_alignment.rs`; native test: `tests/button_alignment.rs`.

## 11. Common code generation mistakes — avoid them

| Wrong | Right |
|---|---|
| Creating controls before `Application::new()` | `Application::new()?` is the first line of `main` |
| Forgetting `.parent(&window)` | every control has a parent |
| Using a control inside the closure without cloning (`move` takes the original) | `let c = c.clone();` before the closure |
| `button.on_click(\|\| label.set_text(..))` capturing by reference | closures are `'static`: use `move` with clones |
| `std::thread::spawn(move \|\| label.set_text(..))` | does not compile; use `spawn_local`/`run_blocking`/`dispatcher` |
| `thread::sleep` on the UI to "wait" | `Timer::once`, or `sleep(..).await` in `spawn_local` |
| Dropping the `Timer` from `Timer::interval` right after creating it | keep the value as long as you want the ticks |
| Dropping the `TrayIcon` | the icon disappears; keep it alive until the end |
| Multiplying coordinates by DPI | the library already scales |
| Expecting `on_value_changed` after `set_value` | events only for user actions; call your logic directly |
| Treating a dialog's `Ok(None)` as an error | it means "cancelled" |
| `unwrap()` on GUI calls | propagate with `?` (`main` returns `GuiResult<()>`) |
| Adding GUI/Win32 crates (`windows`, `winapi`, `winit`…) | forbidden: everything goes through ArqRustUI |
| Positioning everything with fixed coordinates in a resizable window | use layouts |
| Forgetting `window.show()` | windows are born hidden |
| `icons::find("folder")` for a fixed icon | `icons::FOLDER` (`find`/`all` embed the whole pack) |
| `add_item_with_icon` on a regular `ComboBox`/`ListBox` | create it with `.with_icons(true)` (owner drawing is set at creation) |
| Loading an icon with the size multiplied by DPI | pass the logical size; `icon_for(&widget, 16)` or let the control pick the resolution |
| Expecting the `.svg` to be drawn | Win32 does not draw SVG: use the `.ico`/`.png`; `svg_bytes`/`save` only export |

## 12. When the library lacks a feature

1. Look in `docs/COMPONENTS.en.md`, in rustdoc and in `examples/`.
2. Check `docs/FEATURE_MATRIX.en.md` and the limitations in `docs/COMPATIBILITY.en.md`.
3. See whether it can be **composed** from what exists (e.g. custom control = `Panel` + `on_paint` + events — `docs/CUSTOM_CONTROLS.en.md`, level 1).
4. If it is really missing, **implement it in ArqRustUI in a generic way** (section 13) and use the new public API in the project. Do not work around it with `unsafe`/FFI spread across the application.
5. Only as a last resort and in a targeted way: `widget.handle()`, `on_raw_message` and `arqrustui::ffi` (advanced API), with a `// SAFETY:` comment.

## 13. Extending ArqRustUI (for agents that change the library)

### Organization

```
src/ffi/            Win32 declarations per DLL — only types, constants and extern
src/core/           Application, Window, WidgetCore/Widget, events, loop, handles, errors
src/core/native.rs  helpers that wrap recurring unsafe (send, send_ref, send_mut, send_text…)
src/controls/       one file per control; common.rs has the widget_type!, builder_common!, plain_builder! macros
                    large controls are split into submodules by topic, each with its own `impl Control { .. }` and `use super::*`
                    (datagrid/, commandbar/, menus/popup/, menus/strip/, accessibility/tree/,
                    system/notifications/); helpers called by the parent are `pub(super)`
src/layout/         engine.rs = pure computation (testable); mod.rs = layout types
src/{menus,dialogs,graphics,themes,system,input,accessibility,i18n,utilities}/
src/icons/          IconAsset, IconCategory, find/all; catalog.rs = one icon!(..) line per icon
src/controls/icon_support.rs  the control's icon (WidgetIcon) and managed image lists (ManagedImages)
src/controls/modern.rs  base of the drawn modern controls (theme palette, accessible model through closures)
src/core/motion.rs  animations (Easing, animate, transitions) driven by a single Timer
src/graphics/vector.rs  floating-point figures/paints/strokes drawn by GDI+; svg.rs + svg/{xml,parse}.rs
src/input/{gestures.rs, ime.rs + ime/candidates.rs, drag.rs + drag/{source,target,reorder}.rs}
crates/arqrustui-icons/        facade: re-exports the data crates (optional dependency of the `icons` feature)
crates/arqrustui-icons-dataN/  .ico/.png/.svg with include_bytes!, up to 9 MB per crate (crates.io limit)
assets/icons/       README (origin, processing, how to add icons) and preview.png
.github/workflows/ci.yml  CI: fmt, clippy, doc, tests on Windows (MSVC/GNU), MSRV, docs.rs, package size
examples/  tests/  docs/
```

Full architecture: `docs/ARCHITECTURE.en.md`. Template for a new control: `docs/CUSTOM_CONTROLS.en.md` (level 2).

### Inviolable rules

1. **No third-party dependencies.** Currently all sections are empty: only std and the library's own implementations. Any ArqRust dependency must be one of the suite's own, local and in a sibling folder, using `path = "../ArqRustName"`. The rule covers development, build, optional, platform and transitive dependencies. Do not use a registry, Git, copied third-party code or circular dependencies. See [BIBLIOTECA.en.md](BIBLIOTECA.en.md).
2. **Only native, documented Windows APIs**, declared by hand in `src/ffi/` with Microsoft's names. No GTK/Qt/WebView/HTML, and no `windows`/`winapi`/`winit` crates.
3. **Generic and decoupled.** No name, type, text, flow or business rule of a consuming application. A need specific to one project becomes a **general** feature (parameters and callbacks), useful to any application. Examples and tests do not mention consumers either.
4. **Isolated, justified `unsafe`.** It stays in `ffi/`, `core/native.rs` and minimal blocks; every block has a `// SAFETY:` explaining why it is valid; every `pub unsafe fn` has a `# Safety` section.
5. **No manual `Send`/`Sync`** on types bound to windows.
6. **No pointer to Rust state stored in Windows** beyond the duration of a synchronous call.
7. **Correct ownership per handle type** (table in `docs/MEMORY_AND_SAFETY.en.md`); every new native resource gets a RAII wrapper or goes into an existing one.
8. **Explicit errors**: `GuiResult`; never swallow failures with default values; missing feature ⇒ `GuiError::Unsupported`.
9. **APIs that do not exist on every Windows 10/11** are resolved in `ffi/dynamic.rs` (`GetProcAddress`), never imported statically.
10. **Public API in logical units**, documented (`#![warn(missing_docs)]`), without `HWND`/`WPARAM`/`LPARAM` in signatures (except an explicit advanced API).
11. **No panic crosses into Windows.** Every `extern "system"` function with a body (COM method, window procedure, callback) starts with `crate::core::ffi_guard::guard_hresult(move || { .. })` (COM) or `guard(failure_value, move || { .. })`; the `every_system_entry_point_is_guarded` test fails if one is missing.
12. **No placeholders**: no empty functions, `todo!()` or features announced without implementation and tests.

### Conventions

- Type/method names and code documentation (rustdoc, including macros and published examples) in English; guides in Portuguese with an English version next to them (`*.en.md`); messages for developers (`GuiError`, `expect`, `#[deprecated]`) and the texts and comments in `examples/` in English; texts shown to end users only through the `LibraryText` table (pt and en), never hard-coded.
- Control = `Clone` handle with `core: Rc<WidgetCore>` + `XBuilder` + `X::builder()`.
- Events: `on_<event>(impl Fn(..) + 'static) -> HandlerId`, forwarding to `core.on(EventKind::…, …)`; parent notifications handled in `core.set_reflect(..)`.
- Event name (details in `docs/EVENTS.en.md`, "Event names"): present-tense verb for a direct user action (`on_click`, `on_item_click`); participle for a notice after the fact (`on_value_changed`, `on_toggled`, `on_closed`); `on_<action>_request` with a `-> bool` callback for a request that can be refused (`on_close_request`). Noun from the control's vocabulary (`item` in `ListBox`/`TreeView`, `row` in `ListView`/`DataGrid`). Never rename an existing event without keeping the old name as a `#[deprecated]` alias.
- Every new output `pub struct` (events, dialog results, information the library returns and users only read) gets `#[non_exhaustive]`; if users also need to build it, provide a constructor (like `ImeCandidates::new`). Value types (`Color`, `Rect`, `Thickness`…) and styles built with `..Default::default()` do not.
- Every new `pub enum` gets `#[non_exhaustive]`, except a set that is closed by nature (like `Orientation`, `CheckState`, `TreeDropPosition`); inside the library this changes nothing, but examples and tests (which are other crates) need a `_` arm when they `match`.
- Setters do not fire user events.
- Icons: public methods accept `&impl IconSource`; controls with a per-item image use `ManagedImages` (one slot per distinct source, rebuilt when the DPI changes).
- Formatting: the repository's `rustfmt.toml` (`max_width = 120`).

### Definition of done for any change

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets   # zero warnings
cargo test                      # all passing (creates real windows)
cargo build --release
```

The same runs in CI. A change visible to library users goes into `CHANGELOG.md` (`[Não publicado]` section) and `CHANGELOG.en.md` (`[Unreleased]`). Besides that: a new test (unit for pure logic; integration in `tests/` for windows), an example or a use in `examples/demo.rs` when applicable, and updates to `docs/COMPONENTS.md`, `docs/FEATURE_MATRIX.md` and this file. The guides have an English version next to them (`*.en.md`): `AGENTS`, `BIBLIOTECA`, `CHANGELOG` and, in `docs/`, `ARCHITECTURE`, `COMPATIBILITY`, `COMPONENTS`, `CUSTOM_CONTROLS`, `EVENTS`, `FEATURE_MATRIX`, `MEMORY_AND_SAFETY` and `RELEASING`. Every change to one of these files is also made in the other version, in the same piece of work, with the same content.

## 14. Current state and caveats

- Local validation on 2026-11-16: strict Clippy and English rustdoc without
  warnings; 60 doctests and five GDI tests passed. Full suite: 356 passed,
  32 failures matching the previous log, one print dialog excluded for a prior
  hang. Demos inspected at default/minimum sizes at 150%.
  See `docs/CORRECOES_DEMOS_E_RUSTDOC.md` (Portuguese); counts below are historical.
- Version 0.1.0. 145 unit tests + 176 integration + 60 doctests, on 64 and 32 bits; under Wine 9/Xvfb all pass except `window_maximize` (requires a window manager that maximizes).
- **Running was validated under Wine 9 (96 and 144 DPI), not on real Windows** (except two composition items recorded as tested on Windows by earlier work; see section 4.14 of `docs/FEATURE_MATRIX.en.md`). When working on a Windows machine, run `cargo test` and `cargo run --example demo` right at the start and record any difference; the list of what most needs checking is in `docs/COMPATIBILITY.en.md`.
- Partial features or features only validated on Wine (do not present them as complete or as "tested on Windows"): drag and drop (shell drag image and dropping into other programs not observed), system notifications (`SystemNotification` with fields, progress, scheduling and the COM activator: implemented, not observed — Wine does not have the feature), UIA (patterns checked by calling the providers, never with Narrator/NVDA), printing (preview tested; printing on paper not), IME (candidate list with simulated candidates), touch (gestures with synthetic messages; no touch screen), SVG (subset; Wine does not smooth paths in images), `TitleBar` (DWM shadow, maximizing and Windows 11 snap layouts not observed), `DockHost` and detachable tabs (no docking targets while dragging), dark theme (scroll bars depend on Windows 10 1809+), icon on a native button (without Common Controls v6 the button is drawn by the library instead), `MenuStrip`/`CommandBar`/modern controls (validated only on Wine), `Modal` (on Wine the backdrop is a dimmed snapshot of the window, because Wine without a compositor does not compose per-pixel transparency; the real transparency, the shadow over the live window and the Windows 11 corners were not observed). Details in `docs/FEATURE_MATRIX.en.md` and `docs/COMPATIBILITY.en.md`.
