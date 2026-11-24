//! Layout managers automatically positioning native controls.
//!
//! Attach with `set_layout` to a window/panel; reapplied on resize/DPI changes. All values are logical (96 DPI).
//! Nestable: `add` accepts controls (`&button`) and layouts.
//!
//! ```no_run
//! # use arqrustui::prelude::*;
//! # fn demo(window: &Window, name: &TextBox, ok: &Button, cancel: &Button) {
//! let buttons = StackLayout::horizontal().spacing(SPACING);
//! buttons.add_with(ok, LayoutParams::new().width(90));
//! buttons.add_with(cancel, LayoutParams::new().width(90));
//!
//! let root = StackLayout::vertical().spacing(SPACING).padding(Thickness::all(MARGIN));
//! root.add(name);
//! root.add_with(Spacer, LayoutParams::new().weight(1.0));
//! root.add_with(buttons, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT).align_h(Align::End));
//! window.set_layout(root);
//! # }
//! ```

pub mod engine;
mod status;
pub use status::StatusBarLayout;

/// Recommended logical spacing between neighboring controls: 8, following Windows related-control guidelines.
pub const SPACING: i32 = 8;

/// Recommended logical margin between window/panel edge and content.
pub const MARGIN: i32 = 12;

pub use engine::{Align, Anchors, Dock, GridCell, ItemSpec, Orientation, Track};

use crate::core::native;
use crate::core::widget::{Widget, WidgetCore};
use crate::ffi::user32::WS_VISIBLE;
use crate::utilities::{Rect, Size, Thickness};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Anything distributing items within an area.
pub trait Layout {
    /// Positions items in logical container-relative `area`.
    fn arrange(&self, area: Rect);

    /// Preferred combined size when nested.
    fn preferred_size(&self) -> Size {
        Size::default()
    }
}

/// Layout item target.
#[derive(Clone)]
#[non_exhaustive]
pub enum LayoutTarget {
    /// Native control.
    Widget(Rc<WidgetCore>),
    /// Nested layout.
    Nested(Rc<dyn Layout>),
    /// Empty space.
    Space,
}

impl LayoutTarget {
    fn place(&self, rect: Rect) {
        match self {
            LayoutTarget::Widget(core) => core.set_bounds(rect),
            LayoutTarget::Nested(layout) => layout.arrange(rect),
            LayoutTarget::Space => {}
        }
    }

    fn natural_size(&self) -> Size {
        match self {
            LayoutTarget::Widget(core) => core.bounds().size(),
            LayoutTarget::Nested(layout) => layout.preferred_size(),
            LayoutTarget::Space => Size::default(),
        }
    }

    /// Hidden/destroyed controls occupy no space.
    fn is_collapsed(&self) -> bool {
        match self {
            LayoutTarget::Widget(core) => !core.is_alive() || native::style(core.hwnd()) & WS_VISIBLE == 0,
            _ => false,
        }
    }
}

/// Layout item conversion, implemented by `&Widget`, all layouts and [`Spacer`].
pub trait IntoLayoutTarget {
    /// Converts to layout target.
    fn into_target(self) -> LayoutTarget;
}

impl<W: Widget> IntoLayoutTarget for &W {
    fn into_target(self) -> LayoutTarget {
        LayoutTarget::Widget(self.core().clone())
    }
}

/// Empty item for pushing with `weight` or reserving space.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spacer;

impl IntoLayoutTarget for Spacer {
    fn into_target(self) -> LayoutTarget {
        LayoutTarget::Space
    }
}

/// Item parameters within a layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutParams {
    width: Option<i32>,
    height: Option<i32>,
    min: Option<Size>,
    max: Option<Size>,
    margin: Thickness,
    weight: f32,
    h_align: Option<Align>,
    v_align: Option<Align>,
}

impl LayoutParams {
    /// Defaults: current control size, no margin, stretched cross-axis.
    pub fn new() -> Self {
        Self::default()
    }

    /// Preferred width.
    pub fn width(mut self, width: i32) -> Self {
        self.width = Some(width);
        self
    }

    /// Preferred height.
    pub fn height(mut self, height: i32) -> Self {
        self.height = Some(height);
        self
    }

    /// Preferred size.
    pub fn size(self, width: i32, height: i32) -> Self {
        self.width(width).height(height)
    }

    /// Minimum size.
    pub fn min_size(mut self, width: i32, height: i32) -> Self {
        self.min = Some(Size::new(width, height));
        self
    }

    /// Maximum size.
    pub fn max_size(mut self, width: i32, height: i32) -> Self {
        self.max = Some(Size::new(width, height));
        self
    }

    /// Outer margin.
    pub fn margin(mut self, margin: Thickness) -> Self {
        self.margin = margin;
        self
    }

    /// Proportional remaining-space weight (0 = fixed).
    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Horizontal cell alignment.
    pub fn align_h(mut self, align: Align) -> Self {
        self.h_align = Some(align);
        self
    }

    /// Vertical cell alignment.
    pub fn align_v(mut self, align: Align) -> Self {
        self.v_align = Some(align);
        self
    }

    fn resolve(&self, natural: Size) -> ItemSpec {
        ItemSpec {
            size: Size::new(self.width.unwrap_or(natural.width), self.height.unwrap_or(natural.height)),
            min: self.min.unwrap_or_default(),
            max: self.max.unwrap_or(Size::new(i32::MAX, i32::MAX)),
            margin: self.margin,
            weight: self.weight,
            h_align: self.h_align.unwrap_or(Align::Stretch),
            v_align: self.v_align.unwrap_or(Align::Stretch),
        }
    }
}

struct Entry {
    target: LayoutTarget,
    params: LayoutParams,
    natural: Size,
}

impl Entry {
    fn new(target: LayoutTarget, params: LayoutParams) -> Self {
        let natural = target.natural_size();
        Self { target, params, natural }
    }

    fn spec(&self) -> ItemSpec {
        let natural = match &self.target {
            LayoutTarget::Nested(layout) => layout.preferred_size(),
            _ => self.natural,
        };
        self.params.resolve(natural)
    }
}

fn visible(entries: &[Entry]) -> Vec<&Entry> {
    entries.iter().filter(|e| !e.target.is_collapsed()).collect()
}

macro_rules! layout_handle {
    ($name:ident, $inner:ident) => {
        impl IntoLayoutTarget for $name {
            fn into_target(self) -> LayoutTarget {
                LayoutTarget::Nested(self.0)
            }
        }

        impl IntoLayoutTarget for &$name {
            fn into_target(self) -> LayoutTarget {
                LayoutTarget::Nested(self.0.clone())
            }
        }

        impl Layout for $name {
            fn arrange(&self, area: Rect) {
                self.0.arrange(area)
            }

            fn preferred_size(&self) -> Size {
                self.0.preferred_size()
            }
        }
    };
}

// ---- StackLayout -----------------------------------------------------------------

struct StackInner {
    orientation: Orientation,
    spacing: Cell<i32>,
    padding: Cell<Thickness>,
    entries: RefCell<Vec<Entry>>,
}

/// Stacks items in a horizontal row or vertical column.
///
/// Unweighted items use preferred size; weighted items proportionally share remainder.
#[derive(Clone)]
pub struct StackLayout(Rc<StackInner>);

impl StackLayout {
    fn with(orientation: Orientation) -> Self {
        StackLayout(Rc::new(StackInner {
            orientation,
            spacing: Cell::new(0),
            padding: Cell::new(Thickness::default()),
            entries: RefCell::new(Vec::new()),
        }))
    }

    /// Vertical stack (StackPanel).
    pub fn vertical() -> Self {
        Self::with(Orientation::Vertical)
    }

    /// Horizontal stack (StackPanel).
    pub fn horizontal() -> Self {
        Self::with(Orientation::Horizontal)
    }

    /// Item spacing.
    pub fn spacing(self, spacing: i32) -> Self {
        self.0.spacing.set(spacing);
        self
    }

    /// Padding between container edge and items.
    pub fn padding(self, padding: Thickness) -> Self {
        self.0.padding.set(padding);
        self
    }

    /// Adds an item with defaults.
    pub fn add(&self, item: impl IntoLayoutTarget) -> &Self {
        self.add_with(item, LayoutParams::new())
    }

    /// Adds an item with explicit parameters.
    pub fn add_with(&self, item: impl IntoLayoutTarget, params: LayoutParams) -> &Self {
        self.0.entries.borrow_mut().push(Entry::new(item.into_target(), params));
        self
    }

    /// Removes items without destroying controls.
    pub fn clear(&self) {
        self.0.entries.borrow_mut().clear();
    }
}

impl Layout for StackInner {
    fn arrange(&self, area: Rect) {
        let entries = self.entries.borrow();
        let shown = visible(&entries);
        let specs: Vec<ItemSpec> = shown.iter().map(|e| e.spec()).collect();
        let rects = engine::stack(area.inset(self.padding.get()), self.orientation, self.spacing.get(), &specs);
        for (entry, rect) in shown.iter().zip(rects) {
            entry.target.place(rect);
        }
    }

    fn preferred_size(&self) -> Size {
        let entries = self.entries.borrow();
        let shown = visible(&entries);
        let padding = self.padding.get();
        let gaps = self.spacing.get() * (shown.len().saturating_sub(1)) as i32;
        let (mut main, mut cross) = (gaps, 0);
        for entry in &shown {
            let spec = entry.spec();
            let (w, h) = (spec.size.width + spec.margin.horizontal(), spec.size.height + spec.margin.vertical());
            if self.orientation == Orientation::Vertical {
                main += h;
                cross = cross.max(w);
            } else {
                main += w;
                cross = cross.max(h);
            }
        }
        if self.orientation == Orientation::Vertical {
            Size::new(cross + padding.horizontal(), main + padding.vertical())
        } else {
            Size::new(main + padding.horizontal(), cross + padding.vertical())
        }
    }
}

layout_handle!(StackLayout, StackInner);

// ---- GridLayout ------------------------------------------------------------------

struct GridInner {
    columns: RefCell<Vec<Track>>,
    rows: RefCell<Vec<Track>>,
    column_gap: Cell<i32>,
    row_gap: Cell<i32>,
    padding: Cell<Thickness>,
    entries: RefCell<Vec<(Entry, GridCell)>>,
}

/// Row/column grid with fixed, automatic or proportional tracks.
#[derive(Clone)]
pub struct GridLayout(Rc<GridInner>);

impl GridLayout {
    /// Creates grid with given columns/rows.
    pub fn new(columns: &[Track], rows: &[Track]) -> Self {
        GridLayout(Rc::new(GridInner {
            columns: RefCell::new(columns.to_vec()),
            rows: RefCell::new(rows.to_vec()),
            column_gap: Cell::new(0),
            row_gap: Cell::new(0),
            padding: Cell::new(Thickness::default()),
            entries: RefCell::new(Vec::new()),
        }))
    }

    /// Column/row spacing.
    pub fn gaps(self, column_gap: i32, row_gap: i32) -> Self {
        self.0.column_gap.set(column_gap);
        self.0.row_gap.set(row_gap);
        self
    }

    /// Padding.
    pub fn padding(self, padding: Thickness) -> Self {
        self.0.padding.set(padding);
        self
    }

    /// Adds item at (`row`, `column`).
    pub fn add(&self, item: impl IntoLayoutTarget, row: usize, column: usize) -> &Self {
        self.add_span(item, row, column, 1, 1, LayoutParams::new())
    }

    /// Adds item with explicit parameters at (`row`, `column`).
    pub fn add_with(&self, item: impl IntoLayoutTarget, row: usize, column: usize, params: LayoutParams) -> &Self {
        self.add_span(item, row, column, 1, 1, params)
    }

    /// Adds an item spanning rows/columns.
    pub fn add_span(
        &self,
        item: impl IntoLayoutTarget,
        row: usize,
        column: usize,
        row_span: usize,
        column_span: usize,
        params: LayoutParams,
    ) -> &Self {
        let cell = GridCell { row, column, row_span: row_span.max(1), column_span: column_span.max(1) };
        self.0.entries.borrow_mut().push((Entry::new(item.into_target(), params), cell));
        self
    }

    /// Appends a grid row and returns index.
    pub fn add_row(&self, track: Track) -> usize {
        let mut rows = self.0.rows.borrow_mut();
        rows.push(track);
        rows.len() - 1
    }
}

impl GridInner {
    fn specs(&self) -> Vec<(ItemSpec, GridCell)> {
        self.entries
            .borrow()
            .iter()
            .filter(|(e, _)| !e.target.is_collapsed())
            .map(|(e, cell)| (e.spec(), *cell))
            .collect()
    }
}

impl Layout for GridInner {
    fn arrange(&self, area: Rect) {
        let specs = self.specs();
        let rects = engine::grid(
            area.inset(self.padding.get()),
            &self.columns.borrow(),
            &self.rows.borrow(),
            self.column_gap.get(),
            self.row_gap.get(),
            &specs,
        );
        let entries = self.entries.borrow();
        let shown = entries.iter().filter(|(e, _)| !e.target.is_collapsed());
        for ((entry, _), rect) in shown.zip(rects) {
            entry.target.place(rect);
        }
    }

    fn preferred_size(&self) -> Size {
        let specs = self.specs();
        let measure = |tracks: &[Track], gap: i32, horizontal: bool| -> i32 {
            let gaps = gap * (tracks.len().saturating_sub(1)) as i32;
            gaps + tracks
                .iter()
                .enumerate()
                .map(|(index, track)| match track {
                    Track::Fixed(size) => *size,
                    _ => specs
                        .iter()
                        .filter(|(_, c)| if horizontal { c.column == index } else { c.row == index })
                        .map(|(s, _)| {
                            if horizontal {
                                s.size.width + s.margin.horizontal()
                            } else {
                                s.size.height + s.margin.vertical()
                            }
                        })
                        .max()
                        .unwrap_or(0),
                })
                .sum::<i32>()
        };
        let padding = self.padding.get();
        Size::new(
            measure(&self.columns.borrow(), self.column_gap.get(), true) + padding.horizontal(),
            measure(&self.rows.borrow(), self.row_gap.get(), false) + padding.vertical(),
        )
    }
}

layout_handle!(GridLayout, GridInner);

// ---- FlowLayout ------------------------------------------------------------------

struct FlowInner {
    h_gap: Cell<i32>,
    v_gap: Cell<i32>,
    padding: Cell<Thickness>,
    entries: RefCell<Vec<Entry>>,
}

/// Flows left to right, wrapping as needed.
#[derive(Clone)]
pub struct FlowLayout(Rc<FlowInner>);

impl FlowLayout {
    /// Creates flow layout.
    pub fn new() -> Self {
        FlowLayout(Rc::new(FlowInner {
            h_gap: Cell::new(0),
            v_gap: Cell::new(0),
            padding: Cell::new(Thickness::default()),
            entries: RefCell::new(Vec::new()),
        }))
    }

    /// Horizontal/vertical spacing.
    pub fn gaps(self, horizontal: i32, vertical: i32) -> Self {
        self.0.h_gap.set(horizontal);
        self.0.v_gap.set(vertical);
        self
    }

    /// Padding.
    pub fn padding(self, padding: Thickness) -> Self {
        self.0.padding.set(padding);
        self
    }

    /// Adds item at current size.
    pub fn add(&self, item: impl IntoLayoutTarget) -> &Self {
        self.add_with(item, LayoutParams::new())
    }

    /// Adds an item with explicit parameters.
    pub fn add_with(&self, item: impl IntoLayoutTarget, params: LayoutParams) -> &Self {
        self.0.entries.borrow_mut().push(Entry::new(item.into_target(), params));
        self
    }
}

impl Default for FlowLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for FlowInner {
    fn arrange(&self, area: Rect) {
        let entries = self.entries.borrow();
        let shown = visible(&entries);
        let specs: Vec<ItemSpec> = shown.iter().map(|e| e.spec()).collect();
        let rects = engine::flow(area.inset(self.padding.get()), self.h_gap.get(), self.v_gap.get(), &specs);
        for (entry, rect) in shown.iter().zip(rects) {
            entry.target.place(rect);
        }
    }

    fn preferred_size(&self) -> Size {
        let entries = self.entries.borrow();
        let shown = visible(&entries);
        let padding = self.padding.get();
        let gaps = self.h_gap.get() * (shown.len().saturating_sub(1)) as i32;
        let width: i32 = shown.iter().map(|e| e.spec().size.width + e.spec().margin.horizontal()).sum();
        let height = shown.iter().map(|e| e.spec().size.height + e.spec().margin.vertical()).max().unwrap_or(0);
        Size::new(width + gaps + padding.horizontal(), height + padding.vertical())
    }
}

layout_handle!(FlowLayout, FlowInner);

// ---- DockLayout ------------------------------------------------------------------

struct DockInner {
    padding: Cell<Thickness>,
    entries: RefCell<Vec<(Entry, Dock)>>,
}

/// Docks items to container edges; `Dock::Fill` takes remainder.
#[derive(Clone)]
pub struct DockLayout(Rc<DockInner>);

impl DockLayout {
    /// Creates dock layout.
    pub fn new() -> Self {
        DockLayout(Rc::new(DockInner { padding: Cell::new(Thickness::default()), entries: RefCell::new(Vec::new()) }))
    }

    /// Padding.
    pub fn padding(self, padding: Thickness) -> Self {
        self.0.padding.set(padding);
        self
    }

    /// Adds a docked item at current size.
    pub fn add(&self, item: impl IntoLayoutTarget, side: Dock) -> &Self {
        self.add_with(item, side, LayoutParams::new())
    }

    /// Adds a docked item with explicit parameters.
    pub fn add_with(&self, item: impl IntoLayoutTarget, side: Dock, params: LayoutParams) -> &Self {
        self.0.entries.borrow_mut().push((Entry::new(item.into_target(), params), side));
        self
    }
}

impl Default for DockLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for DockInner {
    fn arrange(&self, area: Rect) {
        let entries = self.entries.borrow();
        let shown: Vec<&(Entry, Dock)> = entries.iter().filter(|(e, _)| !e.target.is_collapsed()).collect();
        let specs: Vec<(ItemSpec, Dock)> = shown.iter().map(|(e, side)| (e.spec(), *side)).collect();
        let rects = engine::dock(area.inset(self.padding.get()), &specs);
        for ((entry, _), rect) in shown.iter().zip(rects) {
            entry.target.place(rect);
        }
    }
}

layout_handle!(DockLayout, DockInner);

// ---- AbsoluteLayout --------------------------------------------------------------

struct AbsoluteInner {
    entries: RefCell<Vec<(LayoutTarget, Rect)>>,
}

/// Positions items in fixed logical rectangles scaled by DPI.
#[derive(Clone)]
pub struct AbsoluteLayout(Rc<AbsoluteInner>);

impl AbsoluteLayout {
    /// Creates absolute layout.
    pub fn new() -> Self {
        AbsoluteLayout(Rc::new(AbsoluteInner { entries: RefCell::new(Vec::new()) }))
    }

    /// Adds item at rectangle relative to layout origin.
    pub fn add(&self, item: impl IntoLayoutTarget, bounds: Rect) -> &Self {
        self.0.entries.borrow_mut().push((item.into_target(), bounds));
        self
    }
}

impl Default for AbsoluteLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for AbsoluteInner {
    fn arrange(&self, area: Rect) {
        for (target, bounds) in self.entries.borrow().iter() {
            target.place(Rect::new(area.x + bounds.x, area.y + bounds.y, bounds.width, bounds.height));
        }
    }

    fn preferred_size(&self) -> Size {
        let entries = self.entries.borrow();
        Size::new(
            entries.iter().map(|(_, r)| r.right()).max().unwrap_or(0),
            entries.iter().map(|(_, r)| r.bottom()).max().unwrap_or(0),
        )
    }
}

layout_handle!(AbsoluteLayout, AbsoluteInner);

// ---- AnchorLayout ----------------------------------------------------------------

struct AnchorInner {
    design_size: Size,
    entries: RefCell<Vec<(LayoutTarget, Rect, Anchors)>>,
}

/// Retains distances to selected container edges like Windows Forms `Anchor`.
#[derive(Clone)]
pub struct AnchorLayout(Rc<AnchorInner>);

impl AnchorLayout {
    /// Creates layout; `design_size` is the container size used to design original positions.
    pub fn new(design_size: Size) -> Self {
        AnchorLayout(Rc::new(AnchorInner { design_size, entries: RefCell::new(Vec::new()) }))
    }

    /// Anchors a control from current position/size.
    pub fn add<W: Widget>(&self, widget: &W, anchors: Anchors) -> &Self {
        let bounds = widget.bounds();
        self.0.entries.borrow_mut().push((widget.into_target(), bounds, anchors));
        self
    }

    /// Anchors control/layout from explicit original rectangle.
    pub fn add_at(&self, item: impl IntoLayoutTarget, original: Rect, anchors: Anchors) -> &Self {
        self.0.entries.borrow_mut().push((item.into_target(), original, anchors));
        self
    }
}

impl Layout for AnchorInner {
    fn arrange(&self, area: Rect) {
        for (target, original, anchors) in self.entries.borrow().iter() {
            let rect = engine::anchor(*original, self.design_size, area.size(), *anchors);
            target.place(Rect::new(area.x + rect.x, area.y + rect.y, rect.width, rect.height));
        }
    }

    fn preferred_size(&self) -> Size {
        self.design_size
    }
}

layout_handle!(AnchorLayout, AnchorInner);

// ---- SplitLayout -----------------------------------------------------------------

struct SplitInner {
    orientation: Orientation,
    ratio: Cell<f32>,
    gap: Cell<i32>,
    min_first: Cell<i32>,
    min_second: Cell<i32>,
    first: RefCell<LayoutTarget>,
    second: RefCell<LayoutTarget>,
}

/// Proportionally divides area between two items.
///
/// For draggable dividers use [`SplitContainer`](crate::controls::SplitContainer).
#[derive(Clone)]
pub struct SplitLayout(Rc<SplitInner>);

impl SplitLayout {
    fn with(orientation: Orientation, first: LayoutTarget, second: LayoutTarget) -> Self {
        SplitLayout(Rc::new(SplitInner {
            orientation,
            ratio: Cell::new(0.5),
            gap: Cell::new(6),
            min_first: Cell::new(0),
            min_second: Cell::new(0),
            first: RefCell::new(first),
            second: RefCell::new(second),
        }))
    }

    /// Two side-by-side items.
    pub fn horizontal(first: impl IntoLayoutTarget, second: impl IntoLayoutTarget) -> Self {
        Self::with(Orientation::Horizontal, first.into_target(), second.into_target())
    }

    /// Two vertically stacked items.
    pub fn vertical(first: impl IntoLayoutTarget, second: impl IntoLayoutTarget) -> Self {
        Self::with(Orientation::Vertical, first.into_target(), second.into_target())
    }

    /// First-item area fraction, 0.0 to 1.0.
    pub fn ratio(self, ratio: f32) -> Self {
        self.0.ratio.set(ratio.clamp(0.0, 1.0));
        self
    }

    /// Space between items.
    pub fn gap(self, gap: i32) -> Self {
        self.0.gap.set(gap.max(0));
        self
    }

    /// First/second minimum sizes.
    pub fn min_sizes(self, first: i32, second: i32) -> Self {
        self.0.min_first.set(first);
        self.0.min_second.set(second);
        self
    }
}

impl Layout for SplitInner {
    fn arrange(&self, area: Rect) {
        let total = if self.orientation == Orientation::Horizontal { area.width } else { area.height };
        let position = ((total - self.gap.get()) as f32 * self.ratio.get()).round() as i32;
        let (first, _, second) = engine::split(
            area,
            self.orientation,
            position,
            self.gap.get(),
            self.min_first.get(),
            self.min_second.get(),
        );
        self.first.borrow().place(first);
        self.second.borrow().place(second);
    }
}

layout_handle!(SplitLayout, SplitInner);
