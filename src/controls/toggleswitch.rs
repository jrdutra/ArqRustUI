//! Library-drawn on/off switch with animated thumb sliding.

use super::common::{builder_common, ControlOptions};
use super::modern::{enabled, focused, palette, publish};
use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleRole};
use crate::core::motion::{self, Easing, Transition};
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::graphics::vector::{Figure, Paint, Stroke, VectorPainter};
use crate::graphics::{Canvas, Font};
use crate::i18n::{library_text, LibraryText};
use crate::input::Key;
use crate::utilities::Rect;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const TRACK_W: i32 = 40;
const TRACK_H: i32 = 20;

struct SwitchState {
    on: Cell<bool>,
    /// Animated thumb position (0 = off, 1 = on).
    position: Cell<f64>,
    header: RefCell<String>,
    on_text: RefCell<Option<String>>,
    off_text: RefCell<Option<String>>,
    hot: Cell<bool>,
    transition: RefCell<Option<Transition>>,
}

impl SwitchState {
    fn state_text(&self) -> String {
        if self.on.get() {
            self.on_text.borrow().clone().unwrap_or_else(|| library_text(LibraryText::On))
        } else {
            self.off_text.borrow().clone().unwrap_or_else(|| library_text(LibraryText::Off))
        }
    }
}

/// On/off switch: sliding thumb track and translatable On/Off state text, with optional left title.
///
/// Click, Space or Enter toggles. Accessible as a button with Toggle pattern.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let wifi = ToggleSwitch::builder().text("Wi-Fi").on(true).bounds(10, 10, 220, 28).parent(window).build()?;
/// wifi.on_toggled(|on| println!("Wi-Fi {}", if on { "on" } else { "off" }));
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct ToggleSwitch {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<SwitchState>,
}

impl Widget for ToggleSwitch {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for ToggleSwitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToggleSwitch").field("on", &self.state.on.get()).finish()
    }
}

/// Builder for [`ToggleSwitch`]. `.text` supplies the title on the left.
#[derive(Clone, Debug)]
pub struct ToggleSwitchBuilder {
    opts: ControlOptions,
    on: bool,
    on_text: Option<String>,
    off_text: Option<String>,
}

impl ToggleSwitchBuilder {
    builder_common!();

    /// Initial state.
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// State labels (default: library On/Off); empty strings hide text.
    pub fn state_texts(mut self, on: &str, off: &str) -> Self {
        self.on_text = Some(on.to_string());
        self.off_text = Some(off.to_string());
        self
    }

    /// Creates the switch.
    pub fn build(self) -> GuiResult<ToggleSwitch> {
        let core = self.opts.create("ToggleSwitch", PANEL_CLASS, 0, 0, true, None)?;
        let state = Rc::new(SwitchState {
            on: Cell::new(self.on),
            position: Cell::new(if self.on { 1.0 } else { 0.0 }),
            header: RefCell::new(self.opts.text.clone()),
            on_text: RefCell::new(self.on_text),
            off_text: RefCell::new(self.off_text),
            hot: Cell::new(false),
            transition: RefCell::new(None),
        });
        let switch = ToggleSwitch { core, state };
        switch.install();
        Ok(switch)
    }
}

impl ToggleSwitch {
    /// Starts building (default size 200×28).
    pub fn builder() -> ToggleSwitchBuilder {
        ToggleSwitchBuilder { opts: ControlOptions::new(200, 28), on: false, on_text: None, off_text: None }
    }

    fn install(&self) {
        let state = Rc::downgrade(&self.state);
        let target = Rc::downgrade(&self.core);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(state), Some(core)) = (state.upgrade(), target.upgrade()) {
                paint(&core, &state, canvas);
            }
        }));
        let me = self.clone_weak();
        self.core.on(EventKind::MouseUp, move |event| {
            if let (Some(switch), Event::MouseUp(_)) = (me.upgrade(), event) {
                if enabled(&switch.core) {
                    switch.focus();
                    switch.toggle();
                }
            }
        });
        let me = self.clone_weak();
        self.core.on(EventKind::KeyDown, move |event| {
            if let (Some(switch), Event::KeyDown(key)) = (me.upgrade(), event) {
                if (key.key == Key::SPACE || key.key == Key::ENTER) && !key.repeat {
                    switch.toggle();
                }
            }
        });
        for (kind, hot) in [(EventKind::MouseEnter, true), (EventKind::MouseLeave, false)] {
            let me = self.clone_weak();
            self.core.on(kind, move |_| {
                if let Some(switch) = me.upgrade() {
                    switch.state.hot.set(hot);
                    switch.core.invalidate();
                }
            });
        }
        for kind in [EventKind::Focus, EventKind::Blur] {
            let me = self.clone_weak();
            self.core.on(kind, move |_| {
                if let Some(switch) = me.upgrade() {
                    switch.core.invalidate();
                }
            });
        }
        publish(
            &self.core,
            &self.state,
            |_, state| {
                let name = state.header.borrow().clone();
                let name = if name.is_empty() { state.state_text() } else { name };
                (AccessibleItem::new(0, AccessibleRole::Button, &name).toggle(state.on.get()), Vec::new())
            },
            |core, state, id, action| {
                if id != 0 || action != AccessibleAction::Toggle {
                    return false;
                }
                let switch = ToggleSwitch { core: core.clone(), state: state.clone() };
                switch.toggle();
                true
            },
        );
    }

    fn clone_weak(&self) -> WeakSwitch {
        WeakSwitch { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    /// `true` when on.
    pub fn is_on(&self) -> bool {
        self.state.on.get()
    }

    /// Sets on/off state with animation; fires `on_toggled` if changed.
    pub fn set_on(&self, on: bool) {
        if self.state.on.replace(on) == on {
            return;
        }
        if let Some(old) = self.state.transition.borrow_mut().take() {
            old.cancel();
        }
        let from = self.state.position.get();
        let to = if on { 1.0 } else { 0.0 };
        if !motion::animations_enabled() {
            self.state.position.set(to);
        }
        let weak = Rc::downgrade(&self.state);
        let core = Rc::downgrade(&self.core);
        let transition = motion::animate_value(from, to, Duration::from_millis(160), Easing::EaseOut, move |p| {
            if let (Some(state), Some(core)) = (weak.upgrade(), core.upgrade()) {
                state.position.set(p);
                core.invalidate();
            }
        });
        match transition {
            Ok(t) => *self.state.transition.borrow_mut() = Some(t),
            Err(_) => self.state.position.set(to),
        }
        self.core.invalidate();
        crate::accessibility::raise_accessible_event(self, 0, crate::accessibility::AccessibleEvent::Invoked);
        self.core.emit(&Event::ValueChanged);
    }

    /// Toggles state.
    pub fn toggle(&self) {
        self.set_on(!self.is_on());
    }

    /// Changes the left title.
    pub fn set_header(&self, header: &str) {
        *self.state.header.borrow_mut() = header.to_string();
        self.core.invalidate();
    }

    /// Current state text (On/Off or custom labels).
    pub fn state_text(&self) -> String {
        self.state.state_text()
    }

    /// Called on state change; receives the new state.
    pub fn on_toggled(&self, f: impl Fn(bool) + 'static) -> HandlerId {
        let state = Rc::downgrade(&self.state);
        self.core.on(EventKind::ValueChanged, move |_| {
            if let Some(state) = state.upgrade() {
                f(state.on.get())
            }
        })
    }
}

#[derive(Clone)]
struct WeakSwitch {
    core: std::rc::Weak<WidgetCore>,
    state: std::rc::Weak<SwitchState>,
}

impl WeakSwitch {
    fn upgrade(&self) -> Option<ToggleSwitch> {
        Some(ToggleSwitch { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

/// Track rectangle in a `width` × `height` control with title width `header_width` (0 = no title).
pub(crate) fn track_rect(height: i32, header_width: i32) -> Rect {
    let x = if header_width > 0 { header_width + 12 } else { 0 };
    Rect::new(x, (height - TRACK_H) / 2, TRACK_W, TRACK_H)
}

fn paint(core: &WidgetCore, state: &SwitchState, canvas: &Canvas) {
    let p = palette(core);
    let size = canvas.size();
    canvas.clear(p.background);
    let font = Font::system();
    let header = state.header.borrow().clone();
    let header_width = if header.is_empty() { 0 } else { canvas.measure_text(&header, &font).width };
    let active = enabled(core);
    let text_color = if active { p.foreground } else { p.muted };
    if !header.is_empty() {
        canvas.text_line(&header, Rect::new(0, 0, header_width + 4, size.height), &font, text_color);
    }
    let track = track_rect(size.height, header_width);
    let t = state.position.get().clamp(0.0, 1.0);
    let off_fill = if state.hot.get() { p.hover } else { p.background };
    let fill = crate::core::motion::lerp_color(off_fill, if active { p.accent } else { p.muted }, t);
    let outline = crate::core::motion::lerp_color(p.muted, fill, t);
    let knob = crate::core::motion::lerp_color(p.foreground.mix(p.background, 0.25), p.on_accent, t);
    let _ = canvas.with_gdiplus(|graphics| {
        let painter = VectorPainter { graphics };
        let (x, y, w, h) =
            (track.x as f64 + 0.5, track.y as f64 + 0.5, track.width as f64 - 1.0, track.height as f64 - 1.0);
        let pill = pill(x, y, w, h);
        painter.fill(std::slice::from_ref(&pill), false, &Paint::Solid(fill), 1.0);
        painter.stroke(&[pill], &Stroke { width: 1.0, ..Stroke::default() }, &Paint::Solid(outline), 1.0);
        let radius = h / 2.0 - 4.0 + if state.hot.get() { 1.0 } else { 0.0 };
        let cx = x + h / 2.0 + (w - h) * t;
        painter.fill(&[Figure::ellipse(cx, y + h / 2.0, radius, radius)], false, &Paint::Solid(knob), 1.0);
    });
    let label = state.state_text();
    if !label.is_empty() {
        let rect = Rect::new(track.x + track.width + 10, 0, size.width - track.x - track.width - 10, size.height);
        canvas.text_line(&label, rect, &font, text_color);
    }
    if focused(core) {
        canvas.focus_rect(Rect::new(track.x - 3, track.y - 3, track.width + 6, track.height + 6));
    }
}

/// Pill (rectangle with round ends).
pub(crate) fn pill(x: f64, y: f64, w: f64, h: f64) -> Figure {
    let r = h / 2.0;
    let mut figure = Figure::new((x + r, y));
    figure.segments.push(crate::graphics::Segment::Line((x + w - r, y)));
    crate::graphics::vector::arc_segments(
        &mut figure,
        x + w - r,
        y + r,
        r,
        r,
        0.0,
        -std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    figure.segments.push(crate::graphics::Segment::Line((x + r, y + h)));
    crate::graphics::vector::arc_segments(
        &mut figure,
        x + r,
        y + r,
        r,
        r,
        0.0,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    figure.closed = true;
    figure
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_and_pill_geometry() {
        assert_eq!(track_rect(28, 0), Rect::new(0, 4, TRACK_W, TRACK_H));
        assert_eq!(track_rect(28, 50), Rect::new(62, 4, TRACK_W, TRACK_H));
        let p = pill(0.0, 0.0, 40.0, 20.0);
        let (x0, y0, x1, y1) = crate::graphics::vector::bounds(&[p]).unwrap();
        assert!(x0 >= -0.01 && y0 >= -0.01 && x1 <= 40.01 && y1 <= 20.01, "{x0} {y0} {x1} {y1}");
    }
}
