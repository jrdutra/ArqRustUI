//! Library-drawn star rating (`RatingControl`).

use super::common::{builder_common, ControlOptions};
use super::modern::{enabled, focused, palette, publish};
use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleRange, AccessibleRole};
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::graphics::vector::{Figure, Paint, Segment, Stroke, VectorPainter};
use crate::graphics::Canvas;
use crate::i18n::{library_format, LibraryText};
use crate::input::Key;
use crate::utilities::Rect;
use std::cell::Cell;
use std::rc::Rc;

/// Logical star size and spacing.
const STAR: i32 = 22;
const GAP: i32 = 4;

struct RatingState {
    value: Cell<f64>,
    maximum: Cell<u32>,
    half: Cell<bool>,
    read_only: Cell<bool>,
    hover: Cell<Option<f64>>,
}

/// Rounded value at logical `x` in a row of `maximum` stars.
pub(crate) fn value_at(x: i32, maximum: u32, half: bool) -> f64 {
    let pitch = (STAR + GAP) as f64;
    let raw = ((x as f64 + GAP as f64 / 2.0) / pitch).clamp(0.0, maximum as f64);
    let step = if half { 0.5 } else { 1.0 };
    ((raw / step).ceil() * step).clamp(step, maximum as f64)
}

/// Five-point star outline in square `(x, y, side)`.
pub(crate) fn star(x: f64, y: f64, side: f64) -> Figure {
    let (cx, cy) = (x + side / 2.0, y + side / 2.0 + side * 0.04);
    let (outer, inner) = (side / 2.0, side / 2.0 * 0.45);
    let point = |i: usize| {
        let radius = if i % 2 == 0 { outer } else { inner };
        let angle = (-90.0 + 36.0 * i as f64).to_radians();
        (cx + radius * angle.cos(), cy + radius * angle.sin())
    };
    let mut figure = Figure::new(point(0));
    figure.segments.extend((1..10).map(|i| Segment::Line(point(i))));
    figure.closed = true;
    figure
}

/// Star rating: click/drag to select, arrows to adjust, Home/End for limits, 0/Delete to clear. `allow_half` enables
/// half stars. Accessible as a slider (RangeValue).
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let rating = RatingControl::builder().maximum(5).value(3.5).allow_half(true).parent(window).build()?;
/// rating.on_value_changed(|v| println!("rating {v}"));
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct RatingControl {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<RatingState>,
}

impl Widget for RatingControl {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for RatingControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RatingControl").field("value", &self.state.value.get()).finish()
    }
}

/// Builder for [`RatingControl`].
#[derive(Clone, Debug)]
pub struct RatingControlBuilder {
    opts: ControlOptions,
    value: f64,
    maximum: u32,
    half: bool,
    read_only: bool,
}

impl RatingControlBuilder {
    builder_common!();

    /// Initial value (0 = unrated).
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Star count (default 5).
    pub fn maximum(mut self, maximum: u32) -> Self {
        self.maximum = maximum.max(1);
        self
    }

    /// Allows half stars.
    pub fn allow_half(mut self, half: bool) -> Self {
        self.half = half;
        self
    }

    /// Display only (no interaction).
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Creates the control.
    pub fn build(self) -> GuiResult<RatingControl> {
        let mut opts = self.opts.clone();
        if opts.bounds.width == 0 {
            opts.bounds.width = self.maximum as i32 * (STAR + GAP);
        }
        if opts.bounds.height == 0 {
            opts.bounds.height = STAR + 6;
        }
        let core = opts.create("RatingControl", PANEL_CLASS, 0, 0, !self.read_only, None)?;
        let state = Rc::new(RatingState {
            value: Cell::new(0.0),
            maximum: Cell::new(self.maximum),
            half: Cell::new(self.half),
            read_only: Cell::new(self.read_only),
            hover: Cell::new(None),
        });
        let rating = RatingControl { core, state };
        rating.state.value.set(rating.normalize(self.value));
        rating.install();
        Ok(rating)
    }
}

impl RatingControl {
    /// Starts building (default width: the stars).
    pub fn builder() -> RatingControlBuilder {
        RatingControlBuilder { opts: ControlOptions::new(0, 0), value: 0.0, maximum: 5, half: false, read_only: false }
    }

    fn normalize(&self, value: f64) -> f64 {
        let step = if self.state.half.get() { 0.5 } else { 1.0 };
        ((value / step).round() * step).clamp(0.0, self.state.maximum.get() as f64)
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
        self.core.on(EventKind::MouseMove, move |event| {
            if let (Some(r), Event::MouseMove(m)) = (me.upgrade(), event) {
                if r.interactive() {
                    let v = value_at(m.position.x, r.state.maximum.get(), r.state.half.get());
                    if r.state.hover.replace(Some(v)) != Some(v) {
                        r.core.invalidate();
                    }
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseLeave, move |_| {
            if let Some(r) = me.upgrade() {
                r.state.hover.set(None);
                r.core.invalidate();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseUp, move |event| {
            if let (Some(r), Event::MouseUp(m)) = (me.upgrade(), event) {
                if r.interactive() {
                    r.focus();
                    let v = value_at(m.position.x, r.state.maximum.get(), r.state.half.get());
                    // Clicar na nota atual limpa a avaliação.
                    r.set_value(if (v - r.value()).abs() < 1e-9 { 0.0 } else { v });
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::KeyDown, move |event| {
            if let (Some(r), Event::KeyDown(key)) = (me.upgrade(), event) {
                if !r.interactive() {
                    return;
                }
                let step = if r.state.half.get() { 0.5 } else { 1.0 };
                let max = r.state.maximum.get() as f64;
                match key.key {
                    Key::RIGHT | Key::UP => r.set_value(r.value() + step),
                    Key::LEFT | Key::DOWN => r.set_value(r.value() - step),
                    Key::HOME => r.set_value(step),
                    Key::END => r.set_value(max),
                    Key::DELETE | Key::BACKSPACE | Key(0x30) | Key(0x60) => r.set_value(0.0),
                    Key(code @ 0x31..=0x39) => r.set_value((code - 0x30) as f64),
                    _ => {}
                }
            }
        });
        for kind in [EventKind::Focus, EventKind::Blur] {
            let me = self.weak();
            self.core.on(kind, move |_| {
                if let Some(r) = me.upgrade() {
                    r.core.invalidate();
                }
            });
        }
        publish(
            &self.core,
            &self.state,
            |core, state| {
                let name = crate::core::native::window_text(core.hwnd());
                let text = library_format(
                    LibraryText::RatingValue,
                    &[
                        ("value", &super::chart::plain_number(state.value.get())),
                        ("max", &state.maximum.get().to_string()),
                    ],
                );
                let step = if state.half.get() { 0.5 } else { 1.0 };
                let item = AccessibleItem::new(0, AccessibleRole::Slider, if name.is_empty() { &text } else { &name })
                    .range(AccessibleRange {
                        value: state.value.get(),
                        minimum: 0.0,
                        maximum: state.maximum.get() as f64,
                        small_change: step,
                        large_change: 1.0,
                        read_only: state.read_only.get(),
                    })
                    .value(&text, true);
                (item, Vec::new())
            },
            |core, state, id, action| match (id, action) {
                (0, AccessibleAction::SetRange(v)) if !state.read_only.get() => {
                    if !(0.0..=state.maximum.get() as f64).contains(&v) {
                        return false;
                    }
                    RatingControl { core: core.clone(), state: state.clone() }.set_value(v);
                    true
                }
                _ => false,
            },
        );
    }

    fn interactive(&self) -> bool {
        !self.state.read_only.get() && enabled(&self.core)
    }

    fn weak(&self) -> WeakRating {
        WeakRating { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    /// Current rating (0 = unrated).
    pub fn value(&self) -> f64 {
        self.state.value.get()
    }

    /// Sets rating, rounded to the step and clamped to 0..=maximum.
    pub fn set_value(&self, value: f64) {
        let value = self.normalize(value);
        if (self.state.value.replace(value) - value).abs() < 1e-9 {
            return;
        }
        self.core.invalidate();
        self.core.emit(&Event::ValueChanged);
    }

    /// Star count.
    pub fn maximum(&self) -> u32 {
        self.state.maximum.get()
    }

    /// Enables/disables read-only mode.
    pub fn set_read_only(&self, read_only: bool) {
        self.state.read_only.set(read_only);
        self.state.hover.set(None);
        self.core.invalidate();
    }

    /// Logical rectangle of zero-based star `index`.
    pub fn star_bounds(&self, index: u32) -> Option<Rect> {
        (index < self.state.maximum.get()).then(|| Rect::new(index as i32 * (STAR + GAP) + GAP / 2, 3, STAR, STAR))
    }

    /// Called when rating changes.
    pub fn on_value_changed(&self, f: impl Fn(f64) + 'static) -> HandlerId {
        let state = Rc::downgrade(&self.state);
        self.core.on(EventKind::ValueChanged, move |_| {
            if let Some(state) = state.upgrade() {
                f(state.value.get())
            }
        })
    }
}

#[derive(Clone)]
struct WeakRating {
    core: std::rc::Weak<WidgetCore>,
    state: std::rc::Weak<RatingState>,
}

impl WeakRating {
    fn upgrade(&self) -> Option<RatingControl> {
        Some(RatingControl { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

fn paint(core: &WidgetCore, state: &RatingState, canvas: &Canvas) {
    let p = palette(core);
    canvas.clear(p.background);
    let shown = state.hover.get().unwrap_or(state.value.get());
    let gold = crate::utilities::Color::rgb(0xF5, 0xB4, 0x00);
    let fill = if enabled(core) {
        if state.hover.get().is_some() {
            p.accent
        } else {
            gold
        }
    } else {
        p.muted
    };
    let _ = canvas.with_gdiplus(|graphics| {
        let painter = VectorPainter { graphics };
        for i in 0..state.maximum.get() {
            let x = (i as i32 * (STAR + GAP) + GAP / 2) as f64;
            let figure = star(x, 3.0, STAR as f64);
            let amount = (shown - i as f64).clamp(0.0, 1.0);
            if amount >= 1.0 {
                painter.fill(std::slice::from_ref(&figure), false, &Paint::Solid(fill), 1.0);
            } else if amount > 0.0 {
                // Meia estrela: preenchimento recortado na metade esquerda.
                painter.with_clip(Rect::new(x as i32, 0, (STAR as f64 * amount).round() as i32, STAR + 6), || {
                    painter.fill(std::slice::from_ref(&figure), false, &Paint::Solid(fill), 1.0)
                });
            }
            let outline = if amount > 0.0 { fill } else { p.muted };
            painter.stroke(&[figure], &Stroke { width: 1.2, ..Stroke::default() }, &Paint::Solid(outline), 1.0);
        }
    });
    if focused(core) {
        let size = canvas.size();
        canvas.focus_rect(Rect::new(0, 0, size.width, size.height));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_from_positions() {
        let pitch = STAR + GAP;
        assert_eq!(value_at(2, 5, false), 1.0);
        assert_eq!(value_at(pitch + 3, 5, false), 2.0);
        assert_eq!(value_at(pitch * 2 + 5, 5, true), 2.5, "metade esquerda da terceira estrela");
        assert_eq!(value_at(pitch * 2 + STAR - 2, 5, true), 3.0);
        assert_eq!(value_at(10_000, 5, false), 5.0);
        assert_eq!(value_at(-10, 5, true), 0.5);
        let (x0, y0, x1, y1) = crate::graphics::vector::bounds(&[star(0.0, 0.0, 20.0)]).unwrap();
        assert!(x0 >= 0.0 && x1 <= 20.0 && y0 >= 0.0 && y1 <= 21.0);
    }
}
