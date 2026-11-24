//! Vertical scrolling of native controls composed in a panel.
use super::{Panel, ScrollBar};
use crate::core::{GuiResult, Widget};
use crate::utilities::{Rect, Size};
use std::cell::Cell;
use std::rc::Rc;

struct State {
    viewport: Panel,
    content: Panel,
    bar: ScrollBar,
    extent: Cell<i32>,
    offset: Cell<i32>,
    bar_width: Cell<i32>,
    wheel_step: Cell<i32>,
}

impl State {
    fn arrange(&self) -> Size {
        let page = self.viewport.client_size();
        let height = self.extent.get();
        let overflow = height > page.height;
        let width = (page.width - if overflow { self.bar_width.get() } else { 0 }).max(0);
        let offset = self.offset.get().clamp(0, (height - page.height).max(0));
        self.offset.set(offset);
        self.content.set_bounds(Rect::new(0, -offset, width, height.max(page.height)));
        self.bar.set_bounds(Rect::new(width, 0, self.bar_width.get(), page.height));
        self.bar.configure(0, height.max(1) - 1, page.height.max(1) as u32);
        self.bar.set_value(offset);
        self.bar.set_visible(overflow);
        Size::new(width, height)
    }
    fn scroll_to(&self, position: i32) {
        let maximum = (self.extent.get() - self.viewport.client_size().height).max(0);
        let position = position.clamp(0, maximum);
        self.offset.set(position);
        self.bar.set_value(position);
        self.content.set_position(0, -position);
    }
    fn wheel(&self, delta: i32) {
        self.scroll_to(self.offset.get().saturating_sub(delta.saturating_mul(self.wheel_step.get())));
    }
}

/// Scrollable area with content panel and automatic vertical bar. Controls belong to [`content`](Self::content); add
/// [`viewport`](Self::viewport) to the window layout. Handles are cloneable; the window keeps the mechanism alive until
/// destruction.
#[derive(Clone)]
pub struct ScrollViewport(Rc<State>);

impl ScrollViewport {
    /// Creates native panels/bar without application-specific content.
    pub fn new(parent: &impl Widget) -> GuiResult<Self> {
        let viewport = Panel::builder().parent(parent).build()?;
        let result = (|| {
            let content = Panel::builder().parent(&viewport).build()?;
            let bar = ScrollBar::builder().vertical(true).visible(false).parent(&viewport).build()?;
            Ok::<_, crate::core::GuiError>(Rc::new(State {
                viewport: viewport.clone(),
                content,
                bar,
                extent: Cell::new(0),
                offset: Cell::new(0),
                bar_width: Cell::new(17),
                wheel_step: Cell::new(60),
            }))
        })();
        let state = match result {
            Ok(state) => state,
            Err(error) => {
                viewport.destroy();
                return Err(error);
            }
        };
        let weak = Rc::downgrade(&state);
        viewport.on_resize(move |_| {
            if let Some(state) = weak.upgrade() {
                state.arrange();
            }
        });
        let weak = Rc::downgrade(&state);
        viewport.on_mouse_wheel(move |event| {
            if let Some(state) = weak.upgrade() {
                state.wheel(event.wheel_delta);
            }
        });
        let weak = Rc::downgrade(&state);
        state.bar.on_value_changed(move |position| {
            if let Some(state) = weak.upgrade() {
                state.scroll_to(position);
            }
        });
        // O registro nativo rompe essa retenção em WM_NCDESTROY; callbacks usam Weak.
        viewport.core().keep_alive(state.clone());
        Ok(Self(state))
    }

    /// Visible panel positioned by application layout.
    pub fn viewport(&self) -> Panel {
        self.0.viewport.clone()
    }
    /// Panel hosting scrollable controls.
    pub fn content(&self) -> Panel {
        self.0.content.clone()
    }
    /// Current vertical position in logical units.
    pub fn offset(&self) -> i32 {
        self.0.offset.get()
    }
    /// Whether current content requires the vertical bar.
    pub fn has_overflow(&self) -> bool {
        self.0.extent.get() > self.0.viewport.client_size().height
    }
    /// Logical scrollbar width (minimum 1; default 17).
    pub fn set_scrollbar_width(&self, width: i32) {
        self.0.bar_width.set(width.max(1));
        self.0.arrange();
    }
    /// Mouse wheel step (minimum 1; default 60).
    pub fn set_wheel_step(&self, step: i32) {
        self.0.wheel_step.set(step.max(1));
    }
    /// Sets content height and reapplies limits, position and bar.
    pub fn set_content_height(&self, height: i32) {
        self.0.extent.set(height.max(0));
        self.0.arrange();
    }
    /// Measures responsive content at available width; on overflow measures again subtracting scrollbar width. Measured
    /// height must stay stable or grow when width shrinks. Returns usable width/height.
    pub fn layout_content(&self, mut measure: impl FnMut(i32) -> i32) -> Size {
        let page = self.0.viewport.client_size();
        let mut height = measure(page.width).max(0);
        if height > page.height {
            height = measure((page.width - self.0.bar_width.get()).max(0)).max(0);
        }
        self.0.extent.set(height);
        self.0.arrange()
    }
    /// Scrolls to a position clamped to the available range.
    pub fn scroll_to(&self, position: i32) {
        self.0.scroll_to(position);
    }
    /// Processes wheel steps; positive scrolls up. The window may call this when focus is outside the viewport.
    pub fn scroll_wheel(&self, delta: i32) {
        self.0.wheel(delta);
    }
    /// Brings a content-relative rectangle into view with margin.
    pub fn ensure_visible(&self, rect: Rect, margin: i32) {
        let start = self.offset();
        let page = self.0.viewport.client_size().height;
        let margin = margin.max(0);
        if rect.y < start {
            self.scroll_to(rect.y.saturating_sub(margin));
        } else if rect.bottom() > start + page {
            self.scroll_to(rect.bottom().saturating_add(margin).saturating_sub(page));
        }
    }
}
