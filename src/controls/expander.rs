//! Expander: clickable arrow header showing/hiding a content panel with animated height.

use super::common::{builder_common, ControlOptions};
use super::modern::{enabled, focused, palette, publish};
use super::Panel;
use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleRole};
use crate::core::motion::{self, Easing, Transition};
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::graphics::{Canvas, Font, Pen};
use crate::input::Key;
use crate::utilities::{Point, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Logical header height.
pub const EXPANDER_HEADER: i32 = 40;

struct ExpanderState {
    header: RefCell<String>,
    expanded: Cell<bool>,
    content_height: Cell<i32>,
    hot: Cell<bool>,
    content: RefCell<Option<Panel>>,
    transition: RefCell<Option<Transition>>,
}

/// Expander with title and arrow (˅/˄); click, Enter or Space toggles [`content`](Self::content). Animates its own
/// height (header plus content) and requests container layout. Accessible as a group with ExpandCollapse.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let advanced = Expander::builder().header("Advanced options").content_height(120).bounds(10, 10, 360, 0).parent(window).build()?;
/// CheckBox::builder().text("Log diagnostics").bounds(12, 12, 240, 24).parent(&advanced.content()).build()?;
/// advanced.set_expanded(true);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Expander {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<ExpanderState>,
}

impl Widget for Expander {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for Expander {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Expander").field("expanded", &self.state.expanded.get()).finish()
    }
}

/// Builder for [`Expander`].
#[derive(Clone, Debug)]
pub struct ExpanderBuilder {
    opts: ControlOptions,
    header: String,
    expanded: bool,
    content_height: i32,
}

impl ExpanderBuilder {
    builder_common!();

    /// Header title.
    pub fn header(mut self, header: &str) -> Self {
        self.header = header.to_string();
        self
    }

    /// Initially expanded.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Expanded content height in logical units (default 120).
    pub fn content_height(mut self, height: i32) -> Self {
        self.content_height = height.max(0);
        self
    }

    /// Creates the expander. Height in `bounds` is ignored: header height plus content height when expanded is used.
    pub fn build(self) -> GuiResult<Expander> {
        let mut opts = self.opts.clone();
        opts.bounds.height = EXPANDER_HEADER + if self.expanded { self.content_height } else { 0 };
        let core = opts.create("Expander", PANEL_CLASS, 0, 0, true, None)?;
        crate::core::native::set_style_bits(
            core.hwnd(),
            crate::ffi::user32::GWL_STYLE,
            crate::ffi::user32::WS_CLIPCHILDREN,
            true,
        );
        let content = Panel::builder()
            .bounds(1, EXPANDER_HEADER, (opts.bounds.width - 2).max(0), self.content_height.max(1))
            .visible(self.expanded)
            .parent(&crate::core::Window { core: core.clone() })
            .build()?;
        let state = Rc::new(ExpanderState {
            header: RefCell::new(self.header),
            expanded: Cell::new(self.expanded),
            content_height: Cell::new(self.content_height),
            hot: Cell::new(false),
            content: RefCell::new(Some(content)),
            transition: RefCell::new(None),
        });
        let expander = Expander { core, state };
        expander.install();
        Ok(expander)
    }
}

impl Expander {
    /// Starts building (default width 320).
    pub fn builder() -> ExpanderBuilder {
        ExpanderBuilder {
            opts: ControlOptions::new(320, EXPANDER_HEADER),
            header: String::new(),
            expanded: false,
            content_height: 120,
        }
    }

    /// Content panel (use as `parent` for expander controls).
    pub fn content(&self) -> Panel {
        self.state.content.borrow().clone().expect("content created in build")
    }

    fn install(&self) {
        let state = Rc::downgrade(&self.state);
        let target = Rc::downgrade(&self.core);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(state), Some(core)) = (state.upgrade(), target.upgrade()) {
                paint(&core, &state, canvas);
            }
        }));
        let me = self.weak();
        self.core.on(EventKind::MouseUp, move |event| {
            if let (Some(e), Event::MouseUp(m)) = (me.upgrade(), event) {
                if m.position.y < EXPANDER_HEADER && enabled(&e.core) {
                    e.focus();
                    e.toggle();
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseMove, move |event| {
            if let (Some(e), Event::MouseMove(m)) = (me.upgrade(), event) {
                let hot = m.position.y < EXPANDER_HEADER;
                if e.state.hot.replace(hot) != hot {
                    e.core.invalidate();
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseLeave, move |_| {
            if let Some(e) = me.upgrade() {
                e.state.hot.set(false);
                e.core.invalidate();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::KeyDown, move |event| {
            if let (Some(e), Event::KeyDown(key)) = (me.upgrade(), event) {
                match key.key {
                    Key::ENTER | Key::SPACE if !key.repeat => e.toggle(),
                    Key::RIGHT => e.set_expanded(true),
                    Key::LEFT => e.set_expanded(false),
                    _ => {}
                }
            }
        });
        for kind in [EventKind::Focus, EventKind::Blur] {
            let me = self.weak();
            self.core.on(kind, move |_| {
                if let Some(e) = me.upgrade() {
                    e.core.invalidate();
                }
            });
        }
        let me = self.weak();
        self.core.on(EventKind::Resize, move |_| {
            if let Some(e) = me.upgrade() {
                let width = e.client_size().width;
                let height = e.state.content_height.get().max(1);
                e.content().set_bounds(Rect::new(1, EXPANDER_HEADER, (width - 2).max(0), height));
            }
        });
        publish(
            &self.core,
            &self.state,
            |_, state| {
                let item = AccessibleItem::new(0, AccessibleRole::Group, &state.header.borrow())
                    .expanded(state.expanded.get());
                (item, Vec::new())
            },
            |core, state, id, action| {
                let e = Expander { core: core.clone(), state: state.clone() };
                match (id, action) {
                    (0, AccessibleAction::Expand) => {
                        e.set_expanded(true);
                        true
                    }
                    (0, AccessibleAction::Collapse) => {
                        e.set_expanded(false);
                        true
                    }
                    _ => false,
                }
            },
        );
    }

    fn weak(&self) -> WeakExpander {
        WeakExpander { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    /// `true` when expanded.
    pub fn is_expanded(&self) -> bool {
        self.state.expanded.get()
    }

    /// Expands/collapses with animated height; fires `on_expanded_changed` if changed.
    pub fn set_expanded(&self, expanded: bool) {
        if self.state.expanded.replace(expanded) == expanded {
            return;
        }
        if let Some(old) = self.state.transition.borrow_mut().take() {
            old.cancel();
        }
        let content = self.content();
        if expanded {
            content.set_visible(true);
        }
        let from = self.bounds();
        let target = Rect::new(from.x, from.y, from.width, self.preferred_height());
        let me = self.weak();
        let finish = move || {
            if let Some(e) = me.upgrade() {
                if !e.state.expanded.get() {
                    e.content().set_visible(false);
                }
                e.relayout_parent();
            }
        };
        let me = self.weak();
        match motion::animate(Duration::from_millis(200), Easing::EaseOut, move |t| {
            if let Some(e) = me.upgrade() {
                e.set_bounds(motion::lerp_rect(from, target, t));
                e.relayout_parent();
            }
        }) {
            Ok(transition) => {
                transition.on_finished(finish);
                *self.state.transition.borrow_mut() = Some(transition);
            }
            Err(_) => {
                self.set_bounds(target);
                finish();
            }
        }
        self.core.invalidate();
        self.core.emit(&Event::ValueChanged);
    }

    /// Toggles expanded/collapsed state.
    pub fn toggle(&self) {
        self.set_expanded(!self.is_expanded());
    }

    /// Logical height for the current state (header plus content when expanded).
    pub fn preferred_height(&self) -> i32 {
        EXPANDER_HEADER + if self.is_expanded() { self.state.content_height.get() } else { 0 }
    }

    /// Changes expanded content height.
    pub fn set_content_height(&self, height: i32) {
        self.state.content_height.set(height.max(0));
        if self.is_expanded() {
            let b = self.bounds();
            self.set_bounds(Rect::new(b.x, b.y, b.width, self.preferred_height()));
            self.relayout_parent();
        }
    }

    /// Changes the title.
    pub fn set_header(&self, header: &str) {
        *self.state.header.borrow_mut() = header.to_string();
        self.core.invalidate();
    }

    /// Called on expansion/collapse; receives the new state.
    pub fn on_expanded_changed(&self, f: impl Fn(bool) + 'static) -> HandlerId {
        let state = Rc::downgrade(&self.state);
        self.core.on(EventKind::ValueChanged, move |_| {
            if let Some(state) = state.upgrade() {
                f(state.expanded.get())
            }
        })
    }

    fn relayout_parent(&self) {
        // SAFETY: consulta simples do pai.
        let parent = unsafe { crate::ffi::user32::GetParent(self.core.hwnd()) };
        if let Some(parent) = crate::core::widget::lookup(parent) {
            parent.relayout();
        }
    }
}

#[derive(Clone)]
struct WeakExpander {
    core: std::rc::Weak<WidgetCore>,
    state: std::rc::Weak<ExpanderState>,
}

impl WeakExpander {
    fn upgrade(&self) -> Option<Expander> {
        Some(Expander { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

fn paint(core: &WidgetCore, state: &ExpanderState, canvas: &Canvas) {
    let p = palette(core);
    let size = canvas.size();
    canvas.clear(p.background);
    let surface = if p.dark { p.background.mix(p.foreground, 0.06) } else { p.surface };
    let header = Rect::new(0, 0, size.width, EXPANDER_HEADER);
    let fill = if state.hot.get() { surface.mix(p.foreground, 0.04) } else { surface };
    canvas.round_rect(Rect::new(0, 0, size.width - 1, size.height - 1), 6, Some(&Pen::new(p.border, 1)), Some(surface));
    canvas.round_rect(Rect::new(1, 1, size.width - 3, EXPANDER_HEADER - 2), 5, None, Some(fill));
    if state.expanded.get() {
        canvas.line(
            Point::new(1, EXPANDER_HEADER - 1),
            Point::new(size.width - 2, EXPANDER_HEADER - 1),
            &Pen::new(p.border, 1),
        );
    }
    let color = if enabled(core) { p.foreground } else { p.muted };
    canvas.text_line(
        &state.header.borrow(),
        Rect::new(14, 0, size.width - 56, EXPANDER_HEADER),
        &Font::system(),
        color,
    );
    // Seta: ˅ fechado, ˄ aberto.
    let (cx, cy) = (size.width - 22, EXPANDER_HEADER / 2);
    let points = if state.expanded.get() {
        [Point::new(cx - 5, cy + 3), Point::new(cx, cy - 2), Point::new(cx + 5, cy + 3)]
    } else {
        [Point::new(cx - 5, cy - 2), Point::new(cx, cy + 3), Point::new(cx + 5, cy - 2)]
    };
    canvas.polyline(&points, &Pen::new(color, 1));
    if focused(core) {
        canvas.focus_rect(Rect::new(3, 3, header.width - 6, header.height - 6));
    }
}
