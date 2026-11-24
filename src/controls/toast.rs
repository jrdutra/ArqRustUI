//! [`Toast`]: temporary notification in the window corner.

use super::infobar::{draw_close_glyph, draw_severity_icon, NoticeColors, Severity};
use crate::core::application as app;
use crate::core::native;
use crate::core::widget::{lookup, Widget, WidgetCore};
use crate::core::{EventKind, GuiError, GuiResult};
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::graphics::{Canvas, Font, Pen, TextAlign};
use crate::menus::strip as menu_strip;
use crate::utilities::{to_logical, to_pixels, to_wide, Rect, Size};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};
use std::time::Duration;

const TOAST_CLASS: &str = "ArqRustUI.Toast";
/// Opacity animation timer.
const FADE_TIMER: usize = 1;
/// Display duration timer.
const LIFE_TIMER: usize = 2;
const FADE_STEP_MS: u32 = 15;
const LIFE_STEP_MS: u32 = 100;
/// Opacity steps per frame (255 in ~150 ms).
const FADE_DELTA: u8 = 26;

/// Logical toast width.
const WIDTH: i32 = 340;
const PAD: i32 = 14;
const ICON: i32 = 20;
const TEXT_X: i32 = PAD + ICON + 12;
const CLOSE: i32 = 24;
const ACTION_HEIGHT: i32 = 22;
/// Logical spacing from window edge and between toasts.
const MARGIN: i32 = 16;
const GAP: i32 = 8;

thread_local! {
    static TOASTS: RefCell<Vec<Rc<ToastState>>> = const { RefCell::new(Vec::new()) };
    static WATCHED: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
    static CLASS_READY: Cell<bool> = const { Cell::new(false) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    FadeIn,
    Shown,
    FadeOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Close,
    Action,
}

struct ToastState {
    owner: HWND,
    hwnd: Cell<HWND>,
    severity: Severity,
    title: String,
    message: String,
    action: Option<(String, Rc<dyn Fn()>)>,
    on_dismiss: Option<Rc<dyn Fn()>>,
    /// Remaining milliseconds (`None` = closes only by user/code).
    remaining: Cell<Option<u32>>,
    phase: Cell<Phase>,
    alpha: Cell<u8>,
    animate: bool,
    hovered: Cell<bool>,
    hot: Cell<Option<Part>>,
    /// Logical size.
    size: Cell<Size>,
    /// Logical action text top, if present.
    action_top: Cell<i32>,
    dpi: u32,
}

impl ToastState {
    fn close_rect(&self) -> Rect {
        Rect::new(self.size.get().width - CLOSE - 6, 6, CLOSE, CLOSE)
    }

    fn action_rect(&self, font: &Font) -> Option<Rect> {
        let (text, _) = self.action.as_ref()?;
        let width = menu_strip::with_measure(self.dpi, |c| c.measure_mnemonic(text, font).width).unwrap_or(80);
        Some(Rect::new(TEXT_X, self.action_top.get(), width + 4, ACTION_HEIGHT))
    }

    fn font(&self) -> Font {
        menu_strip::font_for_window(self.owner)
    }

    fn part_at(&self, x: i32, y: i32) -> Option<Part> {
        let point = crate::utilities::Point::new(to_logical(x, self.dpi), to_logical(y, self.dpi));
        if self.close_rect().contains(point) {
            return Some(Part::Close);
        }
        self.action_rect(&self.font()).filter(|r| r.contains(point)).map(|_| Part::Action)
    }
}

/// Temporary notification at a window's bottom-right corner, dismissed automatically.
///
/// Shows severity icon, title, wrapping message, optional action and `×`. Dismisses after [`duration`](Toast::duration)
/// (default 5 s), pausing on hover. Same-window toasts stack, follow moves/resizes and preserve focus. Brief
/// fade-in/out unless Windows reduces animations.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// Toast::new("File saved")
///     .message("report.pdf was saved in Documents.")
///     .severity(Severity::Success)
///     .action("Open folder", || println!("open"))
///     .show(window)?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Toast {
    title: String,
    message: String,
    severity: Severity,
    duration: Option<Duration>,
    action: Option<(String, Rc<dyn Fn()>)>,
    on_dismiss: Option<Rc<dyn Fn()>>,
}

impl std::fmt::Debug for Toast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toast").field("title", &self.title).field("severity", &self.severity).finish_non_exhaustive()
    }
}

impl Toast {
    /// Toast with the given title.
    pub fn new(title: &str) -> Toast {
        Toast {
            title: title.to_string(),
            message: String::new(),
            severity: Severity::Info,
            duration: Some(Duration::from_secs(5)),
            action: None,
            on_dismiss: None,
        }
    }

    /// Message below the title, automatically wrapped.
    pub fn message(mut self, message: &str) -> Toast {
        self.message = message.to_string();
        self
    }

    /// Severity (color/icon; default [`Severity::Info`]).
    pub fn severity(mut self, severity: Severity) -> Toast {
        self.severity = severity;
        self
    }

    /// Display duration (default 5 s).
    pub fn duration(mut self, duration: Duration) -> Toast {
        self.duration = Some(duration);
        self
    }

    /// No automatic dismissal: closes via `×`, action or [`ToastHandle::close`].
    pub fn persistent(mut self) -> Toast {
        self.duration = None;
        self
    }

    /// Action link below message; clicking executes `f` and closes the toast.
    pub fn action(mut self, text: &str, f: impl Fn() + 'static) -> Toast {
        self.action = Some((text.to_string(), Rc::new(f)));
        self
    }

    /// Callback on dismissal by timeout, user or code.
    pub fn on_dismissed(mut self, f: impl Fn() + 'static) -> Toast {
        self.on_dismiss = Some(Rc::new(f));
        self
    }

    /// Former name of [`on_dismissed`](Self::on_dismissed), retained for existing code.
    #[deprecated(since = "0.1.0", note = "use `on_dismissed`: events follow the naming convention in docs/EVENTS.md")]
    pub fn on_dismiss(self, f: impl Fn() + 'static) -> Toast {
        self.on_dismissed(f)
    }

    /// Shows a toast in `owner`'s window corner or its containing window.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<ToastHandle> {
        let owner = native::root_of(owner.core().require_alive()?);
        let owner_core = lookup(owner).ok_or(GuiError::WidgetDestroyed)?;
        if !register_class() {
            return Err(GuiError::last_os_error("RegisterClassExW"));
        }
        let dpi = native::dpi_for_window(owner);
        let font = menu_strip::font_for_window(owner);
        let bold = font.clone().bold();
        let text_width = WIDTH - TEXT_X - PAD - CLOSE + 6;
        let (title_h, message_h) = menu_strip::with_measure(dpi, |c| {
            let title =
                if self.title.is_empty() { 0 } else { c.measure_wrapped(&self.title, &bold, text_width).height };
            let message =
                if self.message.is_empty() { 0 } else { c.measure_wrapped(&self.message, &font, text_width).height };
            (title, message)
        })
        .unwrap_or((16, 0));
        let mut height = PAD + title_h;
        if message_h > 0 {
            height += if title_h > 0 { 2 } else { 0 } + message_h;
        }
        let action_top = height + 6;
        if self.action.is_some() {
            height = action_top + ACTION_HEIGHT;
        }
        height = (height + PAD).max(PAD * 2 + ICON);
        // SAFETY: ponteiro de saída válido para um BOOL.
        let animate = unsafe {
            let mut on: BOOL = 1;
            SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut on as *mut BOOL as *mut _, 0);
            on != 0
        };
        let state = Rc::new(ToastState {
            owner,
            hwnd: Cell::new(std::ptr::null_mut()),
            severity: self.severity,
            title: self.title.clone(),
            message: self.message.clone(),
            action: self.action.clone(),
            on_dismiss: self.on_dismiss.clone(),
            remaining: Cell::new(self.duration.map(|d| d.as_millis().min(u32::MAX as u128) as u32)),
            phase: Cell::new(if animate { Phase::FadeIn } else { Phase::Shown }),
            alpha: Cell::new(if animate { 0 } else { 255 }),
            animate,
            hovered: Cell::new(false),
            hot: Cell::new(None),
            size: Cell::new(Size::new(WIDTH, height)),
            action_top: Cell::new(action_top),
            dpi,
        });
        let class = to_wide(TOAST_CLASS);
        let name = to_wide(&spoken(&state));
        // SAFETY: strings terminadas em NUL; dono é uma janela viva desta thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | if animate { WS_EX_LAYERED } else { 0 },
                class.as_ptr(),
                name.as_ptr(),
                WS_POPUP,
                0,
                0,
                to_pixels(WIDTH, dpi),
                to_pixels(height, dpi),
                owner,
                std::ptr::null_mut(),
                native::instance(),
                std::ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err(GuiError::last_os_error("CreateWindowExW"));
        }
        state.hwnd.set(hwnd);
        if animate {
            // SAFETY: janela recém-criada desta thread, com WS_EX_LAYERED.
            unsafe { SetLayeredWindowAttributes(hwnd, 0, 0, LWA_ALPHA) };
        }
        if crate::system::WindowsVersion::current().is_windows_11_or_later() {
            let corner: u32 = 2; // DWMWCP_ROUND
                                 // SAFETY: valor local do tamanho informado; janela desta thread.
            unsafe {
                crate::ffi::dwmapi::DwmSetWindowAttribute(
                    hwnd,
                    crate::ffi::dwmapi::DWMWA_WINDOW_CORNER_PREFERENCE,
                    &corner as *const u32 as *const _,
                    4,
                );
            }
        }
        TOASTS.with(|t| t.borrow_mut().push(state.clone()));
        watch_owner(&owner_core);
        // O toast não recebe foco: o anúncio ativo leva o aviso ao leitor de tela
        // (Windows 10 1709+; a falha é inofensiva — o nome da janela continua lá).
        let _ = crate::accessibility::announce_with(
            &crate::core::Window { core: owner_core.clone() },
            &spoken(&state),
            self.severity == super::infobar::Severity::Error,
        );
        restack(owner);
        // SAFETY: temporizadores da própria janela; tratados em `toast_proc`.
        unsafe {
            if state.animate {
                SetTimer(hwnd, FADE_TIMER, FADE_STEP_MS, std::ptr::null());
            }
            if state.remaining.get().is_some() {
                SetTimer(hwnd, LIFE_TIMER, LIFE_STEP_MS, std::ptr::null());
            }
        }
        Ok(ToastHandle { state: Rc::downgrade(&state) })
    }

    /// Immediately closes all toasts in `owner`'s window.
    pub fn dismiss_all(owner: &impl Widget) {
        let owner = native::root_of(owner.core().hwnd());
        for state in toasts_of(owner) {
            destroy(&state);
        }
    }

    /// Open toast count in `owner`'s window.
    pub fn count(owner: &impl Widget) -> usize {
        toasts_of(native::root_of(owner.core().hwnd())).len()
    }
}

/// Displayed toast handle for early dismissal.
#[derive(Clone)]
pub struct ToastHandle {
    state: Weak<ToastState>,
}

impl std::fmt::Debug for ToastHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToastHandle").field("open", &self.is_open()).finish()
    }
}

impl ToastHandle {
    /// Closes with a fade when animations are enabled.
    pub fn close(&self) {
        if let Some(state) = self.state.upgrade() {
            dismiss(&state);
        }
    }

    /// Whether the toast is still displayed.
    pub fn is_open(&self) -> bool {
        self.state.upgrade().is_some_and(|s| !s.hwnd.get().is_null())
    }
}

fn spoken(state: &ToastState) -> String {
    match (state.title.is_empty(), state.message.is_empty()) {
        (false, false) => format!("{}. {}", state.title, state.message),
        (false, true) => state.title.clone(),
        _ => state.message.clone(),
    }
}

fn toasts_of(owner: HWND) -> Vec<Rc<ToastState>> {
    TOASTS.with(|t| t.borrow().iter().filter(|s| s.owner == owner).cloned().collect())
}

fn state_of(hwnd: HWND) -> Option<Rc<ToastState>> {
    TOASTS.with(|t| t.borrow().iter().find(|s| s.hwnd.get() == hwnd).cloned())
}

/// Repositions toasts: newest at bottom, others above.
fn restack(owner: HWND) {
    let toasts = toasts_of(owner);
    if toasts.is_empty() {
        return;
    }
    // SAFETY: consultas simples.
    let hidden = unsafe { IsIconic(owner) != 0 || IsWindowVisible(owner) == 0 };
    let dpi = native::dpi_for_window(owner);
    let client = native::client_rect(owner);
    let mut corner = POINT { x: client.right, y: client.bottom };
    // SAFETY: ponteiro válido para um POINT local.
    unsafe { ClientToScreen(owner, &mut corner) };
    let mut bottom = corner.y - to_pixels(MARGIN, dpi);
    for state in toasts.iter().rev() {
        let size = state.size.get();
        let (w, h) = (to_pixels(size.width, dpi), to_pixels(size.height, dpi));
        let x = corner.x - to_pixels(MARGIN, dpi) - w;
        let flags = SWP_NOACTIVATE | if hidden { SWP_HIDEWINDOW } else { SWP_SHOWWINDOW };
        // SAFETY: janela popup viva desta thread.
        unsafe { SetWindowPos(state.hwnd.get(), HWND_TOP, x, bottom - h, w, h, flags) };
        bottom -= h + to_pixels(GAP, dpi);
    }
}

/// Connects owner move/resize/destroy events once.
fn watch_owner(owner: &Rc<WidgetCore>) {
    let key = owner.hwnd() as isize;
    if !WATCHED.with(|w| w.borrow_mut().insert(key)) {
        return;
    }
    let hwnd = owner.hwnd();
    owner.on(EventKind::Move, move |_| restack(hwnd));
    owner.on(EventKind::Resize, move |_| restack(hwnd));
    owner.on(EventKind::Destroyed, move |_| {
        WATCHED.with(|w| w.borrow_mut().remove(&key));
        for state in toasts_of(hwnd) {
            destroy(&state);
        }
    });
}

/// Starts fading out or closes immediately without animations.
fn dismiss(state: &Rc<ToastState>) {
    let hwnd = state.hwnd.get();
    if hwnd.is_null() || state.phase.get() == Phase::FadeOut {
        return;
    }
    if !state.animate {
        destroy(state);
        return;
    }
    state.phase.set(Phase::FadeOut);
    set_layered(hwnd, true, state.alpha.get());
    // SAFETY: temporizador da própria janela.
    unsafe { SetTimer(hwnd, FADE_TIMER, FADE_STEP_MS, std::ptr::null()) };
}

/// Destroys the window, removes the toast from the stack and notifies `on_dismiss`.
fn destroy(state: &Rc<ToastState>) {
    let hwnd = state.hwnd.replace(std::ptr::null_mut());
    if hwnd.is_null() {
        return;
    }
    TOASTS.with(|t| t.borrow_mut().retain(|s| !Rc::ptr_eq(s, state)));
    // SAFETY: janela popup criada por este módulo, nesta thread (já fora do registro).
    unsafe { DestroyWindow(hwnd) };
    restack(state.owner);
    if let Some(f) = state.on_dismiss.clone() {
        f();
    }
}

fn on_timer(state: &Rc<ToastState>, id: usize) {
    let hwnd = state.hwnd.get();
    match id {
        FADE_TIMER => {
            let alpha = state.alpha.get();
            match state.phase.get() {
                Phase::FadeIn => {
                    let next = alpha.saturating_add(FADE_DELTA);
                    state.alpha.set(next);
                    if next == 255 {
                        state.phase.set(Phase::Shown);
                        // SAFETY: encerra o temporizador da própria janela.
                        unsafe { KillTimer(hwnd, FADE_TIMER) };
                        // Opaco: volta a ser uma janela comum (não depende de composição).
                        set_layered(hwnd, false, 255);
                        return;
                    }
                }
                Phase::FadeOut => {
                    let next = alpha.saturating_sub(FADE_DELTA);
                    state.alpha.set(next);
                    if next == 0 {
                        destroy(state);
                        return;
                    }
                }
                Phase::Shown => {
                    // SAFETY: encerra o temporizador da própria janela.
                    unsafe { KillTimer(hwnd, FADE_TIMER) };
                    return;
                }
            }
            // SAFETY: janela viva desta thread.
            unsafe { SetLayeredWindowAttributes(hwnd, 0, state.alpha.get(), LWA_ALPHA) };
        }
        LIFE_TIMER => {
            if state.hovered.get() || state.phase.get() == Phase::FadeOut {
                return;
            }
            let Some(remaining) = state.remaining.get() else { return };
            let left = remaining.saturating_sub(LIFE_STEP_MS);
            state.remaining.set(Some(left));
            if left == 0 {
                // SAFETY: encerra o temporizador da própria janela.
                unsafe { KillTimer(hwnd, LIFE_TIMER) };
                dismiss(state);
            }
        }
        _ => {}
    }
}

/// Toggles window transparency, used only during fades.
fn set_layered(hwnd: HWND, on: bool, alpha: u8) {
    native::set_style_bits(hwnd, GWL_EXSTYLE, WS_EX_LAYERED, on);
    // SAFETY: janela viva desta thread.
    unsafe {
        if on {
            SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
        } else {
            RedrawWindow(hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_UPDATENOW | RDW_FRAME);
        }
    }
}

fn paint(state: &ToastState, canvas: &Canvas) {
    let style = menu_strip::style_for_window(state.owner);
    let colors = NoticeColors::for_window(state.owner, state.severity);
    let size = canvas.size();
    let font = state.font();
    let bold = font.clone().bold();
    canvas.fill_rect(Rect::new(0, 0, size.width, size.height), style.popup_background);
    canvas.rect(Rect::new(0, 0, size.width, size.height), Some(&Pen::new(style.popup_border, 1)), None);
    draw_severity_icon(canvas, state.severity, PAD, PAD, ICON, colors.accent);
    let text_width = size.width - TEXT_X - PAD - CLOSE + 6;
    let mut y = PAD;
    if !state.title.is_empty() {
        let h = canvas.measure_wrapped(&state.title, &bold, text_width).height;
        canvas.wrapped_text(&state.title, Rect::new(TEXT_X, y, text_width, h), &bold, style.popup_foreground);
        y += h + 2;
    }
    if !state.message.is_empty() {
        let h = canvas.measure_wrapped(&state.message, &font, text_width).height;
        canvas.wrapped_text(&state.message, Rect::new(TEXT_X, y, text_width, h), &font, style.shortcut_foreground);
    }
    if let (Some((text, _)), Some(rect)) = (&state.action, state.action_rect(&font)) {
        let link = if state.hot.get() == Some(Part::Action) { font.clone().underline() } else { font.clone() };
        canvas.mnemonic_text(text, rect, &link, colors.accent, TextAlign::Left, false);
    }
    let close = state.close_rect();
    if state.hot.get() == Some(Part::Close) {
        canvas.round_rect(close, style.corner_radius, None, Some(style.highlight));
    }
    draw_close_glyph(canvas, close.x + close.width / 2, close.y + close.height / 2, style.popup_foreground);
}

fn register_class() -> bool {
    if CLASS_READY.with(Cell::get) {
        return true;
    }
    let name = to_wide(TOAST_CLASS);
    // SAFETY: cursor compartilhado do sistema (não precisa ser liberado).
    let cursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW as PCWSTR) };
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_DROPSHADOW,
        lpfnWndProc: Some(toast_proc),
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
    let ok = unsafe { RegisterClassExW(&class) } != 0
        || unsafe { crate::ffi::kernel32::GetLastError() } == crate::ffi::kernel32::ERROR_CLASS_ALREADY_EXISTS;
    CLASS_READY.with(|c| c.set(ok));
    ok
}

/// Toast window procedure.
///
/// # Safety
/// Called only by Windows on the window's owning thread.
unsafe extern "system" fn toast_proc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if msg == WM_MOUSEACTIVATE {
            return Some(MA_NOACTIVATE);
        }
        let state = state_of(hwnd)?;
        match msg {
            WM_ERASEBKGND => Some(1),
            WM_PAINT => {
                // SAFETY: PAINTSTRUCT aceita zeros; par BeginPaint/EndPaint desta janela.
                unsafe {
                    let mut ps: PAINTSTRUCT = std::mem::zeroed();
                    let hdc = BeginPaint(hwnd, &mut ps);
                    if !hdc.is_null() {
                        let client = native::client_rect(hwnd);
                        let background = menu_strip::style_for_window(state.owner).popup_background;
                        crate::graphics::gdi::paint_buffered(hdc, client, state.dpi, Some(background), |canvas| {
                            paint(&state, canvas)
                        });
                    }
                    EndPaint(hwnd, &ps);
                }
                Some(0)
            }
            WM_TIMER => {
                on_timer(&state, wp);
                Some(0)
            }
            WM_MOUSEMOVE => {
                if !state.hovered.replace(true) {
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: estrutura local corretamente dimensionada.
                    unsafe { TrackMouseEvent(&mut track) };
                }
                let hot = state.part_at(x_lparam(lp), y_lparam(lp));
                if state.hot.replace(hot) != hot {
                    // SAFETY: invalida a própria janela.
                    unsafe { InvalidateRect(hwnd, std::ptr::null(), FALSE) };
                }
                Some(0)
            }
            WM_MOUSELEAVE => {
                state.hovered.set(false);
                if state.hot.replace(None).is_some() {
                    // SAFETY: invalida a própria janela.
                    unsafe { InvalidateRect(hwnd, std::ptr::null(), FALSE) };
                }
                Some(0)
            }
            WM_LBUTTONUP => {
                match state.part_at(x_lparam(lp), y_lparam(lp)) {
                    Some(Part::Close) => dismiss(&state),
                    Some(Part::Action) => {
                        let action = state.action.as_ref().map(|(_, f)| f.clone());
                        dismiss(&state);
                        if let Some(action) = action {
                            action();
                        }
                    }
                    None => {}
                }
                Some(0)
            }
            WM_SETCURSOR if state.hot.get() == Some(Part::Action) => {
                // SAFETY: cursor compartilhado do sistema.
                unsafe { SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_HAND as PCWSTR)) };
                Some(1)
            }
            _ => None,
        }
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

/// Observable toast state for tests: title and opacity.
#[doc(hidden)]
pub fn debug_toasts(owner: &impl Widget) -> Vec<(String, u8)> {
    toasts_of(native::root_of(owner.core().hwnd())).iter().map(|s| (s.title.clone(), s.alpha.get())).collect()
}
