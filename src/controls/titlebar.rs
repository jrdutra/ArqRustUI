//! Custom title bar: removes system frame while retaining resize borders, snapping, animations and shadow. Library
//! draws icon, title, control space and minimize/maximize/restore/close buttons.

use super::common::ControlOptions;
use super::modern::{palette, publish};
use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleRole};
use crate::core::native;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, Window};
use crate::ffi::types::{x_lparam, y_lparam};
use crate::ffi::user32::*;
use crate::ffi::{HWND, POINT, RECT};
use crate::graphics::{Canvas, Font, Pen};
use crate::i18n::{library_text, LibraryText};
use crate::utilities::{to_logical, to_pixels, Color, Point, Rect};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

const HT_CAPTION: isize = 2;
const HT_MAXBUTTON: isize = 9;
const HT_LEFT: isize = 10;
const HT_RIGHT: isize = 11;
const HT_TOP: isize = 12;
const HT_TOPLEFT: isize = 13;
const HT_TOPRIGHT: isize = 14;
const HT_BOTTOM: isize = 15;
const HT_BOTTOMLEFT: isize = 16;
const HT_BOTTOMRIGHT: isize = 17;
const SM_CXSIZEFRAME: i32 = 32;
const SM_CYSIZEFRAME: i32 = 33;
const SM_CXPADDEDBORDER: i32 = 92;
const SC_MINIMIZE: usize = 0xF020;
const SC_MAXIMIZE: usize = 0xF030;
const SC_RESTORE: usize = 0xF120;

/// Default logical bar height and button width.
pub const TITLE_BAR_HEIGHT: i32 = 32;
const BUTTON: i32 = 46;

/// Title bar button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CaptionButton {
    /// Minimize.
    Minimize,
    /// Maximize or restore.
    Maximize,
    /// Close.
    Close,
}

/// Hit testing in window pixel coordinates: resize border, button, drag strip or client area. `frame` is resize border
/// thickness; `bar` is bar height; `buttons` is button width.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hit_test(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    frame: i32,
    bar: i32,
    button: i32,
    maximized: bool,
) -> isize {
    if !maximized {
        let (left, right, top, bottom) = (x < frame, x >= width - frame, y < frame, y >= height - frame);
        match (left, right, top, bottom) {
            (true, _, true, _) => return HT_TOPLEFT,
            (_, true, true, _) => return HT_TOPRIGHT,
            (true, _, _, true) => return HT_BOTTOMLEFT,
            (_, true, _, true) => return HT_BOTTOMRIGHT,
            (true, ..) => return HT_LEFT,
            (_, true, ..) => return HT_RIGHT,
            (_, _, true, _) => return HT_TOP,
            (.., true) => return HT_BOTTOM,
            _ => {}
        }
    }
    if y < bar {
        // O botão maximizar responde como o do sistema (layouts de encaixe do Windows 11).
        if x >= width - 2 * button && x < width - button {
            return HT_MAXBUTTON;
        }
        return HT_CAPTION;
    }
    HTCLIENT as isize
}

struct TitleState {
    window: Weak<WidgetCore>,
    hot: Cell<Option<CaptionButton>>,
    pressed: Cell<Option<CaptionButton>>,
    active: Cell<bool>,
    height: Cell<i32>,
    /// Widths reserved for application controls on the left after the title.
    reserved: RefCell<Vec<Rect>>,
}

/// Library-drawn title bar created by [`TitleBar::attach`], removing the system frame while retaining border resizing,
/// title dragging, double-click maximizing, Alt+Space, snapping and Windows 11 maximize-button snap layouts.
///
/// Place the bar at the top of the window layout:
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo() -> GuiResult<()> {
/// let window = Window::builder().title("Editor").size(800, 600).build()?;
/// let bar = TitleBar::attach(&window)?;
/// let search = TextBox::builder().placeholder("Search").parent(&bar).build()?;
/// bar.add_control(&search, 220); // controls inside the bar remain clickable
/// let root = StackLayout::vertical();
/// root.add_with(&bar, LayoutParams::new().height(bar.height()));
/// // ... content
/// window.set_layout(root);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct TitleBar {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<TitleState>,
}

impl Widget for TitleBar {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for TitleBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TitleBar").field("core", &self.core).finish()
    }
}

fn frame_thickness(dpi: u32) -> (i32, i32) {
    // SAFETY: consultas de métricas do sistema.
    let (x, y, padded) = unsafe {
        (GetSystemMetrics(SM_CXSIZEFRAME), GetSystemMetrics(SM_CYSIZEFRAME), GetSystemMetrics(SM_CXPADDEDBORDER))
    };
    let scale = |v: i32| v * dpi as i32 / 96;
    (scale(x + padded).max(4), scale(y + padded).max(4))
}

fn is_maximized(hwnd: HWND) -> bool {
    // SAFETY: consulta simples.
    unsafe { IsZoomed(hwnd) != 0 }
}

impl TitleBar {
    /// Removes `window`'s system frame and creates the bar at its top.
    pub fn attach(window: &Window) -> GuiResult<TitleBar> {
        let window_core = window.core().clone();
        let hwnd = window_core.require_alive()?;
        let mut opts = ControlOptions::new(window.client_size().width, TITLE_BAR_HEIGHT);
        opts.parent = Some(window_core.clone());
        opts.text = window.title();
        let core = opts.create("TitleBar", PANEL_CLASS, WS_CLIPCHILDREN, 0, false, None)?;
        let state = Rc::new(TitleState {
            window: Rc::downgrade(&window_core),
            hot: Cell::new(None),
            pressed: Cell::new(None),
            active: Cell::new(true),
            height: Cell::new(TITLE_BAR_HEIGHT),
            reserved: RefCell::new(Vec::new()),
        });
        let bar = TitleBar { core, state };
        bar.install(&window_core);
        // Sombra e cantos do DWM continuam com uma margem mínima de quadro.
        let margins =
            crate::ffi::dwmapi::MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 1, cyBottomHeight: 0 };
        // SAFETY: janela viva desta thread; estrutura local. Sem DWM (Wine) só falha.
        unsafe { crate::ffi::dwmapi::DwmExtendFrameIntoClientArea(hwnd, &margins) };
        // SAFETY: recalcula o quadro da janela viva.
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
        Ok(bar)
    }

    fn install(&self, window: &Rc<WidgetCore>) {
        let state = Rc::downgrade(&self.state);
        let target = Rc::downgrade(&self.core);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(state), Some(core)) = (state.upgrade(), target.upgrade()) {
                paint(&core, &state, canvas);
            }
        }));
        // A janela: sem moldura do sistema e com as regiões de arrastar/redimensionar.
        let bar = Rc::downgrade(&self.core);
        let bar_state = Rc::downgrade(&self.state);
        window.add_message_filter(move |raw| {
            let hwnd = raw.hwnd as HWND;
            match raw.message {
                WM_NCCALCSIZE if raw.wparam != 0 => {
                    if is_maximized(hwnd) {
                        // Maximizada, a janela passa da tela pela espessura da borda.
                        let dpi = native::dpi_for_window(hwnd);
                        let (fx, fy) = frame_thickness(dpi);
                        // SAFETY: com WPARAM verdadeiro o LPARAM aponta para NCCALCSIZE_PARAMS.
                        let params = unsafe { &mut *(raw.lparam as *mut NCCALCSIZE_PARAMS) };
                        let rect = &mut params.rgrc[0];
                        rect.left += fx;
                        rect.right -= fx;
                        rect.top += fy;
                        rect.bottom -= fy;
                    }
                    Some(0)
                }
                WM_NCHITTEST => {
                    let (Some(_), Some(state)) = (bar.upgrade(), bar_state.upgrade()) else { return None };
                    let mut window_rect = RECT::default();
                    // SAFETY: consulta de geometria da janela viva.
                    unsafe { GetWindowRect(hwnd, &mut window_rect) };
                    let dpi = native::dpi_for_window(hwnd);
                    let (frame, _) = frame_thickness(dpi);
                    let x = x_lparam(raw.lparam) - window_rect.left;
                    let y = y_lparam(raw.lparam) - window_rect.top;
                    let hit = hit_test(
                        x,
                        y,
                        window_rect.right - window_rect.left,
                        window_rect.bottom - window_rect.top,
                        frame,
                        to_pixels(state.height.get(), dpi),
                        to_pixels(BUTTON, dpi),
                        is_maximized(hwnd),
                    );
                    Some(hit)
                }
                WM_NCLBUTTONDOWN if raw.wparam as isize == HT_MAXBUTTON => {
                    if let Some(state) = bar_state.upgrade() {
                        state.pressed.set(Some(CaptionButton::Maximize));
                    }
                    Some(0)
                }
                WM_NCLBUTTONUP if raw.wparam as isize == HT_MAXBUTTON => {
                    let pressed = bar_state.upgrade().and_then(|s| s.pressed.replace(None));
                    if pressed == Some(CaptionButton::Maximize) {
                        toggle_maximized(hwnd);
                    }
                    Some(0)
                }
                WM_NCMOUSEMOVE => {
                    let (Some(core), Some(state)) = (bar.upgrade(), bar_state.upgrade()) else { return None };
                    let hot = (raw.wparam as isize == HT_MAXBUTTON).then_some(CaptionButton::Maximize);
                    if state.hot.replace(hot) != hot {
                        core.invalidate();
                    }
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE | TME_NONCLIENT,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: estrutura local corretamente dimensionada.
                    unsafe { TrackMouseEvent(&mut track) };
                    None
                }
                WM_NCMOUSELEAVE => {
                    if let (Some(core), Some(state)) = (bar.upgrade(), bar_state.upgrade()) {
                        if state.hot.get() == Some(CaptionButton::Maximize) {
                            state.hot.set(None);
                            core.invalidate();
                        }
                        state.pressed.set(None);
                    }
                    None
                }
                _ => None,
            }
        });
        // A barra: transparente ao teste do mouse fora dos botões minimizar/fechar e dos
        // controles da aplicação (a janela responde "título" ou "maximizar").
        let me = Rc::downgrade(&self.state);
        self.core.add_message_filter(move |raw| {
            if raw.message != WM_NCHITTEST {
                return None;
            }
            let state = me.upgrade()?;
            let mut point = POINT { x: x_lparam(raw.lparam), y: y_lparam(raw.lparam) };
            // SAFETY: converte para a barra viva desta thread.
            unsafe { ScreenToClient(raw.hwnd as HWND, &mut point) };
            let dpi = native::dpi_for_window(raw.hwnd as HWND);
            let at = Point::new(to_logical(point.x, dpi), to_logical(point.y, dpi));
            let width = to_logical(native::client_rect(raw.hwnd as HWND).right, dpi);
            match button_at(at, width, state.height.get()) {
                Some(CaptionButton::Minimize) | Some(CaptionButton::Close) => Some(HTCLIENT as isize),
                _ => Some(HTTRANSPARENT),
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseMove, move |event| {
            if let (Some(bar), Event::MouseMove(m)) = (me.upgrade(), event) {
                let hot = button_at(m.position, bar.client_size().width, bar.height());
                if bar.state.hot.replace(hot) != hot {
                    bar.core.invalidate();
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseLeave, move |_| {
            if let Some(bar) = me.upgrade() {
                bar.state.hot.set(None);
                bar.state.pressed.set(None);
                bar.core.invalidate();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseDown, move |event| {
            if let (Some(bar), Event::MouseDown(m)) = (me.upgrade(), event) {
                bar.state.pressed.set(button_at(m.position, bar.client_size().width, bar.height()));
                bar.core.invalidate();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseUp, move |event| {
            if let (Some(bar), Event::MouseUp(m)) = (me.upgrade(), event) {
                let pressed = bar.state.pressed.replace(None);
                let here = button_at(m.position, bar.client_size().width, bar.height());
                bar.core.invalidate();
                if let (Some(button), true) = (pressed, pressed == here) {
                    bar.press(button);
                }
            }
        });
        // Acompanha a largura, o título e a ativação da janela.
        let (me, owner) = (self.weak(), Rc::downgrade(window));
        window.on(EventKind::Resize, move |_| {
            if let (Some(bar), Some(owner)) = (me.upgrade(), owner.upgrade()) {
                let width = owner.client_size().width;
                let b = bar.bounds();
                if b.width != width {
                    bar.set_bounds(Rect::new(b.x, b.y, width, b.height));
                }
                bar.core.invalidate();
            }
        });
        for (kind, active) in [(EventKind::Activated, true), (EventKind::Deactivated, false)] {
            let me = self.weak();
            window.on(kind, move |_| {
                if let Some(bar) = me.upgrade() {
                    bar.state.active.set(active);
                    bar.core.invalidate();
                }
            });
        }
        publish(
            &self.core,
            &self.state,
            |core, state| {
                let window = state.window.upgrade().map(|w| native::window_text(w.hwnd())).unwrap_or_default();
                let root = AccessibleItem::new(0, AccessibleRole::TitleBar, &window);
                let width = to_logical(native::client_rect(core.hwnd()).right, core.dpi());
                let maximized = state.window.upgrade().is_some_and(|w| is_maximized(w.hwnd()));
                let names = [
                    (CaptionButton::Minimize, library_text(LibraryText::Minimize)),
                    (
                        CaptionButton::Maximize,
                        library_text(if maximized { LibraryText::Restore } else { LibraryText::Maximize }),
                    ),
                    (CaptionButton::Close, library_text(LibraryText::Close)),
                ];
                let children = names
                    .iter()
                    .enumerate()
                    .map(|(i, (button, name))| {
                        AccessibleItem::new(1 + i as u32, AccessibleRole::Button, name)
                            .bounds(button_rect(*button, width, state.height.get()))
                            .invokable()
                    })
                    .collect();
                (root, children)
            },
            |core, state, id, action| {
                let button = match id {
                    1 => CaptionButton::Minimize,
                    2 => CaptionButton::Maximize,
                    3 => CaptionButton::Close,
                    _ => return false,
                };
                if action != AccessibleAction::Invoke {
                    return false;
                }
                TitleBar { core: core.clone(), state: state.clone() }.press(button);
                true
            },
        );
    }

    fn weak(&self) -> WeakTitle {
        WeakTitle { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    fn window_hwnd(&self) -> Option<HWND> {
        self.state.window.upgrade().map(|w| w.hwnd())
    }

    /// Invokes a button as if clicked.
    pub fn press(&self, button: CaptionButton) {
        let Some(hwnd) = self.window_hwnd() else { return };
        match button {
            // SAFETY: comandos de sistema para a janela viva desta thread.
            CaptionButton::Minimize => unsafe {
                PostMessageW(hwnd, WM_SYSCOMMAND, SC_MINIMIZE, 0);
            },
            CaptionButton::Maximize => toggle_maximized(hwnd),
            // SAFETY: idem.
            CaptionButton::Close => unsafe {
                PostMessageW(hwnd, WM_SYSCOMMAND, SC_CLOSE, 0);
            },
        }
    }

    /// Logical bar height.
    pub fn height(&self) -> i32 {
        self.state.height.get()
    }

    /// Changes bar height (minimum 24).
    pub fn set_height(&self, height: i32) {
        self.state.height.set(height.max(24));
        let b = self.bounds();
        self.set_bounds(Rect::new(b.x, b.y, b.width, self.height()));
    }

    /// Places a child control after the title at the given logical width; it remains clickable while the remaining
    /// strip drags the window.
    pub fn add_control(&self, widget: &impl Widget, width: i32) {
        let x = {
            let reserved = self.state.reserved.borrow();
            reserved.last().map_or(self.title_end() + 16, |r| r.x + r.width + 8)
        };
        let height = (self.height() - 6).min(crate::controls::DEFAULT_CONTROL_HEIGHT);
        let rect = Rect::new(x, (self.height() - height) / 2, width, height);
        widget.set_bounds(rect);
        self.state.reserved.borrow_mut().push(rect);
    }

    fn title_end(&self) -> i32 {
        let title = self.window_hwnd().map_or(String::new(), native::window_text);
        let mut width = 0;
        let _ =
            crate::graphics::Bitmap::render(1, 1, |canvas| width = canvas.measure_text(&title, &Font::system()).width);
        12 + 16 + 8 + width
    }

    /// Logical button rectangle.
    pub fn button_bounds(&self, button: CaptionButton) -> Rect {
        button_rect(button, self.client_size().width, self.height())
    }
}

fn toggle_maximized(hwnd: HWND) {
    let command = if is_maximized(hwnd) { SC_RESTORE } else { SC_MAXIMIZE };
    // SAFETY: comando de sistema para a janela viva desta thread.
    unsafe { PostMessageW(hwnd, WM_SYSCOMMAND, command, 0) };
}

fn button_rect(button: CaptionButton, width: i32, height: i32) -> Rect {
    let slot = match button {
        CaptionButton::Close => 1,
        CaptionButton::Maximize => 2,
        CaptionButton::Minimize => 3,
    };
    Rect::new(width - slot * BUTTON, 0, BUTTON, height)
}

fn button_at(at: Point, width: i32, height: i32) -> Option<CaptionButton> {
    if at.y < 0 || at.y >= height {
        return None;
    }
    [CaptionButton::Minimize, CaptionButton::Maximize, CaptionButton::Close].into_iter().find(|b| {
        let r = button_rect(*b, width, height);
        at.x >= r.x && at.x < r.x + r.width
    })
}

#[derive(Clone)]
struct WeakTitle {
    core: Weak<WidgetCore>,
    state: Weak<TitleState>,
}

impl WeakTitle {
    fn upgrade(&self) -> Option<TitleBar> {
        Some(TitleBar { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

fn paint(core: &WidgetCore, state: &TitleState, canvas: &Canvas) {
    let p = palette(core);
    let size = canvas.size();
    canvas.clear(p.background);
    let window = state.window.upgrade();
    let active = state.active.get();
    let text = if active { p.foreground } else { p.muted };
    let title = window.as_ref().map(|w| native::window_text(w.hwnd())).unwrap_or_default();
    let mut x = 12;
    // Ícone pequeno da janela, se houver (WM_GETICON / ICON_SMALL2).
    let icon = window.as_ref().map_or(0, |w| native::send(w.hwnd(), 0x007F, 2, 0));
    if icon != 0 {
        // SAFETY: HICON da janela viva, só desenhado; coordenadas lógicas do canvas.
        unsafe { DrawIconEx(canvas.hdc(), x, (size.height - 16) / 2, icon as _, 16, 16, 0, std::ptr::null_mut(), 3) };
        x += 24;
    }
    canvas.text_line(&title, Rect::new(x, 0, (size.width - x - 3 * BUTTON).max(0), size.height), &Font::system(), text);
    let maximized = window.as_ref().is_some_and(|w| is_maximized(w.hwnd()));
    for button in [CaptionButton::Minimize, CaptionButton::Maximize, CaptionButton::Close] {
        let r = button_rect(button, size.width, size.height);
        let hot = state.hot.get() == Some(button);
        let pressed = state.pressed.get() == Some(button);
        let (fill, glyph) = match (button, hot || pressed) {
            (CaptionButton::Close, true) => (Some(Color::rgb(0xC4, 0x2B, 0x1C)), Color::WHITE),
            (_, true) => (Some(if pressed { p.hover.mix(p.foreground, 0.1) } else { p.hover }), text),
            _ => (None, text),
        };
        if let Some(fill) = fill {
            canvas.fill_rect(r, fill);
        }
        let (cx, cy) = (r.x + r.width / 2, r.y + r.height / 2);
        let pen = Pen::new(glyph, 1);
        match button {
            CaptionButton::Minimize => canvas.line(Point::new(cx - 5, cy), Point::new(cx + 5, cy), &pen),
            CaptionButton::Maximize if maximized => {
                canvas.rect(Rect::new(cx - 5, cy - 3, 8, 8), Some(&pen), None);
                canvas.polyline(
                    &[
                        Point::new(cx - 3, cy - 3),
                        Point::new(cx - 3, cy - 5),
                        Point::new(cx + 5, cy - 5),
                        Point::new(cx + 5, cy + 3),
                        Point::new(cx + 3, cy + 3),
                    ],
                    &pen,
                );
            }
            CaptionButton::Maximize => canvas.rect(Rect::new(cx - 5, cy - 5, 10, 10), Some(&pen), None),
            CaptionButton::Close => {
                canvas.line(Point::new(cx - 5, cy - 5), Point::new(cx + 5, cy + 5), &pen);
                canvas.line(Point::new(cx + 5, cy - 5), Point::new(cx - 5, cy + 5), &pen);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_testing_regions() {
        let t = |x, y, maximized| hit_test(x, y, 800, 600, 8, 32, 46, maximized);
        assert_eq!(t(2, 2, false), HT_TOPLEFT);
        assert_eq!(t(798, 300, false), HT_RIGHT);
        assert_eq!(t(400, 597, false), HT_BOTTOM);
        assert_eq!(t(400, 3, false), HT_TOP);
        assert_eq!(t(400, 20, false), HT_CAPTION);
        assert_eq!(t(800 - 46 - 10, 20, false), HT_MAXBUTTON);
        assert_eq!(t(400, 300, false), HTCLIENT as isize);
        assert_eq!(t(2, 2, true), HT_CAPTION, "maximizada não redimensiona pelas bordas");
        assert_eq!(button_at(Point::new(790, 10), 800, 32), Some(CaptionButton::Close));
        assert_eq!(button_at(Point::new(800 - 3 * 46 + 1, 10), 800, 32), Some(CaptionButton::Minimize));
        assert_eq!(button_at(Point::new(100, 10), 800, 32), None);
    }
}
