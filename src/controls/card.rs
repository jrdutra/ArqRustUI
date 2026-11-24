//! Card container with rounded corners, border, optional title and subtitle, and button-like highlighting and focus
//! when clickable.

use super::common::{builder_common, ControlOptions};
use super::modern::{enabled, focused, palette, publish};
use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleRole};
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::graphics::{Canvas, Font, Pen};
use crate::input::Key;
use crate::utilities::{Rect, Thickness};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const RADIUS: i32 = 8;

/// Event and resulting state adjustment.
type Hover = (EventKind, fn(&CardState));

struct CardState {
    title: RefCell<String>,
    subtitle: RefCell<String>,
    clickable: Cell<bool>,
    hot: Cell<bool>,
    pressed: Cell<bool>,
    padding: Cell<Thickness>,
}

/// Card container (use as `parent` for other controls) with surface background, rounded corners, border, title and
/// subtitle at the top. When clickable, responds to hover, clicks and Enter/Space, and participates in tab order.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let card = Card::builder().title("Storage").subtitle("72% used").bounds(10, 10, 260, 140).parent(window).build()?;
/// let area = card.content_bounds();
/// ProgressBar::builder().value(72).bounds(area.x, area.y, area.width, 8).parent(&card).build()?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Card {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<CardState>,
}

impl Widget for Card {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Card").field("title", &self.state.title.borrow()).finish()
    }
}

/// Builder for [`Card`].
#[derive(Clone, Debug)]
pub struct CardBuilder {
    opts: ControlOptions,
    title: String,
    subtitle: String,
    clickable: bool,
    padding: Thickness,
}

impl CardBuilder {
    builder_common!();

    /// Title (bold).
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Subtitle (below the title, muted color).
    pub fn subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = subtitle.to_string();
        self
    }

    /// Makes the card clickable (highlight, focus and `on_click`).
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    /// Inner padding (default 16).
    pub fn padding(mut self, padding: Thickness) -> Self {
        self.padding = padding;
        self
    }

    /// Creates the card.
    pub fn build(self) -> GuiResult<Card> {
        let core = self.opts.create("Card", PANEL_CLASS, 0, 0, self.clickable, None)?;
        // Filhos não desenham por cima do contorno arredondado.
        crate::core::native::set_style_bits(
            core.hwnd(),
            crate::ffi::user32::GWL_STYLE,
            crate::ffi::user32::WS_CLIPCHILDREN,
            true,
        );
        let state = Rc::new(CardState {
            title: RefCell::new(self.title),
            subtitle: RefCell::new(self.subtitle),
            clickable: Cell::new(self.clickable),
            hot: Cell::new(false),
            pressed: Cell::new(false),
            padding: Cell::new(self.padding),
        });
        let card = Card { core, state };
        card.install();
        Ok(card)
    }
}

impl Card {
    /// Starts building (default size 280×160).
    pub fn builder() -> CardBuilder {
        CardBuilder {
            opts: ControlOptions::new(280, 160),
            title: String::new(),
            subtitle: String::new(),
            clickable: false,
            padding: Thickness::all(16),
        }
    }

    fn install(&self) {
        let state = Rc::downgrade(&self.state);
        let target = Rc::downgrade(&self.core);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(state), Some(core)) = (state.upgrade(), target.upgrade()) {
                paint(&core, &state, canvas);
            }
        }));
        let set = |state: &Rc<CardState>, f: fn(&CardState)| {
            let s = Rc::downgrade(state);
            move |core: &Rc<WidgetCore>| {
                if let Some(s) = s.upgrade() {
                    if s.clickable.get() {
                        f(&s);
                        core.invalidate();
                    }
                }
            }
        };
        let handlers: [Hover; 4] = [
            (EventKind::MouseEnter, |s| s.hot.set(true)),
            (EventKind::MouseLeave, |s| {
                s.hot.set(false);
                s.pressed.set(false)
            }),
            (EventKind::MouseDown, |s| s.pressed.set(true)),
            (EventKind::Focus, |_| {}),
        ];
        for (kind, f) in handlers {
            let apply = set(&self.state, f);
            let core = Rc::downgrade(&self.core);
            self.core.on(kind, move |_| {
                if let Some(core) = core.upgrade() {
                    apply(&core);
                }
            });
        }
        let (state, core) = (Rc::downgrade(&self.state), Rc::downgrade(&self.core));
        self.core.on(EventKind::Blur, move |_| {
            if let (Some(_), Some(core)) = (state.upgrade(), core.upgrade()) {
                core.invalidate();
            }
        });
        let (state, core) = (Rc::downgrade(&self.state), Rc::downgrade(&self.core));
        self.core.on(EventKind::MouseUp, move |_| {
            if let (Some(state), Some(core)) = (state.upgrade(), core.upgrade()) {
                let was = state.pressed.replace(false);
                if was && state.clickable.get() && enabled(&core) {
                    core.invalidate();
                    core.emit(&Event::ItemActivated(0));
                }
            }
        });
        let (state, core) = (Rc::downgrade(&self.state), Rc::downgrade(&self.core));
        self.core.on(EventKind::KeyDown, move |event| {
            if let (Some(state), Some(core), Event::KeyDown(key)) = (state.upgrade(), core.upgrade(), event) {
                if state.clickable.get() && (key.key == Key::ENTER || key.key == Key::SPACE) && !key.repeat {
                    core.emit(&Event::ItemActivated(0));
                }
            }
        });
        publish(
            &self.core,
            &self.state,
            |_, state| {
                let role = if state.clickable.get() { AccessibleRole::Button } else { AccessibleRole::Group };
                let mut item = AccessibleItem::new(0, role, &state.title.borrow());
                if state.clickable.get() {
                    item = item.invokable();
                }
                (item, Vec::new())
            },
            |core, state, id, action| {
                if id == 0 && action == AccessibleAction::Invoke && state.clickable.get() {
                    core.emit(&Event::ItemActivated(0));
                    return true;
                }
                false
            },
        );
    }

    /// Changes the title.
    pub fn set_title(&self, title: &str) {
        *self.state.title.borrow_mut() = title.to_string();
        self.core.invalidate();
    }

    /// Changes the subtitle.
    pub fn set_subtitle(&self, subtitle: &str) {
        *self.state.subtitle.borrow_mut() = subtitle.to_string();
        self.core.invalidate();
    }

    /// Current title.
    pub fn title(&self) -> String {
        self.state.title.borrow().clone()
    }

    /// Logical content area (below the title and inside the padding).
    pub fn content_bounds(&self) -> Rect {
        let size = self.client_size();
        let pad = self.state.padding.get();
        let mut top = pad.top;
        if !self.state.title.borrow().is_empty() {
            top += 22;
        }
        if !self.state.subtitle.borrow().is_empty() {
            top += 18;
        }
        if top > pad.top {
            top += 8;
        }
        Rect::new(pad.left, top, (size.width - pad.left - pad.right).max(0), (size.height - top - pad.bottom).max(0))
    }

    /// Called on a clickable card's click or Enter/Space.
    pub fn on_click(&self, f: impl Fn() + 'static) -> HandlerId {
        self.core.on(EventKind::ItemActivated, move |_| f())
    }
}

fn paint(core: &WidgetCore, state: &CardState, canvas: &Canvas) {
    let p = palette(core);
    let size = canvas.size();
    // O fundo em volta dos cantos é o do contêiner pai.
    // SAFETY: consulta simples do pai.
    let parent = unsafe { crate::ffi::user32::GetParent(core.hwnd()) };
    let outside =
        crate::core::widget::lookup(parent).and_then(|parent| parent.effective_background()).unwrap_or(p.background);
    canvas.clear(outside);
    let surface = if p.dark { p.background.mix(p.foreground, 0.06) } else { p.surface };
    let fill = match (state.clickable.get(), state.pressed.get(), state.hot.get()) {
        (true, true, _) => surface.mix(p.foreground, 0.08),
        (true, false, true) => surface.mix(p.foreground, 0.04),
        _ => surface,
    };
    let rect = Rect::new(0, 0, size.width - 1, size.height - 1);
    let border = if focused(core) { p.accent } else { p.border };
    canvas.round_rect(rect, RADIUS, Some(&Pen::new(border, 1)), Some(fill));
    let pad = state.padding.get();
    let mut y = pad.top;
    let title = state.title.borrow();
    if !title.is_empty() {
        let font = Font::system().bold().scaled(1.1);
        canvas.text_line(&title, Rect::new(pad.left, y, size.width - pad.left - pad.right, 22), &font, p.foreground);
        y += 22;
    }
    let subtitle = state.subtitle.borrow();
    if !subtitle.is_empty() {
        canvas.text_line(
            &subtitle,
            Rect::new(pad.left, y, size.width - pad.left - pad.right, 18),
            &Font::system(),
            p.muted,
        );
    }
}
