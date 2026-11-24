# Architecture

*Português: [ARCHITECTURE.md](ARCHITECTURE.md).*

## Layered view

```
┌────────────────────────────────────────────────────────────────────────┐
│ Application (any Rust project)               use arqrustui::prelude::*;  │
├────────────────────────────────────────────────────────────────────────┤
│ High-level public API                                                  │
│ controls · layout · menus · dialogs · graphics · themes · system       │
│ input · accessibility · i18n                                           │
├────────────────────────────────────────────────────────────────────────┤
│ core — Application, Window, Widget/WidgetCore, events, message loop,   │
│        dispatcher, timers, RAII handles, errors                        │
│ core::native — wrapped Win32 calling patterns (isolated unsafe)        │
├────────────────────────────────────────────────────────────────────────┤
│ ffi — hand-written #[repr(C)] / extern "system" declarations           │
├────────────────────────────────────────────────────────────────────────┤
│ Windows DLLs: user32, gdi32, comctl32, comdlg32, shell32, ole32,       │
│ uxtheme, dwmapi, d2d1, dwrite, gdiplus, imm32, uiautomationcore…       │
└────────────────────────────────────────────────────────────────────────┘
```

Dependency rule: each layer only knows the ones below it. `ffi` contains no logic; the high-level modules do not declare `extern`.

## Module structure

```
src/
├── lib.rs              crate root + prelude
├── ffi/                one DLL per file (types, constants, extern)
│   ├── types.rs        HWND, RECT, GUID, LOWORD/HIWORD helpers…
│   ├── user32.rs  gdi32.rs  kernel32.rs  comctl32.rs  comdlg32.rs
│   ├── shell32.rs ole32.rs  uxtheme.rs   dwmapi.rs    d2d1.rs  dwrite.rs
│   ├── gdiplus.rs imm32.rs  uiautomation.rs
│   └── dynamic.rs      LoadLibrary/GetProcAddress + optional APIs by version
├── core/
│   ├── application.rs  Application, per-thread state, dispatch window
│   ├── event_loop.rs   GetMessage → drawn menus → shortcuts → IsDialogMessage → Dispatch
│   ├── widget.rs       WidgetCore, HWND→core registry, window_proc, Widget trait;
│   │                   widget/dispatch.rs = message dispatch (the `match` that becomes events)
│   ├── window.rs       Window, WindowBuilder, window classes
│   ├── events.rs       Event, EventKind, HandlerId
│   ├── dispatcher.rs   UiDispatcher, spawn_local, run_blocking, sleep
│   ├── timer.rs        Timer
│   ├── motion.rs       animations (Easing, animate, transitions)
│   ├── handles.rs      GdiObject, Icon, ImageList, MemoryDc, WindowDc, GlobalMemory
│   ├── errors.rs       GuiError, GuiResult
│   ├── messages.rs     RawMessage (advanced API)
│   └── native.rs       internal helpers (send, send_ref, styles, DPI)
├── controls/           one file per control or family; modern.rs = base of the modern controls;
│                       the largest have submodules by topic (datagrid/{drawing,editing,export,filtering,
│                       reorder,styling}.rs, commandbar/{access,customize,flyout,layout,painting}.rs)
├── layout/             engine.rs (pure computation) + mod.rs (layouts bound to widgets)
├── menus/              mod.rs (Menu, MenuBar, ContextMenu); popup.rs + popup/{tracking,painting,access}.rs
│                       (drawn menus); strip.rs + strip/{bar,messages}.rs (MenuStrip)
├── dialogs/  themes/
├── system/             clipboard, printing, tray, monitors…; notifications.rs +
│                       notifications/{activator,winrt}.rs (Windows notifications)
├── graphics/           gdi.rs, gdiplus.rs, direct2d.rs, vector.rs, svg.rs + svg/{xml,parse}.rs
├── input/              keyboard, mouse, ime.rs + ime/, gestures.rs, drag.rs + drag/{source,target,reorder}.rs
├── icons/              IconAsset, categories, search; catalog.rs with the 392 icons
├── accessibility/      mod.rs (IAccessible/announcements), model.rs, tree.rs (own UIA tree) +
│                       tree/{provider,patterns,debug}.rs (COM methods, patterns, calls for the tests)
├── i18n/  utilities/
examples/               complete applications (one per area)
tests/                  integration (real windows) + FFI layout validation
docs/                   this documentation
crates/                 arqrustui-icons (facade) + arqrustui-icons-data1..7 (icon bytes)
assets/icons/           origin and processing of the icon pack, preview.png sheet
.github/workflows/      CI (GitHub Actions)
```

## The core: `WidgetCore`

Every native window created by the library — top-level window or control — has exactly one `WidgetCore`:

- it stores the `HWND` (cleared on destruction), the event handlers, font, colors, layout, cursor, context menu, shortcuts and resources kept alive;
- it lives in a **per-thread registry** `HashMap<HWND, Rc<WidgetCore>>`, which keeps it alive while the native window exists;
- the public types (`Button`, `Window`, …) are just `struct { core: Rc<WidgetCore> }` — cloneable *handles*.

### A single window procedure

`core::widget::window_proc` serves **every** window:

- the library's own classes (`ArqRustUI.Window`, `ArqRustUI.Panel`) use it as their `WNDPROC`;
- system controls (`BUTTON`, `EDIT`, `SysListView32`…) are *subclassed* with `SetWindowLongPtrW(GWLP_WNDPROC)`; the original procedure is stored and called through `CallWindowProcW`.

Flow of a message:

1. `window_proc` looks up the `WidgetCore` in the registry (without one → `DefWindowProcW`).
2. The call is wrapped in `catch_unwind`: a panic in a handler never crosses the FFI boundary; it is stored and re-raised by the event loop. The same holds for every other entry point called by Windows (COM methods and callbacks), which run inside `core::ffi_guard::guard`/`guard_hresult` — on a panic Windows receives `E_UNEXPECTED` or a neutral value.
3. `WidgetCore::handle` turns the message into an `Event` and calls the handlers.
4. Notifications that Windows delivers **to the parent** (`WM_COMMAND`, `WM_NOTIFY`, `WM_HSCROLL/VSCROLL`, `WM_DRAWITEM`, `WM_CTLCOLOR*`) are **reflected** to the core of the child control that sent them. This is what lets `button.on_click(..)` work with any container (window, panel, group box, tab).
5. The rest goes on to the original procedure.

### Life cycle

```
build() ──► CreateWindowExW ──► registry.insert(hwnd, core)
                                        │
                 events ◄───────────────┤
                                        │
destroy()/parent destroyed ──► WM_NCDESTROY ──► emit(Destroyed)
                                                registry.remove(hwnd)
                                                core.hwnd = null
                                                handlers, layout, menus, resources released
```

After `WM_NCDESTROY`, the handles the application still holds become **inert**: methods become no-ops, `is_alive()` returns `false`, operations that return `GuiResult` return `GuiError::WidgetDestroyed`. Releasing all handlers at that moment breaks the `Rc` cycles that closures such as `button.on_click(move || button2.…)` create.

## Application and event loop

`Application::new()` (one per UI thread):

1. enables per-monitor DPI v2 (`SetProcessDpiAwarenessContext`, with a fallback);
2. enables Common Controls v6 through an activation context created at run time (manifest embedded in `shell32.dll`), so the application needs no manifest, and only then loads `comctl32.dll` dynamically;
3. initializes OLE/COM (STA);
4. registers the window classes and creates the **dispatch window** (message-only), which receives queued tasks, timers and tray messages.

`Application::run()` runs: `GetMessageW` → menu mode of the `MenuStrip`/drawn menus (`menus::popup::pre_translate`: keyboard and mouse while a menu is open; otherwise Alt, F10, Alt+letter and item shortcuts) → modeless dialogs → window keyboard shortcuts → `IsDialogMessageW` (Tab, arrows, mnemonics, Enter/Esc) → `TranslateMessage`/`DispatchMessageW`. Modal windows run a nested loop with the owner window disabled.

### Dark theme complement

Some Windows controls ignore colors when visual styles are active. `themes::dark` covers those cases without replacing the control: with a dark theme on the window, `WidgetCore::handle` routes the `WM_PAINT`/`WM_PRINTCLIENT` of tabs, status bar, headers (`SysHeader32`, including the ListView's, adopted as a core with `WidgetCore::adopt`) and `GroupBox` to its own painting, which reads the native control's state (items, texts, icons, selection), and paints the 3D border after the default `WM_NCPAINT`. The toolbar gets its colors in `NM_CUSTOMDRAW`; calendar, progress bar and rebar through color messages in classic mode. With a light theme nothing is intercepted and drawing is 100% native again.

### Drawn menus (`MenuStrip`)

The `MenuStrip` band is part of the top-level window's **non-client** area: `WidgetCore` forwards the window's messages to `menus::strip::root_message`, which adds the band's height in `WM_NCCALCSIZE`, draws it after the default `WM_NCPAINT`/`WM_NCACTIVATE` (DC of the whole window, double-buffered) and answers `HTMENU` in `WM_NCHITTEST`. `outer_rect_for_client` includes the band, so `size(..)` is still the client area size.

The lists are `WS_POPUP` windows of the `ArqRustUI.MenuPopup` class, with `WS_EX_NOACTIVATE` and `MA_NOACTIVATE`: the owner window stays active and keeps the focus. There is no modal loop; a single tracker per thread receives keyboard and mouse at the start of the message loop while a menu is open, and is cancelled when the window loses activation, changes size, receives `WM_CANCELMODE` or is destroyed. No `RefCell` stays borrowed during calls that send messages (creating/destroying windows) or during an item's handler, which only runs after menu mode ends.

`CommandBar` reuses the same tracker: a menu button opens its list with `popup::open_dropdown` (anchored below the button), gets a notice when menu mode ends, and a switch function, consulted when the cursor moves outside the lists, that opens another button's menu on the bar.

The items stay in the `Menu`'s `HMENU` (a single source for the native bar and the drawn one): the list reads text, state, type and submenus with `GetMenuItemInfoW` when it opens, and the icons come from their original form (`IconSource`), reloaded at the window's DPI.

### Accessibility of drawn controls (`accessibility::tree`)

Each drawn control implements `VirtualTree`: a flat list of `Node`s (UIA role, name, screen rectangle, states and which patterns it has) and `act(id, Action)`. `TreeHost` publishes this as UIA fragments: a single COM object type with one vtable per interface (`IRawElementProviderSimple`, `…Fragment`, `…FragmentRoot` only at the root, `IInvokeProvider`, `IToggleProvider`, `IExpandCollapseProvider`, `IValueProvider`), created on demand at each navigation and identified by their `RuntimeId` (`UiaAppendRuntimeId`, id). The root is the window itself: it returns `UiaHostProviderFromHwnd` and leaves name and type to the default provider when the tree does not define them (the case of a window with a `MenuStrip`). The elements only keep a `Weak` to the tree and the thread they came from; from another thread they answer `UIA_E_ELEMENTNOTAVAILABLE`. `WidgetCore` answers `WM_GETOBJECT` with the tree; the menu lists, which have their own window, do the same in `popup_proc`. Focus changes from the keyboard raise `UIA_AutomationFocusChangedEventId` (only when clients are listening); the lists also raise `MenuOpened`/`MenuClosed` and the MSAA events `EVENT_SYSTEM_MENUPOPUPSTART/END`.

Besides these, `IRawElementProviderAdviseEvents` (events are only raised when clients are listening) and the Selection/SelectionItem, Scroll, Grid/GridItem, RangeValue and Text patterns live in `accessibility/tree/patterns.rs`. The application publishes its own tree with `set_accessible_model` (`accessibility/model.rs`): an `AccessibleModel` returns `AccessibleItem`s (converted into `Node`s with screen rectangles) and receives `AccessibleAction`s. The modern controls use this same path through `controls/modern.rs::publish`, which keeps only weak references to the control's state.

### System notifications (`system::notifications`)

`SystemNotification` builds the `ToastGeneric` XML and talks to the Windows Runtime through vtable indices, like the rest of the library's COM: `combase.dll` is resolved dynamically (`ffi::winrt`), `RoActivateInstance` creates the `XmlDocument`, `RoGetActivationFactory` gives the `ToastNotification` factory and the `ToastNotificationManager` (`CreateToastNotifierWithId`). The `Activated`, `Dismissed` and `Failed` events get the library's own delegates (`TypedEventHandler<…>`, IIDs computed by the parameterized type rule and checked in a test with SHA-1), with atomic reference counting and `IAgileObject`: Windows calls them on any thread, they read the arguments there and pass the event to the UI thread through the `UiDispatcher`, where the callbacks (`Rc`) live. The *AppUserModelID* is registered in `HKCU` and associated with the process with `SetCurrentProcessExplicitAppUserModelID`. For clicks with the program closed, `notifications/activator.rs` registers a per-user local COM server (`HKCU\Software\Classes\CLSID\{…}\LocalServer32` and `ToastActivatorCLSID` on the AUMID) and a class factory (`CoRegisterClassObject`) that creates `INotificationActivationCallback` objects; the activation arrives on a COM thread and is passed to the UI through the `UiDispatcher`.

### Drag and drop (`input::drag`)

The module is split into `drag/source.rs` (OLE source: `start_drag`, drag image), `drag/target.rs` (target: `accept_drops`, `DropData`) and `drag/reorder.rs` (reordering by mouse capture, moving several selected items as a block, effects outside); `drag.rs` only re-exports. Two complementary mechanisms, both without dependencies:

* **OLE** for everything that crosses control or program boundaries. The source builds an `IDataObject` (the shell's, `SHCreateDataObject`; without it — Wine, stripped-down systems — the library's own object from `input/data_object.rs`, with `IEnumFORMATETC`) from the same blocks as the clipboard (`ClipboardContent::blocks`), and calls `DoDragDrop` with its own `IDropSource`. The target is the library's own `IDropTarget`, registered with `RegisterDragDrop` and kept in the `WidgetCore`'s `DragState`; `WM_DESTROY` calls `RevokeDragDrop`. The COM objects are `#[repr(C)]` with a static vtable and reference counting in a `Cell` (they all live on the UI's STA thread). User callbacks run under `catch_unwind`, and the panic is re-raised by the event loop.
* **Mouse capture** for reordering inside `ListBox`/`ListView`/`DataGrid`/`TreeView`: the drag starts at the `SM_CXDRAG` threshold (ListBox) or on `LVN_BEGINDRAG`/`TVN_BEGINDRAGW` (intercepted in the parent's `WM_NOTIFY`); from there the control keeps the capture and a timer scrolls automatically. The item is copied into a DIB section and shown in a mouse-transparent `WS_POPUP` window (`ArqRustUI.DragVisual`, translucent with `WS_EX_LAYERED` outside Wine); the insertion line is another window of the same class, clipped with `SetWindowRgn`. When the cursor leaves the control and there is an `on_drag_out`, the session ends and becomes a `DoDragDrop`, carrying the item's drawing as the drag image (`IDragSourceHelper`). No `RefCell` stays borrowed while windows are moved (Windows may generate a synthetic `WM_MOUSEMOVE` that comes back into the module).

### Animations (`core::motion`)

All animations of a thread share a single ~15 ms `Timer`, created when the first one starts and released when the last one ends. Each animation keeps its start, duration, curve (`Easing`) and frame callback; the frame is computed from the clock (`Instant`), not from the tick count, so a delay in the loop does not stretch the animation. The callbacks are copied out of the `RefCell` before being called (they may start or cancel other animations). With "Show animations" turned off in Windows (`SPI_GETCLIENTAREAANIMATION`), each animation jumps to its end on the first frame. The drawn controls answer `WM_PRINTCLIENT` (they paint into the received DC), which makes `AnimateWindow` and screenshots possible.

### Vectors and SVG (`graphics::vector`, `graphics::svg`)

`vector.rs` defines figures with lines and cubic Béziers in `f64`, solid paints and gradients, and the stroke, and draws them with the flat GDI+ API (`GdipCreatePath`, `GdipCreateLineBrush` + `GdipSetLinePresetBlend`, `GdipCreatePathGradientFromPath`, `GdipCreatePen2`). GDI+ repeats linear gradients outside their axis; SVG's `Pad` mode is emulated by extending the axis until it covers the figure and repositioning the stops, and the radial one paints the area outside the ellipse with the last color (clipping that excludes the ellipse). When the transform is a similarity, the points are transformed in Rust and the stroke width is scaled here — the result does not depend on how each GDI+ scales pens (Wine's does not scale them).

SVG is read by a minimal XML reader (`svg/xml.rs`) and interpreted in `svg.rs`: cascade (presentation attributes → `<style>` rules by specificity → `style` attribute), inheritance, `use`, gradients with an `href` chain, and `viewBox`/`preserveAspectRatio`. The result is a list of operations (`DrawOp`) that can be tested without windows, drawn later by the `VectorPainter`.

### Modern controls and frameless window

`ToggleSwitch`, `NumberBox`, `RatingControl`, `Card`, `Expander`, `BreadcrumbBar`, `Chart`, `PrintPreview`, `DockHost`/`DockPane` and `TitleBar` are panels of the library's class with their own `paint`; the geometry of each one (chart scales, collapsed trail, docking zones, title bar regions) is a pure function with a unit test. `TitleBar` adds a message filter to the window: `WM_NCCALCSIZE` returns the whole window as client area (with the border adjustment when maximized) and `WM_NCHITTEST` answers borders, `HTCAPTION` and `HTMAXBUTTON` (Windows 11 snap layouts); the bar itself answers `HTTRANSPARENT` outside its buttons, so dragging, double-clicking and the system menu stay with Windows. Floating a pane or detaching a tab changes the parent of the content window (`SetParent`) — the child controls and state go with it; closing the floating window gives the content back before it is destroyed.

### Modal (`dialogs::modal`)

These are two native windows owned by the application window. The **backdrop** is a `WS_EX_LAYERED | WS_EX_NOACTIVATE` popup updated with `UpdateLayeredWindow` from a premultiplied DIB section: black with the dimming opacity and a shadow computed in Rust (distance to the card's rounded rectangle, stronger at the bottom); the animation only changes `SourceConstantAlpha`. The **card** is a frameless library window, owner = backdrop (always above it), with the owner window's theme, the normal controls and DWM corners on Windows 11. The owner is disabled (`EnableWindow`) as with a regular modal and re-enabled before the card disappears, so activation goes back to it; the previous focus is restored and stored in the owner (it comes back even if the owner is inactive). `show` runs the message loop until the `Closed` phase. On Wine (no compositor) the layer gets a dimmed, opaque copy of the screen under the owner.

### Gestures (`input::gestures`)

`GestureRecognizer` is pure logic fed with timestamped contacts: the centroid and, with two fingers, the distance and angle between the first two give translation, scale and rotation; changing the set of contacts resets the reference (no "jump"); velocity is an exponential average used by inertia, which decays exponentially on a clock that only runs when needed. The binding to the widget uses `WM_POINTER*`; without them (Windows 7) it translates `WM_GESTURE` into the same events.

## DPI and coordinates

The whole public API uses **logical units (96 DPI)**. The conversion happens in `WidgetCore::set_bounds` and in mouse events. On `WM_DPICHANGED`, the window adopts the suggested rectangle and every descendant gets its bounds and fonts reapplied at the new DPI; then the layout runs again. The GDI `Canvas` sets up the DC (`MM_ANISOTROPIC`) so that drawing is also in logical units; Direct2D uses DIPs natively.

Deliberate exception: the **position** of top-level windows is in screen pixels (virtual desktop coordinates), because a logical screen coordinate would be ambiguous between monitors with different scales.

## Layout

`layout::engine` contains pure functions (`stack`, `grid`, `flow`, `dock`, `anchor`, `split`) that take specifications and return rectangles — testable without windows. The `StackLayout`, `GridLayout`, etc. types hold the items (controls or nested layouts) and apply the rectangles with `set_bounds`. A layout attached with `set_layout` is reapplied on `WM_SIZE` and on DPI changes.

## Graphics

Three independent backends, chosen by the type used:

| Backend | Entry point | Notes |
|---|---|---|
| GDI | `Canvas` (in `on_paint`, `Bitmap::render`, printing) | double buffering by default |
| GDI+ | `canvas.with_gdiplus(\|g\| …)` | antialiasing, alpha, PNG/JPEG |
| Direct2D + DirectWrite | `Direct2DSurface::attach(&panel, \|d2d\| …)` | COM through vtable indices; recreates the target when the device is lost |

On top of GDI+ sit the floating-point vectors and SVG (section above); `Bitmap::render_at` draws at any DPI into a DIB section (used by print preview, which draws the same page function as the printer).

## Icons

`icons::IconAsset` holds `'static` references to the bytes of a `.ico` (several sizes), a `.png` and, optionally, a `.svg`; the 392 pack icons are `static`s generated in `icons/catalog.rs` (one `icon!` line per icon) with `include_bytes!`, behind the `icons` feature. The bytes come from the `arqrustui-icons` crate, the optional dependency of the `icons` feature: it only re-exports the eight data crates in `crates/`, each with up to 9 MB (crates.io accepts up to 10 MiB per package) and one `(.ico, Option<.png>, Option<.svg>)` constant per icon; the `icon!` line takes that constant by the same name. When the PNG is an exact copy of the `.ico`'s 256 px frame, the data crate stores `None` and `IconAsset::png_bytes` returns that frame as a slice of the `.ico` (170 of the 392 icons). The `EXTENSIONS` table in the same file maps extensions (`pdf`, `yml`, `tar.gz` → `gz`...) to the `IconCategory::FileTypes` icons; `for_extension`/`for_path` only walk it, so with LTO only those icons go into the executable. Code that draws icons does not get a ready-made `HICON`, but an `IconSource` — implemented by `Icon` and `IconAsset` — and asks it for the pixel size it needs, computed from the control's DPI. `IconAsset` picks the closest image in the `.ico` (`CreateIconFromResourceEx`, which accepts PNG frames) and keeps the result in a per-thread cache.

Two mechanisms in `controls/icon_support.rs` bring icons to the controls:

| Mechanism | Controls | How it works |
|---|---|---|
| `WidgetIcon` in `WidgetCore` | `Button`, `Label`, `Window` | stores the source, the logical size and the position; the icon is reloaded at the current DPI on every paint (label, button drawn by the library) or sent again to the control (`WM_SETICON`) when the DPI changes. On the native `Button` (v6), icon and text are drawn in the `CDDS_PREPAINT` of `NM_CUSTOMDRAW`, over the background the theme has already painted; the automatic size is 60% of the height |
| `ManagedImages` in `WidgetCore` | `TreeView`, `ListView`, `DataGrid`, `TabControl`, `ToolBar`, `ComboBox`, `ListBox`, `StatusBar` | an `ImageList` at the DPI's size; each distinct source takes one slot (reused by key); on a DPI change the list is rebuilt with the same slots and sent to the control again |

`ComboBox` and `ListBox` with icons are `*_OWNERDRAWFIXED`: the item stores in its data (`*_SETITEMDATA`) the source's slot + 1, so the icon follows the item in sorted lists; `WM_MEASUREITEM` is answered by the parent with a default height (the control is not in the registry yet) and corrected right after creation.

Non-editable combo boxes are always `CBS_OWNERDRAWFIXED | CBS_HASSTRINGS`: the field's text is centered at any height and the items accept icons; in editable ones, the inner edit box is centered again (`GetComboBoxInfo`) at each adjustment.

Single-line controls that draw their text at the top of the client area (single-line `TextBox`, `PasswordBox`, `SpinButton`, `DatePicker`, `TimePicker`, `HotKeyBox`) turn on `WidgetCore::set_centered_text`: in `WM_NCCALCSIZE` the client area is reduced to the height of one line with equal bands above and below; `WM_NCPAINT` paints the bands with the brush the parent would give the control (`WM_CTLCOLOREDIT`/`WM_CTLCOLORSTATIC`, which respects themes and the disabled state) and `WM_NCHITTEST` treats the bands as client area.

The height of a closed `ComboBox` does not follow the window's: the native control computes it from the font. So that the box has the requested logical height (by default `DEFAULT_CONTROL_HEIGHT`, the same as `Button`), `WidgetCore` calls `combobox::fit_field_height` after each `set_bounds` and each font change (which covers DPI changes): the selection field's height is corrected with `CB_SETITEMHEIGHT(-1)` by the difference between the measured closed height and the requested one, without going below the text height.

## Relevant design decisions

| Decision | Reason |
|---|---|
| `Clone` handles with `Rc`, no lifetimes in the API | Event closures need to capture controls; the real state belongs to the native window |
| Registry by `HWND` instead of a pointer in `GWLP_USERDATA` | No raw pointer to Rust state is stored in Windows; rules out *use-after-free* |
| Classic subclassing instead of `SetWindowSubclass` | Does not depend on the loaded comctl32 version |
| `comctl32` resolved dynamically | A static import would bind to v5.82 (without `TaskDialogIndirect`) in processes without a manifest |
| Recent APIs through `GetProcAddress` | Capability detection; the executable loads on any supported Windows |
| No `Send`/`Sync` on visual types | Win32 thread affinity guaranteed by the compiler |
| Icons as `static`s with `include_bytes!`, behind a feature | No files to ship; with LTO only the icons used go into the executable; those who do not use the pack turn it off |
| Controls receive an `IconSource`, not an `HICON` | The control picks the resolution from its own DPI and can reload it when the DPI changes |
