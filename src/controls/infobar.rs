//! [`InfoBar`]: in-window notification strip, and infrastructure shared with [`Toast`](super::toast::Toast):
//! [`Severity`], colors and severity icons.

use super::button::Button;
use super::common::{builder_common, ControlOptions};
use crate::core::native;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::{Event, EventKind, GuiResult};
use crate::ffi::user32::*;
use crate::graphics::{Canvas, Pen, TextAlign};
use crate::input::MouseButton;
use crate::utilities::{Color, Point, Rect};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Default logical [`InfoBar`] height.
pub const INFO_BAR_HEIGHT: i32 = 44;

/// Notification severity: determines color and icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Severity {
    /// Neutral information (blue).
    #[default]
    Info,
    /// Completed operation (green).
    Success,
    /// Warning (yellow/orange).
    Warning,
    /// Error (red).
    Error,
}

impl Severity {
    /// Severity accent color adjusted for light/dark backgrounds.
    pub fn accent(self, dark: bool) -> Color {
        match (self, dark) {
            (Severity::Info, false) => Color::rgb(0, 95, 184),
            (Severity::Info, true) => Color::rgb(96, 205, 255),
            (Severity::Success, false) => Color::rgb(15, 123, 15),
            (Severity::Success, true) => Color::rgb(108, 203, 95),
            (Severity::Warning, false) => Color::rgb(157, 93, 0),
            (Severity::Warning, true) => Color::rgb(252, 225, 0),
            (Severity::Error, false) => Color::rgb(196, 43, 28),
            (Severity::Error, true) => Color::rgb(255, 153, 164),
        }
    }

    /// Name read by screen readers.
    pub(crate) fn spoken(self) -> String {
        use crate::i18n::{library_text, LibraryText};
        library_text(match self {
            Severity::Info => LibraryText::Information,
            Severity::Success => LibraryText::Success,
            Severity::Warning => LibraryText::Warning,
            Severity::Error => LibraryText::Error,
        })
    }
}

/// Notification colors derived from window theme or system.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NoticeColors {
    /// Window background surrounding the notification.
    pub window: Color,
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
    pub hover: Color,
}

impl NoticeColors {
    pub(crate) fn for_window(hwnd: crate::ffi::HWND, severity: Severity) -> NoticeColors {
        let root = crate::core::widget::lookup(native::root_of(hwnd));
        let theme = root.as_ref().and_then(|r| r.theme.borrow().clone());
        let high_contrast = crate::system::accessibility::high_contrast();
        // SAFETY: consultas de cores do sistema.
        let system = |index| Color::from_colorref(unsafe { GetSysColor(index) });
        let (window, foreground, dark) = match (&theme, high_contrast) {
            (Some(theme), false) => (theme.background, theme.foreground, theme.dark),
            _ => (system(COLOR_BTNFACE), system(COLOR_BTNTEXT), false),
        };
        if high_contrast {
            return NoticeColors {
                window,
                background: system(COLOR_WINDOW),
                border: system(COLOR_WINDOWTEXT),
                foreground: system(COLOR_WINDOWTEXT),
                muted: system(COLOR_WINDOWTEXT),
                accent: system(COLOR_HIGHLIGHT),
                hover: system(COLOR_HIGHLIGHT),
            };
        }
        let accent = severity.accent(dark);
        let background = window.mix(accent, if dark { 0.16 } else { 0.1 });
        NoticeColors {
            window,
            background,
            border: background.mix(accent, 0.35),
            foreground,
            muted: foreground.mix(background, 0.25),
            accent,
            hover: background.mix(foreground, 0.1),
        }
    }
}

/// Severity icon: accent-colored circle with a cutout symbol (side `size`).
pub(crate) fn draw_severity_icon(canvas: &Canvas, severity: Severity, x: i32, y: i32, size: i32, accent: Color) {
    let glyph = if accent.luminance() > 0.55 { Color::BLACK } else { Color::WHITE };
    let drawn = canvas.with_gdiplus(|g| {
        let (x, y, s) = (x as f32, y as f32, size as f32);
        g.fill_ellipse(x, y, s, s, accent);
        let (cx, cy) = (x + s / 2.0, y + s / 2.0);
        let stroke = (s / 9.0).max(1.5);
        match severity {
            Severity::Info => {
                g.fill_ellipse(cx - stroke * 0.6, y + s * 0.22, stroke * 1.2, stroke * 1.2, glyph);
                g.line(cx, y + s * 0.42, cx, y + s * 0.76, glyph, stroke);
            }
            Severity::Warning => {
                g.line(cx, y + s * 0.24, cx, y + s * 0.58, glyph, stroke);
                g.fill_ellipse(cx - stroke * 0.6, y + s * 0.68, stroke * 1.2, stroke * 1.2, glyph);
            }
            Severity::Success => {
                let points = [(x + s * 0.28, cy), (x + s * 0.44, y + s * 0.66), (x + s * 0.73, y + s * 0.36)];
                g.line(points[0].0, points[0].1, points[1].0, points[1].1, glyph, stroke);
                g.line(points[1].0, points[1].1, points[2].0, points[2].1, glyph, stroke);
            }
            Severity::Error => {
                let d = s * 0.2;
                g.line(cx - d, cy - d, cx + d, cy + d, glyph, stroke);
                g.line(cx + d, cy - d, cx - d, cy + d, glyph, stroke);
            }
        }
    });
    if drawn.is_err() {
        // Sem GDI+: círculo simples, sem antialiasing.
        canvas.ellipse(Rect::new(x, y, size, size), None, Some(accent));
    }
}

/// Close `×` centered at (`cx`, `cy`).
pub(crate) fn draw_close_glyph(canvas: &Canvas, cx: i32, cy: i32, color: Color) {
    let pen = Pen::new(color, 1);
    canvas.line(Point::new(cx - 4, cy - 4), Point::new(cx + 5, cy + 5), &pen);
    canvas.line(Point::new(cx + 4, cy - 4), Point::new(cx - 5, cy + 5), &pen);
}

/// Horizontal padding.
const PAD: i32 = 12;
/// Severity icon size.
const ICON: i32 = 18;
/// Close button width.
const CLOSE: i32 = 32;
/// Action button spacing.
const ACTION_GAP: i32 = 8;

struct InfoState {
    core: Weak<WidgetCore>,
    severity: Cell<Severity>,
    title: RefCell<String>,
    message: RefCell<String>,
    closable: Cell<bool>,
    actions: RefCell<Vec<Button>>,
    close_hot: Cell<bool>,
    close_pressed: Cell<bool>,
    on_close: RefCell<Vec<Rc<dyn Fn()>>>,
}

impl InfoState {
    fn core(&self) -> Option<Rc<WidgetCore>> {
        self.core.upgrade().filter(|c| c.is_alive())
    }

    fn size(&self) -> (i32, i32) {
        self.core().map(|c| (c.bounds().width, c.bounds().height)).unwrap_or((0, 0))
    }

    fn close_rect(&self) -> Option<Rect> {
        if !self.closable.get() {
            return None;
        }
        let (w, h) = self.size();
        Some(Rect::new(w - CLOSE - 4, (h - 28) / 2, CLOSE, 28))
    }

    /// Positions action buttons on the right before the close button.
    fn place_actions(&self) {
        let (w, h) = self.size();
        let mut right = w - if self.closable.get() { CLOSE + 8 } else { PAD };
        let actions = self.actions.borrow().clone();
        for button in actions.iter().rev() {
            let width = button.bounds().width;
            right -= width;
            button.set_bounds(Rect::new(right, (h - 28) / 2, width, 28));
            right -= ACTION_GAP;
        }
    }

    /// Screen reader text: severity, title and message.
    fn refresh_name(&self) {
        let Some(core) = self.core() else { return };
        let title = self.title.borrow();
        let message = self.message.borrow();
        let spoken = match (title.is_empty(), message.is_empty()) {
            (false, false) => format!("{}: {title}. {message}", self.severity.get().spoken()),
            (false, true) => format!("{}: {title}", self.severity.get().spoken()),
            _ => format!("{}: {message}", self.severity.get().spoken()),
        };
        native::set_window_text(core.hwnd(), &spoken);
        core.background.set(Some(NoticeColors::for_window(core.hwnd(), self.severity.get()).background));
        core.invalidate();
    }
}

fn paint(state: &InfoState, canvas: &Canvas) {
    let Some(core) = state.core() else { return };
    let colors = NoticeColors::for_window(core.hwnd(), state.severity.get());
    let size = canvas.size();
    canvas.fill_rect(Rect::new(0, 0, size.width, size.height), colors.window);
    let frame = Rect::new(0, 0, size.width, size.height);
    canvas.round_rect(frame, 4, Some(&Pen::new(colors.border, 1)), Some(colors.background));
    let icon_y = (size.height - ICON) / 2;
    draw_severity_icon(canvas, state.severity.get(), PAD, icon_y, ICON, colors.accent);

    let font = core.font();
    let bold = font.clone().bold();
    let left = PAD + ICON + 10;
    let actions_left = state
        .actions
        .borrow()
        .iter()
        .map(|b| b.bounds().x)
        .min()
        .unwrap_or(size.width - if state.closable.get() { CLOSE + 8 } else { PAD });
    let right = actions_left - ACTION_GAP;
    let title = state.title.borrow();
    let mut x = left;
    if !title.is_empty() {
        let width = canvas.measure_mnemonic(&title, &bold).width.min((right - x).max(0));
        canvas.mnemonic_text(
            &title,
            Rect::new(x, 0, width, size.height),
            &bold,
            colors.foreground,
            TextAlign::Left,
            false,
        );
        x += width + 8;
    }
    let message = state.message.borrow();
    if !message.is_empty() && x < right {
        let area = Rect::new(x, 0, right - x, size.height);
        canvas.mnemonic_text(&message, area, &font, colors.muted, TextAlign::Left, false);
    }
    if let Some(close) = state.close_rect() {
        if state.close_hot.get() || state.close_pressed.get() {
            let fill = if state.close_pressed.get() { colors.hover.mix(colors.foreground, 0.08) } else { colors.hover };
            canvas.round_rect(close, 4, None, Some(fill));
        }
        draw_close_glyph(canvas, close.x + close.width / 2, close.y + close.height / 2, colors.foreground);
    }
}

/// In-window notification strip: severity icon, title, message, action buttons and close button.
///
/// Regular control: reserve [`INFO_BAR_HEIGHT`] or the chosen height in layouts. [`open`](InfoBar::open) and
/// [`close`](InfoBar::close) show/hide it and relayout the window, adding/removing its space. Colors follow severity
/// and window theme (light, dark or high contrast); screen readers announce severity, title and message.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let notice = InfoBar::builder()
///     .severity(Severity::Warning)
///     .title("No connection")
///     .message("Changes will be sent when the network returns.")
///     .parent(window)
///     .build()?;
/// notice.add_action("Try now", || println!("reconnect"))?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct InfoBar {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<InfoState>,
}

impl std::fmt::Debug for InfoBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InfoBar").field("severity", &self.state.severity.get()).finish_non_exhaustive()
    }
}

impl Widget for InfoBar {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

/// Builder for [`InfoBar`].
#[derive(Clone, Debug)]
pub struct InfoBarBuilder {
    opts: ControlOptions,
    severity: Severity,
    title: String,
    message: String,
    closable: bool,
}

impl InfoBarBuilder {
    builder_common!();

    /// Severity (default: [`Severity::Info`]).
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Bold title.
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Message to the right of the title.
    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    /// Shows the close `×` button (default: yes).
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Creates the strip.
    pub fn build(self) -> GuiResult<InfoBar> {
        let core = self.opts.create(
            "InfoBar",
            crate::core::window::PANEL_CLASS,
            WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            WS_EX_CONTROLPARENT,
            false,
            None,
        )?;
        let state = Rc::new(InfoState {
            core: Rc::downgrade(&core),
            severity: Cell::new(self.severity),
            title: RefCell::new(self.title),
            message: RefCell::new(self.message),
            closable: Cell::new(self.closable),
            actions: RefCell::new(Vec::new()),
            close_hot: Cell::new(false),
            close_pressed: Cell::new(false),
            on_close: RefCell::new(Vec::new()),
        });
        let bar = InfoBar { core, state };
        bar.wire();
        bar.state.refresh_name();
        Ok(bar)
    }
}

impl InfoBar {
    /// Starts building (default size 480 × [`INFO_BAR_HEIGHT`]).
    pub fn builder() -> InfoBarBuilder {
        InfoBarBuilder {
            opts: ControlOptions::new(480, INFO_BAR_HEIGHT),
            severity: Severity::Info,
            title: String::new(),
            message: String::new(),
            closable: true,
        }
    }

    fn wire(&self) {
        let weak = Rc::downgrade(&self.state);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let Some(state) = weak.upgrade() {
                paint(&state, canvas);
            }
        }));
        let state = self.state.clone();
        self.core.on(EventKind::Resize, move |_| {
            state.place_actions();
            state.invalidate_self();
        });
        let state = self.state.clone();
        self.core.on(EventKind::ThemeChanged, move |_| state.refresh_name());
        let state = self.state.clone();
        self.core.on(EventKind::MouseMove, move |event| {
            let Event::MouseMove(e) = event else { return };
            let hot = state.close_rect().is_some_and(|r| r.contains(e.position));
            if state.close_hot.replace(hot) != hot {
                state.invalidate_self();
            }
        });
        let state = self.state.clone();
        self.core.on(EventKind::MouseLeave, move |_| {
            if state.close_hot.replace(false) {
                state.invalidate_self();
            }
        });
        let state = self.state.clone();
        self.core.on(EventKind::MouseDown, move |event| {
            let Event::MouseDown(e) = event else { return };
            if e.button == MouseButton::Left && state.close_rect().is_some_and(|r| r.contains(e.position)) {
                state.close_pressed.set(true);
                // SAFETY: captura do mouse pela própria faixa até soltar o botão.
                unsafe { SetCapture(state.core().map_or(std::ptr::null_mut(), |c| c.hwnd())) };
                state.invalidate_self();
            }
        });
        let state = self.state.clone();
        self.core.on(EventKind::MouseUp, move |event| {
            let Event::MouseUp(e) = event else { return };
            if !state.close_pressed.replace(false) {
                return;
            }
            // SAFETY: libera a captura obtida no MouseDown.
            unsafe { ReleaseCapture() };
            state.invalidate_self();
            if state.close_rect().is_some_and(|r| r.contains(e.position)) {
                close_state(&state);
            }
        });
    }

    /// Adds an action button on the right before `×`. Returns the button.
    pub fn add_action(&self, text: &str, on_click: impl Fn() + 'static) -> GuiResult<Button> {
        let button = Button::builder().text(text).parent(self).build()?;
        let width = crate::menus::strip::with_measure(self.core.dpi(), |canvas| {
            canvas.measure_mnemonic(text, &self.core.font()).width
        })
        .unwrap_or(60)
            + 24;
        button.set_size(width.max(72), 28);
        button.on_click(on_click);
        self.state.actions.borrow_mut().push(button.clone());
        self.state.place_actions();
        self.core.invalidate();
        Ok(button)
    }

    /// Current severity.
    pub fn severity(&self) -> Severity {
        self.state.severity.get()
    }

    /// Changes severity (color and icon).
    pub fn set_severity(&self, severity: Severity) {
        self.state.severity.set(severity);
        self.state.refresh_name();
    }

    /// Current title.
    pub fn title(&self) -> String {
        self.state.title.borrow().clone()
    }

    /// Changes the title.
    pub fn set_title(&self, title: &str) {
        *self.state.title.borrow_mut() = title.to_string();
        self.state.refresh_name();
    }

    /// Current message.
    pub fn message(&self) -> String {
        self.state.message.borrow().clone()
    }

    /// Changes the message.
    pub fn set_message(&self, message: &str) {
        *self.state.message.borrow_mut() = message.to_string();
        self.state.refresh_name();
    }

    /// Shows/hides the `×` button.
    pub fn set_closable(&self, closable: bool) {
        self.state.closable.set(closable);
        self.state.place_actions();
        self.core.invalidate();
    }

    /// Shows the strip, relayouts the window and announces the notification to screen readers.
    pub fn open(&self) {
        let was_open = self.is_open();
        self.show();
        relayout_root(&self.core);
        if !was_open {
            // Sem o recurso (antes do Windows 10 1709) o nome acessível continua lá.
            let _ = self.announce();
        }
    }

    /// Asks screen readers to read the notification now (automatic on opening; errors interrupt ongoing speech).
    /// Returns `Unsupported` before Windows 10 1709.
    pub fn announce(&self) -> crate::core::GuiResult<()> {
        let text = native::window_text(self.core.hwnd());
        crate::accessibility::announce_with(self, &text, self.severity() == Severity::Error)
    }

    /// Shows the strip with another severity, title and message.
    pub fn open_with(&self, severity: Severity, title: &str, message: &str) {
        self.state.severity.set(severity);
        *self.state.title.borrow_mut() = title.to_string();
        *self.state.message.borrow_mut() = message.to_string();
        self.state.refresh_name();
        let was_open = self.is_open();
        self.open();
        if was_open {
            // Já aberta: o conteúdo novo também é anunciado.
            let _ = self.announce();
        }
    }

    /// Hides the strip, relayouts and notifies [`on_closed`](InfoBar::on_closed).
    pub fn close(&self) {
        close_state(&self.state);
    }

    /// Whether the strip is visible.
    pub fn is_open(&self) -> bool {
        native::style(self.core.hwnd()) & WS_VISIBLE != 0
    }

    /// Called when the strip closes via `×` or [`close`](InfoBar::close).
    pub fn on_closed(&self, f: impl Fn() + 'static) {
        self.state.on_close.borrow_mut().push(Rc::new(f));
    }

    /// Former name of [`on_closed`](Self::on_closed), retained for existing code.
    #[deprecated(since = "0.1.0", note = "use `on_closed`: events follow the naming convention in docs/EVENTS.md")]
    pub fn on_close(&self, f: impl Fn() + 'static) {
        self.on_closed(f)
    }
}

impl InfoState {
    fn invalidate_self(&self) {
        if let Some(core) = self.core() {
            core.invalidate();
        }
    }
}

fn close_state(state: &InfoState) {
    let Some(core) = state.core() else { return };
    if native::style(core.hwnd()) & WS_VISIBLE == 0 {
        return;
    }
    // SAFETY: esconde a própria janela filha viva.
    unsafe { ShowWindow(core.hwnd(), SW_HIDE) };
    relayout_root(&core);
    let handlers = state.on_close.borrow().clone();
    for handler in handlers {
        handler();
    }
}

fn relayout_root(core: &Rc<WidgetCore>) {
    let parent_of = |hwnd| {
        // SAFETY: consulta simples.
        crate::core::widget::lookup(unsafe { GetParent(hwnd) })
    };
    let mut current = parent_of(core.hwnd());
    // Refaz o layout do contêiner mais próximo que tenha um (ou da janela).
    while let Some(container) = current {
        if container.has_layout() || container.is_top_level() {
            container.relayout();
            return;
        }
        current = parent_of(container.hwnd());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_accents_are_readable_on_their_backgrounds() {
        for severity in [Severity::Info, Severity::Success, Severity::Warning, Severity::Error] {
            let light = severity.accent(false);
            let dark = severity.accent(true);
            assert!(light.luminance() < 0.45, "{severity:?} no claro: {}", light.luminance());
            assert!(dark.luminance() > 0.35, "{severity:?} no escuro: {}", dark.luminance());
        }
        assert_eq!(Severity::default(), Severity::Info);
    }
}
