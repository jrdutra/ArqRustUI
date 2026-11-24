//! NumberBox: text field with ▲/▼ buttons, limits, step, decimals and simple expressions (`2*3+1`) evaluated on
//! confirmation.

use super::common::{builder_common, ControlOptions};
use super::modern::{enabled, palette, publish};
use super::TextBox;
use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleRange, AccessibleRole};
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::graphics::{Canvas, Pen};
use crate::input::Key;
use crate::utilities::{Point, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Logical width of the ▲/▼ button column.
const SPIN: i32 = 22;

/// Evaluates simple arithmetic: numbers with `.`/`,` decimal separator, `+ - * /`, `%` remainder, parentheses and
/// signs. `None` if invalid or dividing by zero.
pub fn evaluate(text: &str) -> Option<f64> {
    struct Parser<'a> {
        chars: std::iter::Peekable<std::str::Chars<'a>>,
    }
    impl Parser<'_> {
        fn skip(&mut self) {
            while self.chars.peek().is_some_and(|c| c.is_whitespace() || *c == '\u{a0}') {
                self.chars.next();
            }
        }
        fn expr(&mut self) -> Option<f64> {
            let mut value = self.term()?;
            loop {
                self.skip();
                match self.chars.peek() {
                    Some('+') => {
                        self.chars.next();
                        value += self.term()?;
                    }
                    Some('-') | Some('−') => {
                        self.chars.next();
                        value -= self.term()?;
                    }
                    _ => return Some(value),
                }
            }
        }
        fn term(&mut self) -> Option<f64> {
            let mut value = self.factor()?;
            loop {
                self.skip();
                match self.chars.peek() {
                    Some('*') | Some('×') | Some('x') => {
                        self.chars.next();
                        value *= self.factor()?;
                    }
                    Some('/') | Some('÷') => {
                        self.chars.next();
                        let divisor = self.factor()?;
                        if divisor == 0.0 {
                            return None;
                        }
                        value /= divisor;
                    }
                    Some('%') => {
                        self.chars.next();
                        let divisor = self.factor()?;
                        if divisor == 0.0 {
                            return None;
                        }
                        value %= divisor;
                    }
                    _ => return Some(value),
                }
            }
        }
        fn factor(&mut self) -> Option<f64> {
            self.skip();
            match self.chars.peek()? {
                '-' | '−' => {
                    self.chars.next();
                    Some(-self.factor()?)
                }
                '+' => {
                    self.chars.next();
                    self.factor()
                }
                '(' => {
                    self.chars.next();
                    let value = self.expr()?;
                    self.skip();
                    (self.chars.next()? == ')').then_some(value)
                }
                _ => {
                    let mut digits = String::new();
                    while let Some(&c) = self.chars.peek() {
                        if c.is_ascii_digit() {
                            digits.push(c);
                        } else if c == '.' || c == ',' {
                            digits.push('.');
                        } else {
                            break;
                        }
                        self.chars.next();
                    }
                    if digits.matches('.').count() > 1 {
                        return None;
                    }
                    digits.parse().ok()
                }
            }
        }
    }
    let mut parser = Parser { chars: text.trim().chars().peekable() };
    let value = parser.expr()?;
    parser.skip();
    (parser.chars.next().is_none() && value.is_finite()).then_some(value)
}

/// Value text with `decimals` places and the given decimal separator.
pub(crate) fn format_number(value: f64, decimals: u32, comma: bool) -> String {
    let text = format!("{value:.*}", decimals as usize);
    let text = if text == "-0" || text.starts_with("-0.") && text[3..].chars().all(|c| c == '0') {
        text.trim_start_matches('-').to_string()
    } else {
        text
    };
    if comma {
        text.replace('.', ",")
    } else {
        text
    }
}

/// `true` if the user's locale uses decimal commas.
fn system_uses_comma() -> bool {
    crate::i18n::Locale::user_default().format_number(1.5).contains(',')
}

struct NumberState {
    value: Cell<f64>,
    minimum: Cell<f64>,
    maximum: Cell<f64>,
    step: Cell<f64>,
    large_step: Cell<f64>,
    decimals: Cell<u32>,
    comma: Cell<bool>,
    hot: Cell<Option<bool>>,
    field: RefCell<Option<TextBox>>,
}

/// Numeric field accepting numbers/expressions such as `2*3+1`, evaluated on Enter/focus loss. ▲/▼, arrows and Page
/// Up/Down change by the step; values stay within limits. Accessible as a spinner (RangeValue pattern).
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let price = NumberBox::builder().range(0.0, 1000.0).step(0.5).decimals(2).value(9.9).parent(window).build()?;
/// price.on_value_changed(|v| println!("{v}"));
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct NumberBox {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<NumberState>,
}

impl Widget for NumberBox {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for NumberBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NumberBox").field("value", &self.state.value.get()).finish()
    }
}

/// Builder for [`NumberBox`].
#[derive(Clone, Debug)]
pub struct NumberBoxBuilder {
    opts: ControlOptions,
    value: f64,
    minimum: f64,
    maximum: f64,
    step: f64,
    large_step: Option<f64>,
    decimals: u32,
    comma: Option<bool>,
}

impl NumberBoxBuilder {
    builder_common!();

    /// Initial value.
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Limits (default: no practical limits).
    pub fn range(mut self, minimum: f64, maximum: f64) -> Self {
        self.minimum = minimum.min(maximum);
        self.maximum = maximum.max(minimum);
        self
    }

    /// Arrow key/button step (default 1).
    pub fn step(mut self, step: f64) -> Self {
        self.step = step.abs().max(f64::EPSILON);
        self
    }

    /// Page Up/Down step (default: 10 × step).
    pub fn large_step(mut self, step: f64) -> Self {
        self.large_step = Some(step.abs());
        self
    }

    /// Displayed decimal places (default 0); values are rounded accordingly.
    pub fn decimals(mut self, decimals: u32) -> Self {
        self.decimals = decimals.min(10);
        self
    }

    /// Forces decimal separator (`true` = comma). Default: user locale.
    pub fn decimal_comma(mut self, comma: bool) -> Self {
        self.comma = Some(comma);
        self
    }

    /// Creates the box.
    pub fn build(self) -> GuiResult<NumberBox> {
        let core = self.opts.create("NumberBox", PANEL_CLASS, 0, 0, false, None)?;
        let state = Rc::new(NumberState {
            value: Cell::new(0.0),
            minimum: Cell::new(self.minimum),
            maximum: Cell::new(self.maximum),
            step: Cell::new(self.step),
            large_step: Cell::new(self.large_step.unwrap_or(self.step * 10.0)),
            decimals: Cell::new(self.decimals),
            comma: Cell::new(self.comma.unwrap_or_else(system_uses_comma)),
            hot: Cell::new(None),
            field: RefCell::new(None),
        });
        let size = self.opts.bounds;
        let field = TextBox::builder()
            .bounds(0, 0, (size.width - SPIN).max(10), size.height)
            .parent(&crate::core::Window { core: core.clone() })
            .build()?;
        if let Some(font) = &self.opts.font {
            field.set_font(font.clone());
        }
        *state.field.borrow_mut() = Some(field);
        let number = NumberBox { core, state };
        number.state.value.set(number.clamp(self.value));
        number.show_value();
        number.install();
        Ok(number)
    }
}

impl NumberBox {
    /// Starts building (default size 140 × default control height).
    pub fn builder() -> NumberBoxBuilder {
        NumberBoxBuilder {
            opts: ControlOptions::new(140, crate::controls::DEFAULT_CONTROL_HEIGHT),
            value: 0.0,
            minimum: -1e15,
            maximum: 1e15,
            step: 1.0,
            large_step: None,
            decimals: 0,
            comma: None,
        }
    }

    /// Internal text field.
    pub fn text_box(&self) -> TextBox {
        self.state.field.borrow().clone().expect("field created in build")
    }

    fn clamp(&self, value: f64) -> f64 {
        let factor = 10f64.powi(self.state.decimals.get() as i32);
        let rounded = (value * factor).round() / factor;
        rounded.clamp(self.state.minimum.get(), self.state.maximum.get())
    }

    fn show_value(&self) {
        let text = format_number(self.state.value.get(), self.state.decimals.get(), self.state.comma.get());
        self.text_box().set_text(&text);
    }

    fn install(&self) {
        let state = Rc::downgrade(&self.state);
        let target = Rc::downgrade(&self.core);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(state), Some(core)) = (state.upgrade(), target.upgrade()) {
                paint(&core, &state, canvas);
            }
        }));
        let field = self.text_box();
        let me = self.weak();
        field.on_key_down(move |key| {
            if let Some(number) = me.upgrade() {
                match key.key {
                    Key::UP => number.step_by(1.0, false),
                    Key::DOWN => number.step_by(-1.0, false),
                    Key::PAGE_UP => number.step_by(1.0, true),
                    Key::PAGE_DOWN => number.step_by(-1.0, true),
                    Key::ENTER => {
                        number.commit();
                    }
                    Key::ESCAPE => number.show_value(),
                    _ => {}
                }
            }
        });
        let me = self.weak();
        field.on_blur(move || {
            if let Some(number) = me.upgrade() {
                number.commit();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseDown, move |event| {
            if let (Some(number), Event::MouseDown(m)) = (me.upgrade(), event) {
                if let Some(up) = number.spin_part(m.position) {
                    if enabled(&number.core) {
                        number.commit();
                        number.step_by(if up { 1.0 } else { -1.0 }, false);
                    }
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseMove, move |event| {
            if let (Some(number), Event::MouseMove(m)) = (me.upgrade(), event) {
                let hot = number.spin_part(m.position);
                if number.state.hot.replace(hot) != hot {
                    number.core.invalidate();
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseLeave, move |_| {
            if let Some(number) = me.upgrade() {
                number.state.hot.set(None);
                number.core.invalidate();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::Resize, move |_| {
            if let Some(number) = me.upgrade() {
                let size = number.client_size();
                number.text_box().set_bounds(Rect::new(0, 0, (size.width - SPIN).max(10), size.height));
            }
        });
        publish(
            &self.core,
            &self.state,
            |core, state| {
                let name = crate::core::native::window_text(core.hwnd());
                let text = format_number(state.value.get(), state.decimals.get(), state.comma.get());
                let item = AccessibleItem::new(0, AccessibleRole::Spinner, &name)
                    .range(AccessibleRange {
                        value: state.value.get(),
                        minimum: state.minimum.get(),
                        maximum: state.maximum.get(),
                        small_change: state.step.get(),
                        large_change: state.large_step.get(),
                        read_only: false,
                    })
                    .value(&text, false);
                (item, Vec::new())
            },
            |core, state, id, action| {
                let number = NumberBox { core: core.clone(), state: state.clone() };
                match (id, action) {
                    (0, AccessibleAction::SetRange(v)) => {
                        if v < state.minimum.get() || v > state.maximum.get() {
                            return false;
                        }
                        number.set_value(v);
                        true
                    }
                    (0, AccessibleAction::SetValue(text)) => match evaluate(&text) {
                        Some(v) => {
                            number.set_value(v);
                            true
                        }
                        None => false,
                    },
                    _ => false,
                }
            },
        );
    }

    fn weak(&self) -> WeakNumber {
        WeakNumber { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    /// `Some(true)` over ▲, `Some(false)` over ▼.
    fn spin_part(&self, at: Point) -> Option<bool> {
        let size = self.client_size();
        (at.x >= size.width - SPIN && at.x < size.width && at.y >= 0 && at.y < size.height)
            .then_some(at.y < size.height / 2)
    }

    fn step_by(&self, direction: f64, large: bool) {
        let step = if large { self.state.large_step.get() } else { self.state.step.get() };
        self.set_value(self.state.value.get() + direction * step);
    }

    /// Evaluates entered text; invalid input restores the current value. Returns `true` if accepted.
    pub fn commit(&self) -> bool {
        let text = self.text_box().text();
        match evaluate(&text) {
            Some(value) => {
                self.set_value(value);
                self.show_value();
                true
            }
            None => {
                self.show_value();
                false
            }
        }
    }

    /// Current value.
    pub fn value(&self) -> f64 {
        self.state.value.get()
    }

    /// Sets the value, rounded to decimal places and clamped to limits.
    pub fn set_value(&self, value: f64) {
        let value = self.clamp(value);
        let changed = (self.state.value.replace(value) - value).abs() > f64::EPSILON;
        self.show_value();
        if changed {
            self.core.emit(&Event::ValueChanged);
        }
    }

    /// Changes limits, adjusting out-of-range values.
    pub fn set_range(&self, minimum: f64, maximum: f64) {
        self.state.minimum.set(minimum.min(maximum));
        self.state.maximum.set(maximum.max(minimum));
        self.set_value(self.value());
    }

    /// Current limits.
    pub fn range(&self) -> (f64, f64) {
        (self.state.minimum.get(), self.state.maximum.get())
    }

    /// Called when the value changes.
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
struct WeakNumber {
    core: std::rc::Weak<WidgetCore>,
    state: std::rc::Weak<NumberState>,
}

impl WeakNumber {
    fn upgrade(&self) -> Option<NumberBox> {
        Some(NumberBox { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

fn paint(core: &WidgetCore, state: &NumberState, canvas: &Canvas) {
    let p = palette(core);
    let size = canvas.size();
    canvas.clear(p.background);
    let column = Rect::new(size.width - SPIN, 0, SPIN, size.height);
    canvas.rect(Rect::new(column.x, 0, SPIN, size.height), Some(&Pen::new(p.border, 1)), Some(p.surface));
    let half = size.height / 2;
    for (up, top) in [(true, 0), (false, half)] {
        if state.hot.get() == Some(up) {
            canvas.fill_rect(Rect::new(column.x + 1, top + 1, SPIN - 2, half - 1), p.hover);
        }
        let (cx, cy) = (column.x + SPIN / 2, top + half / 2);
        let points = if up {
            [Point::new(cx - 4, cy + 2), Point::new(cx + 4, cy + 2), Point::new(cx, cy - 2)]
        } else {
            [Point::new(cx - 4, cy - 2), Point::new(cx + 4, cy - 2), Point::new(cx, cy + 2)]
        };
        let at_limit =
            if up { state.value.get() >= state.maximum.get() } else { state.value.get() <= state.minimum.get() };
        let color = if at_limit || !enabled(core) { p.border } else { p.foreground };
        canvas.polygon(&points, None, Some(color));
    }
    canvas.line(Point::new(column.x, half), Point::new(column.x + SPIN, half), &Pen::new(p.border, 1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expressions() {
        assert_eq!(evaluate("2*3+1"), Some(7.0));
        assert_eq!(evaluate(" (1,5 + 2.5) / 2 "), Some(2.0), "vírgula ou ponto decimal");
        assert_eq!(evaluate("-3 - -2"), Some(-1.0));
        assert_eq!(evaluate("10 % 4"), Some(2.0));
        assert_eq!(evaluate("2 × 3 ÷ 4"), Some(1.5));
        assert_eq!(evaluate("1/0"), None);
        assert_eq!(evaluate("1.2.3"), None);
        assert_eq!(evaluate("2+"), None);
        assert_eq!(evaluate("(1"), None);
        assert_eq!(evaluate("abc"), None);
        assert_eq!(evaluate(""), None);
        assert_eq!(format_number(1.236, 2, true), "1,24");
        assert_eq!(format_number(-0.001, 2, false), "0.00");
        assert_eq!(format_number(12.0, 0, true), "12");
    }
}
