//! Increment controls: native [`UpDown`] and [`SpinButton`] (field with arrows).

use super::common::{builder_common, plain_builder, widget_type, ControlOptions};
use super::textbox::reflect_edit;
use crate::core::native;
use crate::core::widget::{Reflected, Widget, WidgetCore};
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;
use crate::utilities::Rect;
use std::rc::Rc;

widget_type!(
    /// Native Up-Down arrow pair without a buddy field.
    UpDown
);

plain_builder!(
    /// Builder for [`UpDown`].
    UpDownBuilder
);

fn install_updown_reflect(core: &Rc<WidgetCore>, notify: Rc<WidgetCore>) {
    core.set_reflect(move |core, reflected| match reflected {
        Reflected::Notify { code, .. } if *code == UDN_DELTAPOS => {
            // A posição só é atualizada depois da notificação; o evento é adiado
            // para que os manipuladores leiam o valor novo.
            let target = notify.clone();
            let _ = crate::core::defer(move || {
                target.emit(&Event::ValueChanged);
            });
            let _ = core;
            None
        }
        _ => None,
    });
}

impl UpDownBuilder {
    /// Creates the control with range 0 to 100.
    pub fn build(self) -> GuiResult<UpDown> {
        let core = self.opts.create("UpDown", "msctls_updown32", UDS_ARROWKEYS, 0, true, None)?;
        install_updown_reflect(&core, core.clone());
        let updown = UpDown { core };
        updown.set_range(0, 100);
        Ok(updown)
    }
}

impl UpDown {
    /// Starts building (default size 18 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
    pub fn builder() -> UpDownBuilder {
        UpDownBuilder { opts: ControlOptions::new(18, super::DEFAULT_CONTROL_HEIGHT) }
    }

    /// Sets the value range.
    pub fn set_range(&self, min: i32, max: i32) {
        native::send(self.core.hwnd(), UDM_SETRANGE32, min as usize, max as isize);
    }

    /// Current value.
    pub fn value(&self) -> i32 {
        native::send(self.core.hwnd(), UDM_GETPOS32, 0, 0) as i32
    }

    /// Sets the value programmatically.
    pub fn set_value(&self, value: i32) {
        native::send(self.core.hwnd(), UDM_SETPOS32, 0, value as isize);
    }

    /// The user clicked an arrow; receives the new value.
    pub fn on_value_changed(&self, f: impl Fn(i32) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.value()))
    }
}

/// Numeric field with increment arrows (composition: `EDIT` and attached Up-Down).
///
/// The exposed [`Widget`] is the text field; arrows follow automatically.
#[derive(Clone, Debug)]
pub struct SpinButton {
    pub(crate) core: Rc<WidgetCore>,
    arrows: Rc<WidgetCore>,
}

impl Widget for SpinButton {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }

    fn set_bounds(&self, bounds: Rect) {
        self.core.set_bounds(bounds);
        // O Up-Down se realinha à borda direita do campo quando o parceiro é reatribuído.
        native::send(self.arrows.hwnd(), UDM_SETBUDDY, self.core.hwnd() as usize, 0);
    }

    fn set_enabled(&self, enabled: bool) {
        // SAFETY: EnableWindow tolera HWND nulo/inválido.
        unsafe {
            EnableWindow(self.core.hwnd(), enabled as i32);
            EnableWindow(self.arrows.hwnd(), enabled as i32);
        }
    }

    fn set_visible(&self, visible: bool) {
        let command = if visible { SW_SHOW } else { SW_HIDE };
        // SAFETY: ShowWindow tolera HWND nulo/inválido.
        unsafe {
            ShowWindow(self.core.hwnd(), command);
            ShowWindow(self.arrows.hwnd(), command);
        }
    }

    fn destroy(&self) {
        self.arrows.destroy();
        self.core.destroy();
    }
}

/// Builder for [`SpinButton`].
#[derive(Clone, Debug)]
pub struct SpinButtonBuilder {
    opts: ControlOptions,
    range: (i32, i32),
    value: i32,
}

impl SpinButtonBuilder {
    builder_common!();

    /// Value range (default: 0 to 100).
    pub fn range(mut self, min: i32, max: i32) -> Self {
        self.range = (min, max);
        self
    }

    /// Initial value.
    pub fn value(mut self, value: i32) -> Self {
        self.value = value;
        self
    }

    /// Creates the field and arrows.
    pub fn build(self) -> GuiResult<SpinButton> {
        let core = self.opts.create("SpinButton", "EDIT", ES_AUTOHSCROLL | ES_RIGHT, WS_EX_CLIENTEDGE, true, None)?;
        core.set_centered_text(true);
        let parent = self.opts.parent.clone().ok_or(crate::core::GuiError::MissingParent)?;
        let mut style = UDS_SETBUDDYINT | UDS_ALIGNRIGHT | UDS_ARROWKEYS | UDS_NOTHOUSANDS;
        if self.opts.visible {
            style |= WS_VISIBLE;
        }
        let arrows =
            WidgetCore::create_child("UpDown", &parent, "msctls_updown32", "", style, 0, Rect::new(0, 0, 0, 0), None)?;
        native::send(arrows.hwnd(), UDM_SETBUDDY, core.hwnd() as usize, 0);
        core.set_reflect(|core, reflected| {
            let handled = reflect_edit(core, reflected);
            if handled.is_some() {
                core.emit(&Event::ValueChanged);
            }
            handled
        });
        let spin = SpinButton { core, arrows };
        spin.set_range(self.range.0, self.range.1);
        spin.set_value(self.value);
        Ok(spin)
    }
}

impl SpinButton {
    /// Starts building (default size 90 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
    pub fn builder() -> SpinButtonBuilder {
        SpinButtonBuilder { opts: ControlOptions::new(90, super::DEFAULT_CONTROL_HEIGHT), range: (0, 100), value: 0 }
    }

    /// Sets the value range.
    pub fn set_range(&self, min: i32, max: i32) {
        native::send(self.arrows.hwnd(), UDM_SETRANGE32, min as usize, max as isize);
    }

    /// Current value (entered text validated against range).
    pub fn value(&self) -> i32 {
        native::send(self.arrows.hwnd(), UDM_GETPOS32, 0, 0) as i32
    }

    /// Sets the value programmatically.
    pub fn set_value(&self, value: i32) {
        native::send(self.arrows.hwnd(), UDM_SETPOS32, 0, value as isize);
    }

    /// Value changed by arrows, keyboard or typing; receives the new value.
    pub fn on_value_changed(&self, f: impl Fn(i32) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.value()))
    }
}
