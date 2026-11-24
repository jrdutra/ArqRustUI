//! Containers: [`Panel`] and [`GroupBox`].

use super::common::{builder_common, widget_type, ControlOptions};
use crate::core::window::PANEL_CLASS;
use crate::core::GuiResult;
use crate::ffi::user32::*;
use crate::graphics::Canvas;
use crate::layout::Layout;
use crate::utilities::Color;
use std::rc::Rc;

widget_type!(
    /// Panel: generic control container and custom control base.
    ///
    /// * As container: use `.parent(&panel)` for other controls and attach a layout with
    ///   [`set_layout`](Panel::set_layout).
    /// * As custom control: register [`on_paint`](Panel::on_paint) and [`Widget`](crate::core::Widget) mouse/keyboard
    ///   events.
    Panel
);

/// Builder for [`Panel`].
#[derive(Clone, Debug)]
pub struct PanelBuilder {
    opts: ControlOptions,
    border: bool,
    background: Option<Color>,
    focusable: bool,
}

impl PanelBuilder {
    builder_common!();

    /// Draws a thin panel border.
    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }

    /// Background color.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Allows keyboard focus for custom controls.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Creates the panel.
    pub fn build(self) -> GuiResult<Panel> {
        let mut style = WS_CLIPCHILDREN | WS_CLIPSIBLINGS;
        if self.border {
            style |= WS_BORDER;
        }
        let core = self.opts.create("Panel", PANEL_CLASS, style, WS_EX_CONTROLPARENT, self.focusable, None)?;
        core.background.set(self.background);
        let panel = Panel { core };
        if self.focusable {
            // Controles personalizados focáveis recebem o foco ao serem clicados.
            let target = panel.clone();
            panel.core.on(crate::core::EventKind::MouseDown, move |_| crate::core::Widget::focus(&target));
        }
        Ok(panel)
    }
}

impl Panel {
    /// Starts building (default size 200×150).
    pub fn builder() -> PanelBuilder {
        PanelBuilder { opts: ControlOptions::new(200, 150), border: false, background: None, focusable: false }
    }

    /// Attaches a layout to panel children, reapplied on each resize.
    pub fn set_layout(&self, layout: impl Layout + 'static) {
        self.core.set_layout(Some(Rc::new(layout)));
    }

    /// Reapplies layout immediately.
    pub fn relayout(&self) {
        self.core.relayout();
    }

    /// Registers panel drawing (GDI, double buffered by default).
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
}

widget_type!(
    /// Group box: titled frame visually grouping other controls.
    ///
    /// Grouped controls may be children (`.parent(&group)`) or siblings positioned over the box.
    GroupBox
);

/// Builder for [`GroupBox`].
#[derive(Clone, Debug)]
pub struct GroupBoxBuilder {
    opts: ControlOptions,
}

impl GroupBoxBuilder {
    builder_common!();

    /// Creates the group box.
    pub fn build(self) -> GuiResult<GroupBox> {
        let core = self.opts.create("GroupBox", "BUTTON", BS_GROUPBOX, WS_EX_CONTROLPARENT, false, None)?;
        Ok(GroupBox { core })
    }
}

impl GroupBox {
    /// Starts building (default size 200×120).
    pub fn builder() -> GroupBoxBuilder {
        GroupBoxBuilder { opts: ControlOptions::new(200, 120) }
    }

    /// Attaches a layout to box children.
    pub fn set_layout(&self, layout: impl Layout + 'static) {
        self.core.set_layout(Some(Rc::new(layout)));
    }
}
