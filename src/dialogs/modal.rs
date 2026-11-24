//! Web-style modal: semitransparent backdrop dims owner; centered card has title, content and buttons.
//!
//! Two native windows owned by the application window:
//!
//! * **backdrop** — layered popup (`WS_EX_LAYERED` + `UpdateLayeredWindow`) with per-pixel transparency covering
//!   owner's client area and a soft card shadow; does not receive focus;
//! * **card** — opaque popup owned by backdrop, with regular Windows controls and Windows 11 DWM rounded corners.
//!
//! Owner is disabled while open; both follow moves/resizes. Entry/exit animate opacity and a slight slide, respecting
//! Windows animation settings.

use crate::controls::modern::palette;
use crate::controls::{Button, Label, Panel};
use crate::core::application as app;
use crate::core::motion::{self, Easing, Transition};
use crate::core::native;
use crate::core::widget::{lookup, Widget, WidgetCore};
use crate::core::{EventKind, GuiError, GuiResult, HandlerId, Window};
use crate::dialogs::DialogResult;
use crate::ffi::gdi32::*;
use crate::ffi::types::{FALSE, HBITMAP, HGDIOBJ, TRUE};
use crate::ffi::user32::*;
use crate::ffi::{HDC, HWND, POINT, RECT, SIZE};
use crate::graphics::{Canvas, Font, Pen};
use crate::i18n::{library_text, LibraryText};
use crate::themes::{ButtonStyle, StateColors};
use crate::utilities::{to_logical, to_pixels, Color, Point, Rect, Size};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Logical card padding, footer height and button height.
const PAD: i32 = 24;
const FOOTER: i32 = 72;
const BUTTON_H: i32 = 32;
const CLOSE: i32 = 32;
const RADIUS: i32 = 8;

/// Modal phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Created,
    Opening,
    Open,
    Closing,
    Closed,
}

/// Premultiplied black ARGB backdrop: `base` over the entire area, soft shadow around `card` in backdrop pixels, darker
/// immediately below.
pub(crate) fn backdrop_pixels(width: i32, height: i32, card: Rect, radius: i32, base: f64, scale: f64) -> Vec<u32> {
    let (w, h) = (width.max(1) as usize, height.max(1) as usize);
    let base_alpha = (base.clamp(0.0, 1.0) * 255.0).round() as u32;
    let mut pixels = vec![base_alpha << 24; w * h];
    let blur = 28.0 * scale;
    let offset = 8.0 * scale;
    let strength = 0.34;
    let r = radius as f64;
    // Só a vizinhança do cartão recebe sombra.
    let reach = (blur * 1.6).ceil() as i32;
    let (x0, x1) = ((card.x - reach).max(0), (card.x + card.width + reach).min(width));
    let (y0, y1) =
        ((card.y - reach + offset as i32).max(0), (card.y + card.height + reach + offset as i32).min(height));
    let (cx, cy) = (card.x as f64 + card.width as f64 / 2.0, card.y as f64 + card.height as f64 / 2.0 + offset);
    let (hw, hh) = (card.width as f64 / 2.0 - r, card.height as f64 / 2.0 - r);
    for y in y0..y1 {
        for x in x0..x1 {
            // Distância até o retângulo arredondado (deslocado para baixo).
            let (dx, dy) = (((x as f64 + 0.5 - cx).abs() - hw).max(0.0), ((y as f64 + 0.5 - cy).abs() - hh).max(0.0));
            let d = ((dx * dx + dy * dy).sqrt() - r).max(0.0);
            let shadow = strength * (-(d / (blur * 0.55)).powi(2)).exp();
            let a = base + shadow * (1.0 - base);
            pixels[y as usize * w + x as usize] = ((a.clamp(0.0, 1.0) * 255.0).round() as u32) << 24;
        }
    }
    pixels
}

/// Composes premultiplied black dimming in `shade` over an opaque BGRX `photo`, leaving opaque output in `shade`.
pub(crate) fn darken_onto(shade: &mut [u32], photo: &[u32]) {
    for (s, p) in shade.iter_mut().zip(photo) {
        let a = (*s >> 24) & 0xFF;
        let keep = 255 - a;
        let channel = |shift: u32| ((((p >> shift) & 0xFF) * keep + 127) / 255) << shift;
        *s = 0xFF00_0000 | channel(16) | channel(8) | channel(0);
    }
}

/// Logical card height for content and footer presence.
pub(crate) fn card_height(header: i32, content: i32, footer: bool) -> i32 {
    header + content + if footer { PAD + FOOTER } else { PAD }
}

/// In-memory DIB section for the layered backdrop.
struct Layer {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u32,
    size: (i32, i32),
}

impl Layer {
    fn new(width: i32, height: i32) -> Option<Layer> {
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width.max(1),
            biHeight: -height.max(1),
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        // SAFETY: DC e seção DIB criados aqui e liberados no `Drop`; `bits` aponta para
        // largura × altura × 4 bytes enquanto o bitmap existir.
        unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            if dc.is_null() {
                return None;
            }
            let bitmap = CreateDIBSection(dc, &header, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
            if bitmap.is_null() || bits.is_null() {
                DeleteDC(dc);
                return None;
            }
            let previous = SelectObject(dc, bitmap as HGDIOBJ);
            Some(Layer { dc, bitmap, previous, bits: bits as *mut u32, size: (width.max(1), height.max(1)) })
        }
    }

    fn fill(&self, pixels: &[u32]) {
        let count = (self.size.0 * self.size.1) as usize;
        // SAFETY: `bits` tem exatamente `count` pixels (ver `new`).
        unsafe { std::ptr::copy_nonoverlapping(pixels.as_ptr(), self.bits, count.min(pixels.len())) };
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        // SAFETY: objetos criados em `new`, liberados uma única vez.
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap as HGDIOBJ);
            DeleteDC(self.dc);
        }
    }
}

type ResultFn = Rc<dyn Fn(DialogResult)>;
type RequestFn = Rc<dyn Fn(DialogResult) -> bool>;

struct ModalState {
    owner: Weak<WidgetCore>,
    backdrop: Window,
    card: Window,
    content: Panel,
    footer: Option<Panel>,
    close_button: Option<Panel>,
    buttons: RefCell<Vec<(Button, DialogResult, bool)>>,
    title: RefCell<String>,
    size: Cell<Size>,
    header: i32,
    dim: f64,
    close_on_backdrop: bool,
    result: Cell<DialogResult>,
    phase: Cell<Phase>,
    progress: Cell<f64>,
    shake: Cell<f64>,
    layer: RefCell<Option<Layer>>,
    transition: RefCell<Option<Transition>>,
    request: RefCell<Option<RequestFn>>,
    closed: RefCell<Vec<ResultFn>>,
    focus: Cell<HWND>,
    owner_handlers: RefCell<Vec<HandlerId>>,
    close_hot: Cell<bool>,
    rounded: Cell<bool>,
    surface: Color,
}

/// Web-style modal: dims owner and displays a centered card with title, [`content`](Self::content) and footer buttons.
///
/// * Esc and ✕ close with [`DialogResult::Cancel`]; Enter invokes the primary button.
/// * Backdrop clicks close when [`close_on_backdrop`](ModalBuilder::close_on_backdrop) is enabled; otherwise the card
///   shakes for attention.
/// * Focus stays inside the card (Tab cycles controls), returning to the previously focused control on closing.
/// * [`show`](Self::show) blocks until closing and returns the result, like `await`; [`show_then`](Self::show_then)
///   returns immediately and invokes a closing callback.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let modal = Modal::builder()
///     .title("Delete file?")
///     .message("The file \"report.docx\" will be permanently deleted.")
///     .button("Cancel", DialogResult::Cancel)
///     .primary_button("Delete", DialogResult::Ok)
///     .build(window)?;
/// if modal.show()? == DialogResult::Ok {
///     // delete
/// }
/// // Shortcuts:
/// if Modal::confirm(window, "Exit?", "Unsaved changes will be lost.")? { /* ... */ }
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Modal {
    state: Rc<ModalState>,
}

impl std::fmt::Debug for Modal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Modal")
            .field("title", &self.state.title.borrow())
            .field("phase", &self.state.phase.get())
            .finish()
    }
}

/// Builder for [`Modal`].
#[derive(Clone, Debug)]
pub struct ModalBuilder {
    title: String,
    message: Option<String>,
    width: i32,
    content_height: Option<i32>,
    buttons: Vec<(String, DialogResult, bool)>,
    close_on_escape: bool,
    close_on_backdrop: bool,
    close_button: bool,
    dim: Option<f64>,
}

impl Default for ModalBuilder {
    fn default() -> Self {
        ModalBuilder {
            title: String::new(),
            message: None,
            width: 440,
            content_height: None,
            buttons: Vec::new(),
            close_on_escape: true,
            close_on_backdrop: false,
            close_button: true,
            dim: None,
        }
    }
}

impl ModalBuilder {
    /// Card title, also the screen reader window name.
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Message text: content label with text-adjusted height.
    pub fn message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }

    /// Logical card width (default 440, minimum 280).
    pub fn width(mut self, width: i32) -> Self {
        self.width = width.max(280);
        self
    }

    /// Logical content height (default: message height or 120).
    pub fn content_height(mut self, height: i32) -> Self {
        self.content_height = Some(height.max(0));
        self
    }

    /// Footer button closing with `result`. Buttons appear right-aligned in insertion order.
    pub fn button(mut self, text: &str, result: DialogResult) -> Self {
        self.buttons.push((text.to_string(), result, false));
        self
    }

    /// Primary button (accent color; also invoked by Enter).
    pub fn primary_button(mut self, text: &str, result: DialogResult) -> Self {
        self.buttons.push((text.to_string(), result, true));
        self
    }

    /// Esc closes with Cancel (default: yes).
    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.close_on_escape = close;
        self
    }

    /// Backdrop click closes with Cancel (default: no; card shakes).
    pub fn close_on_backdrop(mut self, close: bool) -> Self {
        self.close_on_backdrop = close;
        self
    }

    /// Shows card-corner ✕ (default: yes).
    pub fn close_button(mut self, show: bool) -> Self {
        self.close_button = show;
        self
    }

    /// Dimming opacity 0.0 to 0.9 (default: 0.40 light, 0.55 dark).
    pub fn dim(mut self, opacity: f64) -> Self {
        self.dim = Some(opacity.clamp(0.0, 0.9));
        self
    }

    /// Builds over `owner`'s window, initially hidden; call `show`/`show_then`.
    pub fn build(self, owner: &impl Widget) -> GuiResult<Modal> {
        app::require()?;
        let root_hwnd = native::root_of(owner.core().require_alive()?);
        let root = lookup(root_hwnd).ok_or(GuiError::WidgetDestroyed)?;
        let owner_window = Window { core: root.clone() };
        let theme = root.theme.borrow().clone();

        let backdrop = Window::builder().borderless().tool_window().owner(&owner_window).build()?;
        native::set_style_bits(backdrop.core().hwnd(), GWL_EXSTYLE, WS_EX_LAYERED | WS_EX_NOACTIVATE, true);

        // Altura do conteúdo: a da mensagem quebrada na largura do cartão.
        let inner = self.width - 2 * PAD;
        let title_font = title_font();
        let mut title_height = 0;
        let mut message_height = 0;
        let _ = crate::graphics::Bitmap::render(1, 1, |canvas| {
            title_height = canvas.measure_wrapped(&self.title, &title_font, inner - CLOSE).height.max(24);
            if let Some(message) = &self.message {
                message_height = canvas.measure_wrapped(message, &Font::system(), inner).height;
            }
        });
        let header = PAD + title_height + 12;
        let content_height =
            self.content_height.unwrap_or(if self.message.is_some() { message_height + 4 } else { 120 });
        let has_footer = !self.buttons.is_empty();
        let height = card_height(header, content_height, has_footer);
        let card = Window::builder()
            .title(&self.title)
            .size(self.width, height)
            .borderless()
            .tool_window()
            .owner(&backdrop)
            .close_on_escape(self.close_on_escape)
            .build()?;
        if let Some(theme) = &theme {
            theme.apply(&card);
        }
        let colors = palette(card.core());
        let surface = surface_of(&colors);
        card.core().background.set(Some(surface));
        let content =
            Panel::builder().bounds(PAD, header, inner, content_height).background(surface).parent(&card).build()?;
        if let Some(message) = &self.message {
            let label = Label::builder().text(message).bounds(0, 0, inner, content_height).parent(&content).build()?;
            label.set_background(surface);
        }
        let footer = if has_footer {
            let footer_color = footer_of(&colors);
            let footer = Panel::builder()
                .bounds(0, height - FOOTER, self.width, FOOTER)
                .background(footer_color)
                .parent(&card)
                .build()?;
            let line = colors.border.mix(footer_color, 0.4);
            footer.on_paint(move |canvas| {
                let width = canvas.size().width;
                canvas.line(Point::new(0, 0), Point::new(width, 0), &Pen::new(line, 1));
            });
            Some(footer)
        } else {
            None
        };
        let close_button = if self.close_button {
            Some(
                Panel::builder()
                    .bounds(self.width - PAD / 2 - CLOSE, PAD / 2, CLOSE, CLOSE)
                    .background(surface)
                    .parent(&card)
                    .build()?,
            )
        } else {
            None
        };
        let dim = self.dim.unwrap_or(if colors.dark { 0.55 } else { 0.40 });
        let state = Rc::new(ModalState {
            owner: Rc::downgrade(&root),
            backdrop,
            card,
            content,
            footer,
            close_button,
            buttons: RefCell::new(Vec::new()),
            title: RefCell::new(self.title.clone()),
            size: Cell::new(Size::new(self.width, height)),
            header,
            dim,
            close_on_backdrop: self.close_on_backdrop,
            result: Cell::new(DialogResult::Cancel),
            phase: Cell::new(Phase::Created),
            progress: Cell::new(0.0),
            shake: Cell::new(0.0),
            layer: RefCell::new(None),
            transition: RefCell::new(None),
            request: RefCell::new(None),
            closed: RefCell::new(Vec::new()),
            focus: Cell::new(std::ptr::null_mut()),
            owner_handlers: RefCell::new(Vec::new()),
            close_hot: Cell::new(false),
            rounded: Cell::new(false),
            surface,
        });
        let modal = Modal { state };
        for (text, result, primary) in self.buttons {
            modal.add_button(&text, result, primary)?;
        }
        modal.install(&title_font);
        Ok(modal)
    }
}

fn title_font() -> Font {
    Font::system().bold().scaled(1.35)
}

fn surface_of(p: &crate::controls::modern::Palette) -> Color {
    if p.dark {
        p.background.mix(Color::WHITE, 0.06)
    } else {
        Color::WHITE
    }
}

fn footer_of(p: &crate::controls::modern::Palette) -> Color {
    if p.dark {
        p.background.mix(Color::BLACK, 0.12)
    } else {
        Color::rgb(243, 243, 243)
    }
}

/// Secondary footer button style (neutral, themed).
fn neutral_style(p: &crate::controls::modern::Palette) -> ButtonStyle {
    let base = surface_of(p);
    let state = |background: Color, foreground: Color| StateColors { background, foreground, border: p.border };
    ButtonStyle {
        normal: state(base, p.foreground),
        hover: state(base.mix(p.foreground, 0.06), p.foreground),
        pressed: state(base.mix(p.foreground, 0.12), p.foreground),
        disabled: state(base, p.muted),
        focus_ring: p.accent,
        corner_radius: 4,
    }
}

impl Modal {
    /// Starts building.
    pub fn builder() -> ModalBuilder {
        ModalBuilder::default()
    }

    /// Notification with OK; blocks until closed.
    pub fn alert(owner: &impl Widget, title: &str, message: &str) -> GuiResult<()> {
        Modal::builder()
            .title(title)
            .message(message)
            .primary_button(&library_text(LibraryText::Ok), DialogResult::Ok)
            .build(owner)?
            .show()?;
        Ok(())
    }

    /// OK/Cancel confirmation; `true` if confirmed. Blocks until closed.
    pub fn confirm(owner: &impl Widget, title: &str, message: &str) -> GuiResult<bool> {
        let result = Modal::builder()
            .title(title)
            .message(message)
            .button(&library_text(LibraryText::Cancel), DialogResult::Cancel)
            .primary_button(&library_text(LibraryText::Ok), DialogResult::Ok)
            .build(owner)?
            .show()?;
        Ok(result == DialogResult::Ok)
    }

    fn weak(&self) -> Weak<ModalState> {
        Rc::downgrade(&self.state)
    }

    fn upgrade(weak: &Weak<ModalState>) -> Option<Modal> {
        weak.upgrade().map(|state| Modal { state })
    }

    fn add_button(&self, text: &str, result: DialogResult, primary: bool) -> GuiResult<()> {
        let Some(footer) = &self.state.footer else { return Ok(()) };
        let colors = palette(self.state.card.core());
        let style =
            if primary { ButtonStyle::from_accent(colors.accent, colors.on_accent) } else { neutral_style(&colors) };
        let button = Button::builder().text(text).default(primary).style(style).parent(footer).build()?;
        let weak = self.weak();
        button.on_click(move || {
            if let Some(modal) = Modal::upgrade(&weak) {
                modal.close_with(result);
            }
        });
        self.state.buttons.borrow_mut().push((button, result, primary));
        self.layout_buttons();
        Ok(())
    }

    fn layout_buttons(&self) {
        let buttons = self.state.buttons.borrow();
        let font = Font::system();
        let mut widths = Vec::new();
        let _ = crate::graphics::Bitmap::render(1, 1, |canvas| {
            widths = buttons.iter().map(|(b, ..)| (canvas.measure_text(&b.text(), &font).width + 40).max(96)).collect();
        });
        let width = self.state.size.get().width;
        let mut x = width - PAD;
        for ((button, ..), w) in buttons.iter().zip(widths).rev() {
            x -= w;
            button.set_bounds(Rect::new(x, (FOOTER - BUTTON_H) / 2, w, BUTTON_H));
            x -= 8;
        }
    }

    fn install(&self, title_font: &Font) {
        let state = &self.state;
        // Cartão: título, borda e o botão ✕.
        let weak = self.weak();
        let font = title_font.clone();
        *state.card.core().paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            let Some(modal) = Modal::upgrade(&weak) else { return };
            let s = &modal.state;
            let p = palette(s.card.core());
            let size = canvas.size();
            canvas.clear(s.surface);
            let close = if s.close_button.is_some() { CLOSE + 8 } else { 0 };
            let title = s.title.borrow().clone();
            canvas.text_in_rect(
                &title,
                Rect::new(PAD, PAD, size.width - 2 * PAD - close, s.header - PAD),
                &font,
                p.foreground,
                crate::graphics::TextAlign::Left,
            );
            canvas.rect(
                Rect::new(0, 0, size.width, size.height),
                Some(&Pen::new(p.border.mix(s.surface, 0.35), 1)),
                None,
            );
        }));
        if let Some(close) = &state.close_button {
            let weak = self.weak();
            close.on_paint(move |canvas| {
                let Some(modal) = Modal::upgrade(&weak) else { return };
                let p = palette(modal.state.card.core());
                let size = canvas.size();
                let surface = modal.state.surface;
                canvas.clear(surface);
                if modal.state.close_hot.get() {
                    canvas.round_rect(
                        Rect::new(0, 0, size.width, size.height),
                        4,
                        None,
                        Some(surface.mix(p.foreground, 0.08)),
                    );
                }
                let (cx, cy) = (size.width / 2, size.height / 2);
                let pen = Pen::new(p.muted, 1);
                canvas.line(Point::new(cx - 5, cy - 5), Point::new(cx + 6, cy + 6), &pen);
                canvas.line(Point::new(cx + 5, cy - 5), Point::new(cx - 6, cy + 6), &pen);
            });
            let _ = crate::accessibility::set_custom_role(
                close,
                crate::accessibility::AccessibleRole::Button,
                &library_text(LibraryText::Close),
            );
            for (kind, hot) in [(EventKind::MouseEnter, true), (EventKind::MouseLeave, false)] {
                let weak = self.weak();
                close.core().on(kind, move |_| {
                    if let Some(modal) = Modal::upgrade(&weak) {
                        modal.state.close_hot.set(hot);
                        if let Some(close) = &modal.state.close_button {
                            close.core().invalidate();
                        }
                    }
                });
            }
            let weak = self.weak();
            close.core().on(EventKind::MouseUp, move |_| {
                if let Some(modal) = Modal::upgrade(&weak) {
                    modal.close_with(DialogResult::Cancel);
                }
            });
        }
        // Esc (WM_CLOSE do cartão) e Enter (IDOK → botão principal).
        let weak = self.weak();
        state.card.on_close_request(move || {
            if let Some(modal) = Modal::upgrade(&weak) {
                modal.close_with(DialogResult::Cancel);
            }
            false
        });
        let weak = self.weak();
        state.card.core().add_message_filter(move |raw| {
            if raw.message == WM_COMMAND && raw.lparam == 0 && (raw.wparam & 0xFFFF) == 1 {
                let modal = Modal::upgrade(&weak)?;
                let primary = modal.state.buttons.borrow().iter().find(|b| b.2).map(|b| b.0.clone());
                if let Some(button) = primary {
                    button.click();
                }
                return Some(0);
            }
            None
        });
        // Clique no fundo.
        let weak = self.weak();
        state.backdrop.core().on(EventKind::MouseUp, move |_| {
            if let Some(modal) = Modal::upgrade(&weak) {
                if modal.state.close_on_backdrop {
                    modal.close_with(DialogResult::Cancel);
                } else {
                    modal.shake();
                }
            }
        });
        // O fundo nunca fica com o foco: devolve ao cartão.
        let weak = self.weak();
        state.backdrop.core().add_message_filter(move |raw| {
            if raw.message == WM_MOUSEACTIVATE {
                return Some(MA_NOACTIVATE);
            }
            if raw.message == WM_ACTIVATE && (raw.wparam & 0xFFFF) != 0 {
                if let Some(modal) = Modal::upgrade(&weak) {
                    // SAFETY: ativa o cartão vivo desta thread.
                    unsafe { SetActiveWindow(modal.state.card.core().hwnd()) };
                }
            }
            None
        });
    }

    /// Content panel for parenting modal controls.
    pub fn content(&self) -> Panel {
        self.state.content.clone()
    }

    /// Card window for theme/position access or controls outside content.
    pub fn card(&self) -> &Window {
        &self.state.card
    }

    /// Dimmed backdrop window.
    pub fn backdrop(&self) -> &Window {
        &self.state.backdrop
    }

    /// Card-corner ✕ button, if present.
    pub fn close_button(&self) -> Option<Panel> {
        self.state.close_button.clone()
    }

    /// Footer buttons in insertion order.
    pub fn buttons(&self) -> Vec<Button> {
        self.state.buttons.borrow().iter().map(|b| b.0.clone()).collect()
    }

    /// Changes the title.
    pub fn set_title(&self, title: &str) {
        *self.state.title.borrow_mut() = title.to_string();
        self.state.card.set_title(title);
        self.state.card.core().invalidate();
    }

    /// `true` while open, including entry/exit animations.
    pub fn is_open(&self) -> bool {
        matches!(self.state.phase.get(), Phase::Opening | Phase::Open | Phase::Closing)
    }

    /// Closing result, or Cancel if not yet closed.
    pub fn result(&self) -> DialogResult {
        self.state.result.get()
    }

    /// Called before closing with requested result; return `false` to stay open, e.g. on failed form validation.
    pub fn on_close_request(&self, f: impl Fn(DialogResult) -> bool + 'static) {
        *self.state.request.borrow_mut() = Some(Rc::new(f));
    }

    /// Called after closing with result.
    pub fn on_closed(&self, f: impl Fn(DialogResult) + 'static) {
        self.state.closed.borrow_mut().push(Rc::new(f));
    }

    /// Shows and blocks until closed, continuing message processing; returns result.
    pub fn show(&self) -> GuiResult<DialogResult> {
        let app_state = app::require()?;
        self.open()?;
        let state = self.state.clone();
        crate::core::event_loop::run_loop(&app_state, move || state.phase.get() == Phase::Closed)?;
        Ok(self.state.result.get())
    }

    /// Shows without blocking; `on_closed` receives closing result.
    pub fn show_then(&self, on_closed: impl Fn(DialogResult) + 'static) -> GuiResult<()> {
        self.on_closed(on_closed);
        self.open()
    }

    fn owner(&self) -> Option<Rc<WidgetCore>> {
        self.state.owner.upgrade().filter(|o| o.is_alive())
    }

    /// Owner client area in screen pixels.
    fn owner_area(&self) -> Option<RECT> {
        let owner = self.owner()?;
        let mut client = native::client_rect(owner.hwnd());
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: converte coordenadas da janela dona viva.
        unsafe { ClientToScreen(owner.hwnd(), &mut origin) };
        client.left += origin.x;
        client.right += origin.x;
        client.top += origin.y;
        client.bottom += origin.y;
        Some(client)
    }

    /// Centered card screen pixel position including current slide/shake.
    fn card_origin(&self, area: &RECT) -> (i32, i32, i32, i32) {
        let dpi = native::dpi_for_window(self.state.card.core().hwnd());
        let size = self.state.size.get();
        let (w, h) = (to_pixels(size.width, dpi), to_pixels(size.height, dpi));
        let (aw, ah) = (area.right - area.left, area.bottom - area.top);
        let t = self.state.progress.get();
        let slide = to_pixels(((1.0 - t) * 14.0).round() as i32, dpi);
        let shake = to_pixels(self.state.shake.get().round() as i32, dpi);
        let x = area.left + (aw - w) / 2 + shake;
        // Um pouco acima do centro, como os diálogos do sistema; nunca acima da área.
        let y = area.top + ((ah - h) * 2 / 5).max(0) + slide;
        (x, y, w, h)
    }

    /// Repositions backdrop/card over owner and redraws backdrop if resized.
    fn reposition(&self, rerender: bool) {
        let Some(area) = self.owner_area() else { return };
        let (aw, ah) = ((area.right - area.left).max(1), (area.bottom - area.top).max(1));
        let (x, y, w, h) = self.card_origin(&area);
        // SAFETY: janelas vivas desta thread.
        unsafe {
            SetWindowPos(
                self.state.card.core().hwnd(),
                std::ptr::null_mut(),
                x,
                y,
                w,
                h,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        let size_changed = self.state.layer.borrow().as_ref().map_or(true, |l| l.size != (aw, ah));
        if rerender || size_changed {
            let dpi = native::dpi_for_window(self.state.backdrop.core().hwnd());
            let card_rect = Rect::new(
                x - area.left,
                y - area.top - to_pixels(((1.0 - self.state.progress.get()) * 14.0) as i32, dpi),
                w,
                h,
            );
            if let Some(layer) = Layer::new(aw, ah) {
                let rounded = if self.has_round_corners() { to_pixels(RADIUS, dpi) } else { 0 };
                let mut pixels = backdrop_pixels(aw, ah, card_rect, rounded, self.state.dim, dpi as f64 / 96.0);
                if crate::input::drag::running_on_wine() {
                    // Sem compositor (Wine), a transparência por pixel não é composta: o fundo
                    // vira uma foto escurecida e opaca da janela dona.
                    if let Some(owner) = self.owner() {
                        let backdrop_visible = self.state.backdrop.core().is_alive()
                            // SAFETY: consulta simples.
                            && unsafe { IsWindowVisible(self.state.backdrop.core().hwnd()) } != 0;
                        // SAFETY: copia da tela (antes de o fundo aparecer) ou pede à janela dona
                        // viva que se desenhe no DC da camada.
                        unsafe {
                            if backdrop_visible {
                                PrintWindow(owner.hwnd(), layer.dc, 1);
                            } else {
                                let screen = GetDC(std::ptr::null_mut());
                                BitBlt(layer.dc, 0, 0, aw, ah, screen, area.left, area.top, SRCCOPY);
                                ReleaseDC(std::ptr::null_mut(), screen);
                            }
                        }
                        // SAFETY: `bits` tem aw × ah pixels (ver `Layer::new`).
                        let photo = unsafe { std::slice::from_raw_parts(layer.bits, pixels.len()) };
                        darken_onto(&mut pixels, photo);
                    }
                }
                layer.fill(&pixels);
                *self.state.layer.borrow_mut() = Some(layer);
            }
        }
        self.update_backdrop(area.left, area.top);
    }

    /// Updates backdrop layer with animation opacity.
    fn update_backdrop(&self, left: i32, top: i32) {
        let layer = self.state.layer.borrow();
        let Some(layer) = layer.as_ref() else { return };
        let alpha = (self.state.progress.get().clamp(0.0, 1.0) * 255.0).round() as u8;
        let blend = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: AC_SRC_ALPHA };
        let position = POINT { x: left, y: top };
        let size = SIZE { cx: layer.size.0, cy: layer.size.1 };
        let source = POINT { x: 0, y: 0 };
        // SAFETY: camada válida (DC com a seção DIB selecionada) e janela em camadas viva.
        unsafe {
            UpdateLayeredWindow(
                self.state.backdrop.core().hwnd(),
                std::ptr::null_mut(),
                &position,
                &size,
                layer.dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            );
        }
    }

    fn has_round_corners(&self) -> bool {
        self.state.rounded.get()
    }

    fn set_card_opacity(&self, opacity: f64) {
        let hwnd = self.state.card.core().hwnd();
        let layered = native::ex_style(hwnd) & WS_EX_LAYERED != 0;
        if opacity >= 1.0 {
            if layered {
                // Opaco: sem camada, para o texto manter o ClearType.
                native::set_style_bits(hwnd, GWL_EXSTYLE, WS_EX_LAYERED, false);
                // SAFETY: redesenha a janela viva desta thread.
                unsafe {
                    RedrawWindow(
                        hwnd,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN,
                    )
                };
            }
        } else {
            self.state.card.set_opacity(opacity as f32);
        }
    }

    fn frame(&self, t: f64) {
        self.state.progress.set(t);
        if let Some(area) = self.owner_area() {
            let (x, y, w, h) = self.card_origin(&area);
            // SAFETY: janela viva desta thread.
            unsafe {
                SetWindowPos(
                    self.state.card.core().hwnd(),
                    std::ptr::null_mut(),
                    x,
                    y,
                    w,
                    h,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            self.update_backdrop(area.left, area.top);
        }
        self.set_card_opacity(t);
    }

    fn open(&self) -> GuiResult<()> {
        if self.state.phase.get() != Phase::Created {
            return Err(GuiError::InvalidArgument("the modal has already been shown".into()));
        }
        let owner = self.owner().ok_or(GuiError::WidgetDestroyed)?;
        self.state.phase.set(Phase::Opening);
        // SAFETY: consultas e alterações de janelas vivas desta thread.
        unsafe {
            self.state.focus.set(GetFocus());
            EnableWindow(owner.hwnd(), FALSE);
        }
        // Cantos arredondados do Windows 11 (sem efeito antes dele).
        let preference: u32 = 2; // DWMWCP_ROUND
                                 // SAFETY: atributo DWM de uma janela viva; falha sem efeito onde não existe.
        let rounded = unsafe {
            crate::ffi::dwmapi::DwmSetWindowAttribute(
                self.state.card.core().hwnd(),
                crate::ffi::dwmapi::DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const u32 as *const std::ffi::c_void,
                4,
            )
        } >= 0;
        self.state.rounded.set(rounded);
        // Acompanha a janela dona.
        for kind in [EventKind::Move, EventKind::Resize] {
            let weak = self.weak();
            let id = owner.on(kind, move |_| {
                if let Some(modal) = Modal::upgrade(&weak) {
                    if modal.is_open() {
                        modal.reposition(false);
                    }
                }
            });
            self.state.owner_handlers.borrow_mut().push(id);
        }
        let weak = self.weak();
        let id = owner.on(EventKind::Destroyed, move |_| {
            if let Some(modal) = Modal::upgrade(&weak) {
                modal.finish(true);
            }
        });
        self.state.owner_handlers.borrow_mut().push(id);
        let animated = motion::animations_enabled();
        self.state.progress.set(if animated { 0.0 } else { 1.0 });
        self.set_card_opacity(self.state.progress.get());
        self.reposition(true);
        let (backdrop, card) = (self.state.backdrop.core().hwnd(), self.state.card.core().hwnd());
        // SAFETY: janelas vivas desta thread; o fundo aparece sem ativar, o cartão com foco.
        unsafe {
            ShowWindow(backdrop, SW_SHOWNOACTIVATE);
            ShowWindow(card, SW_SHOW);
            SetForegroundWindow(card);
            SetActiveWindow(card);
            // Foco: o primeiro controle do conteúdo; sem controles, o botão principal.
            let first = GetNextDlgTabItem(card, std::ptr::null_mut(), FALSE);
            let in_content = !first.is_null() && IsChild(self.state.content.core().hwnd(), first) != 0;
            let primary = self.state.buttons.borrow().iter().find(|b| b.2).map(|b| b.0.core().hwnd());
            let fallback = if first.is_null() || first == card { card } else { first };
            let target = if in_content { first } else { primary.unwrap_or(fallback) };
            SetFocus(target);
        }
        let text = self.state.title.borrow().clone();
        let _ = crate::accessibility::announce(&self.state.card, &text);
        if !animated {
            self.state.phase.set(Phase::Open);
            self.set_card_opacity(1.0);
            return Ok(());
        }
        let weak = self.weak();
        let transition = motion::animate(Duration::from_millis(200), Easing::EaseOut, move |t| {
            if let Some(modal) = Modal::upgrade(&weak) {
                modal.frame(t);
            }
        })?;
        let weak = self.weak();
        transition.on_finished(move || {
            if let Some(modal) = Modal::upgrade(&weak) {
                if modal.state.phase.get() == Phase::Opening {
                    modal.state.phase.set(Phase::Open);
                }
            }
        });
        *self.state.transition.borrow_mut() = Some(transition);
        Ok(())
    }

    /// Closes with given result and exit animation. Ignored unless open or if
    /// [`on_close_request`](Self::on_close_request) rejects.
    pub fn close_with(&self, result: DialogResult) {
        if !matches!(self.state.phase.get(), Phase::Opening | Phase::Open) {
            return;
        }
        let request = self.state.request.borrow().clone();
        if let Some(request) = request {
            if !request(result) {
                return;
            }
        }
        self.state.result.set(result);
        self.state.phase.set(Phase::Closing);
        if let Some(old) = self.state.transition.borrow_mut().take() {
            old.cancel();
        }
        // A janela dona volta a aceitar entrada antes do cartão sumir: a ativação vai para ela.
        if let Some(owner) = self.owner() {
            // SAFETY: janela viva desta thread.
            unsafe { EnableWindow(owner.hwnd(), TRUE) };
        }
        let from = self.state.progress.get();
        if !motion::animations_enabled() || from <= 0.0 {
            self.finish(false);
            return;
        }
        let weak = self.weak();
        let transition = motion::animate(Duration::from_millis(140), Easing::EaseIn, move |t| {
            if let Some(modal) = Modal::upgrade(&weak) {
                modal.frame(from * (1.0 - t));
            }
        });
        match transition {
            Ok(transition) => {
                let weak = self.weak();
                transition.on_finished(move || {
                    if let Some(modal) = Modal::upgrade(&weak) {
                        modal.finish(false);
                    }
                });
                *self.state.transition.borrow_mut() = Some(transition);
            }
            Err(_) => self.finish(false),
        }
    }

    /// Destroys windows, restores focus and notifies listeners.
    fn finish(&self, owner_gone: bool) {
        if self.state.phase.replace(Phase::Closed) == Phase::Closed {
            return;
        }
        if let Some(old) = self.state.transition.borrow_mut().take() {
            old.cancel();
        }
        if let Some(owner) = self.owner() {
            // SAFETY: janela viva desta thread.
            unsafe { EnableWindow(owner.hwnd(), TRUE) };
            let handlers: Vec<HandlerId> = self.state.owner_handlers.borrow_mut().drain(..).collect();
            for id in handlers {
                owner.remove_handler(id);
            }
            if !owner_gone {
                // SAFETY: ativa a janela dona viva antes de o cartão sumir.
                unsafe { SetActiveWindow(owner.hwnd()) };
            }
        }
        self.state.card.on_close_request(|| true);
        self.state.card.destroy();
        self.state.backdrop.destroy();
        self.state.layer.borrow_mut().take();
        let focus = self.state.focus.get();
        // SAFETY: devolve o foco a um controle ainda vivo desta thread.
        unsafe {
            if !owner_gone && !focus.is_null() && IsWindow(focus) != 0 {
                // Se a janela dona não estiver ativa agora (o usuário trocou de programa),
                // o foco volta ao controle quando ela for reativada.
                if let Some(owner) = self.owner() {
                    if native::root_of(focus) == owner.hwnd() {
                        owner.remember_focus(focus);
                    }
                }
                SetFocus(focus);
            }
        }
        let result = self.state.result.get();
        let callbacks = self.state.closed.borrow().clone();
        for callback in callbacks {
            callback(result);
        }
    }

    /// Shakes card for backdrop click with `close_on_backdrop` disabled.
    pub fn shake(&self) {
        if !self.is_open() || !motion::animations_enabled() {
            // SAFETY: som padrão do sistema.
            unsafe { MessageBeep(0) };
            return;
        }
        let weak = self.weak();
        let _ = motion::animate(Duration::from_millis(320), Easing::Linear, move |t| {
            if let Some(modal) = Modal::upgrade(&weak) {
                let amplitude = 9.0 * (1.0 - t);
                modal.state.shake.set(amplitude * (t * std::f64::consts::PI * 6.0).sin());
                if let Some(area) = modal.owner_area() {
                    let (x, y, w, h) = modal.card_origin(&area);
                    // SAFETY: janela viva desta thread.
                    unsafe {
                        SetWindowPos(
                            modal.state.card.core().hwnd(),
                            std::ptr::null_mut(),
                            x,
                            y,
                            w,
                            h,
                            SWP_NOZORDER | SWP_NOACTIVATE,
                        )
                    };
                }
            }
        });
    }

    /// Logical card rectangle relative to owner client area.
    pub fn card_bounds(&self) -> Rect {
        let Some(area) = self.owner_area() else { return Rect::default() };
        let rect = native::window_rect(self.state.card.core().hwnd());
        let dpi = native::dpi_for_window(self.state.card.core().hwnd());
        Rect::new(
            to_logical(rect.left - area.left, dpi),
            to_logical(rect.top - area.top, dpi),
            to_logical(rect.right - rect.left, dpi),
            to_logical(rect.bottom - rect.top, dpi),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backdrop_shadow_and_heights() {
        let card = Rect::new(100, 80, 200, 120);
        let pixels = backdrop_pixels(400, 300, card, 8, 0.4, 1.0);
        let alpha = |x: usize, y: usize| pixels[y * 400 + x] >> 24;
        assert_eq!(alpha(2, 2), 102, "longe do cartão: só o escurecimento");
        assert_eq!(pixels[2] & 0x00FF_FFFF, 0, "preto pré-multiplicado");
        let below = alpha(200, 80 + 120 + 6);
        let above = alpha(200, 80 - 6);
        assert!(below > above && above > 102, "sombra mais forte embaixo: {below} > {above}");
        assert!(alpha(200, 299) < below);
        assert_eq!(card_height(60, 100, true), 60 + 100 + PAD + FOOTER);
        assert_eq!(card_height(60, 100, false), 60 + 100 + PAD);
        let mut shade = vec![128u32 << 24, 0];
        darken_onto(&mut shade, &[0x00FF_8040, 0x0010_2030]);
        assert_eq!(shade, vec![0xFF7F_4020, 0xFF10_2030], "meio escurecido; sem sombra, a foto intacta e opaca");
    }
}
