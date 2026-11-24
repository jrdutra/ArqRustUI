//! Shared window/control core: state, registry and message dispatch.

use super::application as app;
use super::errors::{GuiError, GuiResult};
use super::events::{Event, EventKind, HandlerId};
use super::messages::RawMessage;
use super::native;
use crate::ffi::gdi32::{SetBkColor, SetTextColor};
use crate::ffi::imm32;
use crate::ffi::shell32::{DragFinish, DragQueryFileW};
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::graphics::{Canvas, Font};
use crate::input::{
    Key, KeyEvent, Modifiers, MouseButton, MouseEvent, PointerEvent, PointerKind, PointerPhase, Shortcut,
};
use crate::layout::Layout;
use crate::menus::ContextMenu;
use crate::system::Cursor;
use crate::themes::Theme;
use crate::utilities::{from_wide, to_logical, to_pixels, to_wide, Color, Point, Rect, Size};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::rc::Rc;

mod dispatch;

thread_local! {
    static REGISTRY: RefCell<HashMap<isize, Rc<WidgetCore>>> = RefCell::new(HashMap::new());
}

/// Finds the core for an `HWND` on this thread.
pub(crate) fn lookup(hwnd: HWND) -> Option<Rc<WidgetCore>> {
    if hwnd.is_null() {
        return None;
    }
    REGISTRY.with(|r| r.borrow().get(&(hwnd as isize)).cloned())
}

/// All live cores on this thread.
pub(crate) fn all_cores() -> Vec<Rc<WidgetCore>> {
    REGISTRY.with(|r| r.borrow().values().cloned().collect())
}

/// Notification reflected from parent to originating control.
pub(crate) enum Reflected {
    /// `WM_COMMAND`: notification code and identifier.
    Command { code: u16, id: u16 },
    /// `WM_NOTIFY`: code and original `LPARAM` (`NM*` structure pointer).
    Notify { code: i32, lparam: LPARAM },
    /// `WM_HSCROLL` / `WM_VSCROLL`: scroll bar code.
    Scroll { code: u16 },
    /// `WM_DRAWITEM`: original `LPARAM` (`DRAWITEMSTRUCT` pointer).
    DrawItem { lparam: LPARAM },
}

type EventFn = Rc<dyn Fn(&Event)>;
type ReflectFn = Rc<dyn Fn(&Rc<WidgetCore>, &Reflected) -> Option<LRESULT>>;
type RawHookFn = Rc<dyn Fn(&RawMessage) -> Option<isize>>;
type PaintFn = Rc<dyn Fn(&Canvas)>;
/// Keyboard shortcut action.
pub(crate) type ShortcutAction = (Shortcut, Rc<dyn Fn()>);

struct Handler {
    id: u64,
    kind: EventKind,
    callback: EventFn,
}

/// Internal window/control state.
///
/// Exactly one `WidgetCore` exists per library-created `HWND`. The thread registry retains it while the native window
/// exists; `WM_NCDESTROY` detaches it. Methods then become safe no-ops and [`Widget::is_alive`] returns `false`.
///
/// Opaque type: use [`Widget`] methods.
pub struct WidgetCore {
    hwnd: Cell<HWND>,
    original_proc: Cell<isize>,
    top_level: bool,
    handlers: RefCell<Vec<Handler>>,
    next_handler: Cell<u64>,
    pub(crate) close_handler: RefCell<Option<Rc<dyn Fn() -> bool>>>,
    raw_hook: RefCell<Option<RawHookFn>>,
    /// Internal filters run before the application's `on_raw_message` hook, which does not replace them.
    filters: RefCell<Vec<RawHookFn>>,
    /// Single-line text vertically centered with nonclient strips above/below the text area (text fields, date pickers,
    /// etc.).
    centered_text: Cell<bool>,
    /// Top/bottom strips in pixels applied at the latest `WM_NCCALCSIZE`.
    text_inset: Cell<(i32, i32)>,
    reflect: RefCell<Option<ReflectFn>>,
    pub(crate) paint: RefCell<Option<PaintFn>>,
    pub(crate) double_buffered: Cell<bool>,
    logical_bounds: Cell<Option<Rect>>,
    font: RefCell<Option<Font>>,
    font_handle: RefCell<Option<Rc<crate::core::handles::GdiObject>>>,
    pub(crate) foreground: Cell<Option<Color>>,
    pub(crate) background: Cell<Option<Color>>,
    pub(crate) theme: RefCell<Option<Rc<Theme>>>,
    layout: RefCell<Option<Rc<dyn Layout>>>,
    pub(crate) min_size: Cell<Option<Size>>,
    pub(crate) max_size: Cell<Option<Size>>,
    cursor: RefCell<Option<Cursor>>,
    mouse_inside: Cell<bool>,
    pressed: Cell<bool>,
    context_menu: RefCell<Option<ContextMenu>>,
    pub(crate) shortcuts: RefCell<Vec<ShortcutAction>>,
    /// Native window menu bar: item shortcuts queried on each key.
    pub(crate) menu_bar: RefCell<Option<crate::menus::MenuBar>>,
    last_focus: Cell<HWND>,
    pub(crate) modal_owner: Cell<HWND>,
    pub(crate) close_on_escape: Cell<bool>,
    pub(crate) saved_placement: Cell<Option<(u32, WINDOWPLACEMENT)>>,
    pub(crate) button_style: Cell<Option<crate::themes::ButtonStyle>>,
    pub(crate) button_content_alignment: Cell<crate::graphics::TextAlign>,
    pub(crate) tooltip: Cell<HWND>,
    pub(crate) accessible: RefCell<Option<crate::ffi::ole32::ComPtr>>,
    /// Virtual UIA elements of a drawn control (`accessibility::tree`).
    pub(crate) accessible_tree: RefCell<Option<Rc<crate::accessibility::tree::TreeHost>>>,
    /// OLE drop target and drag reordering (`input::drag`).
    pub(crate) drag: RefCell<Option<Rc<crate::input::drag::DragState>>>,
    keep_alive: RefCell<Vec<Box<dyn Any>>>,
    /// Control's own icon (buttons/labels).
    pub(crate) icon: RefCell<Option<crate::controls::icon_support::WidgetIcon>>,
    /// Library-managed image list (trees, lists, tabs, etc.).
    pub(crate) managed_images: RefCell<Option<crate::controls::icon_support::ManagedImages>>,
    pub(crate) kind: &'static str,
}

impl std::fmt::Debug for WidgetCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetCore").field("kind", &self.kind).field("hwnd", &self.hwnd.get()).finish()
    }
}

/// Native window creation parameters.
pub(crate) struct CreateParams<'a> {
    pub kind: &'static str,
    pub class: &'a str,
    pub text: &'a str,
    pub style: u32,
    pub ex_style: u32,
    /// Parent (controls) or owner (top-level windows).
    pub parent: HWND,
    /// Rectangle in physical pixels.
    pub pixel_bounds: RECT,
    /// Control identifier (0 for top-level windows).
    pub id: u16,
    pub top_level: bool,
    /// `true` for system classes requiring subclassing to generate events.
    pub subclass: bool,
}

impl WidgetCore {
    /// Creates the native window and registers its core.
    pub(crate) fn create(params: CreateParams<'_>) -> GuiResult<Rc<WidgetCore>> {
        app::require()?;
        let class = to_wide(params.class);
        let text = to_wide(params.text);
        let r = params.pixel_bounds;
        // SAFETY: strings terminadas em NUL e válidas durante a chamada; o pai é
        // nulo ou um HWND desta thread; falha é sinalizada por retorno nulo.
        let hwnd = unsafe {
            CreateWindowExW(
                params.ex_style,
                class.as_ptr(),
                text.as_ptr(),
                params.style,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                params.parent,
                params.id as usize as HMENU,
                native::instance(),
                std::ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err(GuiError::last_os_error("CreateWindowExW"));
        }
        Ok(Self::register(hwnd, params.kind, params.top_level, params.subclass))
    }

    /// Registers/subclasses a child window created by Windows, e.g. a ListView header. Returns the existing core if
    /// already registered.
    pub(crate) fn adopt(hwnd: HWND, kind: &'static str) -> Option<Rc<WidgetCore>> {
        // SAFETY: consulta simples.
        if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
            return None;
        }
        if let Some(existing) = lookup(hwnd) {
            return Some(existing);
        }
        Some(Self::register(hwnd, kind, false, true))
    }

    fn register(hwnd: HWND, kind: &'static str, top_level: bool, subclass: bool) -> Rc<WidgetCore> {
        let core = Rc::new(WidgetCore {
            hwnd: Cell::new(hwnd),
            original_proc: Cell::new(0),
            top_level,
            handlers: RefCell::new(Vec::new()),
            next_handler: Cell::new(1),
            close_handler: RefCell::new(None),
            raw_hook: RefCell::new(None),
            filters: RefCell::new(Vec::new()),
            centered_text: Cell::new(false),
            text_inset: Cell::new((0, 0)),
            reflect: RefCell::new(None),
            paint: RefCell::new(None),
            double_buffered: Cell::new(true),
            logical_bounds: Cell::new(None),
            font: RefCell::new(None),
            font_handle: RefCell::new(None),
            foreground: Cell::new(None),
            background: Cell::new(None),
            theme: RefCell::new(None),
            layout: RefCell::new(None),
            min_size: Cell::new(None),
            max_size: Cell::new(None),
            cursor: RefCell::new(None),
            mouse_inside: Cell::new(false),
            pressed: Cell::new(false),
            context_menu: RefCell::new(None),
            shortcuts: RefCell::new(Vec::new()),
            menu_bar: RefCell::new(None),
            last_focus: Cell::new(std::ptr::null_mut()),
            modal_owner: Cell::new(std::ptr::null_mut()),
            close_on_escape: Cell::new(false),
            saved_placement: Cell::new(None),
            button_style: Cell::new(None),
            button_content_alignment: Cell::new(crate::graphics::TextAlign::Center),
            tooltip: Cell::new(std::ptr::null_mut()),
            accessible: RefCell::new(None),
            accessible_tree: RefCell::new(None),
            drag: RefCell::new(None),
            keep_alive: RefCell::new(Vec::new()),
            icon: RefCell::new(None),
            managed_images: RefCell::new(None),
            kind,
        });
        REGISTRY.with(|r| r.borrow_mut().insert(hwnd as isize, core.clone()));
        if subclass {
            // SAFETY: troca o procedimento de uma janela recém-criada nesta thread por
            // `window_proc`, guardando o original para encaminhar as mensagens.
            let previous = unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize) };
            core.original_proc.set(previous);
        }
        if top_level {
            app::top_level_created();
        }
        core
    }

    /// Creates a child of `parent` with logical bounds.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_child(
        kind: &'static str,
        parent: &Rc<WidgetCore>,
        class: &str,
        text: &str,
        style: u32,
        ex_style: u32,
        bounds: Rect,
        id: Option<u16>,
    ) -> GuiResult<Rc<WidgetCore>> {
        let parent_hwnd = parent.hwnd();
        if parent_hwnd.is_null() {
            return Err(GuiError::WidgetDestroyed);
        }
        let dpi = native::dpi_for_window(parent_hwnd);
        let subclass = !class.starts_with("ArqRustUI.");
        let core = Self::create(CreateParams {
            kind,
            class,
            text,
            style: style | WS_CHILD,
            ex_style,
            parent: parent_hwnd,
            pixel_bounds: scale_rect(bounds, dpi),
            id: id.unwrap_or_else(app::next_control_id),
            top_level: false,
            subclass,
        })?;
        core.logical_bounds.set(Some(bounds));
        core.refresh_font();
        crate::themes::apply_to_new_control(&core);
        Ok(core)
    }

    /// Native handle, null after window destruction.
    pub(crate) fn hwnd(&self) -> HWND {
        self.hwnd.get()
    }

    /// Whether the mouse cursor is over the control.
    pub(crate) fn is_hovered(&self) -> bool {
        self.mouse_inside.get()
    }

    pub(crate) fn is_alive(&self) -> bool {
        !self.hwnd.get().is_null()
    }

    pub(crate) fn is_top_level(&self) -> bool {
        self.top_level
    }

    pub(crate) fn require_alive(&self) -> GuiResult<HWND> {
        let hwnd = self.hwnd.get();
        if hwnd.is_null() {
            Err(GuiError::WidgetDestroyed)
        } else {
            Ok(hwnd)
        }
    }

    pub(crate) fn dpi(&self) -> u32 {
        native::dpi_for_window(self.hwnd.get())
    }

    /// Core of the containing top-level window.
    pub(crate) fn root(&self) -> Option<Rc<WidgetCore>> {
        lookup(native::root_of(self.hwnd.get()))
    }

    /// Registers an event handler.
    pub(crate) fn on(&self, kind: EventKind, callback: impl Fn(&Event) + 'static) -> HandlerId {
        let id = self.next_handler.get();
        self.next_handler.set(id + 1);
        if self.is_alive() {
            self.handlers.borrow_mut().push(Handler { id, kind, callback: Rc::new(callback) });
        }
        HandlerId(id)
    }

    pub(crate) fn remove_handler(&self, id: HandlerId) {
        self.handlers.borrow_mut().retain(|h| h.id != id.0);
    }

    pub(crate) fn has_handler(&self, kind: EventKind) -> bool {
        self.handlers.borrow().iter().any(|h| h.kind == kind)
    }

    /// Delivers an event to registered handlers. Returns `true` if any existed.
    ///
    /// Copies handlers before invocation, allowing callbacks to safely register/remove handlers or destroy the control.
    pub(crate) fn emit(&self, event: &Event) -> bool {
        let kind = event.kind();
        let callbacks: Vec<EventFn> = self
            .handlers
            .borrow()
            .iter()
            .filter(|h| h.kind == kind || h.kind == EventKind::Any)
            .map(|h| h.callback.clone())
            .collect();
        for callback in &callbacks {
            callback(event);
        }
        !callbacks.is_empty()
    }

    pub(crate) fn set_reflect(&self, f: impl Fn(&Rc<WidgetCore>, &Reflected) -> Option<LRESULT> + 'static) {
        *self.reflect.borrow_mut() = Some(Rc::new(f));
    }

    /// Adds an internal filter before the application hook. Control receiving focus when this top-level window regains
    /// it.
    pub(crate) fn remember_focus(&self, hwnd: HWND) {
        self.last_focus.set(hwnd);
    }

    pub(crate) fn add_message_filter(&self, f: impl Fn(&RawMessage) -> Option<isize> + 'static) {
        self.filters.borrow_mut().push(Rc::new(f));
    }

    pub(crate) fn set_raw_hook(&self, f: impl Fn(&RawMessage) -> Option<isize> + 'static) {
        *self.raw_hook.borrow_mut() = Some(Rc::new(f));
    }

    /// Keeps a resource alive for the window lifetime (image lists, menus, etc.).
    pub(crate) fn keep_alive(&self, resource: impl Any) {
        if self.is_alive() {
            self.keep_alive.borrow_mut().push(Box::new(resource));
        }
    }

    // ---- Geometria ---------------------------------------------------------

    pub(crate) fn set_bounds(&self, bounds: Rect) {
        let hwnd = self.hwnd.get();
        if hwnd.is_null() {
            return;
        }
        if self.top_level {
            let dpi = self.dpi();
            let outer = crate::core::window::outer_rect_for_client(
                hwnd,
                Size::new(to_pixels(bounds.width, dpi), to_pixels(bounds.height, dpi)),
            );
            // SAFETY: reposiciona uma janela viva desta thread.
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    bounds.x,
                    bounds.y,
                    outer.width,
                    outer.height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
        } else {
            self.logical_bounds.set(Some(bounds));
            let r = scale_rect(bounds, self.dpi());
            // SAFETY: reposiciona um controle vivo desta thread.
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            self.fit_native_height();
        }
    }

    /// Requested logical child bounds (`None` for top-level windows).
    pub(crate) fn logical_bounds(&self) -> Option<Rect> {
        self.logical_bounds.get()
    }

    pub(crate) fn bounds(&self) -> Rect {
        let hwnd = self.hwnd.get();
        if hwnd.is_null() {
            return Rect::default();
        }
        if self.top_level {
            let outer = native::window_rect(hwnd);
            let size = self.client_size();
            Rect::new(outer.left, outer.top, size.width, size.height)
        } else if let Some(bounds) = self.logical_bounds.get() {
            bounds
        } else {
            let dpi = self.dpi();
            let r = native::rect_in_parent(hwnd);
            Rect::new(
                to_logical(r.left, dpi),
                to_logical(r.top, dpi),
                to_logical(r.right - r.left, dpi),
                to_logical(r.bottom - r.top, dpi),
            )
        }
    }

    /// Logical client size.
    pub(crate) fn client_size(&self) -> Size {
        let hwnd = self.hwnd.get();
        if hwnd.is_null() {
            return Size::default();
        }
        let dpi = self.dpi();
        let r = native::client_rect(hwnd);
        Size::new(to_logical(r.right, dpi), to_logical(r.bottom, dpi))
    }

    // ---- Fonte, cores, layout ------------------------------------------------

    pub(crate) fn set_font(&self, font: Font) {
        *self.font.borrow_mut() = Some(font);
        self.refresh_font();
    }

    pub(crate) fn font(&self) -> Font {
        if let Some(font) = self.font.borrow().clone() {
            return font;
        }
        let themed = self.root().and_then(|r| r.theme.borrow().as_ref().and_then(|t| t.font.clone()));
        themed.unwrap_or_else(Font::system)
    }

    /// Recreates and applies the native font at current DPI.
    pub(crate) fn refresh_font(&self) {
        let hwnd = self.hwnd.get();
        if hwnd.is_null() {
            return;
        }
        if let Ok(handle) = self.font().handle_for_dpi(self.dpi()) {
            native::send(hwnd, WM_SETFONT, handle.raw() as WPARAM, 1);
            *self.font_handle.borrow_mut() = Some(handle);
        }
        self.fit_native_height();
    }

    /// Corrects native controls whose height is font-driven, such as ComboBox closed height, to requested logical
    /// height.
    fn fit_native_height(&self) {
        if self.kind == "ComboBox" {
            crate::controls::combobox::fit_field_height(self);
        }
        if self.kind == "ListBox" {
            crate::controls::listbox::refit_items(self);
        }
        if self.centered_text.get() {
            self.recalc_frame();
        }
    }

    /// Vertically centers single-line controls drawing at the client area's top (EDIT, SysDateTimePick32, etc.). Equal
    /// nonclient strips reduce the area to text height, use the control background and forward clicks to text.
    pub(crate) fn set_centered_text(&self, centered: bool) {
        self.centered_text.set(centered);
        self.recalc_frame();
    }

    /// Nonclient strips for `centered_text`.
    fn handle_text_inset(&self, hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_NCCALCSIZE if lp != 0 => {
                let result = self.default_proc(hwnd, msg, wp, lp);
                // SAFETY: com WPARAM verdadeiro o LPARAM aponta para NCCALCSIZE_PARAMS, e
                // com WPARAM falso para um RECT; ambos graváveis durante a mensagem.
                let rect = unsafe {
                    if wp != 0 {
                        &mut (*(lp as *mut NCCALCSIZE_PARAMS)).rgrc[0]
                    } else {
                        &mut *(lp as *mut RECT)
                    }
                };
                let (top, bottom) = self.text_insets(hwnd, rect.bottom - rect.top);
                rect.top += top;
                rect.bottom -= bottom;
                self.text_inset.set((top, bottom));
                Some(result)
            }
            WM_NCPAINT => {
                let result = self.default_proc(hwnd, msg, wp, lp);
                self.paint_text_insets(hwnd);
                Some(result)
            }
            // Habilitar, desabilitar e "somente leitura" trocam o fundo do texto.
            WM_ENABLE | EM_SETREADONLY => {
                let result = self.default_proc(hwnd, msg, wp, lp);
                self.redraw_frame();
                Some(result)
            }
            WM_NCHITTEST => {
                let result = self.default_proc(hwnd, msg, wp, lp);
                let hit = result as u16;
                if hit != HTNOWHERE && hit != HTBORDER {
                    return Some(result);
                }
                // Dentro das faixas: vale como área cliente (o clique posiciona o cursor).
                let (top, bottom) = self.text_inset.get();
                let mut client = RECT::default();
                let mut origin = POINT { x: 0, y: 0 };
                // SAFETY: consultas de geometria de uma janela viva.
                unsafe {
                    GetClientRect(hwnd, &mut client);
                    ClientToScreen(hwnd, &mut origin);
                }
                let (x, y) = (x_lparam(lp), y_lparam(lp));
                let inside = x >= origin.x
                    && x < origin.x + client.right
                    && y >= origin.y - top
                    && y < origin.y + client.bottom + bottom;
                Some(if inside { HTCLIENT as LRESULT } else { result })
            }
            _ => None,
        }
    }

    /// Requests Windows nonclient recalculation (`WM_NCCALCSIZE`).
    fn recalc_frame(&self) {
        let hwnd = self.hwnd.get();
        if !hwnd.is_null() {
            // SAFETY: só recalcula o quadro de uma janela viva desta thread.
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
        }
    }

    /// Top/bottom pixel strips centering one text line in `height` pixels.
    fn text_insets(&self, hwnd: HWND, height: i32) -> (i32, i32) {
        let line = crate::controls::icon_support::text_height(hwnd) + to_pixels(2, self.dpi());
        let spare = (height - line).max(0);
        (spare / 2, spare - spare / 2)
    }

    /// Paints `centered_text` strips with the background brush the parent supplies for the control, respecting themes.
    fn paint_text_insets(&self, hwnd: HWND) {
        let mut window = RECT::default();
        let mut client = RECT::default();
        // SAFETY: consultas de geometria e um DC de janela liberado no fim; o pincel
        // devolvido pelo pai (ou pelo sistema) não é destruído aqui.
        unsafe {
            GetWindowRect(hwnd, &mut window);
            GetClientRect(hwnd, &mut client);
            let mut origin = POINT { x: 0, y: 0 };
            ClientToScreen(hwnd, &mut origin);
            let (top, bottom) = self.text_inset.get();
            if top + bottom == 0 {
                return;
            }
            let left = origin.x - window.left;
            let y = origin.y - window.top;
            let strips = [
                RECT { left, top: y - top, right: left + client.right, bottom: y },
                RECT { left, top: y + client.bottom, right: left + client.right, bottom: y + client.bottom + bottom },
            ];
            let hdc = GetWindowDC(hwnd);
            if hdc.is_null() {
                return;
            }
            let read_only = native::style(hwnd) & ES_READONLY != 0 && self.class_is_edit();
            let message = if read_only || IsWindowEnabled(hwnd) == 0 { WM_CTLCOLORSTATIC } else { WM_CTLCOLOREDIT };
            let parent = GetParent(hwnd);
            let mut brush =
                if parent.is_null() { 0 } else { SendMessageW(parent, message, hdc as WPARAM, hwnd as LPARAM) };
            if brush == 0 {
                let index = if message == WM_CTLCOLORSTATIC { COLOR_BTNFACE } else { COLOR_WINDOW };
                brush = GetSysColorBrush(index) as LRESULT;
            }
            for strip in &strips {
                FillRect(hdc, strip, brush as HBRUSH);
            }
            ReleaseDC(hwnd, hdc);
        }
    }

    fn class_is_edit(&self) -> bool {
        matches!(self.kind, "TextBox" | "PasswordBox" | "SpinButton")
    }

    pub(crate) fn set_layout(&self, layout: Option<Rc<dyn Layout>>) {
        *self.layout.borrow_mut() = layout;
        self.relayout();
    }

    /// Reapplies any attached layout to current client area. Whether the core has an attached layout.
    pub(crate) fn has_layout(&self) -> bool {
        self.layout.borrow().is_some()
    }

    pub(crate) fn relayout(&self) {
        let layout = self.layout.borrow().clone();
        if let Some(layout) = layout {
            let size = self.client_size();
            layout.arrange(Rect::new(0, 0, size.width, size.height));
        }
    }

    pub(crate) fn set_cursor(&self, cursor: Option<Cursor>) {
        *self.cursor.borrow_mut() = cursor;
    }

    pub(crate) fn set_context_menu(&self, menu: Option<ContextMenu>) {
        *self.context_menu.borrow_mut() = menu;
    }

    /// Effective background: custom, window theme or inherited from parent.
    pub(crate) fn effective_background(&self) -> Option<Color> {
        if let Some(color) = self.background.get() {
            return Some(color);
        }
        if self.top_level {
            return self.theme.borrow().as_ref().map(|t| t.background);
        }
        // SAFETY: consulta simples do pai de uma janela.
        let parent = unsafe { GetParent(self.hwnd.get()) };
        lookup(parent).and_then(|p| p.effective_background())
    }

    pub(crate) fn invalidate(&self) {
        let hwnd = self.hwnd.get();
        if !hwnd.is_null() {
            // SAFETY: invalida a janela viva e seus descendentes, sem pintura
            // síncrona/reentrante. Controles movidos ou ocultados não deixam
            // áreas antigas sobre os controles que passaram a ocupar seu lugar.
            unsafe {
                RedrawWindow(hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN)
            };
            if self.centered_text.get() {
                self.redraw_frame();
            }
        }
    }

    /// Repaints nonclient area (`centered_text` strips change with control state).
    fn redraw_frame(&self) {
        let hwnd = self.hwnd.get();
        if !hwnd.is_null() {
            // SAFETY: só agenda a repintura do quadro de uma janela viva.
            unsafe { RedrawWindow(hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_FRAME) };
        }
    }

    pub(crate) fn destroy(&self) {
        let hwnd = self.hwnd.get();
        if !hwnd.is_null() {
            // SAFETY: a janela pertence a esta thread; WM_NCDESTROY desfaz o registro.
            unsafe { DestroyWindow(hwnd) };
        }
    }
}

/// Converts logical rectangle to pixel `RECT` at the given DPI.
pub(crate) fn scale_rect(bounds: Rect, dpi: u32) -> RECT {
    let left = to_pixels(bounds.x, dpi);
    let top = to_pixels(bounds.y, dpi);
    RECT { left, top, right: left + to_pixels(bounds.width, dpi), bottom: top + to_pixels(bounds.height, dpi) }
}

fn dropped_files(drop: HDROP) -> Vec<PathBuf> {
    let mut files = Vec::new();
    // SAFETY: `drop` é o HDROP recebido em WM_DROPFILES; cada buffer tem o tamanho
    // informado pela própria API, e DragFinish libera o handle ao final.
    unsafe {
        let count = DragQueryFileW(drop, 0xFFFF_FFFF, std::ptr::null_mut(), 0);
        for index in 0..count {
            let len = DragQueryFileW(drop, index, std::ptr::null_mut(), 0) as usize;
            let mut buffer = vec![0u16; len + 1];
            DragQueryFileW(drop, index, buffer.as_mut_ptr(), buffer.len() as u32);
            files.push(PathBuf::from(from_wide(&buffer)));
        }
        DragFinish(drop);
    }
    files
}

fn pointer_kind(id: u32) -> PointerKind {
    let Some(get) = crate::ffi::dynamic::optional().GetPointerType else {
        return PointerKind::Other;
    };
    let mut kind = 0u32;
    // SAFETY: ponteiro de saída válido; a função falha de forma limpa para ids desconhecidos.
    if unsafe { get(id, &mut kind) } == 0 {
        return PointerKind::Other;
    }
    match kind {
        2 => PointerKind::Touch,
        3 => PointerKind::Pen,
        4 => PointerKind::Mouse,
        _ => PointerKind::Other,
    }
}

/// Single library window procedure for own classes and subclassed controls.
///
/// # Safety
/// Called only by Windows on the window's owning thread.
pub(crate) unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let Some(core) = lookup(hwnd) else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    // Um panic não pode atravessar a fronteira FFI: ele é capturado aqui e
    // relançado pelo loop de eventos, já fora do callback do sistema.
    match catch_unwind(AssertUnwindSafe(|| core.handle(hwnd, msg, wp, lp))) {
        Ok(result) => result,
        Err(payload) => {
            app::store_panic(payload);
            0
        }
    }
}

/// Common window/control interface.
///
/// All visual types implement `Widget`. Cheap cloneable *handles*: cloning does not duplicate controls; dropping does
/// not destroy them. Controls live until [`destroy`](Widget::destroy) or parent destruction.
///
/// Coordinates/sizes are always **logical** (96 DPI); the library converts to monitor pixels.
pub trait Widget {
    /// Internal control core.
    #[doc(hidden)]
    fn core(&self) -> &Rc<WidgetCore>;

    /// Numeric `HWND` (advanced API), zero after destruction.
    fn handle(&self) -> isize {
        self.core().hwnd() as isize
    }

    /// `false` after native window destruction.
    fn is_alive(&self) -> bool {
        self.core().is_alive()
    }

    /// Destroys the control and children; repeated calls are harmless.
    fn destroy(&self) {
        self.core().destroy();
    }

    /// Shows the control.
    fn show(&self) {
        self.set_visible(true);
    }

    /// Hides the control.
    fn hide(&self) {
        self.set_visible(false);
    }

    /// Sets visibility.
    fn set_visible(&self, visible: bool) {
        // SAFETY: ShowWindow tolera HWND nulo/inválido.
        unsafe { ShowWindow(self.core().hwnd(), if visible { SW_SHOW } else { SW_HIDE }) };
    }

    /// Whether the control and all ancestors are visible.
    fn is_visible(&self) -> bool {
        // SAFETY: consulta simples.
        unsafe { IsWindowVisible(self.core().hwnd()) != 0 }
    }

    /// Enables/disables user interaction.
    fn set_enabled(&self, enabled: bool) {
        // SAFETY: EnableWindow tolera HWND nulo/inválido.
        unsafe { EnableWindow(self.core().hwnd(), enabled as BOOL) };
    }

    /// Whether the control accepts interaction.
    fn is_enabled(&self) -> bool {
        // SAFETY: consulta simples.
        unsafe { IsWindowEnabled(self.core().hwnd()) != 0 }
    }

    /// Control text (title, label or content according to type).
    fn text(&self) -> String {
        native::window_text(self.core().hwnd())
    }

    /// Sets control text.
    fn set_text(&self, text: &str) {
        native::set_window_text(self.core().hwnd(), text);
    }

    /// Control rectangle in logical units relative to parent.
    ///
    /// For top-level windows, `x`/`y` are screen pixels; size is logical client size.
    fn bounds(&self) -> Rect {
        self.core().bounds()
    }

    /// Repositions/resizes the control.
    fn set_bounds(&self, bounds: Rect) {
        self.core().set_bounds(bounds);
    }

    /// Control position.
    fn position(&self) -> Point {
        self.bounds().position()
    }

    /// Moves while retaining size.
    fn set_position(&self, x: i32, y: i32) {
        let b = self.bounds();
        self.set_bounds(Rect::new(x, y, b.width, b.height));
    }

    /// Control size.
    fn size(&self) -> Size {
        self.bounds().size()
    }

    /// Resizes while retaining position.
    fn set_size(&self, width: i32, height: i32) {
        let b = self.bounds();
        self.set_bounds(Rect::new(b.x, b.y, width, height));
    }

    /// Logical client size (usable inner area).
    fn client_size(&self) -> Size {
        self.core().client_size()
    }

    /// Gives keyboard focus to the control.
    fn focus(&self) {
        // SAFETY: SetFocus tolera HWND nulo/inválido.
        unsafe { SetFocus(self.core().hwnd()) };
    }

    /// Whether the control has keyboard focus.
    fn has_focus(&self) -> bool {
        let hwnd = self.core().hwnd();
        // SAFETY: consulta simples.
        !hwnd.is_null() && unsafe { GetFocus() } == hwnd
    }

    /// Adds/removes the control from tab order.
    fn set_tab_stop(&self, tab_stop: bool) {
        native::set_style_bits(self.core().hwnd(), GWL_STYLE, WS_TABSTOP, tab_stop);
    }

    /// Sets the control font.
    fn set_font(&self, font: Font) {
        self.core().set_font(font);
    }

    /// Effective control font.
    fn font(&self) -> Font {
        self.core().font()
    }

    /// Sets text color.
    fn set_foreground(&self, color: Color) {
        self.core().foreground.set(Some(color));
        crate::themes::refresh_control_colors(self.core());
    }

    /// Sets background color.
    fn set_background(&self, color: Color) {
        self.core().background.set(Some(color));
        crate::themes::refresh_control_colors(self.core());
    }

    /// Sets hover cursor (`None` restores default).
    fn set_cursor(&self, cursor: Option<Cursor>) {
        self.core().set_cursor(cursor);
    }

    /// Sets tooltip text; empty removes it.
    fn set_tooltip(&self, text: &str) -> GuiResult<()> {
        crate::controls::tooltip::set_for(self.core(), text)
    }

    /// Attaches a context menu for right click/menu key.
    fn set_context_menu(&self, menu: &ContextMenu) {
        self.core().set_context_menu(Some(menu.clone()));
    }

    /// Requests control repaint.
    fn invalidate(&self) {
        self.core().invalidate();
    }

    /// Effective DPI of the control's monitor.
    fn dpi(&self) -> u32 {
        self.core().dpi()
    }

    /// Scale factor (`dpi / 96`).
    fn scale_factor(&self) -> f32 {
        self.dpi() as f32 / 96.0
    }

    /// Sets the screen reader accessible name.
    fn set_accessible_name(&self, name: &str) -> GuiResult<()> {
        crate::accessibility::set_name(self.core(), name)
    }

    /// Sets accessible description/help text.
    fn set_accessible_description(&self, description: &str) -> GuiResult<()> {
        crate::accessibility::set_description(self.core(), description)
    }

    // ---- Eventos ---------------------------------------------------------------

    /// Registers a handler for any control event.
    fn on_event(&self, f: impl Fn(&Event) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::Any, f)
    }

    /// Removes a registered handler.
    fn remove_handler(&self, id: HandlerId) {
        self.core().remove_handler(id);
    }

    /// Click.
    fn on_click(&self, f: impl Fn() + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::Click, move |_| f())
    }

    /// Double click.
    fn on_double_click(&self, f: impl Fn() + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::DoubleClick, move |_| f())
    }

    /// Mouse movement over the control.
    fn on_mouse_move(&self, f: impl Fn(&MouseEvent) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::MouseMove, move |e| {
            if let Event::MouseMove(m) = e {
                f(m)
            }
        })
    }

    /// Cursor entered the control.
    fn on_mouse_enter(&self, f: impl Fn() + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::MouseEnter, move |_| f())
    }

    /// Cursor left the control.
    fn on_mouse_leave(&self, f: impl Fn() + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::MouseLeave, move |_| f())
    }

    /// Mouse button pressed.
    fn on_mouse_down(&self, f: impl Fn(&MouseEvent) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::MouseDown, move |e| {
            if let Event::MouseDown(m) = e {
                f(m)
            }
        })
    }

    /// Mouse button released.
    fn on_mouse_up(&self, f: impl Fn(&MouseEvent) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::MouseUp, move |e| {
            if let Event::MouseUp(m) = e {
                f(m)
            }
        })
    }

    /// Mouse wheel.
    fn on_mouse_wheel(&self, f: impl Fn(&MouseEvent) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::MouseWheel, move |e| {
            if let Event::MouseWheel(m) = e {
                f(m)
            }
        })
    }

    /// Key pressed while focused.
    fn on_key_down(&self, f: impl Fn(&KeyEvent) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::KeyDown, move |e| {
            if let Event::KeyDown(k) = e {
                f(k)
            }
        })
    }

    /// Key released while focused.
    fn on_key_up(&self, f: impl Fn(&KeyEvent) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::KeyUp, move |e| {
            if let Event::KeyUp(k) = e {
                f(k)
            }
        })
    }

    /// Character typed.
    fn on_char(&self, f: impl Fn(char) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::Char, move |e| {
            if let Event::Char(c) = e {
                f(*c)
            }
        })
    }

    /// Control gained focus.
    fn on_focus(&self, f: impl Fn() + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::Focus, move |_| f())
    }

    /// Control lost focus.
    fn on_blur(&self, f: impl Fn() + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::Blur, move |_| f())
    }

    /// Client area resized.
    fn on_resize(&self, f: impl Fn(Size) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::Resize, move |e| {
            if let Event::Resize(s) = e {
                f(*s)
            }
        })
    }

    /// Control destroyed.
    fn on_destroy(&self, f: impl Fn() + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::Destroyed, move |_| f())
    }

    /// Context menu requested at a control-relative logical position.
    fn on_context_menu(&self, f: impl Fn(Point) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::ContextMenu, move |e| {
            if let Event::ContextMenu(p) = e {
                f(*p)
            }
        })
    }

    /// Touch/pen event.
    fn on_pointer(&self, f: impl Fn(&PointerEvent) + 'static) -> HandlerId
    where
        Self: Sized,
    {
        self.core().on(EventKind::Pointer, move |e| {
            if let Event::Pointer(p) = e {
                f(p)
            }
        })
    }

    /// Intercepts raw Win32 messages (advanced API).
    ///
    /// `Some(value)` ends processing with that result; `None` continues library/Windows processing. Replaces any
    /// earlier hook.
    fn on_raw_message(&self, f: impl Fn(&RawMessage) -> Option<isize> + 'static)
    where
        Self: Sized,
    {
        if self.core().is_alive() {
            self.core().set_raw_hook(f);
        }
    }
}
