//! Progress bar.

use super::common::{builder_common, widget_type, ControlOptions};
use crate::core::native;
use crate::core::GuiResult;
use crate::ffi::comctl32::*;
use crate::ffi::user32::GWL_STYLE;

/// Progress bar visual state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProgressState {
    /// In progress (green).
    Normal,
    /// Error (red).
    Error,
    /// Paused (yellow).
    Paused,
}

widget_type!(
    /// Determinate or indeterminate (marquee) progress bar.
    ProgressBar
);

/// Builder for [`ProgressBar`].
#[derive(Clone, Debug)]
pub struct ProgressBarBuilder {
    opts: ControlOptions,
    range: (i32, i32),
    value: i32,
    vertical: bool,
    marquee: bool,
}

impl ProgressBarBuilder {
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

    /// Vertical orientation.
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Indeterminate mode: continuous animation without a value.
    pub fn marquee(mut self, marquee: bool) -> Self {
        self.marquee = marquee;
        self
    }

    /// Creates the bar.
    pub fn build(self) -> GuiResult<ProgressBar> {
        let mut style = PBS_SMOOTH;
        if self.vertical {
            style |= PBS_VERTICAL;
        }
        if self.marquee {
            style |= PBS_MARQUEE;
        }
        let core = self.opts.create("ProgressBar", "msctls_progress32", style, 0, false, None)?;
        let bar = ProgressBar { core };
        bar.set_range(self.range.0, self.range.1);
        bar.set_value(self.value);
        if self.marquee {
            bar.set_marquee(true);
        }
        Ok(bar)
    }
}

impl ProgressBar {
    /// Starts building (default size 200×20).
    pub fn builder() -> ProgressBarBuilder {
        ProgressBarBuilder {
            opts: ControlOptions::new(200, 20),
            range: (0, 100),
            value: 0,
            vertical: false,
            marquee: false,
        }
    }

    /// Sets the value range.
    pub fn set_range(&self, min: i32, max: i32) {
        native::send(self.core.hwnd(), PBM_SETRANGE32, min as usize, max as isize);
    }

    /// Sets the current value.
    pub fn set_value(&self, value: i32) {
        native::send(self.core.hwnd(), PBM_SETPOS, value as usize, 0);
    }

    /// Current value.
    pub fn value(&self) -> i32 {
        native::send(self.core.hwnd(), PBM_GETPOS, 0, 0) as i32
    }

    /// Advances by `amount`.
    pub fn advance(&self, amount: i32) {
        self.set_value(self.value() + amount);
    }

    /// Enables/disables indeterminate mode.
    pub fn set_marquee(&self, enabled: bool) {
        native::set_style_bits(self.core.hwnd(), GWL_STYLE, PBS_MARQUEE, enabled);
        native::send(self.core.hwnd(), PBM_SETMARQUEE, enabled as usize, 30);
    }

    /// Sets visual state (normal, error or paused).
    pub fn set_state(&self, state: ProgressState) {
        let value = match state {
            ProgressState::Normal => 1,
            ProgressState::Error => 2,
            ProgressState::Paused => 3,
        };
        native::send(self.core.hwnd(), PBM_SETSTATE, value, 0);
    }
}
