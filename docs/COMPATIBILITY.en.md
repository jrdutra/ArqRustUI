# Compatibility, validation and known limitations

*Português: [COMPATIBILITY.md](COMPATIBILITY.md).*

## Maximize check on Windows — 2026-11-14

With MSVC and 144 DPI scaling, the height shrank from 1,550 to 92 pixels when `StatusBarLayout::fit_height` was called after hiding content in a maximized window. Windows kept the maximized state, but the content adjustment replaced its rectangle. The method now only resizes the window in the normal state.

The four tests in `tests/window_maximize.rs` passed: maximize/restore cycles with content updates, preservation of the monitor's work area and of the footer, minimize, full screen and DPI reapplication. The DPI message is simulated with the current scale; a physical move between monitors with different scales is still pending. The three composition tests passed. The `demo` and `composition` examples were opened; maximizing and the footer were checked visually, including showing/hiding content in `composition`.

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo build --release` passed. The full suite (`cargo test --no-fail-fast`) had 223 passing tests (including 35 doctests) and 12 failures in paths that do not use the changed method: file association, UIA client, theme/icon painting, tree state after dragging, InfoBar, notifications and clipboard. Notifications and clipboard returned access denied in that session; the other differences need their own investigation. The file association failure also happened before the fix: Windows returns `OpenWith.exe` for an unknown extension. These results complement, without replacing, the historical validation under Wine described below.

## Platforms

| Item | Status |
|---|---|
| Windows 10 and 11, x86_64 | main target |
| Toolchains | `x86_64-pc-windows-msvc` and `x86_64-pc-windows-gnu` (the DLLs are linked with `#[link(name = "…")]`, available on both) |
| x86 (32-bit) | supported: the whole suite passes on 32-bit Wine (`i686-pc-windows-gnu`; CI runs `i686-pc-windows-msvc`). Pointer-dependent structures have `cfg` where the layout differs (`TBBUTTON`, `PRINTDLGW`); `GetWindowLongPtrW`/`SetWindowLongPtrW` point to `GetWindowLongW`/`SetWindowLongW`, as in `windows.h`. The layout check in `tests/ffi_layout.rs` only runs on 64-bit |
| ARM64 | the FFI layer uses nothing specific to x86_64; it compiles (`cargo check`, and in CI as an experimental target), **never run** |
| Other systems | the crate compiles empty (`#![cfg(windows)]`) |

## Features that depend on the Windows version

No recent API is imported statically: everything below is resolved at run time, and its absence becomes `GuiError::Unsupported` (or a fallback), never a load failure.

| Feature | Requirement | Without the requirement |
|---|---|---|
| Per-monitor DPI v2 | Windows 10 1703 | system DPI (`SetProcessDPIAware`) |
| `GetDpiForWindow`, `AdjustWindowRectExForDpi` | Windows 10 1607 | system DPI / `AdjustWindowRectEx` |
| Visual styles (Common Controls v6) | always present on Windows 10/11 | controls with the classic look; `TaskDialog` returns `Unsupported` |
| Dark title bar | Windows 10 build 19041 | `Unsupported` |
| Corners, border/caption color | Windows 11 | `Unsupported` |
| Mica/Acrylic | Windows 11 22H2 (build 22621) | `Unsupported` |
| `accessibility::announce` | Windows 10 1709 | `Unsupported` |
| Pointer events (touch/pen) | Windows 8 | `PointerKind::Other` / mouse events |
| Direct2D/DirectWrite | Windows 7 SP1+ | `Direct2DSurface::attach` returns an error |
| Icon on a native button (the button's `NM_CUSTOMDRAW`) | Common Controls v6 | the button is drawn by the library instead, with system colors |

Use `WindowsVersion::current()` and its `supports_*` methods to decide before calling.

## How this version was validated

The environment where the library was written did not run Windows. The validation was:

| Check | Result |
|---|---|
| Build for `x86_64-pc-windows-gnu` (library, 20 examples, tests) | no errors or warnings |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets` | no warnings |
| `cargo test` under **Wine 9.0**, with Common Controls v6 active | 129 tests (64 unit, 59 integration with real windows, 6 FFI layout) + 28 doctests — all passing |
| Same suite with the system at **144 DPI** (150 %) | all passing |
| `cargo build --release` | ok (LTO could only be exercised turned off in the validation environment) |
| Visual run of the examples under Wine, with simulated clicks and typing | hello_world, demo, dark_theme, direct2d, responsive_layout, custom_drawing, calculator, text_form, validated_form, tabs, listview_columns, treeview, file_dialogs, icons (96 and 144 DPI) |

### What **still needs** validation on real Windows 10/11

Wine is a reimplementation: it behaves like Windows for the vast majority of the APIs used, but it is not Windows. Before considering the version stable, run on Windows:

```powershell
cargo test
cargo run --example demo
```

and check specifically:

1. **Appearance with visual styles** (Common Controls v6): there was no theme installed in the validation environment, so the controls showed the classic style. Loading of v6 was confirmed indirectly (`TaskDialogIndirect`, exclusive to v6, was resolved and shown).
2. **Dark theme on themed controls**: colors through `WM_CTLCOLOR*`, ListView/TreeView/RichEdit, buttons, tabs, status bar, headers, group box, 3D borders, toolbar and calendar were checked on Wine (the first four painted by the library were also checked pixel by pixel in the tests); the combination with Windows' real visual styles (check boxes, `DarkMode_Explorer` scroll bars, `DarkMode_ItemsView` on the header) was not.
3. **DWM effects**: `set_dark_title_bar` returned success; corners/borders/Mica returned `Unsupported` (the simulated version was Windows 10). The visual effect could not be observed.
4. **DirectWrite text**: the `CreateTextFormat`/`DrawText` calls were confirmed in the Wine trace with the right arguments, but the environment had no fonts installed for DirectWrite to render. Direct2D shapes, gradients, paths and transforms rendered correctly.
5. **Accessible name on native controls** (`set_accessible_name` through `IAccPropServices`): Wine returns `E_NOTIMPL`. The UIA provider for custom controls and `announce` returned success, but were not tested with a screen reader (Narrator/NVDA). The virtual elements of `MenuStrip`, `CommandBar` and `PropertyGrid` were read by Wine's `IUIAutomation` client (type and names); check on Windows with Narrator/Accessibility Insights that the window's native controls are still listed next to the drawn menu bar (Wine's client does not list child windows) and that focus announcements follow the arrow keys.
6. **DPI change at run time** (`WM_DPICHANGED` when dragging between monitors): implemented; only fixed DPI (96 and 144) was exercised.
7. **Tray and notifications**: icon added and removed successfully; showing the notification and the right-click menu were not observed. `SystemNotification` could not be shown (Wine has no `Windows.UI.Notifications`): check on Windows 10/11 the notification with buttons, the name/icon registered in the `AppUserModelID`, `on_activated` with the button's argument and `on_dismissed` when it is closed/expires.
8. **Dialogs**: open file and TaskDialog were shown; color, font, print, page setup and find/replace compiled and follow the layouts checked in `tests/ffi_layout.rs`, but were not operated interactively. On Wine the TaskDialog did not show custom buttons/command links (a Wine limitation).
9. **Drag and drop between programs**: reordering, dragging out and the OLE drop target were exercised on Wine with the mouse (including a complete drag from a list to another control). Check on Windows the shell drag image (`IDragSourceHelper`/`IDropTargetHelper`), the translucent ghost under the cursor and dropping into Explorer, Word and Notepad.
10. **Printing on paper**, **real touch/pen**, **real IME**, **dragging files from Explorer**, **`Animation` with a real AVI**, **several monitors**: no device or no way to simulate.
11. **MSVC toolchain**: only the GNU variant was built.
12. **Icons on native buttons and centered text**: visual styles are enabled through the manifest embedded in `shell32.dll`; when it does not bring v6 (the case on Wine), the library writes an equivalent manifest to the temporary folder, and that is how the tests run under Wine with v6. Check on Windows: the icon and text drawn over the theme background in the button's `NM_CUSTOMDRAW` (`Button::is_icon_native` true), the centered text in text boxes and pickers (non-client bands) and the `ComboBox` height (`CB_SETITEMHEIGHT(-1)`).

13. **Second-stage features only seen on Wine**: check on Windows the appearance of `RichTextBox` lists and tables and, above all, images (`insert_image`: Wine's RichEdit keeps the `\pict` but does not draw metafiles); print preview and printing a page range on a real printer; the library's own candidate list with a Chinese/Japanese IME (selection by click and by the IME keys); antialiasing of SVG and charts and radial gradients with several stops (Wine's GDI+ does not smooth paths inside images and only uses the center and edge colors); gestures with a real touch screen (pinch, rotation, inertia) and `WM_GESTURE` on Windows 7; the window with `TitleBar` (DWM shadow, maximizing without going past the screen, Windows 11 snap layouts over the maximize button, Alt+Space); floating/docking panes and detaching tabs with a real mouse; notifications with fields, progress, scheduling and the COM activator (click with the program closed); `AnimateWindow` on the drawn controls; `Modal` on Windows (real backdrop transparency over the live window, shadow, Windows 11 rounded corners, the enter/exit animation and focus returning when switching programs with the modal open).

## Known limitations (by design or for now)

| Area | Limitation |
|---|---|
| Drag and drop | Reordering moves all selected items as a block (`ListBox`, `ListView`, `DataGrid` also with a virtual source, `TreeView` one node at a time, `CommandBar`), but does not work on `sorted` lists. Dragging out offers copy and, with `set_drag_out_effects`, move (the application deletes the item at the end). On Wine the item image is not translucent and sits beside the cursor (no compositor); `SHCreateDataObject` does not exist on Wine, so the library's own data object is used there. |
| Notifications | `SystemNotification` (Windows 10+) registers an *AppUserModelID* in `HKCU\Software\Classes\AppUserModelId` on the first notification; with `NotificationActivator::register` a click opens the closed program (per-user COM registration, `LocalServer32`). Text/selection fields, progress, scheduling and groups follow the Windows 10 toast schema; none of it could be shown (Wine has no `Windows.UI.Notifications`). On Windows 7 and on Wine it returns `Unsupported` — use `TrayIcon::show_notification`. |
| UI Automation | `MenuStrip`, the drawn menu lists, `CommandBar`, `PropertyGrid`, `DataGrid`, the modern controls, `TitleBar`, `DockPane` and the `set_accessible_model` models publish virtual elements (Invoke, Toggle, ExpandCollapse, Value, RangeValue, Selection, Scroll, Grid, Text) and accept `IRawElementProviderAdviseEvents`. Menu lists stay children of the desktop (like native menus). Nothing was heard with Narrator/NVDA. |
| Clipboard | Images are written as `CF_DIB`, `CF_DIBV5` (with alpha) and `PNG` and read from the best available format; metafiles as `CF_ENHMETAFILE`. `ClipboardWatcher` uses `AddClipboardFormatListener` (Vista+). |
| Dark theme | With a dark theme, the library paints tabs (top tabs only; `TCS_BOTTOM`/vertical/button tabs stay native), the status bar, column headers, `GroupBox`, 3D borders (`WS_EX_CLIENTEDGE`), the `DatePicker`/`TimePicker` field and the `Slider` (repainting over the native drawing); no hover highlight on tabs and headers. Calendar, progress bar and toolbar switch to classic drawing with the theme colors. The native `MenuBar` is automatically replaced by a `MenuStrip` with the theme colors (turn it off with `set_menu_bar_follows_theme(false)`). Scroll bars depend on `DarkMode_Explorer` (Windows 10 1809+). On Wine, scroll bars and the progress bar track stay light. |
| `InfoBar` / `Toast` | The toast uses `WS_EX_LAYERED` only while fading (with Windows set to reduce animations, it appears and disappears without the effect). For the Action Center use `SystemNotification`. Both announce their text to screen readers (`UiaRaiseNotificationEvent`, Windows 10 1709+); the announcement was not heard. |
| `CommandBar` | Items leave from the end of the bar (the rightmost first); hosted controls that do not fit go to `»` with their label. Customization changes order and visibility (`CommandBarLayout`); it does not create commands. |
| Menus | Replacing a window's menu bar does not destroy the previous one. `MenuStrip`: no dragging the bar, no right-aligned items and no opening animation; the list shadow (`CS_DROPSHADOW`) and Windows 11 rounded corners were not observed (Wine does not draw them). `on_highlight_changed` on the native bar depends on `WM_MENUSELECT`. |
| `DataGrid` | In-line cell editing (no drop-down or date editors); grouping by one column; text export (CSV/TSV/HTML), no binary formats. |
| `PropertyGrid` | Value types: text, number, boolean and choice; no custom editors (color, file). |
| `RichTextBox` | Lists, tables and images are inserted as RTF at the cursor and edited by RichEdit itself (no resizing of columns or images through the API). Images become EMF metafiles inside the document. |
| `Rebar`, `Pager`, `Animation`, `HotKeyBox` | Coverage of the essentials of each control. |
| Printing | `PrintDlgW` (one `From/To` page range, without several ranges like `PrintDlgEx`); the preview draws each page into a bitmap at screen scale; copies are made by the driver when it supports them. |
| Touch | Gestures recognized by the library from `WM_POINTER*` (Windows 8+) and, without them, from `WM_GESTURE` (Windows 7). No gestures with three or more fingers, no "swipe back". Wine reports `PT_MOUSE` for synthetic pointers. |
| IME | The library's own candidate list (`Ime::use_candidate_window`) and composition control through `ImmNotifyIME`; the composition window can be hidden, but the text being composed is drawn by the application. No advanced TSF (Text Services Framework) support. |
| Internationalization | Regional formatting and window RTL (`right_to_left`); the library's own texts come in Portuguese and English (`set_library_language`, or `use_system_library_language` for the Windows language) and accept translation through a catalog (`apply_library_translations`). |
| Fonts | Fractional sizes are rounded to tenths of a point. |
| Window position | In physical screen pixels (the size is logical) — see ARCHITECTURE.md (Portuguese). |
| Text with an inner NUL | Truncated by Windows at the first NUL. |
| SVG | No text, embedded images, `clipPath`, `mask`, `pattern`, filters, markers or animations; group `opacity` is applied to each element. |
| Charts (`Chart`) | One value scale (no second axis), categories as text (no date/number axis), pie with the first series. |
| Animations | Driven by a ~15 ms timer on the UI thread; `show_animated`/`hide_animated` use `AnimateWindow` (synchronous, blocks until the end). |
| Frameless window (`TitleBar`) | Does not combine with a `MenuStrip` in the non-client area of the same window (put the `MenuStrip`/`CommandBar` below the bar). Shadow and corners depend on DWM. |
| Dockable panes | They change side through `dock_to` or the button; there are no docking targets while dragging the floating window. A zone stacks its panes (no tabs inside the zone). |
| `Modal` | The backdrop covers the owner window's client area (its title bar stays visible and disabled, as with any Windows modal). Per-pixel transparency with `UpdateLayeredWindow` (Windows 2000+, without depending on DWM); on Wine, without a compositor, the backdrop becomes a dimmed, opaque snapshot of the window taken when it opens. The card's rounded corners come from DWM (Windows 11); before that the card is rectangular. |
| Architectures | x64 and x86 (32-bit) tested (on Wine); ARM64 only compiles. See "Platforms" above. |
| Detachable tabs | The tab always comes back at the end of the strip; tab icons do not follow the detached window. |
| Icons | ComboBox and ListBox show icons only if created with `with_icons(true)`. Loaded pack icons stay cached until the thread ends. SVG is drawn by the library's own subset of SVG 1.1 (see `graphics::svg`). |
| Executable size with the `icons` feature | Without LTO, the GNU (mingw) toolchain does not discard unused icons (`function-sections` is off for that target) and embeds the whole pack (~66 MB); `debug` builds on any toolchain too. In `release` with LTO (the repository's profile) or MSVC with `/OPT:REF`, only the icons used. Turn it off with `default-features = false` if you do not use the pack. |
