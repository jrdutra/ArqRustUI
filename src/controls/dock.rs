//! Dockable panels (`DockHost` and `DockPane`): central area and left/right/bottom panes with draggable splitters; each
//! pane can float in its own window, redock and close/reopen.

use super::common::ControlOptions;
use super::modern::{palette, publish};
use super::Panel;
use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleRole};
use crate::core::native;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiError, GuiResult, Window};
use crate::graphics::{Canvas, Font, Pen};
use crate::i18n::{library_text, LibraryText};
use crate::utilities::{Point, Rect, Size};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Pane docking side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DockSide {
    /// Left of the center (full height).
    Left,
    /// Right of the center (full height).
    Right,
    /// Below the center (between side columns).
    Bottom,
}

const SPLITTER: i32 = 5;
const HEADER: i32 = 28;
const MIN_ZONE: i32 = 60;

/// Rectangles (center, left, right, bottom) and splitters for each occupied zone of a host with `size` and zone
/// widths/height (`None` = empty).
pub(crate) fn dock_layout(
    size: Size,
    left: Option<i32>,
    right: Option<i32>,
    bottom: Option<i32>,
) -> (Rect, [Option<Rect>; 3], [Option<Rect>; 3]) {
    let (w, h) = (size.width.max(0), size.height.max(0));
    let clamp = |v: i32, total: i32| {
        v.clamp(MIN_ZONE.min(total / 3), (total - MIN_ZONE - SPLITTER).max(MIN_ZONE.min(total / 3)))
    };
    let l = left.map(|v| clamp(v, w));
    let r = right.map(|v| clamp(v, w - l.map_or(0, |l| l + SPLITTER)));
    let mut x0 = 0;
    let mut x1 = w;
    let mut zones = [None, None, None];
    let mut splitters = [None, None, None];
    if let Some(l) = l {
        zones[0] = Some(Rect::new(0, 0, l, h));
        splitters[0] = Some(Rect::new(l, 0, SPLITTER, h));
        x0 = l + SPLITTER;
    }
    if let Some(r) = r {
        zones[1] = Some(Rect::new(w - r, 0, r, h));
        splitters[1] = Some(Rect::new(w - r - SPLITTER, 0, SPLITTER, h));
        x1 = w - r - SPLITTER;
    }
    let mut y1 = h;
    if let Some(b) = bottom.map(|v| clamp(v, h)) {
        zones[2] = Some(Rect::new(x0, h - b, (x1 - x0).max(0), b));
        splitters[2] = Some(Rect::new(x0, h - b - SPLITTER, (x1 - x0).max(0), SPLITTER));
        y1 = h - b - SPLITTER;
    }
    (Rect::new(x0, 0, (x1 - x0).max(0), y1.max(0)), zones, splitters)
}

fn side_index(side: DockSide) -> usize {
    match side {
        DockSide::Left => 0,
        DockSide::Right => 1,
        DockSide::Bottom => 2,
    }
}

struct PaneState {
    title: RefCell<String>,
    side: Cell<DockSide>,
    floating: RefCell<Option<Window>>,
    closed: Cell<bool>,
    hot: Cell<Option<usize>>,
    host: Weak<HostState>,
    host_core: Weak<WidgetCore>,
    content: RefCell<Option<Panel>>,
}

struct HostState {
    center: RefCell<Option<Panel>>,
    panes: RefCell<Vec<DockPane>>,
    sizes: Cell<[i32; 3]>,
    dragging: Cell<Option<(usize, i32, i32)>>,
}

/// Dockable pane host: central area ([`center`](Self::center)) and panes added with [`add_pane`](Self::add_pane). Drag
/// splitters to resize zones. Each pane header has a title and Float/Dock and Close buttons; double-clicking toggles
/// floating/docking.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let host = DockHost::builder().bounds(0, 0, 900, 600).parent(window).build()?;
/// let editor = TextBox::builder().multiline(true).bounds(0, 0, 400, 300).parent(&host.center()).build()?;
/// let files = host.add_pane("Files", DockSide::Left)?;
/// TreeView::builder().bounds(0, 0, 200, 400).parent(&files.content()).build()?;
/// let output = host.add_pane("Output", DockSide::Bottom)?;
/// output.float()?; // in a separate window; `output.dock()` restores it
/// # drop(editor); Ok(()) }
/// ```
#[derive(Clone)]
pub struct DockHost {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<HostState>,
}

impl Widget for DockHost {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for DockHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockHost").field("panes", &self.state.panes.borrow().len()).finish()
    }
}

/// Builder for [`DockHost`].
#[derive(Clone, Debug)]
pub struct DockHostBuilder {
    opts: ControlOptions,
}

impl DockHostBuilder {
    super::common::builder_common!();

    /// Creates the host.
    pub fn build(self) -> GuiResult<DockHost> {
        let core = self.opts.create("DockHost", PANEL_CLASS, crate::ffi::user32::WS_CLIPCHILDREN, 0, false, None)?;
        let state = Rc::new(HostState {
            center: RefCell::new(None),
            panes: RefCell::new(Vec::new()),
            sizes: Cell::new([220, 260, 180]),
            dragging: Cell::new(None),
        });
        let center = Panel::builder().parent(&Window { core: core.clone() }).build()?;
        *state.center.borrow_mut() = Some(center);
        let host = DockHost { core, state };
        host.install();
        host.arrange();
        Ok(host)
    }
}

impl DockHost {
    /// Starts building (default size 640×420).
    pub fn builder() -> DockHostBuilder {
        DockHostBuilder { opts: ControlOptions::new(640, 420) }
    }

    /// Central area panel.
    pub fn center(&self) -> Panel {
        self.state.center.borrow().clone().expect("created in build")
    }

    fn install(&self) {
        let target = Rc::downgrade(&self.core);
        let state = Rc::downgrade(&self.state);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(core), Some(state)) = (target.upgrade(), state.upgrade()) {
                let p = palette(&core);
                canvas.clear(p.background.mix(p.foreground, 0.04));
                let host = DockHost { core: core.clone(), state };
                for splitter in host.layout().2.iter().flatten() {
                    canvas.fill_rect(*splitter, p.border.mix(p.background, 0.5));
                }
            }
        }));
        let me = self.weak();
        self.core.on(EventKind::Resize, move |_| {
            if let Some(host) = me.upgrade() {
                host.arrange();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseDown, move |event| {
            if let (Some(host), Event::MouseDown(m)) = (me.upgrade(), event) {
                if let Some(zone) = host.splitter_at(m.position) {
                    let size = host.state.sizes.get()[zone];
                    let start = if zone == 2 { m.position.y } else { m.position.x };
                    host.state.dragging.set(Some((zone, start, size)));
                    // SAFETY: captura o mouse para o hospedeiro vivo desta thread.
                    unsafe { crate::ffi::user32::SetCapture(host.core.hwnd()) };
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseMove, move |event| {
            if let (Some(host), Event::MouseMove(m)) = (me.upgrade(), event) {
                if let Some((zone, start, size)) = host.state.dragging.get() {
                    let now = if zone == 2 { m.position.y } else { m.position.x };
                    // Esquerda cresce para a direita; direita e embaixo, ao contrário.
                    let delta = if zone == 0 { now - start } else { start - now };
                    host.set_zone_size(side_of(zone), size + delta);
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseUp, move |_| {
            if let Some(host) = me.upgrade() {
                if host.state.dragging.replace(None).is_some() {
                    // SAFETY: devolve a captura pega no MouseDown.
                    unsafe { crate::ffi::user32::ReleaseCapture() };
                }
            }
        });
        let me = self.weak();
        self.core.add_message_filter(move |raw| {
            if raw.message != crate::ffi::user32::WM_SETCURSOR
                || raw.wparam as crate::ffi::HWND != raw.hwnd as crate::ffi::HWND
            {
                return None;
            }
            let host = me.upgrade()?;
            let mut point = crate::ffi::POINT::default();
            // SAFETY: posição do cursor convertida para o hospedeiro vivo.
            unsafe {
                crate::ffi::user32::GetCursorPos(&mut point);
                crate::ffi::user32::ScreenToClient(host.core.hwnd(), &mut point);
            }
            let dpi = host.core.dpi();
            let at = Point::new(crate::utilities::to_logical(point.x, dpi), crate::utilities::to_logical(point.y, dpi));
            let zone = host.splitter_at(at).or(host.state.dragging.get().map(|d| d.0))?;
            let cursor = if zone == 2 { crate::ffi::user32::IDC_SIZENS } else { crate::ffi::user32::IDC_SIZEWE };
            // SAFETY: cursor compartilhado do sistema.
            unsafe {
                crate::ffi::user32::SetCursor(crate::ffi::user32::LoadCursorW(std::ptr::null_mut(), cursor as _))
            };
            Some(1)
        });
    }

    fn weak(&self) -> WeakHost {
        WeakHost { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    /// Zones occupied by visible docked panes.
    fn occupied(&self) -> [bool; 3] {
        let mut used = [false; 3];
        for pane in self.state.panes.borrow().iter() {
            if pane.is_docked() {
                used[side_index(pane.side())] = true;
            }
        }
        used
    }

    fn layout(&self) -> (Rect, [Option<Rect>; 3], [Option<Rect>; 3]) {
        let used = self.occupied();
        let sizes = self.state.sizes.get();
        let pick = |i: usize| used[i].then_some(sizes[i]);
        dock_layout(self.client_size(), pick(0), pick(1), pick(2))
    }

    fn splitter_at(&self, at: Point) -> Option<usize> {
        let (_, _, splitters) = self.layout();
        splitters.iter().position(|s| {
            s.is_some_and(|r| at.x >= r.x - 1 && at.x < r.x + r.width + 1 && at.y >= r.y && at.y < r.y + r.height)
        })
    }

    /// Repositions the center and docked panes.
    pub fn arrange(&self) {
        let (center, zones, _) = self.layout();
        self.center().set_bounds(center);
        let panes = self.state.panes.borrow().clone();
        for (index, zone) in zones.iter().enumerate() {
            let Some(zone) = zone else { continue };
            let here: Vec<&DockPane> =
                panes.iter().filter(|p| p.is_docked() && side_index(p.side()) == index).collect();
            let n = here.len() as i32;
            for (i, pane) in here.iter().enumerate() {
                let i = i as i32;
                let rect = if index == 2 {
                    let w = (zone.width - SPLITTER * (n - 1)) / n;
                    Rect::new(
                        zone.x + i * (w + SPLITTER),
                        zone.y,
                        if i == n - 1 { zone.width - i * (w + SPLITTER) } else { w },
                        zone.height,
                    )
                } else {
                    let h = (zone.height - SPLITTER * (n - 1)) / n;
                    Rect::new(
                        zone.x,
                        zone.y + i * (h + SPLITTER),
                        zone.width,
                        if i == n - 1 { zone.height - i * (h + SPLITTER) } else { h },
                    )
                };
                pane.set_bounds(rect);
            }
        }
        self.core.invalidate();
    }

    /// Adds a pane docked on `side`.
    pub fn add_pane(&self, title: &str, side: DockSide) -> GuiResult<DockPane> {
        let mut opts = ControlOptions::new(200, 200);
        opts.parent = Some(self.core.clone());
        opts.text = title.to_string();
        let core = opts.create("DockPane", PANEL_CLASS, crate::ffi::user32::WS_CLIPCHILDREN, 0, false, None)?;
        let content = Panel::builder().bounds(0, HEADER, 200, 172).parent(&Window { core: core.clone() }).build()?;
        let state = Rc::new(PaneState {
            title: RefCell::new(title.to_string()),
            side: Cell::new(side),
            floating: RefCell::new(None),
            closed: Cell::new(false),
            hot: Cell::new(None),
            host: Rc::downgrade(&self.state),
            host_core: Rc::downgrade(&self.core),
            content: RefCell::new(Some(content)),
        });
        let pane = DockPane { core, state };
        pane.install();
        self.state.panes.borrow_mut().push(pane.clone());
        self.arrange();
        Ok(pane)
    }

    /// Host panes (docked, floating and closed).
    pub fn panes(&self) -> Vec<DockPane> {
        self.state.panes.borrow().clone()
    }

    /// Logical zone width (left/right) or height (bottom).
    pub fn zone_size(&self, side: DockSide) -> i32 {
        self.state.sizes.get()[side_index(side)]
    }

    /// Sets logical zone width/height.
    pub fn set_zone_size(&self, side: DockSide, size: i32) {
        let mut sizes = self.state.sizes.get();
        let total = if side == DockSide::Bottom { self.client_size().height } else { self.client_size().width };
        sizes[side_index(side)] = size.clamp(MIN_ZONE, (total - MIN_ZONE - SPLITTER).max(MIN_ZONE));
        self.state.sizes.set(sizes);
        self.arrange();
    }

    /// Logical zone splitter rectangle, if occupied.
    pub fn splitter_bounds(&self, side: DockSide) -> Option<Rect> {
        self.layout().2[side_index(side)]
    }
}

fn side_of(index: usize) -> DockSide {
    match index {
        0 => DockSide::Left,
        1 => DockSide::Right,
        _ => DockSide::Bottom,
    }
}

#[derive(Clone)]
struct WeakHost {
    core: Weak<WidgetCore>,
    state: Weak<HostState>,
}

impl WeakHost {
    fn upgrade(&self) -> Option<DockHost> {
        Some(DockHost { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

/// Dockable pane created by [`DockHost::add_pane`]: titled header with buttons and a content panel
/// ([`content`](Self::content)).
#[derive(Clone)]
pub struct DockPane {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<PaneState>,
}

impl Widget for DockPane {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for DockPane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockPane")
            .field("title", &self.state.title.borrow())
            .field("side", &self.state.side.get())
            .finish()
    }
}

/// Header buttons: 0 = float/dock, 1 = close.
fn header_button(width: i32, index: usize) -> Rect {
    Rect::new(width - (index as i32 + 1) * (HEADER - 4) - 2, 2, HEADER - 4, HEADER - 4)
}

impl DockPane {
    fn install(&self) {
        let target = Rc::downgrade(&self.core);
        let state = Rc::downgrade(&self.state);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(core), Some(state)) = (target.upgrade(), state.upgrade()) {
                paint_pane(&core, &state, canvas);
            }
        }));
        let me = self.weak();
        self.core.on(EventKind::Resize, move |_| {
            if let Some(pane) = me.upgrade() {
                let size = pane.client_size();
                pane.content().set_bounds(Rect::new(0, HEADER, size.width, (size.height - HEADER).max(0)));
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseMove, move |event| {
            if let (Some(pane), Event::MouseMove(m)) = (me.upgrade(), event) {
                let hot = pane.button_at(m.position);
                if pane.state.hot.replace(hot) != hot {
                    pane.core.invalidate();
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseLeave, move |_| {
            if let Some(pane) = me.upgrade() {
                pane.state.hot.set(None);
                pane.core.invalidate();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseUp, move |event| {
            if let (Some(pane), Event::MouseUp(m)) = (me.upgrade(), event) {
                match pane.button_at(m.position) {
                    Some(0) => {
                        let _ = pane.toggle_floating();
                    }
                    Some(1) => pane.close(),
                    _ => {}
                }
            }
        });
        let me = self.weak();
        self.core.on(EventKind::DoubleClick, move |_| {
            if let Some(pane) = me.upgrade() {
                let _ = pane.toggle_floating();
            }
        });
        publish(
            &self.core,
            &self.state,
            |core, state| {
                let width = crate::utilities::to_logical(native::client_rect(core.hwnd()).right, core.dpi());
                let floating = state.floating.borrow().is_some();
                let root = AccessibleItem::new(0, AccessibleRole::Pane, &state.title.borrow());
                let float = library_text(if floating { LibraryText::Dock } else { LibraryText::Float });
                let children = vec![
                    AccessibleItem::new(1, AccessibleRole::Button, &float).bounds(header_button(width, 1)).invokable(),
                    AccessibleItem::new(2, AccessibleRole::Button, &library_text(LibraryText::Close))
                        .bounds(header_button(width, 0))
                        .invokable(),
                ];
                (root, children)
            },
            |core, state, id, action| {
                if action != AccessibleAction::Invoke {
                    return false;
                }
                let pane = DockPane { core: core.clone(), state: state.clone() };
                match id {
                    1 => pane.toggle_floating().is_ok(),
                    2 => {
                        pane.close();
                        true
                    }
                    _ => false,
                }
            },
        );
    }

    fn weak(&self) -> WeakPane {
        WeakPane { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    fn button_at(&self, at: Point) -> Option<usize> {
        let width = self.client_size().width;
        // Ordem na tela: [flutuar][fechar] — o fechar fica na ponta.
        [(1usize, 0usize), (0, 1)].into_iter().find_map(|(slot, button)| {
            let r = header_button(width, slot);
            (at.x >= r.x && at.x < r.x + r.width && at.y >= r.y && at.y < r.y + r.height).then_some(button)
        })
    }

    fn host(&self) -> Option<DockHost> {
        Some(DockHost { core: self.state.host_core.upgrade()?, state: self.state.host.upgrade()? })
    }

    /// Content panel (use as `parent`).
    pub fn content(&self) -> Panel {
        self.state.content.borrow().clone().expect("created in add_pane")
    }

    /// Title.
    pub fn title(&self) -> String {
        self.state.title.borrow().clone()
    }

    /// Docking side (last side if floating or closed).
    pub fn side(&self) -> DockSide {
        self.state.side.get()
    }

    /// `true` if floating in its own window.
    pub fn is_floating(&self) -> bool {
        self.state.floating.borrow().is_some()
    }

    /// `true` if closed.
    pub fn is_closed(&self) -> bool {
        self.state.closed.get()
    }

    /// `true` if docked and visible in the host.
    pub fn is_docked(&self) -> bool {
        !self.is_floating() && !self.is_closed()
    }

    /// Window in which the pane floats.
    pub fn floating_window(&self) -> Option<Window> {
        self.state.floating.borrow().clone()
    }

    /// Moves the pane to its own tool window, owned by the host window, preserving its size. Closing that window
    /// redocks the pane.
    pub fn float(&self) -> GuiResult<()> {
        if self.is_floating() {
            return Ok(());
        }
        let host = self.host().ok_or(GuiError::WidgetDestroyed)?;
        let root = crate::core::widget::lookup(native::root_of(host.core.hwnd())).ok_or(GuiError::WidgetDestroyed)?;
        let owner = Window { core: root };
        let size = self.client_size();
        let window = Window::builder()
            .title(&self.title())
            .size(size.width.max(160), size.height.max(120))
            .tool_window()
            .owner(&owner)
            .build()?;
        let me = self.weak();
        window.on_close_request(move || {
            if let Some(pane) = me.upgrade() {
                pane.dock_back();
            }
            true
        });
        let me = self.weak();
        window.core().on(EventKind::Resize, move |event| {
            if let (Some(pane), Event::Resize(size)) = (me.upgrade(), event) {
                if pane.is_floating() {
                    pane.set_bounds(Rect::new(0, 0, size.width, size.height));
                }
            }
        });
        self.state.closed.set(false);
        reparent(
            &self.core,
            &window.core().clone(),
            Rect::new(0, 0, window.client_size().width, window.client_size().height),
        );
        self.set_visible(true);
        *self.state.floating.borrow_mut() = Some(window.clone());
        window.show();
        host.arrange();
        self.core.invalidate();
        Ok(())
    }

    /// Returns a floating or closed pane to its current docking side.
    pub fn dock(&self) {
        let window = self.state.floating.borrow_mut().take();
        self.dock_back();
        if let Some(window) = window {
            window.on_close_request(|| true);
            window.destroy();
        }
    }

    fn dock_back(&self) {
        let Some(host) = self.host() else { return };
        self.state.floating.borrow_mut().take();
        self.state.closed.set(false);
        if self.core.is_alive() {
            reparent(&self.core, &host.core, Rect::new(0, 0, 200, 200));
            self.set_visible(true);
        }
        host.arrange();
    }

    /// Docks on another side.
    pub fn dock_to(&self, side: DockSide) {
        self.state.side.set(side);
        self.dock();
    }

    /// Toggles floating/docked state.
    pub fn toggle_floating(&self) -> GuiResult<()> {
        if self.is_floating() {
            self.dock();
            Ok(())
        } else {
            self.float()
        }
    }

    /// Closes (hides) the pane; [`show`](Self::show) reopens it where it was.
    pub fn close(&self) {
        if self.is_floating() {
            self.dock();
        }
        self.state.closed.set(true);
        self.set_visible(false);
        if let Some(host) = self.host() {
            host.arrange();
        }
    }

    /// Reopens a closed pane.
    pub fn show(&self) {
        if self.state.closed.replace(false) {
            self.set_visible(true);
            if let Some(host) = self.host() {
                host.arrange();
            }
        }
    }
}

#[derive(Clone)]
struct WeakPane {
    core: Weak<WidgetCore>,
    state: Weak<PaneState>,
}

impl WeakPane {
    fn upgrade(&self) -> Option<DockPane> {
        Some(DockPane { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

/// Reparents and positions a control, retaining its child style.
pub(crate) fn reparent(core: &Rc<WidgetCore>, parent: &Rc<WidgetCore>, bounds: Rect) {
    // SAFETY: as duas janelas são vivas desta thread; o controle continua WS_CHILD.
    unsafe { crate::ffi::user32::SetParent(core.hwnd(), parent.hwnd()) };
    core.set_bounds(bounds);
    core.invalidate();
}

fn paint_pane(core: &WidgetCore, state: &PaneState, canvas: &Canvas) {
    let p = palette(core);
    let size = canvas.size();
    canvas.clear(p.background);
    let header = Rect::new(0, 0, size.width, HEADER);
    canvas.fill_rect(header, p.background.mix(p.foreground, 0.07));
    canvas.text_line(
        &state.title.borrow(),
        Rect::new(8, 0, size.width - 2 * HEADER - 8, HEADER),
        &Font::system().bold(),
        p.foreground,
    );
    let floating = state.floating.borrow().is_some();
    for (slot, button) in [(1usize, 0usize), (0, 1)] {
        let r = header_button(size.width, slot);
        if state.hot.get() == Some(button) {
            canvas.round_rect(r, 4, None, Some(p.hover.mix(p.foreground, 0.08)));
        }
        let (cx, cy) = (r.x + r.width / 2, r.y + r.height / 2);
        let pen = Pen::new(p.foreground, 1);
        if button == 1 {
            canvas.line(Point::new(cx - 4, cy - 4), Point::new(cx + 4, cy + 4), &pen);
            canvas.line(Point::new(cx + 4, cy - 4), Point::new(cx - 4, cy + 4), &pen);
        } else if floating {
            // Acoplar: retângulo com a borda de baixo grossa.
            canvas.rect(Rect::new(cx - 5, cy - 5, 10, 10), Some(&pen), None);
            canvas.fill_rect(Rect::new(cx - 5, cy + 2, 10, 3), p.foreground);
        } else {
            // Flutuar: dois retângulos sobrepostos.
            canvas.rect(Rect::new(cx - 5, cy - 2, 8, 7), Some(&pen), None);
            canvas.polyline(
                &[
                    Point::new(cx - 2, cy - 2),
                    Point::new(cx - 2, cy - 5),
                    Point::new(cx + 5, cy - 5),
                    Point::new(cx + 5, cy + 2),
                    Point::new(cx + 3, cy + 2),
                ],
                &pen,
            );
        }
    }
    canvas.line(Point::new(0, HEADER - 1), Point::new(size.width, HEADER - 1), &Pen::new(p.border, 1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_and_splitters() {
        let (center, zones, splitters) = dock_layout(Size::new(800, 600), Some(200), Some(150), Some(100));
        assert_eq!(zones[0], Some(Rect::new(0, 0, 200, 600)));
        assert_eq!(zones[1], Some(Rect::new(650, 0, 150, 600)));
        assert_eq!(zones[2], Some(Rect::new(205, 500, 440, 100)), "embaixo entre as colunas");
        assert_eq!(center, Rect::new(205, 0, 440, 495));
        assert_eq!(splitters[0], Some(Rect::new(200, 0, SPLITTER, 600)));
        assert_eq!(splitters[2], Some(Rect::new(205, 495, 440, SPLITTER)));
        let (center, zones, _) = dock_layout(Size::new(800, 600), None, None, None);
        assert_eq!(center, Rect::new(0, 0, 800, 600));
        assert!(zones.iter().all(Option::is_none));
        let (_, zones, _) = dock_layout(Size::new(300, 300), Some(1000), None, None);
        assert_eq!(zones[0].unwrap().width, 300 - MIN_ZONE - SPLITTER, "a central fica com o mínimo");
    }
}
