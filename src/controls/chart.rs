//! Line, area, grouped/stacked bar and pie charts drawn by the library with antialiasing (GDI+), legend, value axis,
//! hover highlighting, point clicks and animated data transitions.

use super::common::{builder_common, ControlOptions};
use crate::core::motion::{self, Easing, Transition};
use crate::core::widget::{lookup, Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::graphics::vector::{Figure, LineCap, LineJoin, Paint, Segment, Stroke, VectorPainter};
use crate::graphics::{Canvas, Font, Pen, TextAlign};
use crate::utilities::{Color, Point, Rect, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Chart type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChartKind {
    /// Lines with markers.
    #[default]
    Line,
    /// Lines with area filled down to zero.
    Area,
    /// Vertical bars side by side per category.
    Bar,
    /// Vertical bars stacked per category.
    StackedBar,
    /// Pie of the first series (one slice per category).
    Pie,
}

/// Value series (one value per category).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartSeries {
    /// Name (legend).
    pub name: String,
    /// Values.
    pub values: Vec<f64>,
    /// Custom color (`None` uses the palette).
    pub color: Option<Color>,
}

impl ChartSeries {
    /// Series with a name and values.
    pub fn new(name: &str, values: &[f64]) -> ChartSeries {
        ChartSeries { name: name.to_string(), values: values.to_vec(), color: None }
    }

    /// The same series with a custom color.
    pub fn with_color(mut self, color: Color) -> ChartSeries {
        self.color = Some(color);
        self
    }
}

/// Default palette for series and pie slices.
pub const CHART_PALETTE: [Color; 8] = [
    Color::rgb(0x4E, 0x79, 0xA7),
    Color::rgb(0xF2, 0x8E, 0x2B),
    Color::rgb(0xE1, 0x57, 0x59),
    Color::rgb(0x76, 0xB7, 0xB2),
    Color::rgb(0x59, 0xA1, 0x4F),
    Color::rgb(0xED, 0xC9, 0x48),
    Color::rgb(0xB0, 0x7A, 0xA1),
    Color::rgb(0xFF, 0x9D, 0xA7),
];

/// Round steps (1, 2, 5 × 10ⁿ) for the axis between `min` and `max`, with at most `count` divisions. Returns (start,
/// end, step).
pub(crate) fn nice_scale(min: f64, max: f64, count: usize) -> (f64, f64, f64) {
    let (mut min, mut max) = (min.min(max), min.max(max));
    if (max - min).abs() < 1e-12 {
        if max == 0.0 {
            max = 1.0;
        } else if max > 0.0 {
            min = 0.0;
        } else {
            max = 0.0;
        }
    }
    let raw = (max - min) / count.max(1) as f64;
    let magnitude = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0]
        .iter()
        .map(|f| f * magnitude)
        .find(|s| *s >= raw - 1e-12)
        .unwrap_or(10.0 * magnitude);
    ((min / step).floor() * step, (max / step).ceil() * step, step)
}

/// Default value text: integer when possible, otherwise up to two decimal places.
pub(crate) fn plain_number(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{}", value.round() as i64)
    } else {
        let text = format!("{value:.2}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Chart item under a point: `(series, index)`; pie series is 0.
pub type ChartHit = (usize, usize);

/// Pie: center, radius and each slice's (start, sweep) in degrees.
pub(crate) type PieGeometry = ((f64, f64), f64, Vec<(f64, f64)>);

/// Geometry calculated for a size and data set.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Geometry {
    pub plot: Rect,
    pub title: Option<Rect>,
    /// Axis ticks: (value, y).
    pub ticks: Vec<(f64, f64)>,
    /// Category x centers.
    pub columns: Vec<f64>,
    /// Bar rectangles: (series, index, x, y, width, height).
    pub bars: Vec<(usize, usize, f64, f64, f64, f64)>,
    /// Points of each series (line/area).
    pub points: Vec<Vec<(f64, f64)>>,
    /// Zero's y coordinate (bar and area baseline).
    pub baseline: f64,
    /// Pie: center, radius and each slice's (start, sweep) in degrees.
    pub pie: Option<PieGeometry>,
    /// Legend items: (label, color sample rectangle).
    pub legend: Vec<(String, Rect)>,
}

pub(crate) struct LayoutInput<'a> {
    pub size: Size,
    pub kind: ChartKind,
    pub title: &'a str,
    pub categories: &'a [String],
    pub series: &'a [ChartSeries],
    pub legend: bool,
    /// Entry animation progress (1.0 = full values).
    pub progress: f64,
    /// Previous values (transition starting point).
    pub previous: &'a [Vec<f64>],
}

const PAD: i32 = 8;
const LINE: i32 = 18;
const SWATCH: i32 = 10;

/// Calculates geometry. `measure` returns text width in logical units.
pub(crate) fn layout(input: &LayoutInput, measure: &dyn Fn(&str) -> i32, format: &dyn Fn(f64) -> String) -> Geometry {
    let mut g = Geometry::default();
    let mut area = Rect::new(PAD, PAD, (input.size.width - 2 * PAD).max(0), (input.size.height - 2 * PAD).max(0));
    if !input.title.is_empty() {
        g.title = Some(Rect::new(area.x, area.y, area.width, LINE + 4));
        area = Rect::new(area.x, area.y + LINE + 8, area.width, (area.height - LINE - 8).max(0));
    }
    let count = input.categories.len().max(input.series.iter().map(|s| s.values.len()).max().unwrap_or(0));
    let value = |series: usize, index: usize| -> f64 {
        let to = input.series.get(series).and_then(|s| s.values.get(index)).copied().unwrap_or(0.0);
        let to = if to.is_finite() { to } else { 0.0 };
        let from = input.previous.get(series).and_then(|v| v.get(index)).copied().unwrap_or(0.0);
        let from = if from.is_finite() { from } else { 0.0 };
        from + (to - from) * input.progress
    };
    // Legenda embaixo, em linhas que quebram.
    let labels: Vec<String> = if input.kind == ChartKind::Pie {
        (0..count).map(|i| input.categories.get(i).cloned().unwrap_or_else(|| format!("{}", i + 1))).collect()
    } else {
        input.series.iter().map(|s| s.name.clone()).collect()
    };
    if input.legend && !labels.is_empty() {
        let mut rows = vec![Vec::new()];
        let mut x = 0;
        for label in &labels {
            let width = SWATCH + 6 + measure(label) + 16;
            if x > 0 && x + width > area.width {
                rows.push(Vec::new());
                x = 0;
            }
            rows.last_mut().unwrap().push((label.clone(), x, width));
            x += width;
        }
        let height = rows.len() as i32 * LINE;
        let top = area.y + area.height - height;
        for (row, items) in rows.iter().enumerate() {
            let used: i32 = items.iter().map(|i| i.2).sum();
            let left = area.x + (area.width - used).max(0) / 2;
            for (label, x, _) in items {
                let y = top + row as i32 * LINE + (LINE - SWATCH) / 2;
                g.legend.push((label.clone(), Rect::new(left + x, y, SWATCH, SWATCH)));
            }
        }
        area = Rect::new(area.x, area.y, area.width, (area.height - height - 6).max(0));
    }
    if input.kind == ChartKind::Pie {
        g.plot = area;
        let radius = (area.width.min(area.height) as f64 / 2.0 - 4.0).max(0.0);
        let center = (area.x as f64 + area.width as f64 / 2.0, area.y as f64 + area.height as f64 / 2.0);
        let values: Vec<f64> = (0..count).map(|i| value(0, i).max(0.0)).collect();
        let total: f64 = values.iter().sum();
        let mut slices = Vec::new();
        let mut angle = -90.0;
        for v in values {
            let sweep = if total > 0.0 { v / total * 360.0 * input.progress.min(1.0) } else { 0.0 };
            slices.push((angle, sweep));
            angle += sweep;
        }
        g.pie = Some((center, radius, slices));
        return g;
    }
    // Faixa de valores (as barras empilhadas somam por categoria).
    let (mut low, mut high) = (0.0f64, 0.0f64);
    for index in 0..count {
        if input.kind == ChartKind::StackedBar {
            let (mut up, mut down) = (0.0, 0.0);
            for series in 0..input.series.len() {
                let v = value(series, index);
                if v >= 0.0 {
                    up += v
                } else {
                    down += v
                }
            }
            high = high.max(up);
            low = low.min(down);
        } else {
            for series in 0..input.series.len() {
                let v = value(series, index);
                high = high.max(v);
                low = low.min(v);
            }
        }
    }
    let ticks_wanted = (area.height / 40).clamp(2, 10) as usize;
    let (start, end, step) = nice_scale(low, high, ticks_wanted);
    let label_width = (0..=((end - start) / step).round() as i64)
        .map(|i| measure(&format(start + i as f64 * step)))
        .max()
        .unwrap_or(0);
    let plot = Rect::new(
        area.x + label_width + 8,
        area.y + 6,
        (area.width - label_width - 8).max(1),
        (area.height - LINE - 10).max(1),
    );
    g.plot = plot;
    let y_of = |v: f64| plot.y as f64 + plot.height as f64 * (1.0 - (v - start) / (end - start));
    g.baseline = y_of(0.0f64.clamp(start, end));
    let mut v = start;
    while v <= end + step / 2.0 {
        g.ticks.push((v, y_of(v)));
        v += step;
    }
    if count == 0 {
        return g;
    }
    let band = plot.width as f64 / count as f64;
    g.columns = (0..count).map(|i| plot.x as f64 + band * (i as f64 + 0.5)).collect();
    match input.kind {
        ChartKind::Bar => {
            let n = input.series.len().max(1) as f64;
            let group = band * 0.72;
            let width = group / n;
            for index in 0..count {
                for series in 0..input.series.len() {
                    let x = g.columns[index] - group / 2.0 + width * series as f64;
                    let y = y_of(value(series, index));
                    let (top, bottom) = (y.min(g.baseline), y.max(g.baseline));
                    g.bars.push((series, index, x + 1.0, top, (width - 2.0).max(1.0), bottom - top));
                }
            }
        }
        ChartKind::StackedBar => {
            let width = band * 0.6;
            for index in 0..count {
                let (mut up, mut down) = (0.0, 0.0);
                for series in 0..input.series.len() {
                    let v = value(series, index);
                    let (from, to) = if v >= 0.0 {
                        up += v;
                        (up - v, up)
                    } else {
                        down += v;
                        (down - v, down)
                    };
                    let (a, b) = (y_of(from), y_of(to));
                    g.bars.push((series, index, g.columns[index] - width / 2.0, a.min(b), width, (a - b).abs()));
                }
            }
        }
        ChartKind::Line | ChartKind::Area => {
            for series in 0..input.series.len() {
                let count = input.series[series].values.len().min(count);
                g.points.push((0..count).map(|i| (g.columns[i], y_of(value(series, i)))).collect());
            }
        }
        ChartKind::Pie => {}
    }
    g
}

/// Item under point `(x, y)`.
pub(crate) fn hit(g: &Geometry, kind: ChartKind, x: f64, y: f64) -> Option<ChartHit> {
    if let Some(((cx, cy), radius, slices)) = &g.pie {
        let (dx, dy) = (x - cx, y - cy);
        if (dx * dx + dy * dy).sqrt() > *radius {
            return None;
        }
        let mut angle = dy.atan2(dx).to_degrees();
        while angle < -90.0 {
            angle += 360.0;
        }
        return slices
            .iter()
            .position(|(start, sweep)| *sweep > 0.0 && angle >= *start && angle < start + sweep)
            .map(|i| (0, i));
    }
    match kind {
        ChartKind::Bar | ChartKind::StackedBar => g
            .bars
            .iter()
            .rev()
            .find(|(_, _, bx, by, w, h)| x >= *bx && x <= bx + w && y >= *by && y <= by + h.max(1.0))
            .map(|(s, i, ..)| (*s, *i)),
        _ => {
            let mut best: Option<(f64, ChartHit)> = None;
            for (series, points) in g.points.iter().enumerate() {
                for (index, (px, py)) in points.iter().enumerate() {
                    let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
                    if d <= 8.0 && best.is_none_or_farther(d) {
                        best = Some((d, (series, index)));
                    }
                }
            }
            best.map(|b| b.1)
        }
    }
}

trait Closest {
    fn is_none_or_farther(&self, d: f64) -> bool;
}

impl Closest for Option<(f64, ChartHit)> {
    fn is_none_or_farther(&self, d: f64) -> bool {
        self.map_or(true, |(best, _)| d < best)
    }
}

type FormatFn = Rc<dyn Fn(f64) -> String>;

struct ChartState {
    kind: Cell<ChartKind>,
    title: RefCell<String>,
    categories: RefCell<Vec<String>>,
    series: RefCell<Vec<ChartSeries>>,
    previous: RefCell<Vec<Vec<f64>>>,
    progress: Cell<f64>,
    legend: Cell<bool>,
    animated: Cell<bool>,
    hot: Cell<Option<ChartHit>>,
    format: RefCell<FormatFn>,
    transition: RefCell<Option<Transition>>,
}

/// Chart (custom-drawn control).
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let chart = Chart::builder()
///     .kind(ChartKind::Bar)
///     .title("Quarterly sales")
///     .categories(&["T1", "T2", "T3", "T4"])
///     .series(ChartSeries::new("2025", &[12.0, 18.0, 9.0, 21.0]))
///     .series(ChartSeries::new("2026", &[15.0, 22.0, 14.0, 25.0]))
///     .bounds(10, 10, 480, 300)
///     .parent(window)
///     .build()?;
/// chart.on_point_click(|series, index| println!("series {series}, category {index}"));
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Chart {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<ChartState>,
}

impl Widget for Chart {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for Chart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chart")
            .field("kind", &self.state.kind.get())
            .field("title", &self.state.title.borrow())
            .finish()
    }
}

/// Builder for [`Chart`].
#[derive(Clone, Debug)]
pub struct ChartBuilder {
    opts: ControlOptions,
    kind: ChartKind,
    title: String,
    categories: Vec<String>,
    series: Vec<ChartSeries>,
    legend: bool,
    animated: bool,
}

impl ChartBuilder {
    builder_common!();

    /// Chart type (default: lines).
    pub fn kind(mut self, kind: ChartKind) -> Self {
        self.kind = kind;
        self
    }

    /// Title (empty = no title).
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Category labels (horizontal axis or pie slices).
    pub fn categories(mut self, categories: &[&str]) -> Self {
        self.categories = categories.iter().map(|c| c.to_string()).collect();
        self
    }

    /// Adds a series.
    pub fn series(mut self, series: ChartSeries) -> Self {
        self.series.push(series);
        self
    }

    /// Shows the legend (default: yes).
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// Animates entry and data changes (default: yes; respects the Windows setting to disable animations).
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Creates the chart.
    pub fn build(self) -> GuiResult<Chart> {
        let core = self.opts.create("Chart", PANEL_CLASS, 0, 0, false, None)?;
        let state = Rc::new(ChartState {
            kind: Cell::new(self.kind),
            title: RefCell::new(self.title),
            categories: RefCell::new(self.categories),
            series: RefCell::new(Vec::new()),
            previous: RefCell::new(Vec::new()),
            progress: Cell::new(1.0),
            legend: Cell::new(self.legend),
            animated: Cell::new(self.animated),
            hot: Cell::new(None),
            format: RefCell::new(Rc::new(plain_number)),
            transition: RefCell::new(None),
        });
        let chart = Chart { core, state };
        chart.install();
        chart.set_series(self.series);
        Ok(chart)
    }
}

impl Chart {
    /// Starts building (default size 400×260).
    pub fn builder() -> ChartBuilder {
        ChartBuilder {
            opts: ControlOptions::new(400, 260),
            kind: ChartKind::Line,
            title: String::new(),
            categories: Vec::new(),
            series: Vec::new(),
            legend: true,
            animated: true,
        }
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
            if let (Some(chart), Event::MouseMove(mouse)) = (me.upgrade(), event) {
                let hot = chart.hit_test(mouse.position);
                if chart.state.hot.replace(hot) != hot {
                    chart.core.invalidate();
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseLeave, move |_| {
            if let Some(chart) = me.upgrade() {
                if chart.state.hot.replace(None).is_some() {
                    chart.core.invalidate();
                }
            }
        });
    }

    fn weak(&self) -> WeakChart {
        WeakChart { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    fn geometry(&self) -> Geometry {
        let size = self.client_size();
        let font = Font::system();
        let format = self.state.format.borrow().clone();
        let (title, categories, series, previous) = (
            self.state.title.borrow(),
            self.state.categories.borrow(),
            self.state.series.borrow(),
            self.state.previous.borrow(),
        );
        let input = LayoutInput {
            size,
            kind: self.state.kind.get(),
            title: &title,
            categories: &categories,
            series: &series,
            legend: self.state.legend.get(),
            progress: self.state.progress.get(),
            previous: &previous,
        };
        // Mede os textos num DC de memória (as mesmas unidades lógicas da pintura).
        let mut result = Geometry::default();
        let _ = crate::graphics::Bitmap::render(1, 1, |canvas| {
            let measure = |text: &str| canvas.measure_text(text, &font).width;
            result = layout(&input, &measure, &*format);
        });
        result
    }

    /// Starts transitioning from previous to current values.
    fn transition_from(&self, previous: Vec<Vec<f64>>) {
        if let Some(old) = self.state.transition.borrow_mut().take() {
            old.cancel();
        }
        *self.state.previous.borrow_mut() = previous;
        if !self.state.animated.get() || !self.core.is_alive() {
            self.state.progress.set(1.0);
            self.refresh();
            return;
        }
        self.state.progress.set(0.0);
        let me = self.weak();
        match motion::animate(Duration::from_millis(450), Easing::EaseOut, move |t| {
            if let Some(chart) = me.upgrade() {
                chart.state.progress.set(t);
                chart.core.invalidate();
            }
        }) {
            Ok(transition) => *self.state.transition.borrow_mut() = Some(transition),
            Err(_) => self.state.progress.set(1.0),
        }
        self.refresh();
    }

    /// Currently displayed values (intermediate during transitions).
    fn shown_values(&self) -> Vec<Vec<f64>> {
        let progress = self.state.progress.get();
        let previous = self.state.previous.borrow();
        self.state
            .series
            .borrow()
            .iter()
            .enumerate()
            .map(|(s, series)| {
                series
                    .values
                    .iter()
                    .enumerate()
                    .map(|(i, &to)| {
                        let from = previous.get(s).and_then(|v| v.get(i)).copied().unwrap_or(0.0);
                        from + (to - from) * progress
                    })
                    .collect()
            })
            .collect()
    }

    fn refresh(&self) {
        let summary = self.summary();
        let title = self.state.title.borrow().clone();
        let _ = self.set_accessible_name(if title.is_empty() { "Chart" } else { &title });
        let _ = self.set_accessible_description(&summary);
        self.core.invalidate();
    }

    /// Current type.
    pub fn kind(&self) -> ChartKind {
        self.state.kind.get()
    }

    /// Changes the type (transitioning from zero).
    pub fn set_kind(&self, kind: ChartKind) {
        self.state.kind.set(kind);
        self.state.hot.set(None);
        self.transition_from(Vec::new());
    }

    /// Changes the title.
    pub fn set_title(&self, title: &str) {
        *self.state.title.borrow_mut() = title.to_string();
        self.refresh();
    }

    /// Changes categories.
    pub fn set_categories(&self, categories: &[&str]) {
        *self.state.categories.borrow_mut() = categories.iter().map(|c| c.to_string()).collect();
        self.refresh();
    }

    /// Replaces all series (transitioning from displayed values).
    pub fn set_series(&self, series: Vec<ChartSeries>) {
        let previous = self.shown_values();
        *self.state.series.borrow_mut() = series;
        self.state.hot.set(None);
        self.transition_from(previous);
    }

    /// Adds a series and returns its index.
    pub fn add_series(&self, series: ChartSeries) -> usize {
        let previous = self.shown_values();
        let index = {
            let mut list = self.state.series.borrow_mut();
            list.push(series);
            list.len() - 1
        };
        self.transition_from(previous);
        index
    }

    /// Changes a series' values with a transition. Invalid indices are ignored.
    pub fn set_values(&self, series: usize, values: &[f64]) {
        let previous = self.shown_values();
        if let Some(target) = self.state.series.borrow_mut().get_mut(series) {
            target.values = values.to_vec();
        } else {
            return;
        }
        self.transition_from(previous);
    }

    /// Current series (final values).
    pub fn series(&self) -> Vec<ChartSeries> {
        self.state.series.borrow().clone()
    }

    /// Current categories.
    pub fn categories(&self) -> Vec<String> {
        self.state.categories.borrow().clone()
    }

    /// Shows or hides the legend.
    pub fn set_legend(&self, legend: bool) {
        self.state.legend.set(legend);
        self.refresh();
    }

    /// Enables or disables animated transitions.
    pub fn set_animated(&self, animated: bool) {
        self.state.animated.set(animated);
        if !animated {
            if let Some(transition) = self.state.transition.borrow_mut().take() {
                transition.finish();
            }
        }
    }

    /// `true` while a data transition is in progress.
    pub fn is_animating(&self) -> bool {
        self.state.transition.borrow().as_ref().is_some_and(Transition::is_running)
    }

    /// Value formatting for axes, tooltips and accessible summary (default: integer or up to two decimals). Example:
    /// `chart.set_value_format(|v| Locale::user().format_currency(v))`.
    pub fn set_value_format(&self, format: impl Fn(f64) -> String + 'static) {
        *self.state.format.borrow_mut() = Rc::new(format);
        self.refresh();
    }

    /// Color of series (or pie slice) `index`.
    pub fn color_of(&self, index: usize) -> Color {
        let series = self.state.series.borrow();
        let own = if self.kind() == ChartKind::Pie { None } else { series.get(index).and_then(|s| s.color) };
        own.unwrap_or(CHART_PALETTE[index % CHART_PALETTE.len()])
    }

    /// Item (series, index) under a logical point, if any.
    pub fn hit_test(&self, at: Point) -> Option<ChartHit> {
        hit(&self.geometry(), self.kind(), at.x as f64, at.y as f64)
    }

    /// Highlighted item under the cursor.
    pub fn hot_item(&self) -> Option<ChartHit> {
        self.state.hot.get()
    }

    /// Logical center of an item (line point, bar or slice center), for tests and annotation positioning.
    pub fn item_center(&self, series: usize, index: usize) -> Option<Point> {
        let g = self.geometry();
        let round = |x: f64, y: f64| Point::new(x.round() as i32, y.round() as i32);
        if let Some(((cx, cy), radius, slices)) = &g.pie {
            let (start, sweep) = *slices.get(index)?;
            let middle = (start + sweep / 2.0).to_radians();
            return Some(round(cx + radius * 0.85 * middle.cos(), cy + radius * 0.85 * middle.sin()));
        }
        if let Some(&(_, _, x, y, w, h)) = g.bars.iter().find(|b| b.0 == series && b.1 == index) {
            return Some(round(x + w / 2.0, y + h / 2.0));
        }
        g.points.get(series)?.get(index).map(|&(x, y)| round(x, y))
    }

    /// Data description (also the control's accessible description).
    pub fn summary(&self) -> String {
        let format = self.state.format.borrow().clone();
        let categories = self.state.categories.borrow();
        let label = |i: usize| categories.get(i).cloned().unwrap_or_else(|| format!("{}", i + 1));
        let series = self.state.series.borrow();
        let mut parts = Vec::new();
        for (s, item) in series.iter().enumerate() {
            if self.kind() == ChartKind::Pie && s > 0 {
                break;
            }
            let values: Vec<String> =
                item.values.iter().enumerate().map(|(i, v)| format!("{} {}", label(i), format(*v))).collect();
            parts.push(format!("{}: {}", item.name, values.join(", ")));
        }
        parts.join("; ")
    }

    /// Called when an item is clicked: receives (series, index); pie series is 0.
    pub fn on_point_click(&self, f: impl Fn(usize, usize) + 'static) -> HandlerId {
        let me = self.weak();
        self.core.on(EventKind::MouseUp, move |event| {
            if let (Some(chart), Event::MouseUp(mouse)) = (me.upgrade(), event) {
                if let Some((series, index)) = chart.hit_test(mouse.position) {
                    f(series, index);
                }
            }
        })
    }
}

#[derive(Clone)]
struct WeakChart {
    core: std::rc::Weak<WidgetCore>,
    state: std::rc::Weak<ChartState>,
}

impl WeakChart {
    fn upgrade(&self) -> Option<Chart> {
        Some(Chart { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

/// Control background and text colors (custom, window theme or system).
fn colors(core: &WidgetCore) -> (Color, Color) {
    let theme = lookup(crate::core::native::root_of(core.hwnd())).and_then(|root| root.theme.borrow().clone());
    let background = core.effective_background().unwrap_or_else(|| {
        // SAFETY: consulta de cor do sistema.
        Color::from_colorref(unsafe { crate::ffi::user32::GetSysColor(crate::ffi::user32::COLOR_WINDOW) })
    });
    let foreground = core.foreground.get().or_else(|| theme.map(|t| t.foreground)).unwrap_or_else(|| {
        if background.luminance() < 0.5 {
            Color::WHITE
        } else {
            Color::rgb(26, 26, 26)
        }
    });
    (background, foreground)
}

fn paint(core: &WidgetCore, state: &ChartState, canvas: &Canvas) {
    let (background, foreground) = colors(core);
    canvas.clear(background);
    let size = canvas.size();
    let font = Font::system();
    let bold = Font::system().bold().scaled(1.15);
    let format = state.format.borrow().clone();
    let measure = |text: &str| canvas.measure_text(text, &font).width;
    let (title, categories, series, previous) =
        (state.title.borrow(), state.categories.borrow(), state.series.borrow(), state.previous.borrow());
    let kind = state.kind.get();
    let input = LayoutInput {
        size,
        kind,
        title: &title,
        categories: &categories,
        series: &series,
        legend: state.legend.get(),
        progress: state.progress.get(),
        previous: &previous,
    };
    let g = layout(&input, &measure, &*format);
    let grid = foreground.mix(background, 0.85);
    let muted = foreground.mix(background, 0.35);
    if let Some(rect) = g.title {
        canvas.text_in_rect(&title, rect, &bold, foreground, TextAlign::Center);
    }
    let color_of = |i: usize| {
        let own = if kind == ChartKind::Pie { None } else { series.get(i).and_then(|s| s.color) };
        own.unwrap_or(CHART_PALETTE[i % CHART_PALETTE.len()])
    };
    let hot = state.hot.get();
    // Eixo de valores e grade.
    for (value, y) in &g.ticks {
        let y = y.round() as i32;
        canvas.line(Point::new(g.plot.x, y), Point::new(g.plot.x + g.plot.width, y), &Pen::new(grid, 1));
        let label = format(*value);
        let width = measure(&label);
        canvas.text_line(&label, Rect::new(g.plot.x - width - 6, y - LINE / 2, width + 2, LINE), &font, muted);
    }
    if !g.ticks.is_empty() {
        let y = g.baseline.round() as i32;
        canvas.line(Point::new(g.plot.x, y), Point::new(g.plot.x + g.plot.width, y), &Pen::new(muted, 1));
    }
    // Rótulos das categorias.
    if kind != ChartKind::Pie && !g.columns.is_empty() {
        let band = g.plot.width / g.columns.len() as i32;
        for (i, x) in g.columns.iter().enumerate() {
            let label = categories.get(i).cloned().unwrap_or_default();
            let rect = Rect::new(*x as i32 - band / 2, g.plot.y + g.plot.height + 4, band, LINE);
            canvas.text_in_rect(&label, rect, &font, muted, TextAlign::Center);
        }
    }
    let _ = canvas.with_gdiplus(|graphics| {
        let painter = VectorPainter { graphics };
        match kind {
            ChartKind::Bar | ChartKind::StackedBar => {
                for &(s, i, x, y, w, h) in &g.bars {
                    if h <= 0.0 {
                        continue;
                    }
                    let mut color = color_of(s);
                    if hot == Some((s, i)) {
                        color = color.mix(Color::WHITE, 0.25);
                    }
                    painter.fill(&[Figure::rect(x, y, w, h)], false, &Paint::Solid(color), 1.0);
                }
            }
            ChartKind::Line | ChartKind::Area => {
                let line_of = |points: &[(f64, f64)]| {
                    let mut line = Figure::new(points[0]);
                    line.segments.extend(points[1..].iter().map(|&p| Segment::Line(p)));
                    line
                };
                // Todas as áreas primeiro, para nenhuma cobrir a linha de outra série.
                if kind == ChartKind::Area {
                    for (s, points) in g.points.iter().enumerate().filter(|(_, p)| p.len() > 1) {
                        let mut area = line_of(points);
                        area.segments.push(Segment::Line((points[points.len() - 1].0, g.baseline)));
                        area.segments.push(Segment::Line((points[0].0, g.baseline)));
                        area.closed = true;
                        painter.fill(&[area], false, &Paint::Solid(Color { a: 70, ..color_of(s) }), 1.0);
                    }
                }
                for (s, points) in g.points.iter().enumerate() {
                    if points.is_empty() {
                        continue;
                    }
                    let color = color_of(s);
                    let line = line_of(points);
                    let stroke = Stroke { width: 2.0, join: LineJoin::Round, cap: LineCap::Round, ..Stroke::default() };
                    painter.stroke(&[line], &stroke, &Paint::Solid(color), 1.0);
                    for (i, &(x, y)) in points.iter().enumerate() {
                        let radius = if hot == Some((s, i)) { 5.5 } else { 3.5 };
                        painter.fill(&[Figure::ellipse(x, y, radius, radius)], false, &Paint::Solid(color), 1.0);
                        painter.stroke(
                            &[Figure::ellipse(x, y, radius, radius)],
                            &Stroke { width: 1.5, ..Stroke::default() },
                            &Paint::Solid(background),
                            1.0,
                        );
                    }
                }
            }
            ChartKind::Pie => {
                if let Some(((cx, cy), radius, slices)) = &g.pie {
                    for (i, &(start, sweep)) in slices.iter().enumerate() {
                        if sweep <= 0.0 {
                            continue;
                        }
                        let (mut x, mut y) = (*cx, *cy);
                        if hot == Some((0, i)) {
                            let middle = (start + sweep / 2.0).to_radians();
                            x += 6.0 * middle.cos();
                            y += 6.0 * middle.sin();
                        }
                        let slice = if sweep >= 359.99 {
                            Figure::ellipse(x, y, *radius, *radius)
                        } else {
                            Figure::pie(x, y, *radius, start, sweep)
                        };
                        painter.fill(std::slice::from_ref(&slice), false, &Paint::Solid(color_of(i)), 1.0);
                        painter.stroke(
                            &[slice],
                            &Stroke { width: 1.5, ..Stroke::default() },
                            &Paint::Solid(background),
                            1.0,
                        );
                    }
                }
            }
        }
    });
    // Percentuais na pizza.
    if let Some(((cx, cy), radius, slices)) = &g.pie {
        for &(start, sweep) in slices {
            if sweep < 18.0 {
                continue;
            }
            let middle = (start + sweep / 2.0).to_radians();
            let (x, y) = (cx + radius * 0.62 * middle.cos(), cy + radius * 0.62 * middle.sin());
            let text = format!("{}%", plain_number((sweep / 360.0 * 1000.0).round() / 10.0));
            let rect = Rect::new(x as i32 - 30, y as i32 - LINE / 2, 60, LINE);
            canvas.text_in_rect(&text, rect, &font, Color::WHITE, TextAlign::Center);
        }
    }
    // Legenda.
    for (i, (label, swatch)) in g.legend.iter().enumerate() {
        canvas.fill_rect(*swatch, color_of(i));
        let rect = Rect::new(swatch.x + SWATCH + 6, swatch.y + SWATCH / 2 - LINE / 2, measure(label) + 4, LINE);
        canvas.text_line(label, rect, &font, foreground);
    }
    // Dica do item sob o cursor.
    if let Some((s, i)) = hot {
        let name = series.get(s).map(|x| x.name.clone()).unwrap_or_default();
        let category = categories.get(i).cloned().unwrap_or_else(|| format!("{}", i + 1));
        let value = series.get(s).and_then(|x| x.values.get(i)).copied().unwrap_or(0.0);
        let text = if kind == ChartKind::Pie {
            format!("{category}: {}", format(value))
        } else {
            format!("{name} — {category}: {}", format(value))
        };
        let width = measure(&text) + 12;
        let rect = Rect::new((size.width - width - PAD).max(0), PAD, width, LINE + 4);
        canvas.round_rect(rect, 4, Some(&Pen::new(grid, 1)), Some(background.mix(foreground, 0.08)));
        canvas.text_centered(&text, rect, &font, foreground);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(kind: ChartKind, categories: &'a [String], series: &'a [ChartSeries]) -> LayoutInput<'a> {
        LayoutInput {
            size: Size::new(400, 300),
            kind,
            title: "",
            categories,
            series,
            legend: false,
            progress: 1.0,
            previous: &[],
        }
    }

    fn measure(text: &str) -> i32 {
        text.chars().count() as i32 * 7
    }

    #[test]
    fn nice_scales_and_numbers() {
        assert_eq!(nice_scale(0.0, 23.0, 5), (0.0, 25.0, 5.0));
        assert_eq!(nice_scale(-3.0, 8.0, 5), (-5.0, 10.0, 2.5));
        assert_eq!(nice_scale(0.0, 0.0, 5), (0.0, 1.0, 0.2));
        assert_eq!(nice_scale(5.0, 5.0, 5), (0.0, 5.0, 1.0));
        assert_eq!(plain_number(3.0), "3");
        assert_eq!(plain_number(2.5), "2.5");
        assert_eq!(plain_number(1.239), "1.24");
    }

    #[test]
    fn bars_lines_and_stacks() {
        let categories: Vec<String> = ["A", "B"].iter().map(|s| s.to_string()).collect();
        let series = [ChartSeries::new("x", &[10.0, -5.0]), ChartSeries::new("y", &[20.0, 5.0])];
        let g = layout(&input(ChartKind::Bar, &categories, &series), &measure, &plain_number);
        assert_eq!(g.bars.len(), 4);
        let (_, _, _, top, _, height) = g.bars[2]; // série x, categoria B (negativa)
        assert!((top - g.baseline).abs() < 1e-9 && height > 0.0, "barra negativa desce do zero");
        assert!(g.bars[1].4 > 0.0 && g.bars[1].3 < g.bars[0].3, "série y mais alta que x em A");
        assert_eq!(hit(&g, ChartKind::Bar, g.bars[1].2 + 2.0, g.bars[1].3 + 2.0), Some((1, 0)));
        assert_eq!(hit(&g, ChartKind::Bar, 1.0, 1.0), None);

        let stacked = layout(&input(ChartKind::StackedBar, &categories, &series), &measure, &plain_number);
        let (a, b) = (stacked.bars[0], stacked.bars[1]);
        assert!((a.3 - (b.3 + b.5)).abs() < 1e-9, "y empilhada sobre x");
        assert_eq!(stacked.ticks.last().unwrap().0, 30.0, "eixo vai até a soma");

        let line = layout(&input(ChartKind::Line, &categories, &series), &measure, &plain_number);
        assert_eq!(line.points.len(), 2);
        let (x, y) = line.points[0][1];
        assert_eq!(hit(&line, ChartKind::Line, x + 3.0, y - 3.0), Some((0, 1)));

        let animated = LayoutInput { progress: 0.5, ..input(ChartKind::Line, &categories, &series) };
        let half = layout(&animated, &measure, &plain_number);
        assert!(
            half.ticks.last().unwrap().0 < line.ticks.last().unwrap().0,
            "na metade da transição (a partir do zero) os valores e o eixo são menores"
        );
    }

    #[test]
    fn pie_slices_and_legend() {
        let categories: Vec<String> = ["Um", "Dois", "Três"].iter().map(|s| s.to_string()).collect();
        let series = [ChartSeries::new("p", &[1.0, 1.0, 2.0])];
        let input = LayoutInput { legend: true, title: "Título", ..input(ChartKind::Pie, &categories, &series) };
        let g = layout(&input, &measure, &plain_number);
        let (_, radius, slices) = g.pie.clone().unwrap();
        assert!(radius > 50.0);
        assert_eq!(slices, vec![(-90.0, 90.0), (0.0, 90.0), (90.0, 180.0)]);
        assert_eq!(g.legend.len(), 3);
        assert!(g.title.is_some());
        let ((cx, cy), ..) = g.pie.clone().unwrap();
        assert_eq!(hit(&g, ChartKind::Pie, cx + 10.0, cy - 10.0), Some((0, 0)), "quadrante superior direito");
        assert_eq!(hit(&g, ChartKind::Pie, cx - 10.0, cy + 10.0), Some((0, 2)));
        assert_eq!(hit(&g, ChartKind::Pie, cx + radius + 5.0, cy), None);
    }
}
