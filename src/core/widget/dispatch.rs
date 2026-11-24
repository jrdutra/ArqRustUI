//! [`WidgetCore`](super::WidgetCore) message dispatch: matches Windows messages to events, child notification
//! reflection, colors, painting, DPI changes and destruction.

use super::*;

impl WidgetCore {
    // ---- Despacho ------------------------------------------------------------

    pub(crate) fn default_proc(&self, hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
        let original = self.original_proc.get();
        // SAFETY: `original` é o procedimento que a janela tinha antes do subclassing;
        // os parâmetros são os recebidos do sistema, repassados sem alteração.
        unsafe {
            if original != 0 {
                CallWindowProcW(original, hwnd, msg, wp, lp)
            } else {
                DefWindowProcW(hwnd, msg, wp, lp)
            }
        }
    }

    fn is_own_class(&self) -> bool {
        self.original_proc.get() == 0
    }

    fn mouse_event(&self, hwnd: HWND, lp: LPARAM, button: MouseButton, wheel: i32, screen: bool) -> MouseEvent {
        let mut point = POINT { x: x_lparam(lp), y: y_lparam(lp) };
        if screen {
            // SAFETY: ponteiro válido para um POINT local.
            unsafe { ScreenToClient(hwnd, &mut point) };
        }
        let dpi = native::dpi_for_window(hwnd);
        MouseEvent {
            position: Point::new(to_logical(point.x, dpi), to_logical(point.y, dpi)),
            button,
            modifiers: Modifiers::current(),
            wheel_delta: wheel,
        }
    }

    pub(super) fn handle(self: &Rc<Self>, hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if !self.filters.borrow().is_empty() {
            let filters = self.filters.borrow().clone();
            let raw = RawMessage { hwnd: hwnd as isize, message: msg, wparam: wp, lparam: lp };
            for filter in filters {
                if let Some(result) = filter(&raw) {
                    return result;
                }
            }
        }
        let hook = self.raw_hook.borrow().clone();
        if let Some(hook) = hook {
            let raw = RawMessage { hwnd: hwnd as isize, message: msg, wparam: wp, lparam: lp };
            if let Some(result) = hook(&raw) {
                return result;
            }
        }
        if self.drag.borrow().is_some() {
            if let Some(result) = crate::input::drag::handle(self, hwnd, msg, wp, lp) {
                return result;
            }
        }
        if self.top_level {
            if let Some(result) = crate::menus::strip::root_message(self, hwnd, msg, wp, lp) {
                return result;
            }
        }
        if let Some(result) = crate::themes::dark::handle(self, hwnd, msg, wp) {
            return result;
        }
        if msg == WM_NCPAINT && crate::themes::dark::wants_dark_edge(self, hwnd) {
            let result = if self.centered_text.get() {
                self.handle_text_inset(hwnd, msg, wp, lp).unwrap_or_else(|| self.default_proc(hwnd, msg, wp, lp))
            } else {
                self.default_proc(hwnd, msg, wp, lp)
            };
            crate::themes::dark::paint_edge(self, hwnd);
            return result;
        }
        if self.centered_text.get() {
            if let Some(result) = self.handle_text_inset(hwnd, msg, wp, lp) {
                return result;
            }
        }
        // Mensagens registradas (faixa 0xC000..=0xFFFF): diálogos de localizar/substituir.
        if msg >= 0xC000 && crate::dialogs::find::dispatch(msg, lp) {
            return 0;
        }
        match msg {
            WM_COMMAND => {
                if lp != 0 {
                    if let Some(child) = lookup(lp as HWND) {
                        let reflected = Reflected::Command { code: hiword(wp), id: loword(wp) };
                        if let Some(result) = child.reflect_to(&reflected) {
                            return result;
                        }
                    }
                } else {
                    let id = loword(wp) as u32;
                    if app::invoke_menu(id) {
                        return 0;
                    }
                    if id == 2 && self.close_on_escape.get() {
                        // SAFETY: posta WM_CLOSE para a própria janela viva.
                        unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
                        return 0;
                    }
                }
            }
            WM_NOTIFY if lp != 0 => {
                // SAFETY: em WM_NOTIFY o LPARAM aponta para uma estrutura iniciada por
                // NMHDR, válida durante o processamento da mensagem.
                let header = unsafe { *(lp as *const NMHDR) };
                if let Some(child) = lookup(header.hwndFrom) {
                    let code = header.code as i32;
                    if (code == crate::ffi::comctl32::LVN_BEGINDRAG || code == crate::ffi::comctl32::TVN_BEGINDRAGW)
                        && crate::input::drag::begin_native(&child, code, lp)
                    {
                        return 0;
                    }
                    let reflected = Reflected::Notify { code: header.code as i32, lparam: lp };
                    if let Some(result) = child.reflect_to(&reflected) {
                        return result;
                    }
                }
            }
            WM_DRAWITEM if lp != 0 => {
                // SAFETY: em WM_DRAWITEM o LPARAM aponta para um DRAWITEMSTRUCT válido
                // durante o processamento da mensagem.
                let item = unsafe { (*(lp as *const DRAWITEMSTRUCT)).hwndItem };
                if let Some(child) = lookup(item) {
                    if let Some(result) = child.reflect_to(&Reflected::DrawItem { lparam: lp }) {
                        return result;
                    }
                }
            }
            // Listas desenhadas pela biblioteca pedem a altura dos itens ao pai durante a
            // própria criação (antes de estarem no registro): devolve um valor padrão,
            // que o controle ajusta assim que termina de ser criado.
            WM_MEASUREITEM if lp != 0 => {
                // SAFETY: em WM_MEASUREITEM o LPARAM aponta para um MEASUREITEMSTRUCT gravável.
                let info = unsafe { &mut *(lp as *mut MEASUREITEMSTRUCT) };
                if info.CtlType == ODT_COMBOBOX || info.CtlType == ODT_LISTBOX {
                    let dpi = native::dpi_for_window(hwnd);
                    info.itemHeight = to_pixels(crate::controls::DEFAULT_ICON_SIZE + 4, dpi) as u32;
                    return 1;
                }
            }
            WM_INITMENUPOPUP => {
                crate::menus::notify_native_open(wp as HMENU);
            }
            WM_MENUSELECT => {
                crate::menus::native_menu_select(wp, lp);
            }
            WM_HSCROLL | WM_VSCROLL if lp != 0 => {
                if let Some(child) = lookup(lp as HWND) {
                    if let Some(result) = child.reflect_to(&Reflected::Scroll { code: loword(wp) }) {
                        return result;
                    }
                }
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
                if let Some(result) = self.control_color(msg, wp as HDC, lp as HWND) {
                    return result;
                }
            }
            // A caixa de grupo nativa não apaga o próprio fundo; como contêiner de
            // outros controles ela precisa fazê-lo.
            WM_ERASEBKGND if self.is_own_class() || self.kind == "GroupBox" => {
                if self.paint.borrow().is_none() || !self.double_buffered.get() {
                    let rect = native::client_rect(hwnd);
                    let brush = match self.effective_background() {
                        Some(color) => app::brush_for(color),
                        // SAFETY: pincel de sistema, que não precisa ser liberado.
                        None => unsafe { GetSysColorBrush(COLOR_BTNFACE) },
                    };
                    // SAFETY: em WM_ERASEBKGND o WPARAM é o HDC a ser preenchido.
                    unsafe { FillRect(wp as HDC, &rect, brush) };
                }
                return 1;
            }
            WM_PAINT if self.is_own_class() => {
                let painter = self.paint.borrow().clone();
                if let Some(painter) = painter {
                    self.run_paint(hwnd, &painter);
                    return 0;
                }
            }
            // Pintura num DC fornecido (AnimateWindow, capturas, impressão da janela).
            WM_PRINTCLIENT if self.is_own_class() && wp != 0 => {
                let painter = self.paint.borrow().clone();
                if let Some(painter) = painter {
                    let rect = native::client_rect(hwnd);
                    let background = self.effective_background();
                    crate::graphics::gdi::paint_buffered(wp as HDC, rect, self.dpi(), background, |canvas| {
                        painter(canvas)
                    });
                    return 0;
                }
            }
            WM_SIZE => {
                if wp != SIZE_MINIMIZED {
                    self.emit(&Event::Resize(self.client_size()));
                    self.relayout();
                }
            }
            WM_MOVE if self.top_level => {
                self.emit(&Event::Move(Point::new(x_lparam(lp), y_lparam(lp))));
            }
            WM_CLOSE if self.top_level => {
                let handler = self.close_handler.borrow().clone();
                if let Some(handler) = handler {
                    if !handler() {
                        return 0;
                    }
                }
                self.release_modal_owner();
            }
            WM_NCDESTROY => {
                self.emit(&Event::Destroyed);
                let result = self.default_proc(hwnd, msg, wp, lp);
                self.detach(hwnd);
                return result;
            }
            WM_MOUSEMOVE => {
                if !self.mouse_inside.get() {
                    self.mouse_inside.set(true);
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: estrutura local corretamente dimensionada.
                    unsafe { TrackMouseEvent(&mut track) };
                    self.emit(&Event::MouseEnter);
                }
                if self.has_handler(EventKind::MouseMove) || self.has_handler(EventKind::Any) {
                    self.emit(&Event::MouseMove(self.mouse_event(hwnd, lp, MouseButton::None, 0, false)));
                }
            }
            WM_MOUSELEAVE => {
                self.mouse_inside.set(false);
                self.emit(&Event::MouseLeave);
            }
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
                let button = match msg {
                    WM_LBUTTONDOWN => MouseButton::Left,
                    WM_RBUTTONDOWN => MouseButton::Right,
                    _ => MouseButton::Middle,
                };
                if msg == WM_LBUTTONDOWN {
                    self.pressed.set(true);
                }
                self.emit(&Event::MouseDown(self.mouse_event(hwnd, lp, button, 0, false)));
            }
            WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
                let button = match msg {
                    WM_LBUTTONUP => MouseButton::Left,
                    WM_RBUTTONUP => MouseButton::Right,
                    _ => MouseButton::Middle,
                };
                self.emit(&Event::MouseUp(self.mouse_event(hwnd, lp, button, 0, false)));
                if msg == WM_LBUTTONUP && self.pressed.replace(false) && self.is_own_class() {
                    self.emit(&Event::Click);
                }
            }
            WM_LBUTTONDBLCLK => {
                self.emit(&Event::DoubleClick);
            }
            WM_MOUSEWHEEL => {
                let delta = (hiword(wp) as i16 as i32) / WHEEL_DELTA;
                if self.emit(&Event::MouseWheel(self.mouse_event(hwnd, lp, MouseButton::None, delta, true))) {
                    return 0;
                }
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let event =
                    KeyEvent { key: Key(wp as u16), modifiers: Modifiers::current(), repeat: (lp >> 30) & 1 == 1 };
                self.emit(&Event::KeyDown(event));
            }
            WM_KEYUP | WM_SYSKEYUP => {
                let event = KeyEvent { key: Key(wp as u16), modifiers: Modifiers::current(), repeat: false };
                self.emit(&Event::KeyUp(event));
            }
            WM_CHAR => {
                if let Some(c) = char::from_u32(wp as u32) {
                    self.emit(&Event::Char(c));
                }
            }
            WM_GETDLGCODE if self.is_own_class() && self.has_handler(EventKind::KeyDown) => {
                return DLGC_WANTARROWS | DLGC_WANTCHARS;
            }
            WM_SETFOCUS => {
                if self.top_level {
                    let previous = self.last_focus.get();
                    // SAFETY: verifica e devolve o foco a um controle filho ainda existente.
                    unsafe {
                        if !previous.is_null() && IsWindow(previous) != 0 {
                            SetFocus(previous);
                            return 0;
                        }
                    }
                }
                self.emit(&Event::Focus);
            }
            WM_KILLFOCUS => {
                self.emit(&Event::Blur);
            }
            WM_ACTIVATE if self.top_level => {
                if loword(wp) == WA_INACTIVE {
                    // SAFETY: consultas simples de foco e ancestralidade.
                    let focus = unsafe { GetFocus() };
                    if !focus.is_null() && focus != hwnd && native::root_of(focus) == hwnd {
                        self.last_focus.set(focus);
                    }
                    self.emit(&Event::Deactivated);
                } else {
                    self.emit(&Event::Activated);
                }
            }
            WM_GETMINMAXINFO if self.top_level => {
                if self.min_size.get().is_some() || self.max_size.get().is_some() {
                    let dpi = self.dpi();
                    // SAFETY: em WM_GETMINMAXINFO o LPARAM aponta para um MINMAXINFO gravável.
                    let info = unsafe { &mut *(lp as *mut MINMAXINFO) };
                    let to_outer = |size: Size| {
                        crate::core::window::outer_rect_for_client(
                            hwnd,
                            Size::new(to_pixels(size.width, dpi), to_pixels(size.height, dpi)),
                        )
                    };
                    if let Some(min) = self.min_size.get() {
                        let outer = to_outer(min);
                        info.ptMinTrackSize = POINT { x: outer.width, y: outer.height };
                    }
                    if let Some(max) = self.max_size.get() {
                        let outer = to_outer(max);
                        info.ptMaxTrackSize = POINT { x: outer.width, y: outer.height };
                    }
                    return 0;
                }
            }
            WM_DPICHANGED if self.top_level => {
                // SAFETY: em WM_DPICHANGED o LPARAM aponta para o RECT sugerido pelo sistema.
                let suggested = unsafe { *(lp as *const RECT) };
                // SAFETY: reposiciona a própria janela viva.
                unsafe {
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        suggested.left,
                        suggested.top,
                        suggested.right - suggested.left,
                        suggested.bottom - suggested.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
                self.rescale_children();
                crate::core::window::apply_icon(self);
                self.emit(&Event::DpiChanged(hiword(wp) as u32));
                return 0;
            }
            WM_CONTEXTMENU => {
                let menu = self.context_menu.borrow().clone();
                let mut screen = POINT { x: x_lparam(lp), y: y_lparam(lp) };
                if screen.x == -1 && screen.y == -1 {
                    let rect = native::window_rect(hwnd);
                    screen = POINT { x: rect.left + 8, y: rect.top + 8 };
                }
                if let Some(menu) = menu {
                    menu.show_at_screen(hwnd, Point::new(screen.x, screen.y));
                    return 0;
                }
                if self.has_handler(EventKind::ContextMenu) {
                    let mut client = screen;
                    // SAFETY: ponteiro válido para um POINT local.
                    unsafe { ScreenToClient(hwnd, &mut client) };
                    let dpi = self.dpi();
                    self.emit(&Event::ContextMenu(Point::new(to_logical(client.x, dpi), to_logical(client.y, dpi))));
                    return 0;
                }
            }
            WM_SETCURSOR => {
                if loword(lp as usize) == HTCLIENT && wp as HWND == hwnd {
                    if let Some(cursor) = self.cursor.borrow().as_ref() {
                        // SAFETY: o handle de cursor é mantido vivo por `Cursor`.
                        unsafe { SetCursor(cursor.raw()) };
                        return 1;
                    }
                }
            }
            WM_DROPFILES => {
                let files = dropped_files(wp as HDROP);
                self.emit(&Event::DropFiles(files));
                return 0;
            }
            WM_SETTINGCHANGE if self.top_level && lp != 0 => {
                // SAFETY: em WM_SETTINGCHANGE um LPARAM não nulo é uma string terminada em NUL.
                let area = unsafe { crate::utilities::strings::from_wide_ptr(lp as *const u16) };
                if area == "ImmersiveColorSet" || area == "WindowsThemeElement" {
                    self.emit(&Event::ThemeChanged);
                }
            }
            WM_SYSCOMMAND if self.top_level => {
                let id = (wp & 0xFFFF) as u32;
                if id < 0xF000 && app::invoke_menu(id) {
                    return 0;
                }
            }
            WM_POINTERDOWN | WM_POINTERUP | WM_POINTERUPDATE => {
                if self.has_handler(EventKind::Pointer) {
                    let id = loword(wp) as u32;
                    let phase = match msg {
                        WM_POINTERDOWN => PointerPhase::Down,
                        WM_POINTERUP => PointerPhase::Up,
                        _ => PointerPhase::Move,
                    };
                    let event = self.mouse_event(hwnd, lp, MouseButton::None, 0, true);
                    self.emit(&Event::Pointer(PointerEvent {
                        id,
                        kind: pointer_kind(id),
                        phase,
                        position: event.position,
                    }));
                }
            }
            WM_IME_COMPOSITION => {
                if lp as u32 & imm32::GCS_COMPSTR != 0 && self.has_handler(EventKind::ImeComposition) {
                    if let Some(text) = crate::input::ime::composition_string(hwnd, imm32::GCS_COMPSTR) {
                        self.emit(&Event::ImeComposition(text));
                    }
                }
            }
            WM_GETOBJECT if lp as i32 == crate::ffi::uiautomation::UIA_ROOT_OBJECT_ID => {
                let provider = self.accessible.borrow().clone().or_else(|| {
                    let tree = self.accessible_tree.borrow().clone();
                    tree.and_then(|t| t.root_provider())
                });
                if let (Some(provider), Some(ret)) =
                    (provider, crate::ffi::dynamic::optional().UiaReturnRawElementProvider)
                {
                    // SAFETY: `provider` é um IRawElementProviderSimple vivo; a função
                    // adiciona a própria referência.
                    return unsafe { ret(hwnd, wp, lp, provider.as_ptr()) };
                }
            }
            _ => {}
        }
        self.default_proc(hwnd, msg, wp, lp)
    }

    fn reflect_to(self: &Rc<Self>, reflected: &Reflected) -> Option<LRESULT> {
        let handler = self.reflect.borrow().clone();
        handler.and_then(|h| h(self, reflected))
    }

    fn control_color(&self, msg: UINT, hdc: HDC, child_hwnd: HWND) -> Option<LRESULT> {
        let child = lookup(child_hwnd);
        let theme = self.root().and_then(|r| r.theme.borrow().clone());
        let is_input = matches!(msg, WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX);
        let foreground = child
            .as_ref()
            .and_then(|c| c.foreground.get())
            .or_else(|| theme.as_ref().map(|t| if is_input { t.input_foreground } else { t.foreground }));
        let background = child
            .as_ref()
            .and_then(|c| c.background.get())
            .or_else(|| if is_input { theme.as_ref().map(|t| t.input_background) } else { None })
            .or_else(|| if is_input { None } else { self.effective_background() });
        if foreground.is_none() && background.is_none() {
            return None;
        }
        // SAFETY: em WM_CTLCOLOR* o WPARAM é um HDC válido durante a mensagem.
        unsafe {
            if let Some(color) = foreground {
                SetTextColor(hdc, color.to_colorref());
            }
            match background {
                Some(color) => {
                    SetBkColor(hdc, color.to_colorref());
                    Some(app::brush_for(color) as LRESULT)
                }
                None => {
                    let hwnd = self.hwnd.get();
                    Some(self.default_proc(hwnd, msg, hdc as WPARAM, child_hwnd as LPARAM))
                }
            }
        }
    }

    fn run_paint(&self, hwnd: HWND, painter: &PaintFn) {
        // SAFETY: PAINTSTRUCT é preenchido por BeginPaint; todos os campos aceitam zero.
        let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
        // SAFETY: par BeginPaint/EndPaint dentro do tratamento de WM_PAINT desta janela.
        let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
        if hdc.is_null() {
            return;
        }
        let rect = native::client_rect(hwnd);
        let dpi = self.dpi();
        let background = self.effective_background();
        if self.double_buffered.get() {
            crate::graphics::gdi::paint_buffered(hdc, rect, dpi, background, |canvas| painter(canvas));
        } else {
            let canvas = Canvas::for_hdc(hdc, dpi, Size::new(rect.right, rect.bottom));
            painter(&canvas);
        }
        // SAFETY: fecha o BeginPaint acima.
        unsafe { EndPaint(hwnd, &ps) };
    }

    /// Reapplies logical bounds/fonts to descendants after DPI changes.
    fn rescale_children(&self) {
        let root = self.hwnd.get();
        self.refresh_font();
        for core in all_cores() {
            if core.top_level || native::root_of(core.hwnd()) != root {
                continue;
            }
            core.refresh_font();
            if let Some(bounds) = core.logical_bounds.get() {
                core.set_bounds(bounds);
            }
            crate::controls::icon_support::refresh_after_dpi_change(&core);
        }
        self.relayout();
    }

    /// Re-enables a modal window's owner before destruction.
    pub(crate) fn release_modal_owner(&self) {
        let owner = self.modal_owner.replace(std::ptr::null_mut());
        if !owner.is_null() {
            // SAFETY: reabilita e reativa a janela dona, se ainda existir.
            unsafe {
                EnableWindow(owner, TRUE);
                SetActiveWindow(owner);
            }
        }
    }

    /// Detaches the native window and releases callback-held resources.
    fn detach(&self, hwnd: HWND) {
        self.release_modal_owner();
        REGISTRY.with(|r| r.borrow_mut().remove(&(hwnd as isize)));
        self.hwnd.set(std::ptr::null_mut());
        // Mover para variáveis locais garante que os destrutores rodem sem empréstimos ativos.
        let handlers = std::mem::take(&mut *self.handlers.borrow_mut());
        let close = self.close_handler.borrow_mut().take();
        let hook = self.raw_hook.borrow_mut().take();
        let reflect = self.reflect.borrow_mut().take();
        let paint = self.paint.borrow_mut().take();
        let layout = self.layout.borrow_mut().take();
        let menu = self.context_menu.borrow_mut().take();
        let shortcuts = std::mem::take(&mut *self.shortcuts.borrow_mut());
        let resources = std::mem::take(&mut *self.keep_alive.borrow_mut());
        let provider = self.accessible.borrow_mut().take();
        let theme = self.theme.borrow_mut().take();
        let icon = self.icon.borrow_mut().take();
        let images = self.managed_images.borrow_mut().take();
        let drag = self.drag.borrow_mut().take();
        let tree = self.accessible_tree.borrow_mut().take();
        let filters = std::mem::take(&mut *self.filters.borrow_mut());
        let menu_bar = self.menu_bar.borrow_mut().take();
        drop((
            menu_bar, handlers, close, hook, reflect, paint, layout, menu, shortcuts, resources, provider, theme, icon,
            images, drag, tree, filters,
        ));
        if self.top_level {
            app::top_level_closed();
        }
    }
}
