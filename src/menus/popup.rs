//! Library-drawn popups and menu-mode tracker.
//!
//! One thread-local [`Tracker`] exists while a drawn menu is open or keyboard menu mode active. [`pre_translate`]
//! handles mouse/keyboard before others. Lists are never-activated `WS_POPUP` windows, preserving owner
//! activation/focus.
//!
//! Reentrancy: no `RefCell` borrow spans synchronously messaging Windows calls (create/destroy/show) or item handlers.

use super::strip::{self, MenuStyle, SlotMenu, StripInner, ICON};
use super::{mnemonic_of, plain_text, Entry, EntryKind, MenuInner};
use crate::accessibility::tree::{role, Action, Node, TreeHost, VirtualTree};
use crate::core::application as app;
use crate::core::handles::IconSource;
use crate::core::native;
use crate::ffi::kernel32::{GetLastError, ERROR_CLASS_ALREADY_EXISTS};
use crate::ffi::ole32::ComPtr;
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::graphics::{Canvas, Font, Pen, TextAlign};
use crate::system::{Monitor, WindowsVersion};
use crate::utilities::{to_logical, to_pixels, to_wide, Color, Point, Rect};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

mod access;
mod painting;
mod tracking;

use access::*;
pub(crate) use painting::draw_icon;
use painting::*;

const POPUP_CLASS: &str = "ArqRustUI.MenuPopup";
const OPEN_TIMER: usize = 1;

/// Logical library border when DWM does not draw system border.
const BORDER: i32 = 1;
/// Logical space above/below first/last item.
const PAD_Y: i32 = 4;
/// Logical horizontal highlight inset from border.
const INSET_X: i32 = 4;
/// Logical highlight-to-icon spacing.
const ICON_PAD: i32 = 8;
/// Logical icon-to-text spacing.
const ICON_GAP: i32 = 10;
/// Logical highlight-to-text spacing without icons/checks.
const TEXT_PAD: i32 = 12;
/// Minimum logical text-to-shortcut spacing.
const SHORTCUT_GAP: i32 = 32;
/// Logical submenu arrow column.
const ARROW_COLUMN: i32 = 24;
/// Logical right margin without arrows.
const RIGHT_PAD: i32 = 14;
/// Logical separator height.
const SEPARATOR: i32 = 9;
/// Minimum logical list width.
const MIN_WIDTH: i32 = 140;
/// Logical scroll-arrow strip height.
const SCROLL_BAND: i32 = 14;

thread_local! {
    static TRACKER: RefCell<Option<Rc<Tracker>>> = const { RefCell::new(None) };
    static POPUPS: RefCell<HashMap<isize, Rc<Level>>> = RefCell::new(HashMap::new());
    static CLASS_READY: Cell<bool> = const { Cell::new(false) };
    /// Key release to discard (Alt/F10 ending menu mode).
    static SUPPRESSED_UP: Cell<i32> = const { Cell::new(0) };
}

pub(crate) fn suppress_key_up(vk: i32) {
    SUPPRESSED_UP.with(|s| s.set(vk));
}

pub(crate) fn take_suppressed_key_up(vk: i32) -> bool {
    SUPPRESSED_UP.with(|s| {
        if s.get() == vk {
            s.set(0);
            true
        } else {
            false
        }
    })
}

/// Logical list measurements.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    /// Entry (top, height) relative to list top.
    pub rows: Vec<(i32, i32)>,
    pub width: i32,
    pub height: i32,
    pub icon_column: bool,
    pub text_x: i32,
}

/// Logical entry measurements.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Measured {
    pub separator: bool,
    pub text: i32,
    pub shortcut: i32,
    pub submenu: bool,
}

/// Calculates layout from measured widths.
pub(crate) fn compute_layout(entries: &[Measured], row_height: i32, icon_column: bool, min_width: i32) -> Layout {
    let text_x = INSET_X + if icon_column { ICON_PAD + ICON + ICON_GAP } else { TEXT_PAD };
    let max_text = entries.iter().map(|e| e.text).max().unwrap_or(0);
    let max_shortcut = entries.iter().map(|e| e.shortcut).max().unwrap_or(0);
    let has_submenu = entries.iter().any(|e| e.submenu);
    let shortcut = if max_shortcut > 0 { SHORTCUT_GAP + max_shortcut } else { 0 };
    let right = if has_submenu { ARROW_COLUMN } else { RIGHT_PAD } + INSET_X;
    let width = (text_x + max_text + shortcut + right).max(MIN_WIDTH).max(min_width);
    let mut y = BORDER + PAD_Y;
    let mut rows = Vec::with_capacity(entries.len());
    for entry in entries {
        let h = if entry.separator { SEPARATOR } else { row_height };
        rows.push((y, h));
        y += h;
    }
    Layout { rows, width, height: y + PAD_Y + BORDER, icon_column, text_x }
}

/// Open list.
pub(crate) struct Level {
    hwnd: Cell<HWND>,
    entries: Vec<Entry>,
    layout: Layout,
    /// Visible logical height, below `layout.height` when scrolling.
    view_height: Cell<i32>,
    scroll: Cell<i32>,
    hot: Cell<Option<usize>>,
    /// Previous list item opening this list.
    child_of: Option<usize>,
    screen: Cell<RECT>,
    tracking_leave: Cell<bool>,
    dpi: u32,
    style: MenuStyle,
    font: Font,
    /// Source menu for closing a specific context menu.
    menu: Option<Rc<MenuInner>>,
    /// List UIA elements created on screen reader request.
    accessible: RefCell<Option<Rc<TreeHost>>>,
}

impl Level {
    fn repaint(&self) {
        // SAFETY: invalida a própria janela popup; a pintura ocorre no WM_PAINT.
        unsafe { InvalidateRect(self.hwnd.get(), std::ptr::null(), FALSE) };
    }

    fn set_hot(&self, hot: Option<usize>) {
        if self.hot.get() != hot {
            self.hot.set(hot);
            if let Some(index) = hot {
                self.ensure_visible(index);
            }
            self.repaint();
            let host = self.accessible.borrow().clone();
            if let (Some(host), Some(index)) = (host, hot) {
                host.focus_changed(index as u32 + 1);
            }
            self.announce_highlight();
        }
    }

    /// Notifies [`Menu::on_highlight_changed`] of highlighted item.
    fn announce_highlight(&self) {
        if let Some(menu) = self.menu.clone() {
            let id = self.hot.get().and_then(|index| match self.entries.get(index).map(|e| &e.kind) {
                Some(EntryKind::Command(id)) => Some(*id),
                _ => None,
            });
            // Adiado: o manipulador da aplicação roda fora do rastreamento do menu.
            let _ = crate::core::defer(move || super::notify_highlight(&menu, id));
        }
    }

    /// List UIA elements, lazily created.
    fn accessible_host(&self) -> Option<Rc<TreeHost>> {
        let hwnd = self.hwnd.get();
        if hwnd.is_null() {
            return None;
        }
        let existing = self.accessible.borrow().clone();
        Some(existing.unwrap_or_else(|| {
            let host = TreeHost::new(PopupTree { hwnd });
            *self.accessible.borrow_mut() = Some(host.clone());
            host
        }))
    }

    fn contains(&self, point: POINT) -> bool {
        let r = self.screen.get();
        point.x >= r.left && point.x < r.right && point.y >= r.top && point.y < r.bottom
    }

    fn scrollable(&self) -> bool {
        self.layout.height > self.view_height.get()
    }

    fn max_scroll(&self) -> i32 {
        (self.layout.height - self.view_height.get()).max(0)
    }

    fn scroll_by(&self, delta: i32) {
        let value = (self.scroll.get() + delta).clamp(0, self.max_scroll());
        if value != self.scroll.get() {
            self.scroll.set(value);
            self.repaint();
        }
    }

    fn ensure_visible(&self, index: usize) {
        if !self.scrollable() {
            return;
        }
        let (top, height) = self.layout.rows[index];
        let band = SCROLL_BAND;
        let view = self.view_height.get();
        let mut scroll = self.scroll.get();
        if top - scroll < band {
            scroll = top - band;
        } else if top + height - scroll > view - band {
            scroll = top + height - view + band;
        }
        self.scroll.set(scroll.clamp(0, self.max_scroll()));
    }

    /// Nonseparator item at screen point.
    fn item_at(&self, point: POINT) -> Option<usize> {
        if !self.contains(point) {
            return None;
        }
        let r = self.screen.get();
        let y = to_logical(point.y - r.top, self.dpi);
        if self.scrollable() && (y < SCROLL_BAND || y >= self.view_height.get() - SCROLL_BAND) {
            return None;
        }
        let y = y + self.scroll.get();
        self.layout
            .rows
            .iter()
            .position(|&(top, height)| y >= top && y < top + height)
            .filter(|&index| !self.entries[index].is_separator())
    }

    /// Screen pixel rectangle of row `index`.
    fn row_screen_rect(&self, index: usize) -> RECT {
        let r = self.screen.get();
        let (top, height) = self.layout.rows[index];
        let top = r.top + to_pixels(top - self.scroll.get(), self.dpi);
        RECT { left: r.left, top, right: r.right, bottom: top + to_pixels(height, self.dpi) }
    }

    fn selectable(&self) -> Vec<usize> {
        (0..self.entries.len()).filter(|&i| !self.entries[i].is_separator()).collect()
    }

    fn step(&self, forward: bool) -> Option<usize> {
        let items = self.selectable();
        if items.is_empty() {
            return None;
        }
        let position = self.hot.get().and_then(|h| items.iter().position(|&i| i == h));
        Some(match position {
            None if forward => items[0],
            None => items[items.len() - 1],
            Some(p) if forward => items[(p + 1) % items.len()],
            Some(p) => items[(p + items.len() - 1) % items.len()],
        })
    }

    fn destroy(&self) {
        let hwnd = self.hwnd.replace(std::ptr::null_mut());
        if hwnd.is_null() {
            return;
        }
        POPUPS.with(|p| p.borrow_mut().remove(&(hwnd as isize)));
        let host = self.accessible.borrow_mut().take();
        if let Some(host) = &host {
            host.raise(0, crate::ffi::uiautomation::UIA_MENU_CLOSED_EVENT_ID);
        }
        // SAFETY: avisa as tecnologias assistivas (MSAA) e destrói a janela popup criada
        // por este módulo, nesta thread; nenhum empréstimo está ativo (a janela já saiu
        // do registro).
        unsafe {
            NotifyWinEvent(EVENT_SYSTEM_MENUPOPUPEND, hwnd, OBJID_CLIENT, 0);
            DestroyWindow(hwnd);
        }
        drop(host);
    }
}

/// New list placement.
enum Placement {
    /// Below a bar menu.
    Below(RECT),
    /// Beside a preceding-list item.
    Beside { parent: RECT, row: RECT },
    /// At a context menu point.
    At(POINT),
}

/// Menu mode state.
pub(crate) struct Tracker {
    owner: HWND,
    strip: Option<Rc<StripInner>>,
    style: MenuStyle,
    font: Font,
    dpi: u32,
    levels: RefCell<Vec<Rc<Level>>>,
    /// Visible underlines for keyboard menu mode.
    cues: Cell<bool>,
    /// Bar lists track cursor after opening.
    dropped: Cell<bool>,
    /// Timer-deferred action: (list, item to open) or (list, `None` to close subsequent lists).
    pending: Cell<Option<(usize, Option<usize>)>>,
    last_mouse: Cell<Option<(i32, i32)>>,
    /// Called when menu mode ends, restoring button appearance.
    on_end: RefCell<Option<Box<dyn FnOnce()>>>,
    /// Queried outside lists to switch to another same-bar button menu; returns `true` if switched.
    switch: RefCell<Option<SwitchFn>>,
}

/// Switches menu on button hover at a screen point.
pub(crate) type SwitchFn = Rc<dyn Fn(POINT) -> bool>;

pub(crate) fn current() -> Option<Rc<Tracker>> {
    TRACKER.with(|t| t.borrow().clone())
}

/// Active menu mode owner.
pub(crate) fn current_owner() -> Option<HWND> {
    current().map(|t| t.owner)
}

/// Visible underlines in `owner`'s bar.
pub(crate) fn cues_for(owner: HWND) -> bool {
    current().is_some_and(|t| t.owner == owner && t.cues.get())
}

/// Ends menu mode, closing all lists.
pub(crate) fn end() {
    let tracker = TRACKER.with(|t| t.borrow_mut().take());
    if let Some(tracker) = tracker {
        tracker.close_from(0);
        if let Some(strip) = &tracker.strip {
            strip.reset_after_tracking();
        }
        let on_end = tracker.on_end.borrow_mut().take();
        if let Some(on_end) = on_end {
            on_end();
        }
        let _ = crate::core::defer(super::notify_menu_closed);
    }
}

/// Ends mode if owned by `owner`.
pub(crate) fn cancel_for(owner: HWND) {
    if !owner.is_null() && current_owner() == Some(owner) {
        end();
    }
}

/// Closes context `menu` if open.
pub(crate) fn close_menu(menu: &Rc<MenuInner>) {
    let open = current()
        .is_some_and(|t| t.levels.borrow().first().and_then(|l| l.menu.clone()).is_some_and(|m| Rc::ptr_eq(&m, menu)));
    if open {
        end();
    }
}

fn begin(owner: HWND, strip: Option<Rc<StripInner>>) -> Rc<Tracker> {
    end();
    let style = match &strip {
        Some(s) => s.effective_style(),
        None => strip::style_for_window(owner),
    };
    let tracker = Rc::new(Tracker {
        owner,
        strip,
        style,
        font: strip::font_for_window(owner),
        dpi: native::dpi_for_window(owner),
        levels: RefCell::new(Vec::new()),
        cues: Cell::new(false),
        dropped: Cell::new(false),
        pending: Cell::new(None),
        last_mouse: Cell::new(Some(cursor_position())),
        on_end: RefCell::new(None),
        switch: RefCell::new(None),
    });
    TRACKER.with(|t| *t.borrow_mut() = Some(tracker.clone()));
    tracker
}

/// Tracker for `strip`, reusing current if applicable.
fn tracker_for_strip(strip: &Rc<StripInner>) -> Rc<Tracker> {
    match current() {
        Some(t) if t.owner == strip.owner() && t.strip.is_some() => t,
        _ => begin(strip.owner(), Some(strip.clone())),
    }
}

/// Opens list at bar position.
pub(crate) fn open_bar(strip: Rc<StripInner>, slot: usize, keyboard: bool) {
    if strip.owner().is_null() {
        return;
    }
    let tracker = tracker_for_strip(&strip);
    if keyboard {
        tracker.cues.set(true);
    }
    tracker.dropped.set(true);
    tracker.open_slot(slot, keyboard);
}

/// Enters keyboard menu mode selecting first bar menu.
pub(crate) fn enter_bar_keyboard_mode(strip: Rc<StripInner>) {
    let tracker = begin(strip.owner(), Some(strip.clone()));
    tracker.cues.set(true);
    strip.set_state(Some(0), None);
}

/// Keyboard-opens submenu of `item` in the first open list.
pub(crate) fn open_child_of_first(item: usize) {
    if let Some(tracker) = current() {
        if tracker.level(0).is_some_and(|l| item < l.entries.len()) {
            tracker.open_child(0, item, true);
        }
    }
}

/// Button-opened list content.
pub(crate) enum DropSource {
    Menu(Rc<MenuInner>),
    Entries(Vec<Entry>),
}

/// Opens bar-button list below screen rectangle `anchor`. `on_end` runs on mode exit; `switch` enables hover-switching
/// to another button.
pub(crate) fn open_dropdown(
    owner: HWND,
    source: DropSource,
    anchor: RECT,
    keyboard: bool,
    on_end: Box<dyn FnOnce()>,
    switch: Option<SwitchFn>,
) -> bool {
    let owner = native::root_of(owner);
    if owner.is_null() {
        on_end();
        return false;
    }
    let (menu, entries) = match source {
        DropSource::Menu(menu) => {
            menu.notify_open();
            let entries = menu.entries();
            (Some(menu), entries)
        }
        DropSource::Entries(entries) => (None, entries),
    };
    if entries.is_empty() {
        on_end();
        return false;
    }
    let tracker = begin(owner, None);
    *tracker.on_end.borrow_mut() = Some(on_end);
    *tracker.switch.borrow_mut() = switch;
    tracker.cues.set(keyboard || strip::system_keyboard_cues());
    tracker.push_level(menu, entries, Placement::Below(anchor), None, keyboard, 0);
    if tracker.levels.borrow().is_empty() {
        end();
        return false;
    }
    true
}

/// Opens drawn context menu at screen `at`.
pub(crate) fn open_context(owner: HWND, menu: Rc<MenuInner>, at: Point) {
    let owner = native::root_of(owner);
    if owner.is_null() {
        return;
    }
    menu.notify_open();
    let entries = menu.entries();
    if entries.is_empty() {
        return;
    }
    let tracker = begin(owner, None);
    tracker.cues.set(strip::system_keyboard_cues());
    tracker.push_level(Some(menu), entries, Placement::At(POINT { x: at.x, y: at.y }), None, false, 0);
    if tracker.levels.borrow().is_empty() {
        end();
    }
}

enum Hit {
    List(usize, Option<usize>),
    Bar(Option<usize>),
    Outside,
}

fn cursor_position() -> (i32, i32) {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: ponteiro válido para um POINT local.
    unsafe { GetCursorPos(&mut point) };
    (point.x, point.y)
}

fn is_popup(hwnd: HWND) -> bool {
    POPUPS.with(|p| p.borrow().contains_key(&(hwnd as isize)))
}

fn screen_point(msg: &MSG) -> POINT {
    let mut point = POINT { x: x_lparam(msg.lParam), y: y_lparam(msg.lParam) };
    let screen_coords = matches!(
        msg.message,
        WM_MOUSEWHEEL
            | WM_NCMOUSEMOVE
            | WM_NCLBUTTONDOWN
            | WM_NCLBUTTONUP
            | WM_NCLBUTTONDBLCLK
            | WM_NCRBUTTONDOWN
            | WM_NCRBUTTONUP
            | WM_NCRBUTTONDBLCLK
            | WM_NCMBUTTONDOWN
            | WM_NCMBUTTONUP
    );
    if !screen_coords {
        // SAFETY: ponteiro válido para um POINT local; a janela é desta thread.
        unsafe { ClientToScreen(msg.hwnd, &mut point) };
    }
    point
}

/// Message-loop entry: active menu mode/bar keyboard handles first. `true` means consumed.
pub(crate) fn pre_translate(msg: &MSG) -> bool {
    if let Some(tracker) = current() {
        return tracker.translate(msg);
    }
    strip::idle_translate(msg)
}

/// List position/size inside monitor work area.
fn place(placement: &Placement, width: i32, height: i32, work: RECT, row_offset: i32, overlap: i32) -> RECT {
    let height = height.min(work.bottom - work.top);
    let width = width.min(work.right - work.left);
    let (mut x, mut y) = match placement {
        Placement::Below(anchor) => {
            let y = if anchor.bottom + height <= work.bottom || anchor.top - height < work.top {
                anchor.bottom
            } else {
                anchor.top - height
            };
            (anchor.left, y)
        }
        Placement::Beside { parent, row } => {
            let x = if parent.right - overlap + width <= work.right {
                parent.right - overlap
            } else {
                parent.left - width + overlap
            };
            (x, row.top - row_offset)
        }
        Placement::At(p) => {
            let x = if p.x + width <= work.right { p.x } else { p.x - width };
            let y = if p.y + height <= work.bottom { p.y } else { p.y - height };
            (x, y)
        }
    };
    x = x.clamp(work.left, (work.right - width).max(work.left));
    y = y.clamp(work.top, (work.bottom - height).max(work.top));
    RECT { left: x, top: y, right: x + width, bottom: y + height }
}

fn register_class() -> bool {
    if CLASS_READY.with(Cell::get) {
        return true;
    }
    let name = to_wide(POPUP_CLASS);
    // SAFETY: cursor compartilhado do sistema (não precisa ser liberado).
    let cursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW as PCWSTR) };
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_DROPSHADOW,
        lpfnWndProc: Some(popup_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: native::instance(),
        hIcon: std::ptr::null_mut(),
        hCursor: cursor,
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: name.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };
    // SAFETY: estrutura completa; o nome vive durante a chamada.
    let ok = unsafe { RegisterClassExW(&class) } != 0 || unsafe { GetLastError() } == ERROR_CLASS_ALREADY_EXISTS;
    CLASS_READY.with(|c| c.set(ok));
    ok
}

fn create_popup(owner: HWND, rect: RECT, style: &MenuStyle) -> Option<HWND> {
    if !register_class() {
        return None;
    }
    let class = to_wide(POPUP_CLASS);
    let title = to_wide("");
    // SAFETY: strings terminadas em NUL; dono é uma janela viva desta thread.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            owner,
            std::ptr::null_mut(),
            native::instance(),
            std::ptr::null_mut(),
        )
    };
    if hwnd.is_null() {
        return None;
    }
    if WindowsVersion::current().is_windows_11_or_later() {
        // Windows 11: cantos arredondados e borda desenhados pelo DWM, na cor do estilo.
        let corner: u32 = 3; // DWMWCP_ROUNDSMALL
        let border = style.popup_border.to_colorref();
        // SAFETY: valores locais do tamanho informado; a janela é desta thread.
        unsafe {
            crate::ffi::dwmapi::DwmSetWindowAttribute(
                hwnd,
                crate::ffi::dwmapi::DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner as *const u32 as *const _,
                4,
            );
            crate::ffi::dwmapi::DwmSetWindowAttribute(
                hwnd,
                crate::ffi::dwmapi::DWMWA_BORDER_COLOR,
                &border as *const u32 as *const _,
                4,
            );
        }
    }
    Some(hwnd)
}

fn level_of(hwnd: HWND) -> Option<Rc<Level>> {
    POPUPS.with(|p| p.borrow().get(&(hwnd as isize)).cloned())
}

/// Popup list procedure.
///
/// # Safety
/// Called only by Windows on the window's owning thread.
unsafe extern "system" fn popup_proc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let result = catch_unwind(AssertUnwindSafe(|| match msg {
        WM_MOUSEACTIVATE => Some(MA_NOACTIVATE),
        WM_ERASEBKGND => Some(1),
        WM_PAINT => {
            let level = level_of(hwnd)?;
            // SAFETY: PAINTSTRUCT aceita zeros; par BeginPaint/EndPaint desta janela.
            unsafe {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                if !hdc.is_null() {
                    let client = native::client_rect(hwnd);
                    let cues = current().is_some_and(|t| t.cues.get());
                    crate::graphics::gdi::paint_buffered(
                        hdc,
                        client,
                        level.dpi,
                        Some(level.style.popup_background),
                        |canvas| paint_level(&level, canvas, cues),
                    );
                }
                EndPaint(hwnd, &ps);
            }
            Some(0)
        }
        WM_GETOBJECT if lp as i32 == crate::ffi::uiautomation::UIA_ROOT_OBJECT_ID => {
            let provider = level_of(hwnd)?.accessible_host()?.root_provider()?;
            let ret = crate::ffi::dynamic::optional().UiaReturnRawElementProvider?;
            // SAFETY: provedor vivo; a função acrescenta a própria referência.
            Some(unsafe { ret(hwnd, wp, lp, provider.as_ptr()) })
        }
        WM_TIMER if wp == OPEN_TIMER => {
            // SAFETY: encerra o temporizador da própria janela.
            unsafe { KillTimer(hwnd, OPEN_TIMER) };
            if let Some(tracker) = current() {
                tracker.on_timer(hwnd);
            }
            Some(0)
        }
        _ => None,
    }));
    match result {
        Ok(Some(value)) => value,
        Ok(None) => DefWindowProcW(hwnd, msg, wp, lp),
        Err(payload) => {
            app::store_panic(payload);
            0
        }
    }
}

/// Whether assistive technologies listen for UIA events.
pub(crate) fn clients_listening() -> bool {
    // SAFETY: consulta simples.
    crate::ffi::dynamic::optional().UiaClientsAreListening.is_some_and(|f| unsafe { f() } != 0)
}

/// Open list windows in active menu mode.
pub(crate) fn popup_hwnds() -> Vec<HWND> {
    current().map(|t| t.levels.borrow().iter().map(|l| l.hwnd.get()).collect()).unwrap_or_default()
}

/// Open list root UIA provider for tests.
pub(crate) fn accessible_root(hwnd: HWND) -> Option<ComPtr> {
    level_of(hwnd)?.accessible_host()?.root_provider()
}

/// Observable menu-mode state for tests.
#[doc(hidden)]
pub fn debug_state() -> Option<(usize, Vec<Option<usize>>)> {
    current().map(|t| (t.depth(), t.levels.borrow().iter().map(|l| l.hot.get()).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_reserves_columns_only_when_needed() {
        let item = |text, shortcut, submenu| Measured { separator: false, text, shortcut, submenu };
        let separator = Measured { separator: true, ..Measured::default() };
        let plain = compute_layout(&[item(50, 0, false), separator], 28, false, 0);
        assert_eq!(plain.text_x, INSET_X + TEXT_PAD);
        assert_eq!(plain.width, MIN_WIDTH);
        assert_eq!(plain.rows, vec![(BORDER + PAD_Y, 28), (BORDER + PAD_Y + 28, SEPARATOR)]);
        assert_eq!(plain.height, BORDER + PAD_Y + 28 + SEPARATOR + PAD_Y + BORDER);

        let rich = compute_layout(&[item(200, 60, true)], 28, true, 0);
        assert_eq!(rich.text_x, INSET_X + ICON_PAD + ICON + ICON_GAP);
        assert_eq!(rich.width, rich.text_x + 200 + SHORTCUT_GAP + 60 + ARROW_COLUMN + INSET_X);
        assert_eq!(compute_layout(&[], 28, false, 300).width, 300);
    }

    #[test]
    fn lists_stay_inside_the_work_area() {
        let work = RECT { left: 0, top: 0, right: 1000, bottom: 700 };
        let below = place(&Placement::Below(RECT { left: 950, top: 10, right: 990, bottom: 30 }), 200, 300, work, 5, 2);
        assert_eq!((below.left, below.top, below.right), (800, 30, 1000));
        let flipped =
            place(&Placement::Below(RECT { left: 10, top: 650, right: 60, bottom: 670 }), 200, 300, work, 5, 2);
        assert_eq!(flipped.bottom, 650, "abre para cima quando não cabe embaixo");
        let side = place(
            &Placement::Beside {
                parent: RECT { left: 700, top: 100, right: 900, bottom: 400 },
                row: RECT { left: 700, top: 140, right: 900, bottom: 168 },
            },
            200,
            100,
            work,
            5,
            2,
        );
        assert_eq!((side.left, side.top), (502, 135), "abre à esquerda quando não cabe à direita");
        let tall = place(&Placement::At(POINT { x: 10, y: 10 }), 100, 5000, work, 5, 2);
        assert_eq!((tall.top, tall.bottom), (0, 700), "limita a altura à área útil");
        let corner = place(&Placement::At(POINT { x: 990, y: 690 }), 100, 100, work, 5, 2);
        assert_eq!((corner.left, corner.top), (890, 590));
    }
}
