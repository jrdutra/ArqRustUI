//! State controls: [`CheckBox`], [`RadioButton`] and [`ToggleButton`].

use super::common::{builder_common, widget_type, ControlOptions};
use crate::core::native;
use crate::core::widget::{Reflected, WidgetCore};
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::ffi::user32::*;
use std::rc::Rc;

/// Check box state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    /// Unchecked.
    Unchecked,
    /// Checked.
    Checked,
    /// Indeterminate (three-state check boxes only).
    Indeterminate,
}

fn reflect_state(core: &Rc<WidgetCore>, reflected: &Reflected) -> Option<isize> {
    match reflected {
        Reflected::Command { code, .. } if *code == BN_CLICKED => {
            core.emit(&Event::Click);
            core.emit(&Event::ValueChanged);
            Some(0)
        }
        _ => None,
    }
}

fn is_checked(core: &WidgetCore) -> bool {
    native::send(core.hwnd(), BM_GETCHECK, 0, 0) as usize == BST_CHECKED
}

fn set_checked(core: &WidgetCore, checked: bool) {
    native::send(core.hwnd(), BM_SETCHECK, if checked { BST_CHECKED } else { BST_UNCHECKED }, 0);
}

macro_rules! state_control {
    ($name:ident, $builder:ident, $kind:literal, $style:expr, $width:expr) => {
        impl $name {
            /// Starts building the control (default height [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
            pub fn builder() -> $builder {
                $builder { opts: ControlOptions::new($width, super::DEFAULT_CONTROL_HEIGHT), checked: false, extra: 0 }
            }

            /// Whether it is checked.
            pub fn is_checked(&self) -> bool {
                is_checked(&self.core)
            }

            /// Checks or unchecks programmatically (does not fire `on_value_changed`).
            pub fn set_checked(&self, checked: bool) {
                set_checked(&self.core, checked);
            }

            /// The user changed the state; receives `true` when checked.
            pub fn on_value_changed(&self, f: impl Fn(bool) + 'static) -> HandlerId {
                let core = self.core.clone();
                self.core.on(EventKind::ValueChanged, move |_| f(is_checked(&core)))
            }
        }

        impl $builder {
            builder_common!();

            /// Initial state.
            pub fn checked(mut self, checked: bool) -> Self {
                self.checked = checked;
                self
            }

            /// Creates the control.
            pub fn build(self) -> GuiResult<$name> {
                // XOR permite tanto acrescentar bits (WS_GROUP) quanto trocar o tipo base do botão.
                let core = self.opts.create($kind, "BUTTON", $style ^ self.extra, 0, true, None)?;
                core.set_reflect(reflect_state);
                set_checked(&core, self.checked);
                Ok($name { core })
            }
        }
    };
}

widget_type!(
    /// Check box.
    CheckBox
);

/// Builder for [`CheckBox`].
#[derive(Clone, Debug)]
pub struct CheckBoxBuilder {
    opts: ControlOptions,
    checked: bool,
    extra: u32,
}

state_control!(CheckBox, CheckBoxBuilder, "CheckBox", BS_AUTOCHECKBOX, 160);

impl CheckBoxBuilder {
    /// Enables the third state (indeterminate).
    pub fn three_state(mut self, three_state: bool) -> Self {
        // O tipo base BS_AUTOCHECKBOX é trocado por BS_AUTO3STATE via XOR em `build`.
        self.extra = if three_state { BS_AUTOCHECKBOX ^ BS_AUTO3STATE } else { 0 };
        self
    }
}

impl CheckBox {
    /// Full state, including indeterminate.
    pub fn state(&self) -> CheckState {
        match native::send(self.core.hwnd(), BM_GETCHECK, 0, 0) as usize {
            BST_CHECKED => CheckState::Checked,
            BST_INDETERMINATE => CheckState::Indeterminate,
            _ => CheckState::Unchecked,
        }
    }

    /// Sets the full state.
    pub fn set_state(&self, state: CheckState) {
        let value = match state {
            CheckState::Unchecked => BST_UNCHECKED,
            CheckState::Checked => BST_CHECKED,
            CheckState::Indeterminate => BST_INDETERMINATE,
        };
        native::send(self.core.hwnd(), BM_SETCHECK, value, 0);
    }
}

widget_type!(
    /// Radio button. Buttons in the same group are mutually exclusive.
    ///
    /// A group starts at the button created with `.group_start(true)` and ends at the next control marked the same way
    /// (or the end of the container).
    RadioButton
);

/// Builder for [`RadioButton`].
#[derive(Clone, Debug)]
pub struct RadioButtonBuilder {
    opts: ControlOptions,
    checked: bool,
    extra: u32,
}

state_control!(RadioButton, RadioButtonBuilder, "RadioButton", BS_AUTORADIOBUTTON, 160);

impl RadioButtonBuilder {
    /// Marks this button as the first in a new group.
    pub fn group_start(mut self, first: bool) -> Self {
        self.extra = if first { WS_GROUP } else { 0 };
        self
    }
}

widget_type!(
    /// Toggle button: looks like a regular button but stays pressed.
    ToggleButton
);

/// Builder for [`ToggleButton`].
#[derive(Clone, Debug)]
pub struct ToggleButtonBuilder {
    opts: ControlOptions,
    checked: bool,
    extra: u32,
}

state_control!(ToggleButton, ToggleButtonBuilder, "ToggleButton", BS_AUTOCHECKBOX | BS_PUSHLIKE, 96);
