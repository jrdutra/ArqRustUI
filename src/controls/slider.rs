//! Continuous-value controls: [`Slider`] (TrackBar) and [`ScrollBar`].

use super::common::{builder_common, widget_type, ControlOptions};
use crate::core::native;
use crate::core::widget::Reflected;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;

widget_type!(
    /// Slider (TrackBar).
    Slider
);

/// Builder for [`Slider`].
#[derive(Clone, Debug)]
pub struct SliderBuilder {
    opts: ControlOptions,
    range: (i32, i32),
    value: i32,
    vertical: bool,
    tick_frequency: Option<i32>,
}

impl SliderBuilder {
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

    /// Shows ticks every `frequency` units.
    pub fn ticks(mut self, frequency: i32) -> Self {
        self.tick_frequency = Some(frequency);
        self
    }

    /// Creates the control.
    pub fn build(self) -> GuiResult<Slider> {
        let mut style = 0;
        if self.vertical {
            style |= TBS_VERT;
        }
        if self.tick_frequency.is_some() {
            style |= TBS_AUTOTICKS;
        }
        let core = self.opts.create("Slider", "msctls_trackbar32", style, 0, true, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Scroll { .. } => {
                core.emit(&Event::ValueChanged);
                Some(0)
            }
            _ => None,
        });
        let slider = Slider { core };
        slider.set_range(self.range.0, self.range.1);
        if let Some(frequency) = self.tick_frequency {
            native::send(slider.core.hwnd(), TBM_SETTICFREQ, frequency.max(1) as usize, 0);
        }
        slider.set_value(self.value);
        Ok(slider)
    }
}

impl Slider {
    /// Starts building (default size 200×32).
    pub fn builder() -> SliderBuilder {
        SliderBuilder {
            opts: ControlOptions::new(200, 32),
            range: (0, 100),
            value: 0,
            vertical: false,
            tick_frequency: None,
        }
    }

    /// Sets the value range.
    pub fn set_range(&self, min: i32, max: i32) {
        native::send(self.core.hwnd(), TBM_SETRANGEMIN, 0, min as isize);
        native::send(self.core.hwnd(), TBM_SETRANGEMAX, 1, max as isize);
    }

    /// Sets value programmatically without firing `on_value_changed`.
    pub fn set_value(&self, value: i32) {
        native::send(self.core.hwnd(), TBM_SETPOS, 1, value as isize);
    }

    /// Current value.
    pub fn value(&self) -> i32 {
        native::send(self.core.hwnd(), TBM_GETPOS, 0, 0) as i32
    }

    /// Sets step for Page Up/Down and track clicks.
    pub fn set_page_size(&self, size: i32) {
        native::send(self.core.hwnd(), TBM_SETPAGESIZE, 0, size as isize);
    }

    /// The user moved the control; receives the new value.
    pub fn on_value_changed(&self, f: impl Fn(i32) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.value()))
    }
}

widget_type!(
    /// Standalone scroll bar.
    ScrollBar
);

/// Builder for [`ScrollBar`].
#[derive(Clone, Debug)]
pub struct ScrollBarBuilder {
    opts: ControlOptions,
    vertical: bool,
    range: (i32, i32),
    page: u32,
    value: i32,
}

impl ScrollBarBuilder {
    builder_common!();

    /// Vertical orientation (default: horizontal).
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Value range (default: 0 to 100).
    pub fn range(mut self, min: i32, max: i32) -> Self {
        self.range = (min, max);
        self
    }

    /// Page size determines thumb size and page step.
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Initial value.
    pub fn value(mut self, value: i32) -> Self {
        self.value = value;
        self
    }

    /// Creates the scroll bar.
    pub fn build(self) -> GuiResult<ScrollBar> {
        let style = if self.vertical { SBS_VERT } else { SBS_HORZ };
        let core = self.opts.create("ScrollBar", "SCROLLBAR", style, 0, false, None)?;
        // O controle SCROLLBAR não move o polegar sozinho: a posição é atualizada aqui.
        core.set_reflect(|core, reflected| {
            let Reflected::Scroll { code } = reflected else { return None };
            let hwnd = core.hwnd();
            let mut info =
                SCROLLINFO { cbSize: std::mem::size_of::<SCROLLINFO>() as u32, fMask: SIF_ALL, ..Default::default() };
            // SAFETY: estrutura local corretamente dimensionada.
            unsafe { GetScrollInfo(hwnd, SB_CTL, &mut info) };
            let page = (info.nPage as i32).max(1);
            let max_position = info.nMax - (info.nPage as i32 - 1).max(0);
            let target = match *code {
                SB_LINEUP => info.nPos - 1,
                SB_LINEDOWN => info.nPos + 1,
                SB_PAGEUP => info.nPos - page,
                SB_PAGEDOWN => info.nPos + page,
                SB_THUMBTRACK | SB_THUMBPOSITION => info.nTrackPos,
                SB_TOP => info.nMin,
                SB_BOTTOM => max_position,
                _ => info.nPos,
            }
            .clamp(info.nMin, max_position.max(info.nMin));
            if target != info.nPos {
                info.fMask = SIF_POS;
                info.nPos = target;
                // SAFETY: estrutura local corretamente dimensionada.
                unsafe { SetScrollInfo(hwnd, SB_CTL, &info, 1) };
                core.emit(&Event::ValueChanged);
            }
            Some(0)
        });
        let bar = ScrollBar { core };
        bar.configure(self.range.0, self.range.1, self.page);
        bar.set_value(self.value);
        Ok(bar)
    }
}

impl ScrollBar {
    /// Starts building (default size 200×18).
    pub fn builder() -> ScrollBarBuilder {
        ScrollBarBuilder { opts: ControlOptions::new(200, 18), vertical: false, range: (0, 100), page: 10, value: 0 }
    }

    /// Sets range and page size.
    pub fn configure(&self, min: i32, max: i32, page: u32) {
        let info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_RANGE | SIF_PAGE,
            nMin: min,
            nMax: max,
            nPage: page,
            ..Default::default()
        };
        // SAFETY: estrutura local corretamente dimensionada.
        unsafe { SetScrollInfo(self.core.hwnd(), SB_CTL, &info, 1) };
    }

    /// Sets position programmatically without firing `on_value_changed`.
    pub fn set_value(&self, value: i32) {
        let info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_POS,
            nPos: value,
            ..Default::default()
        };
        // SAFETY: estrutura local corretamente dimensionada.
        unsafe { SetScrollInfo(self.core.hwnd(), SB_CTL, &info, 1) };
    }

    /// Current position.
    pub fn value(&self) -> i32 {
        let mut info =
            SCROLLINFO { cbSize: std::mem::size_of::<SCROLLINFO>() as u32, fMask: SIF_POS, ..Default::default() };
        // SAFETY: estrutura local corretamente dimensionada.
        unsafe { GetScrollInfo(self.core.hwnd(), SB_CTL, &mut info) };
        info.nPos
    }

    /// The user scrolled; receives the new position.
    pub fn on_value_changed(&self, f: impl Fn(i32) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.value()))
    }
}
