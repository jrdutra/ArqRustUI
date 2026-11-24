//! Split container with draggable splitter (own implementation).

use super::panel::Panel;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiError, GuiResult, HandlerId};
use crate::ffi::user32::*;
use crate::layout::engine::{self, Orientation};
use crate::system::{Cursor, SystemCursor};
use crate::utilities::Rect;
use std::cell::Cell;
use std::rc::Rc;

struct SplitState {
    orientation: Orientation,
    position: Cell<i32>,
    splitter: i32,
    min_first: Cell<i32>,
    min_second: Cell<i32>,
    dragging: Cell<bool>,
}

/// Two panels separated by a draggable splitter.
///
/// Own implementation: container panel, two child [`Panel`]s and Rust drag logic. Place content in
/// [`first`](Self::first) and [`second`](Self::second).
#[derive(Clone)]
pub struct SplitContainer {
    pub(crate) core: Rc<WidgetCore>,
    first: Panel,
    second: Panel,
    state: Rc<SplitState>,
}

impl Widget for SplitContainer {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

/// Builder for [`SplitContainer`].
#[derive(Clone, Debug)]
pub struct SplitContainerBuilder {
    opts: super::common::ControlOptions,
    orientation: Orientation,
    position: i32,
    min_first: i32,
    min_second: i32,
}

impl SplitContainerBuilder {
    super::common::builder_common!();

    /// Panels stacked with horizontal splitter (default: side by side).
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.orientation = if stacked { Orientation::Vertical } else { Orientation::Horizontal };
        self
    }

    /// Initial first panel size (width/height according to orientation).
    pub fn split_position(mut self, position: i32) -> Self {
        self.position = position;
        self
    }

    /// Minimum sizes of both panels.
    pub fn min_sizes(mut self, first: i32, second: i32) -> Self {
        self.min_first = first;
        self.min_second = second;
        self
    }

    /// Creates the container and both panels.
    pub fn build(self) -> GuiResult<SplitContainer> {
        let core = self.opts.create(
            "SplitContainer",
            PANEL_CLASS,
            WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            WS_EX_CONTROLPARENT,
            false,
            None,
        )?;
        let holder = Panel { core: core.clone() };
        let first = Panel::builder().parent(&holder).build()?;
        let second = Panel::builder().parent(&holder).build()?;
        let state = Rc::new(SplitState {
            orientation: self.orientation,
            position: Cell::new(self.position),
            splitter: 6,
            min_first: Cell::new(self.min_first),
            min_second: Cell::new(self.min_second),
            dragging: Cell::new(false),
        });
        let cursor = Cursor::system(if self.orientation == Orientation::Horizontal {
            SystemCursor::SizeHorizontal
        } else {
            SystemCursor::SizeVertical
        })?;
        core.set_cursor(Some(cursor));
        let split = SplitContainer { core, first, second, state };
        split.install_handlers();
        split.arrange();
        Ok(split)
    }
}

impl SplitContainer {
    /// Starts building (default size 400×300, split at 150).
    pub fn builder() -> SplitContainerBuilder {
        SplitContainerBuilder {
            opts: super::common::ControlOptions::new(400, 300),
            orientation: Orientation::Horizontal,
            position: 150,
            min_first: 40,
            min_second: 40,
        }
    }

    fn install_handlers(&self) {
        let on_resize = self.clone();
        self.core.on(EventKind::Resize, move |_| on_resize.arrange());
        let on_down = self.clone();
        self.core.on(EventKind::MouseDown, move |_| {
            on_down.state.dragging.set(true);
            // SAFETY: captura o mouse para o contêiner vivo durante o arraste.
            unsafe { SetCapture(on_down.core.hwnd()) };
        });
        let on_move = self.clone();
        self.core.on(EventKind::MouseMove, move |event| {
            if let (true, Event::MouseMove(mouse)) = (on_move.state.dragging.get(), event) {
                let along = if on_move.state.orientation == Orientation::Horizontal {
                    mouse.position.x
                } else {
                    mouse.position.y
                };
                on_move.set_split_position(along - on_move.state.splitter / 2);
                on_move.core.emit(&Event::ValueChanged);
            }
        });
        let on_up = self.clone();
        self.core.on(EventKind::MouseUp, move |_| {
            if on_up.state.dragging.replace(false) {
                // SAFETY: libera a captura feita em MouseDown.
                unsafe { ReleaseCapture() };
            }
        });
    }

    fn arrange(&self) {
        let size = self.core.client_size();
        let (first, _, second) = engine::split(
            Rect::new(0, 0, size.width, size.height),
            self.state.orientation,
            self.state.position.get(),
            self.state.splitter,
            self.state.min_first.get(),
            self.state.min_second.get(),
        );
        self.first.set_bounds(first);
        self.second.set_bounds(second);
    }

    /// First panel (left/top).
    pub fn first(&self) -> &Panel {
        &self.first
    }

    /// Second panel (right/bottom).
    pub fn second(&self) -> &Panel {
        &self.second
    }

    /// Effective first panel width/height.
    pub fn split_position(&self) -> i32 {
        let size = self.first.size();
        if self.state.orientation == Orientation::Horizontal {
            size.width
        } else {
            size.height
        }
    }

    /// Moves the splitter, clamped by minimum sizes.
    pub fn set_split_position(&self, position: i32) {
        self.state.position.set(position.max(0));
        self.arrange();
    }

    /// Sets both panels' minimum sizes.
    pub fn set_min_sizes(&self, first: i32, second: i32) -> GuiResult<()> {
        if first < 0 || second < 0 {
            return Err(GuiError::InvalidArgument("minimum sizes cannot be negative".into()));
        }
        self.state.min_first.set(first);
        self.state.min_second.set(second);
        self.arrange();
        Ok(())
    }

    /// The user dragged the splitter; receives the new position.
    pub fn on_split_moved(&self, f: impl Fn(i32) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.split_position()))
    }
}
