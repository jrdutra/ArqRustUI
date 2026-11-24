# Event system, threads and `async`

*Português: [EVENTS.md](EVENTS.md).*

## Registering handlers

Every control implements `Widget`, which provides the common events. Specific controls add typed events.

```rust
button.on_click(|| println!("clicked"));
text.on_text_changed({ let text = text.clone(); move || println!("{}", text.text()) });
slider.on_value_changed(|value: i32| println!("{value}"));
window.on_resize(|size| println!("{}x{}", size.width, size.height));
window.on_close_request(|| MessageBox::confirm("Quit", "Close the window?")); // false cancels
```

### Event names

Every event is `on_<something>`, and the name says **when** it happens:

| Form | When | The callback | Examples |
|---|---|---|---|
| present-tense verb | direct user action, at the moment it happens | returns nothing | `on_click`, `on_double_click`, `on_key_down`, `on_item_click`, `on_column_click`, `on_link_click` |
| participle (`_changed`, `_toggled`, `_closed`…) | notice **after** something changed | returns nothing | `on_value_changed`, `on_selection_changed`, `on_highlight_changed`, `on_toggled`, `on_closed`, `on_dismissed`, `on_row_activated`, `on_item_moved`, `on_finished` |
| `_request` | request **before** something happens, which can be refused | returns `bool` (`false` cancels) | `on_close_request`, `on_item_reorder_request`, `on_rows_reorder_request`, `on_item_drop_request` |

Deliberate exceptions: the basic `Widget` and `Window` events keep the classic Windows
names (`on_resize`, `on_move`, `on_focus`, `on_blur`, `on_destroy`, `on_paint`);
`Menu::on_open` runs right before the menu appears, so it can be prepared;
`on_gesture` and `ImeCandidateWindow::on_pick` (the clicked candidate) are user
actions, in the present tense; and callbacks that **provide** something to the library
are named after the action (`on_drag_out` returns the content to drag,
`DropHandler::on_over` returns the accepted effect).

The noun follows the control's vocabulary: `item` in `ListBox` and `TreeView`,
`row` in `ListView` and `DataGrid`.

Old names that broke the convention still work as `#[deprecated]` aliases (the
compiler warns and points to the new name):

| Type | Old | New |
|---|---|---|
| `Window` | `on_close` | `on_close_request` |
| `InfoBar` | `on_close` | `on_closed` |
| `Toast` | `on_dismiss` | `on_dismissed` |
| `CommandButton` | `on_toggle` | `on_toggled` |
| `Hyperlink` | `on_link_clicked` | `on_link_click` |
| `ListView` | `on_item_activated` | `on_row_activated` |
| `ListView`, `DataGrid` | `on_reorder` | `on_row_reorder_request` |
| `ListView`, `DataGrid` | `on_rows_reordered` | `on_rows_reorder_request` |
| `ListBox` | `on_reorder` | `on_item_reorder_request` |
| `ListBox` | `on_items_reordered` | `on_items_reorder_request` |
| `TreeView` | `on_item_drop` | `on_item_drop_request` |
| `Transition` | `on_done` | `on_finished` |
| `Menu` | `on_highlight` | `on_highlight_changed` |
| `Ime` | `on_candidates` | `on_candidates_changed` |
| `Ime` | `on_composition` | `on_composition_changed` |

### Common events (`Widget`)

| Method | Data | Win32 source |
|---|---|---|
| `on_click` | — | `BN_CLICKED`, `STN_CLICKED`, click on the library's own classes |
| `on_double_click` | — | `WM_LBUTTONDBLCLK`, `BN_DBLCLK`, `NM_DBLCLK` |
| `on_mouse_move` / `on_mouse_down` / `on_mouse_up` / `on_mouse_wheel` | `&MouseEvent` | `WM_MOUSE*`, `WM_*BUTTON*` |
| `on_mouse_enter` / `on_mouse_leave` | — | `TrackMouseEvent` + `WM_MOUSELEAVE` |
| `on_key_down` / `on_key_up` | `&KeyEvent` | `WM_KEYDOWN/UP`, `WM_SYSKEYDOWN/UP` |
| `on_char` | `char` | `WM_CHAR` |
| `on_focus` / `on_blur` | — | `WM_SETFOCUS` / `WM_KILLFOCUS` |
| `on_resize` | `Size` | `WM_SIZE` |
| `on_destroy` | — | `WM_NCDESTROY` |
| `on_context_menu` | `Point` | `WM_CONTEXTMENU` |
| `on_pointer` | `&PointerEvent` | `WM_POINTERDOWN/UPDATE/UP` |
| `on_event` | `&Event` | all |

### Specific events

| Type | Events |
|---|---|
| `Window` | `on_close_request() -> bool`, `on_move`, `on_activated`, `on_deactivated`, `on_drop_files`, `on_dpi_changed`, `on_theme_changed`, `on_paint` |
| `TextBox`, `PasswordBox`, `RichTextBox` | `on_text_changed` (+ `on_selection_changed` on RichTextBox) |
| `CheckBox`, `RadioButton`, `ToggleButton` | `on_value_changed(bool)` |
| `ComboBox` | `on_selection_changed(Option<usize>)`, `on_text_changed` |
| `ListBox` | `on_selection_changed(Option<usize>)`, `on_item_activated(usize)`, `on_item_reorder_request(from, to) -> bool`, `on_items_reorder_request(&[usize], to) -> bool`, `on_drag_out(usize)` |
| `Slider`, `ScrollBar`, `SpinButton`, `UpDown` | `on_value_changed(i32)` |
| `DatePicker`, `TimePicker` | `on_value_changed(Option<DateTime>)` |
| `ListView`, `DataGrid` | `on_selection_changed(Vec<usize>)`, `on_row_activated(usize)`, `on_column_click(usize)`, `on_row_reorder_request(from, to) -> bool`, `on_rows_reorder_request(&[usize], to) -> bool`, `on_drag_out(usize)`; `on_check_changed` (`ListView`); `on_cell_edited`, `on_group_toggled` (`DataGrid`) |
| `TreeView` | `on_selection_changed(Option<TreeItem>)`, `on_item_drop_request(TreeItem, TreeDropPosition) -> bool`, `on_item_moved(TreeItem)`, `on_drag_out(TreeItem)` |
| Any widget (OLE drop target) | `accept_drops(&w, DropHandler::new().on_over(..).on_drop(..).on_leave(..))` |
| `TabControl` | `on_selection_changed(usize)` |

Events that return a `HandlerId` accept several handlers; use `widget.remove_handler(id)` to remove one. Those that return `()` are documented one by one (the `_request` events and `on_paint`, for example, keep a single handler).

## Dispatch rules

- **Events fire on user actions**, not on code calls: `set_checked`, `set_value`, `set_selected_index` do not raise `on_value_changed`/`on_selection_changed`. (Exception inherited from Windows: `set_text` on edit fields raises `on_text_changed`.)
- **Safe reentrancy**: the handler list is copied before dispatch. A handler may register/remove handlers, destroy its own control or close the window.
- **Once destroyed**, a control receives no more events; `Destroyed` is the last one.
- **Panics**: caught in the window procedure and in every other entry point called by Windows (COM methods, callbacks) and re-raised by `Application::run()` / `process_pending()` — they never cross Windows code. On a thread without an application (the COM thread that delivers a notification activation, for example), the panic hook prints the message and the call returns a failure value.

## Capturing controls in closures

Controls are `Clone` handles. Clone what the closure needs:

```rust
let label = Label::builder().parent(&window).build()?;
let target = label.clone();
button.on_click(move || target.set_text("clicked"));
```

Reference cycles (`a` captures `b`, `b` captures `a`) do not leak: all handlers are released when the native window is destroyed.

## Keyboard

- `Tab`/`Shift+Tab`, arrow keys in radio groups and mnemonics (`&Save` → Alt+S) work automatically.
- `Button::builder().default(true)` responds to Enter; `.cancel(true)` responds to Esc; `WindowBuilder::close_on_escape(true)` closes the window with Esc.
- Shortcuts: `window.add_shortcut(Shortcut::ctrl(Key(b'S' as u16)), || …)` or `menu.add_item_with_shortcut(..)`.
- Tab order: creation order, or `accessibility::set_tab_order(&[&a, &b, &c])`.

## Threads

The interface belongs to the thread that created the `Application`. No visual type is `Send`/`Sync` — moving a control to another thread is a compile error.

```rust
let ui = app.dispatcher();              // Send + Sync + Clone
let label = label.clone();              // stays on the UI thread
std::thread::spawn(move || {
    let total = slow_work();
    // `label` cannot come here; only the result goes back to the UI:
    let _ = ui.post(move || println!("{total}"));
});
```

To hand the result back to a control, prefer the forms below, where the completion closure already runs on the UI thread.

## Background work and `async`

```rust
// Callback: `work` runs on another thread; `done` runs on the UI and may capture controls.
spawn_background(|| read_large_file(), move |data| label.set_text(&data))?;

// async/await without an external runtime: the future runs inside the event loop.
spawn_local(async move {
    progress.set_marquee(true);
    let result = run_blocking(|| compute()).await;       // another thread
    sleep(Duration::from_millis(200)).await;             // does not block the UI
    label.set_text(&result);
    progress.set_marquee(false);
})?;
```

| Function | Use |
|---|---|
| `defer(f)` | runs `f` on the next loop iteration (UI thread) |
| `UiDispatcher::post(f)` | runs `f` on the UI from any thread |
| `spawn_local(fut)` | runs a `Future` (need not be `Send`) on the UI |
| `run_blocking(f)` | runs `f` on a worker thread; returns a future |
| `sleep(d)` | future that completes after `d` |
| `spawn_background(work, done)` | shortcut: thread + return to the UI |
| `Timer::interval(d, f)` / `Timer::once(d, f)` | timers on the UI (`interval` stops when dropped) |

**Never** block the UI thread (`thread::sleep`, long I/O, `join`): the window stops responding.

## Raw messages (advanced API)

```rust
widget.on_raw_message(|msg| {
    if msg.message == 0x0084 /* WM_NCHITTEST */ { return Some(2 /* HTCAPTION */); }
    None // let the library and Windows handle it
});
```

There is one hook per widget; `Direct2DSurface::attach` and the `DataGrid` cell editor also use it.
