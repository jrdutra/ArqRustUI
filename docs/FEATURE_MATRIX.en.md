# Feature matrix

*Português: [FEATURE_MATRIX.md](FEATURE_MATRIX.md).*

Local Windows validation, 2026-11-16: English rustdoc generated without warnings;
strict Clippy with `-D warnings` passed. All five GDI tests passed, including
single GDI/GDI+ scaling at 96/144/192 DPI and transform/clip preservation.
Full suite: 356 passed, 32 failed, one print dialog test excluded due to a
previously observed hang. The 32 failures exactly match the previous
`target/bevel-import/test-final.log`; historical counts below do not replace
this run. Screenshots and details:
[CORRECOES_DEMOS_E_RUSTDOC.md](CORRECOES_DEMOS_E_RUSTDOC.md), in Portuguese.

States, in increasing order:

| State | Meaning |
|---|---|
| Not started | no code |
| In progress | partial coverage; the notes column says what is missing |
| Implemented | complete code, without its own automated test |
| Tested | implemented + automated test passing |
| Documented | tested + public documentation (rustdoc and guides) |
| Done | documented + working example + error handling + no external dependencies |

**Tested on Windows** (two items, in section 4.14): besides the automated test, the item was checked on real Windows. The record comes from work earlier than this revision and has not been repeated by CI yet.

> **Caveat for the whole table:** "test passing" means running under Wine 9 (96 and 144 DPI), not on real Windows. Items whose check depends on real Windows are marked with ⚠ and detailed in [COMPATIBILITY.en.md](COMPATIBILITY.en.md). No item uses external crates.

## 4.1 Window management

| Feature | State | Notes |
|---|---|---|
| Creation/destruction, main and secondary windows | Done | `tests/lifecycle.rs`, examples 1 and 18 |
| Modal and modeless | Done | `Window::show_modal`, `Dialog` |
| Child windows / parent and child / owner | Done | `Panel`, `WindowBuilder::owner` |
| Borderless, resizable/fixed, always on top | Done | example 18 |
| Transparency (window opacity) | Done | `set_opacity`; no per-pixel transparency |
| Full screen, maximize, minimize, center | Done | |
| Multiple monitors | Documented ⚠ | `Monitor`, `move_to_monitor`; tested with one monitor |
| Application and window icons | Done | `Icon::system`, `Icon::from_file` |
| Default window logo | Tested | `icons::DEFAULT_WINDOW_ICON`, available without the optional catalog; the library's windows and dialogs; an explicit icon in the builder or in `set_icon` takes precedence. Three native tests on Windows x64 in `tests/default_window_icon.rs`, repeated with and without `icons`; simulated DPI, no real monitor change |
| Title bar, system menu | Done | `system_menu()` |
| Custom client area | Done | `on_paint`, `on_raw_message` |
| Close with veto, size/move events | Done | |
| DPI scales | Done | whole suite repeated at 144 DPI |
| DPI change at run time | Documented ⚠ | simulated `WM_DPICHANGED` (`tests/window_maximize.rs`, `tests/system_events.rs`): suggested rectangle, rescaling, window icon and `on_dpi_changed`; a real DPI change between monitors was not seen |
| Focus, keyboard shortcuts | Done | |
| Dragging files onto the window | Documented ⚠ | `on_drop_files`; `WM_DROPFILES` with an `HDROP` built the way Explorer builds it (`tests/system_events.rs`); a real drag from Explorer was not seen |
| DWM borders, shadows and effects | Documented ⚠ | visual effect not observed |

## 4.2 Basic components

| Component | State | Notes |
|---|---|---|
| Button, Label, TextBox, PasswordBox | Done | |
| CheckBox (2 and 3 states), RadioButton, ToggleButton | Done | |
| ComboBox, ListBox, GroupBox | Done | |
| ProgressBar, Slider (TrackBar), ScrollBar | Done | |
| SpinButton, UpDown | Done | |
| DatePicker, TimePicker, MonthCalendar | Done | |
| RichTextBox | Done ⚠ | character/paragraph formatting, RTF, search; lists (`ListStyle`), tables and images (EMF) inserted at the cursor, `insert_rtf` — lists and tables shown by Wine's RichEdit (`styled_text` example); the image goes into the RTF (checked) but Wine's RichEdit does not draw it |
| Hyperlink | Documented | mouse click (`NM_CLICK`) and Enter (`NM_RETURN`) simulated, with the link address (`tests/system_events.rs`) |
| Tooltip | Documented | creation/removal tested; appears on its own when the pointer rests on the control (`tests/system_events.rs`) |
| StatusBar, ToolBar | Done | |
| CommandBar (drawn command bar) | Tested ⚠ | icon/text buttons, toggles, `▾` menu, split, separators, flexible space, labels, search, hosted controls (also in `»`), per-button tips, keyboard, drag to reorder, customization box (`customize`, `CommandBarLayout`); validated on Wine (96 and 144 DPI), not on real Windows |
| ToggleSwitch, NumberBox, RatingControl, Card (library's own) | Tested ⚠ | animated switch; number box with limits, step and expressions (`2*3+1`); stars with half ratings; clickable card; keyboard, theme and UIA (Toggle, RangeValue, Value, Invoke) checked on Wine (`tests/modern_controls.rs`) |
| Expander, BreadcrumbBar (library's own) | Tested ⚠ | animated height and ExpandCollapse; trail collapsing into "…" and a menu; synthetic messages on Wine |

## 4.3 Advanced components

| Component | State | Notes |
|---|---|---|
| ListView (columns, sorting, multiple selection, check boxes, images) | Done | |
| TreeView | Done | |
| TabControl | Done | |
| Web-style modal (`Modal`) | Tested ⚠ | layered backdrop (`UpdateLayeredWindow`, per-pixel transparency and soft shadow) over the owner's client area + centered native card; owner blocked, Esc/Enter/✕/click on the backdrop, focus trapped and given back, validation before closing, blocking `show` and `show_then`, follows the owner, animation, dark theme; on Wine the backdrop is a dimmed snapshot (no compositor) |
| Detachable tabs | Tested ⚠ | `set_detachable`, `detach_tab`/`attach_tab`: dragging the tab out moves it to its own window; closing the window gives it back; drag with synthetic messages on Wine |
| Dockable panes (`DockHost`, `DockPane`) | Tested ⚠ | left/right/bottom, draggable splitters, float in their own window, dock, close and reopen; no dropping on docking targets while dragging |
| Charts (`Chart`) | Tested ⚠ | line, area, bars, stacked bars and pie; axis with round steps, legend, highlight and tip, click, animated transition, theme; colors checked pixel by pixel on Wine |
| Print preview (`PrintPreview`, `PrintPreviewDialog`) | Tested ⚠ | pages of a `PrintDocument` at any scale, navigation, zoom, cache; the Print button uses the chosen range — printing itself not exercised (no printer) |
| HeaderControl | Documented | used in the tests; no dedicated example |
| Rebar, Pager | Tested | creation and basic operations; coverage of the essentials |
| HotKeyBox, IpAddressBox | Documented | present in the `demo` example |
| Animation | In progress ⚠ | only the error path was tested (a real AVI is missing) |
| SplitContainer (library's own) | Done | drag simulated in the tests |
| DataGrid (library's own): virtualization, sorting, multiple selection, editing, resizable columns | Done | 1,000,000 virtual rows in the test; real editing checked in `demo` |
| DataGrid: per-cell/row style, check box column, virtual editing (`set_virtual_editor`), editor that follows scrolling, filters, collapsible grouping, CSV/TSV/HTML export | Tested ⚠ | `tests/data_grid.rs`; validated on Wine |
| PropertyGrid (library's own) | Done | drop-down list for `Choice` (`set_choice_mode`), configurable Yes/No labels, accessible selection |
| Images attached to items | Done | `ImageList` in ListView/TreeView/ToolBar |
| Context menus on controls | Documented | `set_context_menu`; `WM_CONTEXTMENU` opens the Windows menu (item chosen with the keyboard in the modal loop) and the drawn one, also from the menu key (`tests/system_events.rs`) |

## 4.4 Menus

| Feature | State | Notes |
|---|---|---|
| MenuBar, MenuItem, SubMenu, separators | Done | |
| Enabled/disabled and checkable items | Done | |
| Keyboard shortcuts | Done | items added after `set_menu` also respond |
| Dynamic menus | Done | `remove`, `clear` |
| Menus with icons | Tested | `MenuItem::set_icon` |
| ContextMenu | Documented | Windows menu or drawn (`set_drawn`) |
| Selection events | Done | click on the item and `Menu::on_highlight_changed` (item highlighted by mouse or keyboard, also on the native bar) |
| System menu | Done | |
| Radio items (groups) | Tested | `add_radio_item` |
| Menu filled when opened | Tested | `Menu::on_open` (native bar, `MenuStrip` and context) |
| Drawn bar (`MenuStrip`) | Tested ⚠ | non-client area, light/dark/high-contrast theme, per-DPI icons, submenus, scrolling, `»`; validated on Wine (96 and 144 DPI), not on real Windows |
| Keyboard on the `MenuStrip` | Tested | Alt, F10, Alt+letter, arrows, Home/End, Enter, Esc, mnemonics; shortcuts respect disabled items |

## 4.5 Dialogs

| Dialog | State | Notes |
|---|---|---|
| MessageBox | Done | title, text, blocked owner and Yes/No answers checked (`tests/dialogs.rs`) |
| OpenFileDialog, SaveFileDialog, FolderPicker | Tested ⚠ | `IFileDialog`; `tests/dialogs.rs` opens it, checks the title and the blocked owner, confirms (path of the existing file / suggested name in the requested folder) and cancels (`None`); appearance not checked on Windows |
| ColorDialog, FontDialog | Tested ⚠ | `tests/dialogs.rs`: OK returns the initial color and the initial font (family, size and color), cancel returns `None` |
| PrintDialog, PageSetupDialog | Tested ⚠ | `show_with(PrintOptions)`: page range, selection, copies (`PRINTDLGW` fields tested); `tests/dialogs.rs` checks that opening and cancelling returns without hanging; no printer in the environment, so the result with a printer was not seen |
| FindDialog / Replace (modeless) | Tested ⚠ | `tests/dialogs.rs`: does not block the owner, shows the initial text, "Find next" arrives as a `FindEvent`, `close` closes it |
| TaskDialog | Tested ⚠ | `tests/dialogs.rs`: title, custom button (`TaskButton::Custom`) and verification check box |
| Custom, modal and asynchronous dialogs | Done | `Dialog` |

## 4.6 Layouts

| Feature | State |
|---|---|
| Vertical/horizontal StackLayout | Done |
| GridLayout (fixed, auto, proportional, spans) | Done |
| FlowLayout, DockLayout, AbsoluteLayout, AnchorLayout, SplitLayout | Done |
| Margins, padding, spacing, alignment | Done |
| Minimum/maximum sizes, proportional sizing | Done |
| Responsive to the window size, automatic update | Done |
| Per-monitor DPI | Done (fixed) / Implemented ⚠ (change at run time) |

## 4.7 Events

| Feature | State |
|---|---|
| `on_click`, `on_double_click`, `on_mouse_move/enter/leave`, `on_key_down/up` | Done |
| `on_text_changed`, `on_focus`, `on_blur`, `on_resize`, `on_move`, `on_close_request` | Done |
| `on_selection_changed`, `on_value_changed` | Done |
| Event naming convention (present / participle / `_request`), old names as `#[deprecated]` aliases (`tests/api_compat.rs`) | Done |
| Public enums are `#[non_exhaustive]` (59 of 63; only `Orientation`, `CheckState`, `TreeDropPosition` are exhaustive) | Done |
| Safe dispatch (reentrancy, destruction, panic) | Done |
| Scheduling on the UI thread, `async` without an external runtime | Done |

## 4.8 Drawing

| Feature | State | Notes |
|---|---|---|
| GDI: lines, rectangles, ellipses, polygons, paths, gradients, bitmaps, text, fonts | Done | pixels checked in tests |
| Clipping regions, transforms, double buffering | Done | |
| GDI+: antialiasing, transparency, images | Done | |
| Floating-point vectors (`Figure`, solid/linear/radial `Paint`, `Stroke`, `Matrix`) | Tested ⚠ | `PlusGraphics::fill_figures`/`stroke_figures`; Wine's GDI+ does not smooth paths inside images and ignores the intermediate stops of radial gradients |
| SVG drawing (`SvgImage`) | Tested ⚠ | subset of SVG 1.1 (shapes, paths with arcs, gradients, `use`, transforms, styles and simple `<style>`); 39 earlier SVGs compared visually with PNGs. The social batch adds 40 sources; BeReal, KakaoTalk and Lemon8 contain unsupported text. No text, masks, clipping or filters |
| Animations and transitions (`motion`) | Tested ⚠ | easing curves, `animate`, `animate_bounds`, `fade_window`, `show_animated`/`hide_animated` (`AnimateWindow`), respects Windows' "Show animations" |
| Direct2D: shapes, paths, gradients, transforms, clipping | Done | example 16 rendered |
| DirectWrite (text) | Implemented ⚠ | calls confirmed; no fonts in the environment |
| Backend selection | Done | three independent types |
| Custom fonts from a file | Tested | error path tested |

## 4.9 Appearance

| Feature | State | Notes |
|---|---|---|
| Light and dark theme, custom colors and fonts | Done | example 12 |
| Detection of the system theme and its changes | Documented ⚠ | reading tested; `WM_SETTINGCHANGE` with `ImmersiveColorSet`/`WindowsThemeElement` fires `on_theme_changed`, other areas do not (`tests/system_events.rs`); a real change on Windows was not seen |
| Reusable styles, text scale | Done | |
| Hover/pressed/focused/disabled states | Done | `ButtonStyle` (buttons) |
| Native icons | Done | |
| Modern look, corners and shadows | Documented ⚠ | depends on Windows 11 |
| Dark theme on every control | Tested ⚠ | tabs, status bar, headers, group box and 3D borders painted by the library; toolbar through `NM_CUSTOMDRAW`; calendar, drop-down calendar, progress bar and rebar through color messages; DatePicker/TimePicker and Slider repainted; native `MenuBar` replaced by the themed `MenuStrip` (`set_menu_bar_follows_theme`); scroll bars through `DarkMode_Explorer` (Windows 10 1809+ only; not observed on Wine) |
| Frameless window with its own title bar (`TitleBar`) | Tested ⚠ | `WM_NCCALCSIZE`/`WM_NCHITTEST` (borders, dragging, `HTMAXBUTTON` for Windows 11 snap layouts), controls on the bar, accessible buttons; DWM shadow and maximizing not observed (Wine/Xvfb) |

## 4.10 Windows integration

| Feature | State | Notes |
|---|---|---|
| Tray, icons, notifications | Documented ⚠ | example 14 |
| System notifications (Action Center, `SystemNotification`) | Implemented ⚠ | hand-written WinRT: title, text, images, up to 5 buttons, text and selection fields, updatable progress bar, scheduling (`show_at`), groups and tags (`remove_group`, `clear_all`), scenarios, COM activator for clicks with the program closed (`NotificationActivator`); XML, IIDs and registration tested; Wine has no `Windows.UI.Notifications` (returns `Unsupported`), display not observed |
| In-window notices (`InfoBar`, `Toast`) | Tested ⚠ | four severities, actions, stacking, expiry with pause, theme; validated on Wine (fading not observed: Wine without a compositor does not show translucent windows) |
| Clipboard (text, HTML, images, files, custom formats) | Tested | several formats at once; round trip checked in the tests (pixels, HTML, `CF_HDROP`); images also as `CF_DIBV5` (alpha) and `PNG`; metafiles (`Metafile`, `CF_ENHMETAFILE`); `ClipboardWatcher` |
| Drag and Drop (OLE) | Tested ⚠ | `start_drag`, `accept_drops`, `DropData`; reordering in `ListBox`/`ListView`/`DataGrid` (also virtual)/`TreeView`/`CommandBar`, several items at once, moving out (`DropEffect::Move`); validated on Wine with a real mouse (example 25). Shell drag image and dropping into other programs not observed |
| Opening files, associations, Explorer, known folders | Tested | `system::shell` |
| Standard and custom cursors | Tested | |
| Monitors, resolution, DPI | Done | |
| Accessibility information | Done | |
| Printing | Implemented ⚠ | `PrintDocument` (A4/Letter, `with_page_setup`), `PrintRange`, `print_document` with range and copies, preview; real printing not exercised |
| Keyboard and mouse | Done | |
| Touch | Tested ⚠ | pointer events and gestures (`gestures::on_gesture`): tap, double tap, hold, pinch, rotation, drag with inertia; `WM_GESTURE` on Windows 7; Ctrl+wheel as pinch; recognizer tested with synthetic messages (no touch screen) |
| IME | Tested ⚠ | the library's own candidate list (`Ime::use_candidate_window`, `ImeCandidateWindow`, `on_candidates_changed`) with theme and announcement; simulated candidates — no IME in the environment |

## 4.11 Accessibility

| Feature | State | Notes |
|---|---|---|
| Native control features (UIA/MSAA) | Done | provided by Windows |
| Keyboard navigation, tabbing, mnemonics | Done | |
| Accessible names and descriptions | Documented ⚠ | `IAccPropServices`; not supported by Wine |
| UIA provider for custom controls | Tested ⚠ | `set_accessible_model` (`AccessibleModel`, `AccessibleItem`): role, state and the Invoke, Toggle, ExpandCollapse, Value, RangeValue, Selection/SelectionItem, Scroll, Grid/GridItem and Text patterns; `IRawElementProviderAdviseEvents`; `raise_accessible_event`; `set_custom_role` remains for the simple case |
| UIA elements of the drawn controls (`MenuStrip`, menu lists, `CommandBar`, `PropertyGrid`, `DataGrid`, modern controls, `TitleBar`, `DockPane`) | Tested ⚠ | fragments with Invoke, Toggle, ExpandCollapse and Value, focus and events; checked by calling the providers and with Wine's `IUIAutomation` client; not tested with Narrator/NVDA |
| Screen reader announcements | Documented ⚠ | `announce`; `Toast`, `InfoBar` and the IME candidate list announce on their own; not heard with Narrator/NVDA |
| High contrast, text scale, focus indicators | Done | |

## 4.12 Internationalization

| Feature | State |
|---|---|
| Unicode, UTF-8 ↔ UTF-16 | Done |
| Regional formatting (dates, times, numbers, currency) | Done |
| Configurable texts (`Translations`) | Done |
| Translatable library texts (`LibraryText`, `set_library_language`, `apply_library_translations`) | Done |
| Text direction (RTL) | Tested (detection) / Implemented (mirrored window) |

## 4.13 Icons

| Feature | State | Notes |
|---|---|---|
| Embedded pack (392 icons in 22 categories, ICO/PNG, 79 SVG), constants, search by name/alias, categories | Done | `tests/icons.rs`, example 21; every icon loaded at 4 sizes and the PNG decoded in a test |
| Social networks (58 designs) | Integrated | Bevel update: 22 designs replaced, 18 added, with ICO/PNG 512/SVG; 18 earlier designs kept. Catalog and loading tests; origin, credits and SVG limits in `assets/icons/social/ORIGEM.md` (Portuguese) |
| Icon by file type (`for_extension`, `for_path`; 66 extensions with their own design, 92 extensions recognized) | Done | unit test of the table; checked in a `ListView` at 16, 32 and 48 px on Wine |
| `Icon` from an in-memory `.ico`, disabled version, per-DPI cache | Done | |
| In-memory PNG (`PlusImage::from_bytes`, `to_icon`) | Done | |
| Button with icon | Done ⚠ | icon at 60% of the height and a centered icon + text group, drawn over the native button in `NM_CUSTOMDRAW` (v6); without v6 the button is drawn by the library; both paths tested under Wine |
| Single default height and centered text | Done ⚠ | `DEFAULT_CONTROL_HEIGHT` on buttons, combo boxes, fields, pickers and check boxes; non-client bands center single-line text |
| Label with icon (`SS_OWNERDRAW`) | Done | pixels checked in a test |
| `PictureBox` (scaled icon/image) | Done | |
| Tree, list, grid (in-memory and virtual), tabs | Done | managed image lists, one slot per source |
| ComboBox / ListBox with icons (owner-drawn) | Done | require `with_icons(true)` at creation |
| Toolbar, status bar, menus, window, tray, TaskDialog | Done ⚠ | tray and TaskDialog with a custom icon not observed visually |
| Reloading icons when the DPI changes at run time | Documented ⚠ | the window icon is set again on `WM_DPICHANGED` (`tests/system_events.rs`); control and menu icons at an actually different DPI were not seen |
| SVG drawing | Tested ⚠ | `IconAsset::svg_image`, `PictureBox::set_svg`, `Canvas::draw_svg` (see 4.8) |

## 4.14 Composition and formatted text

| Feature | State | Details |
|---|---|---|
| Text formatted by runs | Documented | StyledText: Unicode, font, color, highlight and styles; serialization tests and native RichEdit; styled_text example. |
| Container redrawing | Implemented | invalidate includes descendants and background erasing; asynchronous painting. |
| Footer and content-based height | Tested on Windows | StatusBarLayout, native measurement and fit_height; preserves maximized, minimized and full-screen states; composition and window_maximize tests (144 DPI). |
| Viewport with vertical scrolling | Documented | ScrollViewport: limits, responsive measurement, focus and lifetime; composition tests. |
| Formatting by Unicode position | Documented | StyledText::push_mapped; grouping of adjacent styles and a Unicode test. |
| Visual styles by range (`RichTextBox::apply_highlights`) | Tested on Windows | Preserves text, selection, scrolling, modified flag and undo history; does not raise edit events; UTF-16 ranges and a language-independent API |

## 4.15 Button content

| Feature | State | Details |
|---|---|---|
| Horizontal alignment of the icon/text group | Tested | `content_alignment`, `set_content_alignment`; left/center/right, inner margin and truncation; native and custom drawing |

## Quality

| Item | State |
|---|---|
| Zero third-party dependencies (`Cargo.lock` with only `arqrustui` and the icon pack's own crates) | Done |
| crates.io packages: `arqrustui` + `arqrustui-icons` + 8 data crates (10 MiB limit per package) | Checked locally | Ten packages; the social import added part 8. Publishing not run; see the result of the update in `docs/REDES_SOCIAIS_BEVEL.md` (Portuguese) |
| docs.rs documentation (`[package.metadata.docs.rs]`, target `x86_64-pc-windows-msvc`) | Done — built without warnings with that target on Linux, as docs.rs does |
| CI (GitHub Actions) and `CHANGELOG.md` | Implemented ⚠ — workflow validated as YAML and the package size step run locally; it has not run on GitHub yet (the repository has no remote) |
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets` | clean |
| Tests | 145 unit + 176 integration + 60 doctests; under Wine 9/Xvfb all pass except `window_maximize` (needs a window manager that maximizes), on 64 and 32 bits |
| Targets | x64 (MSVC and GNU) and x86/i686 (whole suite on 32-bit Wine); ARM64 only compiles |
| `unsafe` blocks with `// SAFETY:` | all |
| No panic crosses into Windows (`catch_unwind` in all 128 `extern` functions called by the system) | Done — checked by a test that scans the code |
| Validation on real Windows | **Partial** — two items recorded as tested on Windows (section 4.14); the rest only under Wine. CI (`.github/workflows/ci.yml`) runs the suite on Windows once the repository is published on GitHub |
