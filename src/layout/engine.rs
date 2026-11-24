//! Pure layout calculation: specifications to rectangles.
//!
//! No window access; algorithms testable independently.

use crate::utilities::{Rect, Size, Thickness};

/// Item alignment within allocated space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Align {
    /// Start (left/top).
    Start,
    /// Center.
    Center,
    /// End (right/bottom).
    End,
    /// Fills available space.
    #[default]
    Stretch,
}

/// Linear layout orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// Side-by-side items.
    Horizontal,
    /// Stacked items.
    Vertical,
}

/// Resolved item specification for layout engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemSpec {
    /// Preferred size.
    pub size: Size,
    /// Minimum size.
    pub min: Size,
    /// Maximum size (`i32::MAX` = unlimited).
    pub max: Size,
    /// Outer margin.
    pub margin: Thickness,
    /// Proportional remaining-space weight (0 = fixed size).
    pub weight: f32,
    /// Horizontal alignment.
    pub h_align: Align,
    /// Vertical alignment.
    pub v_align: Align,
}

impl Default for ItemSpec {
    fn default() -> Self {
        Self {
            size: Size::new(0, 0),
            min: Size::new(0, 0),
            max: Size::new(i32::MAX, i32::MAX),
            margin: Thickness::default(),
            weight: 0.0,
            h_align: Align::Stretch,
            v_align: Align::Stretch,
        }
    }
}

impl ItemSpec {
    /// Fixed-size item.
    pub fn fixed(width: i32, height: i32) -> Self {
        Self { size: Size::new(width, height), ..Default::default() }
    }

    fn clamp_w(&self, w: i32) -> i32 {
        w.max(self.min.width).min(self.max.width.max(self.min.width)).max(0)
    }

    fn clamp_h(&self, h: i32) -> i32 {
        h.max(self.min.height).min(self.max.height.max(self.min.height)).max(0)
    }
}

/// Positions a length within an interval according to alignment.
fn place(start: i32, available: i32, preferred: i32, align: Align, clamp: impl Fn(i32) -> i32) -> (i32, i32) {
    match align {
        Align::Stretch => (start, clamp(available)),
        Align::Start => (start, clamp(preferred.min(available.max(0)))),
        Align::Center => {
            let len = clamp(preferred.min(available.max(0)));
            (start + (available - len) / 2, len)
        }
        Align::End => {
            let len = clamp(preferred.min(available.max(0)));
            (start + available - len, len)
        }
    }
}

/// Fits an item in a cell respecting margin, limits and alignment.
pub fn fit(cell: Rect, item: &ItemSpec) -> Rect {
    let inner = cell.inset(item.margin);
    let (x, width) = place(inner.x, inner.width, item.size.width, item.h_align, |w| item.clamp_w(w));
    let (y, height) = place(inner.y, inner.height, item.size.height, item.v_align, |h| item.clamp_h(h));
    Rect::new(x, y, width, height)
}

/// Distributes `available`: fixed items get preferred size, weighted items share remainder.
fn distribute(available: i32, items: &[(i32, i32, i32, f32)]) -> Vec<i32> {
    // Cada item: (preferido, mínimo, máximo, peso).
    let fixed: i32 = items.iter().filter(|i| i.3 <= 0.0).map(|i| i.0.clamp(i.1, i.2.max(i.1))).sum();
    let total_weight: f32 = items.iter().map(|i| i.3.max(0.0)).sum();
    let mut remaining = (available - fixed).max(0);
    let mut weight_left = total_weight;
    items
        .iter()
        .map(|&(preferred, min, max, weight)| {
            if weight <= 0.0 {
                preferred.clamp(min, max.max(min))
            } else {
                let share =
                    if weight_left > 0.0 { (remaining as f32 * weight / weight_left).round() as i32 } else { 0 };
                let len = share.clamp(min, max.max(min));
                remaining = (remaining - len).max(0);
                weight_left -= weight;
                len
            }
        })
        .collect()
}

/// Linear layout stacking items along orientation.
pub fn stack(area: Rect, orientation: Orientation, spacing: i32, items: &[ItemSpec]) -> Vec<Rect> {
    if items.is_empty() {
        return Vec::new();
    }
    let gaps = spacing * (items.len() as i32 - 1);
    let vertical = orientation == Orientation::Vertical;
    let main_margin = |i: &ItemSpec| if vertical { i.margin.vertical() } else { i.margin.horizontal() };
    let margins: i32 = items.iter().map(main_margin).sum();
    let available = (if vertical { area.height } else { area.width }) - gaps - margins;
    let specs: Vec<(i32, i32, i32, f32)> = items
        .iter()
        .map(|i| {
            if vertical {
                (i.size.height, i.min.height, i.max.height, i.weight)
            } else {
                (i.size.width, i.min.width, i.max.width, i.weight)
            }
        })
        .collect();
    let lengths = distribute(available, &specs);
    let mut cursor = if vertical { area.y } else { area.x };
    items
        .iter()
        .zip(lengths)
        .map(|(item, length)| {
            let extent = length + main_margin(item);
            let cell = if vertical {
                Rect::new(area.x, cursor, area.width, extent)
            } else {
                Rect::new(cursor, area.y, extent, area.height)
            };
            cursor += extent + spacing;
            // No eixo principal o comprimento já foi decidido: o item ocupa a célula.
            let mut spec = *item;
            if vertical {
                spec.v_align = Align::Stretch;
                spec.min.height = 0;
                spec.max.height = i32::MAX;
            } else {
                spec.h_align = Align::Stretch;
                spec.min.width = 0;
                spec.max.width = i32::MAX;
            }
            fit(cell, &spec)
        })
        .collect()
}

/// Grid row/column definition.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Track {
    /// Fixed logical size.
    Fixed(i32),
    /// Largest single-span item size in the track.
    Auto,
    /// Proportional remaining-space fraction.
    Star(f32),
}

/// Grid item position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridCell {
    /// Zero-based starting row.
    pub row: usize,
    /// Zero-based starting column.
    pub column: usize,
    /// Row span, minimum 1.
    pub row_span: usize,
    /// Column span, minimum 1.
    pub column_span: usize,
}

fn resolve_tracks(available: i32, gap: i32, tracks: &[Track], auto_size: impl Fn(usize) -> i32) -> Vec<i32> {
    let gaps = gap * (tracks.len().saturating_sub(1)) as i32;
    let specs: Vec<(i32, i32, i32, f32)> = tracks
        .iter()
        .enumerate()
        .map(|(index, track)| match *track {
            Track::Fixed(size) => (size.max(0), 0, i32::MAX, 0.0),
            Track::Auto => (auto_size(index).max(0), 0, i32::MAX, 0.0),
            Track::Star(weight) => (0, 0, i32::MAX, weight.max(0.0001)),
        })
        .collect();
    distribute(available - gaps, &specs)
}

/// Grid with fixed, automatic and proportional tracks.
pub fn grid(
    area: Rect,
    columns: &[Track],
    rows: &[Track],
    column_gap: i32,
    row_gap: i32,
    items: &[(ItemSpec, GridCell)],
) -> Vec<Rect> {
    if columns.is_empty() || rows.is_empty() {
        return vec![Rect::default(); items.len()];
    }
    let widths = resolve_tracks(area.width, column_gap, columns, |column| {
        items
            .iter()
            .filter(|(_, cell)| cell.column == column && cell.column_span <= 1)
            .map(|(item, _)| item.size.width + item.margin.horizontal())
            .max()
            .unwrap_or(0)
    });
    let heights = resolve_tracks(area.height, row_gap, rows, |row| {
        items
            .iter()
            .filter(|(_, cell)| cell.row == row && cell.row_span <= 1)
            .map(|(item, _)| item.size.height + item.margin.vertical())
            .max()
            .unwrap_or(0)
    });
    let offsets = |sizes: &[i32], start: i32, gap: i32| -> Vec<i32> {
        let mut position = start;
        sizes
            .iter()
            .map(|size| {
                let current = position;
                position += size + gap;
                current
            })
            .collect()
    };
    let xs = offsets(&widths, area.x, column_gap);
    let ys = offsets(&heights, area.y, row_gap);
    items
        .iter()
        .map(|(item, cell)| {
            let column = cell.column.min(columns.len() - 1);
            let row = cell.row.min(rows.len() - 1);
            let last_column = (column + cell.column_span.max(1) - 1).min(columns.len() - 1);
            let last_row = (row + cell.row_span.max(1) - 1).min(rows.len() - 1);
            let bounds = Rect::from_ltrb(
                xs[column],
                ys[row],
                xs[last_column] + widths[last_column],
                ys[last_row] + heights[last_row],
            );
            fit(bounds, item)
        })
        .collect()
}

/// Flow layout: left to right, wrapping when full.
pub fn flow(area: Rect, h_gap: i32, v_gap: i32, items: &[ItemSpec]) -> Vec<Rect> {
    let mut x = area.x;
    let mut y = area.y;
    let mut line_height = 0;
    items
        .iter()
        .map(|item| {
            let width = item.clamp_w(item.size.width) + item.margin.horizontal();
            let height = item.clamp_h(item.size.height) + item.margin.vertical();
            if x > area.x && x + width > area.right() {
                x = area.x;
                y += line_height + v_gap;
                line_height = 0;
            }
            let rect = Rect::new(
                x + item.margin.left,
                y + item.margin.top,
                width - item.margin.horizontal(),
                height - item.margin.vertical(),
            );
            x += width + h_gap;
            line_height = line_height.max(height);
            rect
        })
        .collect()
}

/// Side an item occupies in [`dock`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Dock {
    /// Left, filling remaining height.
    Left,
    /// Top, filling remaining width.
    Top,
    /// Right.
    Right,
    /// Bottom.
    Bottom,
    /// Fills remaining space.
    Fill,
}

/// Dock layout: each item consumes a strip of remaining area in order.
pub fn dock(area: Rect, items: &[(ItemSpec, Dock)]) -> Vec<Rect> {
    let mut remaining = area;
    items
        .iter()
        .map(|(item, side)| {
            let width = (item.clamp_w(item.size.width) + item.margin.horizontal()).min(remaining.width);
            let height = (item.clamp_h(item.size.height) + item.margin.vertical()).min(remaining.height);
            let cell = match side {
                Dock::Left => {
                    let cell = Rect::new(remaining.x, remaining.y, width, remaining.height);
                    remaining = Rect::new(remaining.x + width, remaining.y, remaining.width - width, remaining.height);
                    cell
                }
                Dock::Right => {
                    let cell = Rect::new(remaining.right() - width, remaining.y, width, remaining.height);
                    remaining = Rect::new(remaining.x, remaining.y, remaining.width - width, remaining.height);
                    cell
                }
                Dock::Top => {
                    let cell = Rect::new(remaining.x, remaining.y, remaining.width, height);
                    remaining =
                        Rect::new(remaining.x, remaining.y + height, remaining.width, remaining.height - height);
                    cell
                }
                Dock::Bottom => {
                    let cell = Rect::new(remaining.x, remaining.bottom() - height, remaining.width, height);
                    remaining = Rect::new(remaining.x, remaining.y, remaining.width, remaining.height - height);
                    cell
                }
                Dock::Fill => remaining,
            };
            cell.inset(item.margin)
        })
        .collect()
}

/// Parent edges anchoring an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchors {
    /// Retains left-edge distance.
    pub left: bool,
    /// Retains top-edge distance.
    pub top: bool,
    /// Retains right-edge distance.
    pub right: bool,
    /// Retains bottom-edge distance.
    pub bottom: bool,
}

impl Anchors {
    /// Top-left, without moving/growing.
    pub const TOP_LEFT: Anchors = Anchors { left: true, top: true, right: false, bottom: false };
    /// Top-right.
    pub const TOP_RIGHT: Anchors = Anchors { left: false, top: true, right: true, bottom: false };
    /// Bottom-left.
    pub const BOTTOM_LEFT: Anchors = Anchors { left: true, top: false, right: false, bottom: true };
    /// Bottom-right.
    pub const BOTTOM_RIGHT: Anchors = Anchors { left: false, top: false, right: true, bottom: true };
    /// Horizontal stretch anchored at top.
    pub const TOP_STRETCH: Anchors = Anchors { left: true, top: true, right: true, bottom: false };
    /// Horizontal stretch anchored at bottom.
    pub const BOTTOM_STRETCH: Anchors = Anchors { left: true, top: false, right: true, bottom: true };
    /// Stretches both ways.
    pub const ALL: Anchors = Anchors { left: true, top: true, right: true, bottom: true };
}

fn anchor_axis(start: i32, length: i32, design: i32, current: i32, near: bool, far: bool) -> (i32, i32) {
    let delta = current - design;
    match (near, far) {
        (true, true) => (start, (length + delta).max(0)),
        (false, true) => (start + delta, length),
        (true, false) => (start, length),
        (false, false) => (start + delta / 2, length),
    }
}

/// Anchor layout: recalculates original rectangle as parent changes from `design_size` to `current_size`.
pub fn anchor(original: Rect, design_size: Size, current_size: Size, anchors: Anchors) -> Rect {
    let (x, width) =
        anchor_axis(original.x, original.width, design_size.width, current_size.width, anchors.left, anchors.right);
    let (y, height) =
        anchor_axis(original.y, original.height, design_size.height, current_size.height, anchors.top, anchors.bottom);
    Rect::new(x, y, width, height)
}

/// Splits area into two panels with a divider.
///
/// `position` is first-panel size, clamped by `min_first`/`min_second`. Returns (first panel, divider, second panel).
pub fn split(
    area: Rect,
    orientation: Orientation,
    position: i32,
    splitter: i32,
    min_first: i32,
    min_second: i32,
) -> (Rect, Rect, Rect) {
    let total = if orientation == Orientation::Horizontal { area.width } else { area.height };
    let max_first = (total - splitter - min_second).max(0);
    let first = position.clamp(min_first.min(max_first), max_first);
    let second = (total - splitter - first).max(0);
    if orientation == Orientation::Horizontal {
        (
            Rect::new(area.x, area.y, first, area.height),
            Rect::new(area.x + first, area.y, splitter, area.height),
            Rect::new(area.x + first + splitter, area.y, second, area.height),
        )
    } else {
        (
            Rect::new(area.x, area.y, area.width, first),
            Rect::new(area.x, area.y + first, area.width, splitter),
            Rect::new(area.x, area.y + first + splitter, area.width, second),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weighted(weight: f32) -> ItemSpec {
        ItemSpec { weight, ..Default::default() }
    }

    #[test]
    fn vertical_stack_fixed_and_weighted() {
        let area = Rect::new(10, 10, 200, 300);
        let rects =
            stack(area, Orientation::Vertical, 10, &[ItemSpec::fixed(50, 30), weighted(1.0), ItemSpec::fixed(50, 40)]);
        assert_eq!(rects[0], Rect::new(10, 10, 200, 30));
        assert_eq!(rects[1], Rect::new(10, 50, 200, 210));
        assert_eq!(rects[2], Rect::new(10, 270, 200, 40));
    }

    #[test]
    fn horizontal_stack_proportional_split_uses_all_space() {
        let rects = stack(Rect::new(0, 0, 301, 20), Orientation::Horizontal, 0, &[weighted(1.0), weighted(2.0)]);
        assert_eq!(rects[0].width + rects[1].width, 301);
        assert_eq!(rects[0].width, 100);
        assert_eq!(rects[1].x, 100);
    }

    #[test]
    fn stack_respects_min_max_margin_and_alignment() {
        let mut item = weighted(1.0);
        item.max = Size::new(i32::MAX, 50);
        let mut aligned = ItemSpec::fixed(40, 20);
        aligned.h_align = Align::End;
        aligned.margin = Thickness::all(5);
        let rects = stack(Rect::new(0, 0, 100, 200), Orientation::Vertical, 0, &[item, aligned]);
        assert_eq!(rects[0].height, 50);
        assert_eq!(rects[1], Rect::new(55, 55, 40, 20));
        assert!(stack(Rect::new(0, 0, 10, 10), Orientation::Vertical, 4, &[]).is_empty());
    }

    #[test]
    fn stack_never_produces_negative_sizes() {
        let rects =
            stack(Rect::new(0, 0, 10, 10), Orientation::Vertical, 8, &[weighted(1.0), weighted(1.0), weighted(1.0)]);
        assert!(rects.iter().all(|r| r.width >= 0 && r.height >= 0));
    }

    #[test]
    fn grid_tracks_fixed_auto_star_and_span() {
        let label = ItemSpec::fixed(80, 20);
        let field = ItemSpec::default();
        let items = [
            (label, GridCell { row: 0, column: 0, row_span: 1, column_span: 1 }),
            (field, GridCell { row: 0, column: 1, row_span: 1, column_span: 1 }),
            (field, GridCell { row: 1, column: 0, row_span: 1, column_span: 2 }),
        ];
        let rects = grid(
            Rect::new(0, 0, 300, 100),
            &[Track::Auto, Track::Star(1.0)],
            &[Track::Fixed(30), Track::Star(1.0)],
            10,
            5,
            &items,
        );
        assert_eq!(rects[0], Rect::new(0, 0, 80, 30));
        assert_eq!(rects[1], Rect::new(90, 0, 210, 30));
        assert_eq!(rects[2], Rect::new(0, 35, 300, 65));
        assert_eq!(grid(Rect::new(0, 0, 10, 10), &[], &[], 0, 0, &items).len(), 3);
    }

    #[test]
    fn flow_wraps_lines() {
        let items = [ItemSpec::fixed(60, 20), ItemSpec::fixed(60, 30), ItemSpec::fixed(60, 20)];
        let rects = flow(Rect::new(0, 0, 130, 100), 5, 5, &items);
        assert_eq!(rects[0], Rect::new(0, 0, 60, 20));
        assert_eq!(rects[1], Rect::new(65, 0, 60, 30));
        assert_eq!(rects[2], Rect::new(0, 35, 60, 20));
    }

    #[test]
    fn dock_consumes_remaining_area() {
        let items = [
            (ItemSpec::fixed(0, 30), Dock::Top),
            (ItemSpec::fixed(0, 20), Dock::Bottom),
            (ItemSpec::fixed(100, 0), Dock::Left),
            (ItemSpec::default(), Dock::Fill),
        ];
        let rects = dock(Rect::new(0, 0, 400, 300), &items);
        assert_eq!(rects[0], Rect::new(0, 0, 400, 30));
        assert_eq!(rects[1], Rect::new(0, 280, 400, 20));
        assert_eq!(rects[2], Rect::new(0, 30, 100, 250));
        assert_eq!(rects[3], Rect::new(100, 30, 300, 250));
    }

    #[test]
    fn anchors_follow_parent_resize() {
        let original = Rect::new(300, 250, 80, 30);
        let design = Size::new(400, 300);
        let bigger = Size::new(500, 400);
        assert_eq!(anchor(original, design, bigger, Anchors::BOTTOM_RIGHT), Rect::new(400, 350, 80, 30));
        assert_eq!(anchor(original, design, bigger, Anchors::TOP_LEFT), original);
        assert_eq!(anchor(Rect::new(10, 10, 380, 200), design, bigger, Anchors::ALL), Rect::new(10, 10, 480, 300));
        let none = Anchors { left: false, top: false, right: false, bottom: false };
        assert_eq!(anchor(original, design, bigger, none), Rect::new(350, 300, 80, 30));
    }

    #[test]
    fn split_clamps_position() {
        let (a, s, b) = split(Rect::new(0, 0, 400, 100), Orientation::Horizontal, 150, 6, 50, 50);
        assert_eq!((a.width, s.x, s.width, b.x, b.width), (150, 150, 6, 156, 244));
        let (a, _, b) = split(Rect::new(0, 0, 400, 100), Orientation::Horizontal, 1000, 6, 50, 50);
        assert_eq!((a.width, b.width), (344, 50));
        let (a, _, _) = split(Rect::new(0, 0, 400, 100), Orientation::Vertical, 0, 6, 20, 20);
        assert_eq!(a.height, 20);
    }
}
