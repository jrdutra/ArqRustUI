//! Date/time controls: [`DatePicker`], [`TimePicker`] and [`MonthCalendar`].

use super::common::{builder_common, plain_builder, widget_type, ControlOptions};
use crate::core::native;
use crate::core::widget::Reflected;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::ffi::comctl32::*;
use crate::ffi::{RECT, SYSTEMTIME};
use crate::i18n::DateTime;
use crate::utilities::{to_logical, Size};

macro_rules! picker_methods {
    () => {
        /// Current value; `None` when unset.
        pub fn value(&self) -> Option<DateTime> {
            let mut st = SYSTEMTIME::default();
            let status = native::send_mut(self.core.hwnd(), DTM_GETSYSTEMTIME, 0, &mut st);
            (status as usize == GDT_VALID).then(|| DateTime::from_native(&st))
        }

        /// Sets the value programmatically.
        pub fn set_value(&self, value: &DateTime) {
            native::send_ref(self.core.hwnd(), DTM_SETSYSTEMTIME, GDT_VALID, &value.to_native());
        }

        /// Sets a custom format, such as `"dd/MM/yyyy"` or `"HH:mm"`.
        pub fn set_format(&self, format: &str) {
            native::send_text(self.core.hwnd(), DTM_SETFORMATW, 0, format);
        }

        /// The user changed the value.
        pub fn on_value_changed(&self, f: impl Fn(Option<DateTime>) + 'static) -> HandlerId {
            let this = self.clone();
            self.core.on(EventKind::ValueChanged, move |_| f(this.value()))
        }
    };
}

fn reflect_picker(core: &std::rc::Rc<crate::core::WidgetCore>, reflected: &Reflected) -> Option<isize> {
    match reflected {
        Reflected::Notify { code, .. } if *code == DTN_DATETIMECHANGE => {
            core.emit(&Event::ValueChanged);
            Some(0)
        }
        Reflected::Notify { code, .. } if *code == DTN_DROPDOWN => {
            crate::themes::dark::date_dropdown(core);
            None
        }
        _ => None,
    }
}

widget_type!(
    /// Date picker with drop-down calendar, in the user's regional format.
    DatePicker
);

/// Builder for [`DatePicker`].
#[derive(Clone, Debug)]
pub struct DatePickerBuilder {
    opts: ControlOptions,
    long_format: bool,
}

impl DatePickerBuilder {
    builder_common!();

    /// Uses long date format (e.g. "Thursday, November 12, 2026").
    pub fn long_format(mut self, long: bool) -> Self {
        self.long_format = long;
        self
    }

    /// Creates the picker.
    pub fn build(self) -> GuiResult<DatePicker> {
        let style = if self.long_format { DTS_LONGDATEFORMAT } else { 0 };
        let core = self.opts.create("DatePicker", "SysDateTimePick32", style, 0, true, None)?;
        core.set_reflect(reflect_picker);
        core.set_centered_text(true);
        Ok(DatePicker { core })
    }
}

impl DatePicker {
    /// Starts building (default size 140 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
    pub fn builder() -> DatePickerBuilder {
        DatePickerBuilder { opts: ControlOptions::new(140, super::DEFAULT_CONTROL_HEIGHT), long_format: false }
    }

    picker_methods!();
}

widget_type!(
    /// Time picker with increment arrows.
    TimePicker
);

plain_builder!(
    /// Builder for [`TimePicker`].
    TimePickerBuilder
);

impl TimePickerBuilder {
    /// Creates the picker.
    pub fn build(self) -> GuiResult<TimePicker> {
        let core = self.opts.create("TimePicker", "SysDateTimePick32", DTS_TIMEFORMAT | DTS_UPDOWN, 0, true, None)?;
        core.set_reflect(reflect_picker);
        core.set_centered_text(true);
        Ok(TimePicker { core })
    }
}

impl TimePicker {
    /// Starts building (default size 110 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
    pub fn builder() -> TimePickerBuilder {
        TimePickerBuilder { opts: ControlOptions::new(110, super::DEFAULT_CONTROL_HEIGHT) }
    }

    picker_methods!();
}

widget_type!(
    /// Always-visible monthly calendar.
    MonthCalendar
);

plain_builder!(
    /// Builder for [`MonthCalendar`].
    MonthCalendarBuilder
);

impl MonthCalendarBuilder {
    /// Creates the calendar.
    pub fn build(self) -> GuiResult<MonthCalendar> {
        let core = self.opts.create("MonthCalendar", "SysMonthCal32", 0, 0, true, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Notify { code, .. } if *code == MCN_SELCHANGE => {
                core.emit(&Event::SelectionChanged);
                Some(0)
            }
            _ => None,
        });
        Ok(MonthCalendar { core })
    }
}

impl MonthCalendar {
    /// Starts building (default size 230×170).
    pub fn builder() -> MonthCalendarBuilder {
        MonthCalendarBuilder { opts: ControlOptions::new(230, 170) }
    }

    /// Selected date.
    pub fn selected_date(&self) -> Option<DateTime> {
        let mut st = SYSTEMTIME::default();
        let ok = native::send_mut(self.core.hwnd(), MCM_GETCURSEL, 0, &mut st);
        (ok != 0).then(|| DateTime::date(st.wYear, st.wMonth, st.wDay))
    }

    /// Selects a date programmatically.
    pub fn set_selected_date(&self, date: &DateTime) {
        native::send_ref(self.core.hwnd(), MCM_SETCURSEL, 0, &date.to_native());
    }

    /// Minimum logical size to display a full month with the current font.
    pub fn minimum_size(&self) -> Size {
        let mut rect = RECT::default();
        native::send_mut(self.core.hwnd(), MCM_GETMINREQRECT, 0, &mut rect);
        let dpi = self.core.dpi();
        Size::new(to_logical(rect.right, dpi), to_logical(rect.bottom, dpi))
    }

    /// The user selected another date.
    pub fn on_selection_changed(&self, f: impl Fn(Option<DateTime>) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::SelectionChanged, move |_| f(this.selected_date()))
    }
}
