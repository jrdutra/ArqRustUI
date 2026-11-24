//! Navigation trail (`BreadcrumbBar`): `Home › Documents › Projects`, collapsing leading items into "…" when space is
//! limited.

use super::common::{builder_common, ControlOptions};
use super::modern::{enabled, focused, palette, publish};
use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleRole};
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::graphics::{Canvas, Font, TextAlign};
use crate::i18n::{library_text, LibraryText};
use crate::input::Key;
use crate::utilities::{Point, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SEPARATOR: i32 = 18;
const ITEM_PAD: i32 = 6;
const ELLIPSIS: &str = "…";

/// Visible part of the trail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// "…" containing collapsed items.
    Ellipsis,
    /// Item `index`.
    Item(usize),
}

/// Positions: (part, x, width) for each visible part. `widths` includes the margin.
pub(crate) fn arrange(widths: &[i32], available: i32, ellipsis: i32) -> Vec<(Part, i32, i32)> {
    let n = widths.len();
    if n == 0 {
        return Vec::new();
    }
    let total: i32 = widths.iter().sum::<i32>() + SEPARATOR * (n as i32 - 1);
    let first = if total <= available {
        0
    } else {
        // Do fim para o começo, reservando o "…"; o último item sempre aparece.
        let mut used = ellipsis + SEPARATOR + widths[n - 1];
        let mut first = n - 1;
        while first > 0 && used + widths[first - 1] + SEPARATOR <= available {
            first -= 1;
            used += widths[first] + SEPARATOR;
        }
        first
    };
    let mut parts = Vec::new();
    let mut x = 0;
    if first > 0 {
        parts.push((Part::Ellipsis, x, ellipsis));
        x += ellipsis + SEPARATOR;
    }
    for (index, &width) in widths.iter().enumerate().skip(first) {
        parts.push((Part::Item(index), x, width));
        x += width + SEPARATOR;
    }
    parts
}

struct BreadcrumbState {
    items: RefCell<Vec<String>>,
    hot: Cell<Option<Part>>,
    focus: Cell<Option<Part>>,
    /// Geometry from the latest drawing (for clicks and accessibility).
    parts: RefCell<Vec<(Part, i32, i32)>>,
}

/// Navigation trail with one item per level. Clicking an item calls [`on_item_click`](Self::on_item_click) with its
/// index (applications usually navigate and shorten the trail with [`truncate`](Self::truncate)). The last item is
/// current. When space is limited, leading items move into an "…" menu. Arrow keys move focus; Enter or Space invokes
/// an item.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let path = BreadcrumbBar::builder().items(&["Home", "Documents", "Projects"]).parent(window).build()?;
/// let trail = path.clone();
/// path.on_item_click(move |index| trail.truncate(index + 1));
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct BreadcrumbBar {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<BreadcrumbState>,
}

impl Widget for BreadcrumbBar {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for BreadcrumbBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BreadcrumbBar").field("items", &self.state.items.borrow()).finish()
    }
}

/// Builder for [`BreadcrumbBar`].
#[derive(Clone, Debug)]
pub struct BreadcrumbBarBuilder {
    opts: ControlOptions,
    items: Vec<String>,
}

impl BreadcrumbBarBuilder {
    builder_common!();

    /// Items, from the highest level to the current level.
    pub fn items(mut self, items: &[&str]) -> Self {
        self.items = items.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Creates the trail.
    pub fn build(self) -> GuiResult<BreadcrumbBar> {
        let core = self.opts.create("BreadcrumbBar", PANEL_CLASS, 0, 0, true, None)?;
        let state = Rc::new(BreadcrumbState {
            items: RefCell::new(self.items),
            hot: Cell::new(None),
            focus: Cell::new(None),
            parts: RefCell::new(Vec::new()),
        });
        let bar = BreadcrumbBar { core, state };
        bar.install();
        bar.update_layout();
        Ok(bar)
    }
}

impl BreadcrumbBar {
    /// Starts building (default size 400×32).
    pub fn builder() -> BreadcrumbBarBuilder {
        BreadcrumbBarBuilder { opts: ControlOptions::new(400, 32), items: Vec::new() }
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
        self.core.on(EventKind::Resize, move |_| {
            if let Some(bar) = me.upgrade() {
                bar.update_layout();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseMove, move |event| {
            if let (Some(bar), Event::MouseMove(m)) = (me.upgrade(), event) {
                let hot = bar.part_at(m.position);
                if bar.state.hot.replace(hot) != hot {
                    bar.core.invalidate();
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseLeave, move |_| {
            if let Some(bar) = me.upgrade() {
                bar.state.hot.set(None);
                bar.core.invalidate();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseUp, move |event| {
            if let (Some(bar), Event::MouseUp(m)) = (me.upgrade(), event) {
                if let Some(part) = bar.part_at(m.position) {
                    bar.activate(part);
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::KeyDown, move |event| {
            if let (Some(bar), Event::KeyDown(key)) = (me.upgrade(), event) {
                let parts: Vec<Part> = bar.state.parts.borrow().iter().map(|p| p.0).collect();
                let current = bar.state.focus.get().and_then(|f| parts.iter().position(|p| *p == f));
                let next = match key.key {
                    Key::RIGHT => current.map_or(Some(0), |i| Some((i + 1).min(parts.len().saturating_sub(1)))),
                    Key::LEFT => current.map_or(Some(0), |i| Some(i.saturating_sub(1))),
                    Key::HOME => Some(0),
                    Key::END => Some(parts.len().saturating_sub(1)),
                    Key::ENTER | Key::SPACE => {
                        if let Some(part) = bar.state.focus.get() {
                            bar.activate(part);
                        }
                        None
                    }
                    _ => None,
                };
                if let Some(next) = next.filter(|_| !parts.is_empty()) {
                    bar.state.focus.set(Some(parts[next]));
                    bar.core.invalidate();
                }
            }
        });
        for kind in [EventKind::Focus, EventKind::Blur] {
            let me = self.weak();
            self.core.on(kind, move |_| {
                if let Some(bar) = me.upgrade() {
                    if bar.state.focus.get().is_none() {
                        let last = bar.state.parts.borrow().last().map(|p| p.0);
                        bar.state.focus.set(last);
                    }
                    bar.core.invalidate();
                }
            });
        }
        publish(
            &self.core,
            &self.state,
            |_, state| {
                let root = AccessibleItem::new(0, AccessibleRole::ToolBar, &library_text(LibraryText::Breadcrumb));
                let items = state.items.borrow();
                let children = state
                    .parts
                    .borrow()
                    .iter()
                    .map(|&(part, x, width)| {
                        let (id, name) = match part {
                            Part::Ellipsis => (1, library_text(LibraryText::MoreOptions)),
                            Part::Item(i) => (10 + i as u32, items.get(i).cloned().unwrap_or_default()),
                        };
                        AccessibleItem::new(id, AccessibleRole::Button, &name)
                            .bounds(Rect::new(x, 0, width, 32))
                            .focus(state.focus.get() == Some(part))
                            .invokable()
                    })
                    .collect();
                (root, children)
            },
            |core, state, id, action| {
                if action != AccessibleAction::Invoke {
                    return false;
                }
                let part = if id == 1 { Part::Ellipsis } else { Part::Item(id.wrapping_sub(10) as usize) };
                let bar = BreadcrumbBar { core: core.clone(), state: state.clone() };
                if let Part::Item(i) = part {
                    if i >= bar.len() {
                        return false;
                    }
                }
                bar.activate(part);
                true
            },
        );
    }

    fn weak(&self) -> WeakBreadcrumb {
        WeakBreadcrumb { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    fn update_layout(&self) {
        let font = Font::system();
        let items = self.state.items.borrow().clone();
        let mut widths = Vec::new();
        let mut ellipsis = 0;
        let _ = crate::graphics::Bitmap::render(1, 1, |canvas| {
            widths = items.iter().map(|i| canvas.measure_text(i, &font).width + 2 * ITEM_PAD).collect();
            ellipsis = canvas.measure_text(ELLIPSIS, &font).width + 2 * ITEM_PAD;
        });
        let parts = arrange(&widths, self.client_size().width, ellipsis);
        *self.state.parts.borrow_mut() = parts;
        crate::accessibility::raise_accessible_event(self, 0, crate::accessibility::AccessibleEvent::StructureChanged);
        self.core.invalidate();
    }

    fn part_at(&self, at: Point) -> Option<Part> {
        let height = self.client_size().height;
        if at.y < 0 || at.y >= height {
            return None;
        }
        self.state.parts.borrow().iter().find(|(_, x, w)| at.x >= *x && at.x < x + w).map(|p| p.0)
    }

    fn activate(&self, part: Part) {
        self.state.focus.set(Some(part));
        self.core.invalidate();
        match part {
            Part::Item(index) => {
                self.core.emit(&Event::ItemActivated(index));
            }
            Part::Ellipsis => self.show_hidden_menu(),
        }
    }

    /// Indices of items hidden in "…", in order.
    pub fn hidden_items(&self) -> Vec<usize> {
        let first = self.state.parts.borrow().iter().find_map(|p| match p.0 {
            Part::Item(i) => Some(i),
            Part::Ellipsis => None,
        });
        (0..first.unwrap_or(0)).collect()
    }

    fn show_hidden_menu(&self) {
        let Ok(menu) = crate::menus::ContextMenu::new() else { return };
        let items = self.state.items.borrow().clone();
        for index in self.hidden_items() {
            let core = Rc::downgrade(&self.core);
            let _ = menu.add_item(&items[index], move || {
                if let Some(core) = core.upgrade() {
                    core.emit(&Event::ItemActivated(index));
                }
            });
        }
        let x = self.state.parts.borrow().first().map_or(0, |p| p.1);
        menu.show(self, Point::new(x, self.client_size().height));
    }

    /// Current items.
    pub fn items(&self) -> Vec<String> {
        self.state.items.borrow().clone()
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.state.items.borrow().len()
    }

    /// `true` when there are no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Replaces the items.
    pub fn set_items(&self, items: &[&str]) {
        *self.state.items.borrow_mut() = items.iter().map(|s| s.to_string()).collect();
        self.state.focus.set(None);
        self.update_layout();
    }

    /// Appends a level (the new current item).
    pub fn push(&self, item: &str) {
        self.state.items.borrow_mut().push(item.to_string());
        self.update_layout();
    }

    /// Keeps only the first `len` items.
    pub fn truncate(&self, len: usize) {
        self.state.items.borrow_mut().truncate(len);
        self.state.focus.set(None);
        self.update_layout();
    }

    /// Logical rectangle of item `index`, if visible.
    pub fn item_bounds(&self, index: usize) -> Option<Rect> {
        let height = self.client_size().height;
        self.state.parts.borrow().iter().find(|p| p.0 == Part::Item(index)).map(|&(_, x, w)| Rect::new(x, 0, w, height))
    }

    /// Called when an item is clicked or invoked with the keyboard; receives its index.
    pub fn on_item_click(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        self.core.on(EventKind::ItemActivated, move |event| {
            if let Event::ItemActivated(index) = event {
                f(*index)
            }
        })
    }
}

#[derive(Clone)]
struct WeakBreadcrumb {
    core: std::rc::Weak<WidgetCore>,
    state: std::rc::Weak<BreadcrumbState>,
}

impl WeakBreadcrumb {
    fn upgrade(&self) -> Option<BreadcrumbBar> {
        Some(BreadcrumbBar { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

fn paint(core: &WidgetCore, state: &BreadcrumbState, canvas: &Canvas) {
    let p = palette(core);
    let size = canvas.size();
    canvas.clear(p.background);
    let font = Font::system();
    let items = state.items.borrow();
    let parts = state.parts.borrow();
    let last = items.len().checked_sub(1);
    let active = enabled(core);
    for (n, &(part, x, width)) in parts.iter().enumerate() {
        let rect = Rect::new(x, 2, width, size.height - 4);
        if state.hot.get() == Some(part) && active {
            canvas.round_rect(rect, 4, None, Some(p.hover));
        }
        let (text, color, bold) = match part {
            Part::Ellipsis => (ELLIPSIS.to_string(), p.foreground, false),
            Part::Item(i) => {
                let current = Some(i) == last;
                (items.get(i).cloned().unwrap_or_default(), if current { p.foreground } else { p.muted }, current)
            }
        };
        let color = if active { color } else { p.border };
        let font = if bold { font.clone().bold() } else { font.clone() };
        canvas.text_in_rect(&text, Rect::new(x, 0, width, size.height), &font, color, TextAlign::Center);
        if focused(core) && state.focus.get() == Some(part) {
            canvas.focus_rect(rect);
        }
        if n + 1 < parts.len() {
            canvas.text_in_rect(
                "›",
                Rect::new(x + width, 0, SEPARATOR, size.height),
                &font,
                p.muted,
                TextAlign::Center,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrangement_collapses_leading_items() {
        let widths = [50, 60, 70, 40];
        let all = arrange(&widths, 1000, 20);
        assert_eq!(all.len(), 4);
        assert_eq!(all[1], (Part::Item(1), 50 + SEPARATOR, 60));
        // Cabem o "…" + os dois últimos.
        let narrow = arrange(&widths, 20 + SEPARATOR + 70 + SEPARATOR + 40, 20);
        assert_eq!(narrow.iter().map(|p| p.0).collect::<Vec<_>>(), vec![Part::Ellipsis, Part::Item(2), Part::Item(3)]);
        // Nada cabe: ainda mostra o "…" e o último.
        let tiny = arrange(&widths, 10, 20);
        assert_eq!(tiny.iter().map(|p| p.0).collect::<Vec<_>>(), vec![Part::Ellipsis, Part::Item(3)]);
        assert!(arrange(&[], 100, 20).is_empty());
        assert_eq!(arrange(&[500], 100, 20), vec![(Part::Item(0), 0, 500)], "um só item nunca recolhe");
    }
}
