//! Top-level windows.

use super::application as app;
use super::errors::{GuiError, GuiResult};
use super::event_loop;
use super::events::{Event, EventKind, HandlerId};
use super::handles::IconSource;
use super::native;
use super::widget::{window_proc, CreateParams, Widget, WidgetCore};
use crate::controls::icon_support::WidgetIcon;
use crate::ffi::dynamic::optional;
use crate::ffi::kernel32::{GetLastError, ERROR_CLASS_ALREADY_EXISTS};
use crate::ffi::shell32::DragAcceptFiles;
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::graphics::Canvas;
use crate::input::Shortcut;
use crate::layout::Layout;
use crate::menus::MenuBar;
use crate::system::Monitor;
use crate::utilities::{to_pixels, to_wide, Color, Point, Rect, Size};
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) const WINDOW_CLASS: &str = "ArqRustUI.Window";
pub(crate) const PANEL_CLASS: &str = "ArqRustUI.Panel";

/// Registers library window classes idempotently.
pub(crate) fn register_classes() -> GuiResult<()> {
    for (name, style) in [(WINDOW_CLASS, CS_DBLCLKS), (PANEL_CLASS, CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW)] {
        let wide = to_wide(name);
        // SAFETY: carrega recursos compartilhados do sistema (não precisam ser liberados).
        let (cursor, icon) = unsafe {
            (
                LoadCursorW(std::ptr::null_mut(), IDC_ARROW as PCWSTR),
                LoadIconW(std::ptr::null_mut(), IDI_APPLICATION as PCWSTR),
            )
        };
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: native::instance(),
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: wide.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        // SAFETY: estrutura completa; o nome vive durante a chamada.
        if unsafe { RegisterClassExW(&class) } == 0 {
            // SAFETY: leitura do código de erro da chamada anterior.
            let code = unsafe { GetLastError() };
            if code != ERROR_CLASS_ALREADY_EXISTS {
                return Err(GuiError::from_win32("RegisterClassExW", code));
            }
        }
    }
    Ok(())
}

/// Outer pixel size required for a given pixel client size.
pub(crate) fn outer_rect_for_client(hwnd: HWND, client: Size) -> Size {
    // SAFETY: leituras simples de estilo e menu de uma janela.
    let (style, ex_style, has_menu) = unsafe {
        (
            GetWindowLongPtrW(hwnd, GWL_STYLE) as u32,
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32,
            !GetMenu(hwnd).is_null(),
        )
    };
    let dpi = native::dpi_for_window(hwnd);
    let mut size = outer_size(style, ex_style, has_menu, client, dpi);
    // A MenuStrip ocupa uma faixa da área não cliente, como a barra de menus nativa.
    size.height += crate::menus::strip::band_height_for(hwnd);
    size
}

fn outer_size(style: u32, ex_style: u32, has_menu: bool, client: Size, dpi: u32) -> Size {
    let mut rect = RECT { left: 0, top: 0, right: client.width, bottom: client.height };
    // SAFETY: ponteiro válido para um RECT local; a versão por DPI só é usada se existir.
    unsafe {
        match optional().AdjustWindowRectExForDpi {
            Some(adjust) => adjust(&mut rect, style, has_menu as BOOL, ex_style, dpi),
            None => AdjustWindowRectEx(&mut rect, style, has_menu as BOOL, ex_style),
        };
    }
    Size::new(rect.right - rect.left, rect.bottom - rect.top)
}

/// Rounded corners, shadow and other DWM effects are in [`crate::themes`].
///
/// Top-level window.
///
/// `Window` is a cloneable *handle*: cloning does not create a window. Builder size is **client area** in logical units
/// (96 DPI). Without an explicit icon, uses [`crate::icons::DEFAULT_WINDOW_ICON`], including without the optional
/// `icons` feature.
///
/// ```no_run
/// use arqrustui::prelude::*;
/// # fn main() -> GuiResult<()> {
/// let app = Application::new()?;
/// let window = Window::builder().title("Hello").size(640, 480).centered().build()?;
/// window.on_close_request(|| MessageBox::confirm("Exit", "Do you really want to exit?"));
/// window.show();
/// app.run()
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Window {
    pub(crate) core: Rc<WidgetCore>,
}

impl Widget for Window {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

/// Builder for [`Window`].
#[derive(Debug, Clone)]
pub struct WindowBuilder {
    title: String,
    size: Size,
    position: Option<Point>,
    centered: bool,
    resizable: bool,
    minimizable: bool,
    maximizable: bool,
    borderless: bool,
    topmost: bool,
    tool_window: bool,
    dialog_frame: bool,
    opacity: Option<f32>,
    owner: HWND,
    icon: Option<Rc<dyn IconSource>>,
    min_size: Option<Size>,
    max_size: Option<Size>,
    accept_files: bool,
    close_on_escape: bool,
    background: Option<Color>,
    right_to_left: bool,
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self {
            title: String::new(),
            size: Size::new(800, 600),
            position: None,
            centered: false,
            resizable: true,
            minimizable: true,
            maximizable: true,
            borderless: false,
            topmost: false,
            tool_window: false,
            dialog_frame: false,
            opacity: None,
            owner: std::ptr::null_mut(),
            icon: Some(crate::icons::DEFAULT_WINDOW_ICON.to_shared()),
            min_size: None,
            max_size: None,
            accept_files: false,
            close_on_escape: false,
            background: None,
            right_to_left: false,
        }
    }
}

impl WindowBuilder {
    /// Title bar text.
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Logical client size (default 800×600).
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.size = Size::new(width, height);
        self
    }

    /// Initial screen pixel position; Windows chooses if omitted.
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.position = Some(Point::new(x, y));
        self
    }

    /// Centers on the monitor or owner window.
    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    /// Allows border resizing (default `true`).
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        if !resizable {
            self.maximizable = false;
        }
        self
    }

    /// Shows minimize button (default `true`).
    pub fn minimizable(mut self, minimizable: bool) -> Self {
        self.minimizable = minimizable;
        self
    }

    /// Shows maximize button (default `true`).
    pub fn maximizable(mut self, maximizable: bool) -> Self {
        self.maximizable = maximizable;
        self
    }

    /// Window without title bar or borders.
    pub fn borderless(mut self) -> Self {
        self.borderless = true;
        self
    }

    /// Keeps above other windows (Always on Top).
    pub fn topmost(mut self, topmost: bool) -> Self {
        self.topmost = topmost;
        self
    }

    /// Tool window: small title, no taskbar button.
    pub fn tool_window(mut self) -> Self {
        self.tool_window = true;
        self
    }

    /// Dialog frame: no resizing, minimize/maximize or icon.
    pub fn dialog_frame(mut self) -> Self {
        self.dialog_frame = true;
        self.resizable = false;
        self.minimizable = false;
        self.maximizable = false;
        self
    }

    /// Whole-window opacity, 0.0 invisible to 1.0 opaque.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// Sets owner: the new window remains above it and is destroyed with it.
    ///
    /// Also the window disabled by [`Window::show_modal`].
    pub fn owner(mut self, owner: &Window) -> Self {
        self.owner = owner.core.hwnd();
        self
    }

    /// Window title/taskbar icon. Overrides [`crate::icons::DEFAULT_WINDOW_ICON`].
    pub fn icon(mut self, icon: &impl IconSource) -> Self {
        self.icon = Some(icon.to_shared());
        self
    }

    /// Minimum client size.
    pub fn min_size(mut self, width: i32, height: i32) -> Self {
        self.min_size = Some(Size::new(width, height));
        self
    }

    /// Maximum client size.
    pub fn max_size(mut self, width: i32, height: i32) -> Self {
        self.max_size = Some(Size::new(width, height));
        self
    }

    /// Accepts Explorer file drops (see [`Window::on_drop_files`]).
    pub fn accept_files(mut self, accept: bool) -> Self {
        self.accept_files = accept;
        self
    }

    /// Closes on Esc, useful for dialogs.
    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.close_on_escape = close;
        self
    }

    /// Client background color.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Mirrors layout for right-to-left languages.
    pub fn right_to_left(mut self, rtl: bool) -> Self {
        self.right_to_left = rtl;
        self
    }

    fn styles(&self) -> (u32, u32) {
        let mut style = WS_CLIPCHILDREN;
        if self.borderless {
            style |= WS_POPUP;
        } else {
            style |= WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
            if self.resizable {
                style |= WS_THICKFRAME;
            }
            if self.minimizable {
                style |= WS_MINIMIZEBOX;
            }
            if self.maximizable {
                style |= WS_MAXIMIZEBOX;
            }
        }
        let mut ex_style = 0;
        if self.topmost {
            ex_style |= WS_EX_TOPMOST;
        }
        if self.tool_window {
            ex_style |= WS_EX_TOOLWINDOW;
        }
        if self.dialog_frame {
            ex_style |= WS_EX_DLGMODALFRAME;
        }
        if self.opacity.is_some() {
            ex_style |= WS_EX_LAYERED;
        }
        if self.accept_files {
            ex_style |= WS_EX_ACCEPTFILES;
        }
        if self.right_to_left {
            ex_style |= WS_EX_LAYOUTRTL;
        }
        (style, ex_style)
    }

    /// Creates a hidden window; call [`Widget::show`].
    pub fn build(self) -> GuiResult<Window> {
        app::require()?;
        if self.size.width < 0 || self.size.height < 0 {
            return Err(GuiError::InvalidArgument("the window size cannot be negative".into()));
        }
        let (style, ex_style) = self.styles();
        let dpi = if self.owner.is_null() { native::system_dpi() } else { native::dpi_for_window(self.owner) };
        let client = Size::new(to_pixels(self.size.width, dpi), to_pixels(self.size.height, dpi));
        let outer = outer_size(style, ex_style, false, client, dpi);
        let (x, y) = match self.position {
            Some(p) => (p.x, p.y),
            None => (CW_USEDEFAULT, CW_USEDEFAULT),
        };
        let core = WidgetCore::create(CreateParams {
            kind: "Window",
            class: WINDOW_CLASS,
            text: &self.title,
            style,
            ex_style,
            parent: self.owner,
            pixel_bounds: RECT {
                left: x,
                top: y,
                right: x.wrapping_add(outer.width),
                bottom: y.wrapping_add(outer.height),
            },
            id: 0,
            top_level: true,
            subclass: false,
        })?;
        let window = Window { core };
        window.core.min_size.set(self.min_size);
        window.core.max_size.set(self.max_size);
        window.core.close_on_escape.set(self.close_on_escape);
        window.core.background.set(self.background);
        if native::dpi_for_window(window.core.hwnd()) != dpi {
            // A janela nasceu em um monitor com outro DPI: reajusta ao tamanho lógico pedido.
            let b = window.core.bounds();
            window.core.set_bounds(Rect::new(b.x, b.y, self.size.width, self.size.height));
        }
        if let Some(opacity) = self.opacity {
            window.set_opacity(opacity);
        }
        if let Some(icon) = &self.icon {
            window.set_icon(icon);
        }
        if self.centered {
            window.center();
        }
        Ok(window)
    }
}

impl Window {
    /// Starts building a window.
    pub fn builder() -> WindowBuilder {
        WindowBuilder::default()
    }

    /// Sets title.
    pub fn set_title(&self, title: &str) {
        self.set_text(title);
    }

    /// Current title.
    pub fn title(&self) -> String {
        self.text()
    }

    /// Requests closing as if the user clicked Close.
    ///
    /// [`on_close_request`](Self::on_close_request) may veto closing.
    pub fn close(&self) {
        native::send(self.core.hwnd(), WM_CLOSE, 0, 0);
    }

    fn show_command(&self, command: i32) {
        // SAFETY: ShowWindow tolera HWND nulo/inválido.
        unsafe { ShowWindow(self.core.hwnd(), command) };
    }

    /// Maximizes.
    pub fn maximize(&self) {
        self.show_command(SW_MAXIMIZE);
    }

    /// Minimizes.
    pub fn minimize(&self) {
        self.show_command(SW_MINIMIZE);
    }

    /// Restores normal size.
    pub fn restore(&self) {
        self.show_command(SW_RESTORE);
    }

    /// Whether maximized.
    pub fn is_maximized(&self) -> bool {
        // SAFETY: consulta simples.
        unsafe { IsZoomed(self.core.hwnd()) != 0 }
    }

    /// Whether minimized.
    pub fn is_minimized(&self) -> bool {
        // SAFETY: consulta simples.
        unsafe { IsIconic(self.core.hwnd()) != 0 }
    }

    /// Brings forward and activates.
    pub fn activate(&self) {
        // SAFETY: tolera HWND nulo/inválido.
        unsafe { SetForegroundWindow(self.core.hwnd()) };
    }

    /// Toggles fullscreen, borderless over the current monitor.
    pub fn set_fullscreen(&self, fullscreen: bool) {
        let hwnd = self.core.hwnd();
        if hwnd.is_null() || fullscreen == self.is_fullscreen() {
            return;
        }
        if fullscreen {
            let mut placement =
                WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
            // SAFETY: estrutura local corretamente dimensionada.
            unsafe { GetWindowPlacement(hwnd, &mut placement) };
            let style = native::style(hwnd);
            self.core.saved_placement.set(Some((style, placement)));
            let Some(monitor) = Monitor::from_window_handle(hwnd) else { return };
            let area = monitor.bounds;
            // SAFETY: troca de estilo e reposicionamento de uma janela viva desta thread.
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, ((style & !WS_OVERLAPPEDWINDOW) | WS_POPUP) as isize);
                SetWindowPos(hwnd, HWND_TOP, area.x, area.y, area.width, area.height, SWP_NOZORDER | SWP_FRAMECHANGED);
            }
        } else if let Some((style, placement)) = self.core.saved_placement.take() {
            // SAFETY: restaura estilo e posição salvos ao entrar em tela cheia.
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
                SetWindowPlacement(hwnd, &placement);
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
                );
            }
        }
    }

    /// Whether fullscreen.
    pub fn is_fullscreen(&self) -> bool {
        let saved = self.core.saved_placement.take();
        let fullscreen = saved.is_some();
        self.core.saved_placement.set(saved);
        fullscreen
    }

    /// Centers in the current monitor's work area.
    pub fn center(&self) {
        let hwnd = self.core.hwnd();
        let Some(monitor) = Monitor::from_window_handle(hwnd) else { return };
        let rect = native::window_rect(hwnd);
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        let work = monitor.work_area;
        let x = work.x + (work.width - width) / 2;
        let y = work.y + (work.height - height) / 2;
        // SAFETY: reposiciona uma janela viva desta thread.
        unsafe { SetWindowPos(hwnd, std::ptr::null_mut(), x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
    }

    /// Moves to and centers on the given monitor.
    pub fn move_to_monitor(&self, monitor: &Monitor) {
        let hwnd = self.core.hwnd();
        let work = monitor.work_area;
        // SAFETY: reposiciona uma janela viva; uma eventual mudança de DPI é tratada
        // por WM_DPICHANGED.
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                work.x + 32,
                work.y + 32,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
        self.center();
    }

    /// Current monitor.
    pub fn monitor(&self) -> Option<Monitor> {
        Monitor::from_window_handle(self.core.hwnd())
    }

    /// Outer rectangle including frame/title in screen pixels.
    pub fn outer_bounds(&self) -> Rect {
        Rect::from_native(native::window_rect(self.core.hwnd()))
    }

    /// Sets opacity 0.0 to 1.0, making a layered window.
    pub fn set_opacity(&self, opacity: f32) {
        let hwnd = self.core.hwnd();
        native::set_style_bits(hwnd, GWL_EXSTYLE, WS_EX_LAYERED, true);
        let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        // SAFETY: a janela tem WS_EX_LAYERED; HWND inválido apenas falha.
        unsafe { SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) };
    }

    /// Toggles Always on Top.
    pub fn set_topmost(&self, topmost: bool) {
        let after = if topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
        // SAFETY: altera apenas a ordem Z de uma janela viva.
        unsafe { SetWindowPos(self.core.hwnd(), after, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
    }

    /// Sets title/taskbar/Alt+Tab icon, overriding default or builder icon.
    ///
    /// Bundled icons ([`crate::icons`]) load small/large variants at system sizes for monitor DPI and reload on monitor
    /// changes.
    pub fn set_icon(&self, icon: &impl IconSource) {
        let previous = self.core.icon.borrow_mut().replace(WidgetIcon {
            source: icon.to_shared(),
            size: 0,
            position: Default::default(),
            auto_size: false,
            native_accepted: false,
            saved_style: 0,
        });
        apply_icon(&self.core);
        drop(previous);
    }

    /// Sets window menu bar.
    pub fn set_menu(&self, menu: &MenuBar) -> GuiResult<()> {
        let hwnd = self.core.require_alive()?;
        crate::menus::strip::drop_auto_strip(self);
        let size = self.client_size();
        // SAFETY: associa um HMENU válido (mantido vivo por `keep_alive`) à janela.
        if unsafe { SetMenu(hwnd, menu.raw()) } == 0 {
            return Err(GuiError::last_os_error("SetMenu"));
        }
        // Os atalhos dos itens são consultados a cada tecla (inclusive de itens
        // acrescentados depois desta chamada) e respeitam itens desabilitados.
        *self.core.menu_bar.borrow_mut() = Some(menu.clone());
        // No tema escuro a barra é exibida desenhada, com os mesmos menus.
        if crate::themes::dark::dark_theme(&self.core).is_some() {
            crate::menus::strip::sync_native_bar(self, true);
        }
        // Mantém a área cliente com o tamanho que tinha antes de a barra existir.
        let b = self.core.bounds();
        self.core.set_bounds(Rect::new(b.x, b.y, size.width, size.height));
        Ok(())
    }

    /// Attaches client-area layout, reapplied on resize/DPI changes.
    pub fn set_layout(&self, layout: impl Layout + 'static) {
        self.core.set_layout(Some(Rc::new(layout)));
    }

    /// Reapplies layout immediately after adding/removing controls.
    pub fn relayout(&self) {
        self.core.relayout();
    }

    /// Sets minimum client size.
    pub fn set_min_size(&self, width: i32, height: i32) {
        self.core.min_size.set(Some(Size::new(width, height)));
    }

    /// Sets maximum client size.
    pub fn set_max_size(&self, width: i32, height: i32) {
        self.core.max_size.set(Some(Size::new(width, height)));
    }

    /// Enables/disables file drops.
    pub fn set_accept_files(&self, accept: bool) {
        // SAFETY: tolera HWND nulo/inválido.
        unsafe { DragAcceptFiles(self.core.hwnd(), accept as BOOL) };
    }

    /// Registers close handler; returning `false` cancels.
    pub fn on_close_request(&self, f: impl Fn() -> bool + 'static) {
        if self.core.is_alive() {
            *self.core.close_handler.borrow_mut() = Some(Rc::new(f));
        }
    }

    /// Former name of [`on_close_request`](Self::on_close_request), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_close_request`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_close(&self, f: impl Fn() -> bool + 'static) {
        self.on_close_request(f)
    }

    /// Window moved (client-area screen pixels).
    pub fn on_move(&self, f: impl Fn(Point) + 'static) -> HandlerId {
        self.core.on(EventKind::Move, move |e| {
            if let Event::Move(p) = e {
                f(*p)
            }
        })
    }

    /// Window activated.
    pub fn on_activated(&self, f: impl Fn() + 'static) -> HandlerId {
        self.core.on(EventKind::Activated, move |_| f())
    }

    /// Window deactivated.
    pub fn on_deactivated(&self, f: impl Fn() + 'static) -> HandlerId {
        self.core.on(EventKind::Deactivated, move |_| f())
    }

    /// Files dropped onto the window (requires `accept_files`).
    pub fn on_drop_files(&self, f: impl Fn(&[PathBuf]) + 'static) -> HandlerId {
        self.core.on(EventKind::DropFiles, move |e| {
            if let Event::DropFiles(files) = e {
                f(files)
            }
        })
    }

    /// Window DPI changed, e.g. when moving monitors.
    pub fn on_dpi_changed(&self, f: impl Fn(u32) + 'static) -> HandlerId {
        self.core.on(EventKind::DpiChanged, move |e| {
            if let Event::DpiChanged(dpi) = e {
                f(*dpi)
            }
        })
    }

    /// System theme changed (light/dark/high contrast).
    pub fn on_theme_changed(&self, f: impl Fn() + 'static) -> HandlerId {
        self.core.on(EventKind::ThemeChanged, move |_| f())
    }

    /// Registers client drawing (GDI, double buffered by default).
    pub fn on_paint(&self, f: impl Fn(&Canvas) + 'static) {
        if self.core.is_alive() {
            *self.core.paint.borrow_mut() = Some(Rc::new(f));
            self.core.invalidate();
        }
    }

    /// Enables/disables custom drawing double buffering (default: on).
    pub fn set_double_buffered(&self, enabled: bool) {
        self.core.double_buffered.set(enabled);
    }

    /// Registers a shortcut active when the window/descendants have focus.
    pub fn add_shortcut(&self, shortcut: Shortcut, action: impl Fn() + 'static) {
        if self.core.is_alive() {
            self.core.shortcuts.borrow_mut().push((shortcut, Rc::new(action)));
        }
    }

    /// Shows modally: disables owner and returns only after this window closes.
    ///
    /// UI stays responsive through a nested event loop. Owner is set by [`WindowBuilder::owner`].
    pub fn show_modal(&self) -> GuiResult<()> {
        let hwnd = self.core.require_alive()?;
        let state = app::require()?;
        // SAFETY: consultas e alteração de estado da janela dona, na mesma thread.
        unsafe {
            let owner = GetWindow(hwnd, GW_OWNER);
            if !owner.is_null() && IsWindowEnabled(owner) != 0 {
                EnableWindow(owner, FALSE);
                self.core.modal_owner.set(owner);
            }
            ShowWindow(hwnd, SW_SHOW);
        }
        let core = self.core.clone();
        let result = event_loop::run_loop(&state, move || !core.is_alive());
        self.core.release_modal_owner();
        result
    }
}

/// Sends small/large icons at current DPI to the window.
pub(crate) fn apply_icon(core: &WidgetCore) {
    let hwnd = core.hwnd();
    let Some(icon) = core.icon.borrow().clone() else { return };
    let dpi = core.dpi();
    for (kind, metric) in [(ICON_SMALL, SM_CXSMICON), (ICON_BIG, SM_CXICON)] {
        // O ícone fica vivo pela origem guardada no núcleo ou pelo cache de ícones.
        if let Ok(image) = icon.source.icon_pixels(native::system_metric(metric, dpi)) {
            native::send(hwnd, WM_SETICON, kind, image.raw() as LPARAM);
        }
    }
}
