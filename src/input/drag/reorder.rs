//! Internal list/tree reordering with mouse capture, moving item image and insertion line.

use super::*;

use crate::controls::treeview;
use crate::core::application as app;
use crate::core::native;
use crate::core::widget::{lookup, WidgetCore};
use crate::ffi::kernel32::{GetModuleHandleW, GetProcAddress};
use crate::system::clipboard::ClipboardContent;
use crate::utilities::{to_pixels, to_wide, Color};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Estado por controle e reordenação interna
// ---------------------------------------------------------------------------

/// Control type supporting internal reordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListKind {
    ListBox,
    ListView,
    Tree,
}

pub(crate) type VetoFn = Rc<dyn Fn(usize, usize) -> bool>;
pub(crate) type MoverFn = Rc<dyn Fn(usize, usize)>;
pub(crate) type TreeVetoFn = Rc<dyn Fn(treeview::TreeItem, treeview::TreeDropPosition) -> bool>;
pub(crate) type TreeMovedFn = Rc<dyn Fn(treeview::TreeItem)>;
pub(crate) type OutFn = Rc<dyn Fn(usize) -> Option<ClipboardContent>>;
pub(crate) type BlockMoverFn = Rc<dyn Fn(&[usize], usize)>;
pub(crate) type BlockVetoFn = Rc<dyn Fn(&[usize], usize) -> bool>;
pub(crate) type OutDoneFn = Rc<dyn Fn(usize, DropEffect)>;

/// Control drag configuration/session.
pub(crate) struct ListDrag {
    kind: ListKind,
    pub(crate) reorder: Cell<bool>,
    pub(crate) veto: RefCell<Option<VetoFn>>,
    pub(crate) mover: RefCell<Option<MoverFn>>,
    pub(crate) tree_veto: RefCell<Option<TreeVetoFn>>,
    pub(crate) tree_moved: RefCell<Vec<TreeMovedFn>>,
    pub(crate) drag_out: RefCell<Option<OutFn>>,
    /// Moves multiple selected items at once (items, insertion position).
    pub(crate) block_mover: RefCell<Option<BlockMoverFn>>,
    pub(crate) block_veto: RefCell<Option<BlockVetoFn>>,
    /// External drag effects (default: copy only).
    pub(crate) out_effects: Cell<DWORD>,
    /// Notified of final external effect (`Move` = delete original).
    pub(crate) out_done: RefCell<Option<OutDoneFn>>,
    /// ListBox button-down item (point/index), awaiting threshold.
    press: Cell<Option<(POINT, usize)>>,
    /// Click on multiple selection deferred because button-down clears it; delivered only if no drag starts.
    deferred: Cell<bool>,
    session: RefCell<Option<Session>>,
}

/// Drag/drop state in each participating `WidgetCore`.
#[derive(Default)]
pub(crate) struct DragState {
    pub(crate) drop_target: RefCell<Option<ComPtr>>,
    list: RefCell<Option<Rc<ListDrag>>>,
}

impl DragState {
    pub(crate) fn of(core: &Rc<WidgetCore>) -> Rc<DragState> {
        core.drag.borrow_mut().get_or_insert_with(|| Rc::new(DragState::default())).clone()
    }
}

/// Control drag configuration, created on first use.
pub(crate) fn list_config(core: &Rc<WidgetCore>, kind: ListKind) -> Rc<ListDrag> {
    let state = DragState::of(core);
    let mut list = state.list.borrow_mut();
    list.get_or_insert_with(|| {
        Rc::new(ListDrag {
            kind,
            reorder: Cell::new(false),
            veto: RefCell::new(None),
            mover: RefCell::new(None),
            tree_veto: RefCell::new(None),
            tree_moved: RefCell::new(Vec::new()),
            drag_out: RefCell::new(None),
            block_mover: RefCell::new(None),
            block_veto: RefCell::new(None),
            out_effects: Cell::new(DROPEFFECT_COPY),
            out_done: RefCell::new(None),
            press: Cell::new(None),
            deferred: Cell::new(false),
            session: RefCell::new(None),
        })
    })
    .clone()
}

pub(crate) fn list_of(core: &WidgetCore) -> Option<Rc<ListDrag>> {
    core.drag.borrow().as_ref().and_then(|state| state.list.borrow().clone())
}

/// Calculated cursor destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    /// List insertion position (0..=count).
    Slot(usize),
    /// Tree node/relative position (0 = inside, 1 = before, 2 = after).
    Tree(u8, isize),
}

pub(crate) struct Session {
    from: usize,
    /// Selected items dragged together (empty = only `from`).
    group: Vec<usize>,
    grab: POINT,
    ghost: HWND,
    ghost_bitmap: HBITMAP,
    ghost_size: SIZE,
    marker: HWND,
    target: Option<Target>,
    last: POINT,
    hover: (isize, u32),
    hilite: bool,
    out_refused: bool,
}

pub(crate) const SCROLL_TIMER: usize = 0x5241_4447;
pub(crate) const SCROLL_MS: u32 = 50;
/// Ticks hovering a closed node until auto-expansion (~700 ms).
pub(crate) const EXPAND_TICKS: u32 = 14;

pub(crate) fn lparam_point(lp: LPARAM) -> POINT {
    POINT { x: (lp & 0xFFFF) as i16 as i32, y: ((lp >> 16) & 0xFFFF) as i16 as i32 }
}

pub(crate) fn point_lparam(point: POINT) -> LPARAM {
    ((point.y as u16 as isize) << 16) | point.x as u16 as isize
}

/// Messages for drag-configured controls, called by `WidgetCore::handle`.
pub(crate) fn handle(core: &Rc<WidgetCore>, hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    if msg == WM_DESTROY {
        let state = core.drag.borrow().clone()?;
        if state.drop_target.borrow_mut().take().is_some() {
            // SAFETY: desfaz o registro OLE enquanto a janela ainda existe.
            unsafe { RevokeDragDrop(hwnd) };
        }
        if let Some(list) = state.list.borrow().clone() {
            finish(core, &list, false);
        }
        return None;
    }
    let list = list_of(core)?;
    let active = list.session.borrow().is_some();
    match msg {
        WM_LBUTTONDOWN if list.kind == ListKind::ListBox && !active && list.enabled() => {
            let point = lparam_point(lp);
            let index = list_index_at(ListKind::ListBox, hwnd, point);
            list.press.set(index.map(|index| (point, index)));
            list.deferred.set(false);
            // Sobre um item de uma seleção múltipla, sem modificadores: adia o clique
            // para que a seleção sobreviva ao arraste.
            let plain = wp & (MK_SHIFT | MK_CONTROL) == 0;
            if let Some(index) = index.filter(|_| plain && list.block_mover.borrow().is_some()) {
                let selection = selection_of(ListKind::ListBox, hwnd);
                if selection.len() > 1 && selection.contains(&index) {
                    list.deferred.set(true);
                    // SAFETY: foco e captura de uma janela viva desta thread.
                    unsafe {
                        SetFocus(hwnd);
                        SetCapture(hwnd);
                    }
                    return Some(0);
                }
            }
            None
        }
        WM_MOUSEMOVE if active => {
            update(core, &list, lparam_point(lp));
            Some(0)
        }
        WM_MOUSEMOVE if list.kind == ListKind::ListBox && wp & MK_LBUTTON != 0 => {
            let (start, index) = list.press.get()?;
            let point = lparam_point(lp);
            let dpi = native::dpi_for_window(hwnd);
            let (dx, dy) = (native::system_metric(SM_CXDRAG, dpi).max(2), native::system_metric(SM_CYDRAG, dpi).max(2));
            if (point.x - start.x).abs() < dx && (point.y - start.y).abs() < dy {
                return None;
            }
            list.press.set(None);
            if !list.deferred.replace(false) {
                // Termina o acompanhamento da própria ListBox (seleção no item
                // pressionado) antes de assumir o mouse.
                core.default_proc(hwnd, WM_LBUTTONUP, 0, point_lparam(start));
            }
            begin(core, &list, index, start);
            if list.session.borrow().is_some() {
                update(core, &list, point);
            }
            Some(0)
        }
        WM_LBUTTONUP => {
            let pressed = list.press.take();
            if list.deferred.replace(false) && !active {
                // Não virou arraste: entrega o clique adiado à ListBox.
                // SAFETY: libera a captura obtida no botão pressionado.
                unsafe { ReleaseCapture() };
                if let Some((start, _)) = pressed {
                    core.default_proc(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, point_lparam(start));
                }
                return None;
            }
            if active {
                finish(core, &list, true);
                Some(0)
            } else {
                None
            }
        }
        WM_KEYDOWN if active && wp as i32 == VK_ESCAPE => {
            finish(core, &list, false);
            Some(0)
        }
        WM_CAPTURECHANGED if active && lp as HWND != hwnd => {
            finish(core, &list, false);
            Some(0)
        }
        WM_CANCELMODE if active => {
            finish(core, &list, false);
            None
        }
        WM_TIMER if wp == SCROLL_TIMER => {
            if active {
                tick(core, &list);
            }
            Some(0)
        }
        _ => None,
    }
}

impl ListDrag {
    fn enabled(&self) -> bool {
        self.reorder.get() || self.drag_out.borrow().is_some()
    }
}

/// Control `LVN_BEGINDRAG` / `TVN_BEGINDRAGW`, called from parent `WM_NOTIFY`.
pub(crate) fn begin_native(child: &Rc<WidgetCore>, code: i32, lp: LPARAM) -> bool {
    let Some(list) = list_of(child) else { return false };
    if !list.enabled() || list.session.borrow().is_some() {
        return false;
    }
    let hwnd = child.hwnd();
    // SAFETY: em LVN_BEGINDRAG o LPARAM aponta para NMLISTVIEW e em TVN_BEGINDRAGW para
    // NMTREEVIEWW, válidos durante a notificação.
    let (from, point) = unsafe {
        if code == LVN_BEGINDRAG && list.kind == ListKind::ListView {
            let info = &*(lp as *const NMLISTVIEW);
            if info.iItem < 0 {
                return false;
            }
            (info.iItem as usize, info.ptAction)
        } else if code == TVN_BEGINDRAGW && list.kind == ListKind::Tree {
            let info = &*(lp as *const NMTREEVIEWW);
            if info.itemNew.hItem.is_null() {
                return false;
            }
            native::send(hwnd, TVM_SELECTITEM, TVGN_CARET, info.itemNew.hItem as isize);
            (info.itemNew.hItem as usize, info.ptDrag)
        } else {
            return false;
        }
    };
    begin(child, &list, from, point);
    true
}

pub(crate) fn begin(core: &Rc<WidgetCore>, list: &Rc<ListDrag>, from: usize, point: POINT) {
    let hwnd = core.hwnd();
    let Some(mut rect) = item_rect(list.kind, hwnd, from) else { return };
    let client = native::client_rect(hwnd);
    let dpi = native::dpi_for_window(hwnd);
    rect.left = rect.left.max(client.left);
    rect.right = rect.right.min(client.right).min(rect.left + to_pixels(360, dpi));
    rect.top = rect.top.max(client.top);
    rect.bottom = rect.bottom.min(client.bottom);
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    if width <= 0 || height <= 0 {
        return;
    }
    let bitmap = snapshot(hwnd, rect).unwrap_or(std::ptr::null_mut());
    let accent = accent_for(hwnd);
    let root = native::root_of(hwnd);
    let ghost = visual_window(root, Visual::Ghost { bitmap, accent }, !running_on_wine());
    let marker = visual_window(root, Visual::Marker { accent }, false);
    let group = if list.block_mover.borrow().is_some() { selection_of(list.kind, hwnd) } else { Vec::new() };
    let group = if group.len() > 1 && group.contains(&from) { group } else { Vec::new() };
    let session = Session {
        from,
        group,
        grab: POINT { x: point.x - rect.left, y: point.y - rect.top },
        ghost,
        ghost_bitmap: bitmap,
        ghost_size: SIZE { cx: width, cy: height },
        marker,
        target: None,
        last: point,
        hover: (0, 0),
        hilite: false,
        out_refused: false,
    };
    *list.session.borrow_mut() = Some(session);
    // SAFETY: captura e temporizador de uma janela viva desta thread.
    unsafe {
        SetCapture(hwnd);
        SetTimer(hwnd, SCROLL_TIMER, SCROLL_MS, std::ptr::null());
    }
    update(core, list, point);
}

pub(crate) fn update(core: &Rc<WidgetCore>, list: &Rc<ListDrag>, point: POINT) {
    let hwnd = core.hwnd();
    let mut screen = point;
    // SAFETY: conversão de coordenadas de uma janela viva.
    unsafe { ClientToScreen(hwnd, &mut screen) };
    let window = native::window_rect(hwnd);
    let inside =
        screen.x >= window.left && screen.x < window.right && screen.y >= window.top && screen.y < window.bottom;
    let wants_out = !inside && list.drag_out.borrow().is_some();
    let refused = list.session.borrow().as_ref().is_some_and(|s| s.out_refused);
    if wants_out && !refused {
        drag_out(core, list);
        return;
    }
    let target = if list.reorder.get() && inside { compute_target(list, hwnd, point) } else { None };
    // Copia o necessário e solta o empréstimo: mover janelas pode gerar mensagens
    // (até um WM_MOUSEMOVE sintético) que voltam a este módulo.
    let (ghost, marker, size, grab, changed, hilite) = {
        let mut session = list.session.borrow_mut();
        let Some(session) = session.as_mut() else { return };
        session.last = point;
        let changed = target != session.target;
        session.target = target;
        (session.ghost, session.marker, session.ghost_size, session.grab, changed, session.hilite)
    };
    // Fantasma: translúcido sob o cursor; sem transparência (Wine sem compositor), ao lado.
    let (gx, gy) = if running_on_wine() {
        let offset = to_pixels(16, native::dpi_for_window(hwnd));
        (screen.x + offset, screen.y + offset)
    } else {
        (screen.x - grab.x, screen.y - grab.y)
    };
    // SAFETY: posiciona janelas próprias vivas, sem ativá-las.
    unsafe {
        SetWindowPos(ghost, HWND_TOPMOST, gx, gy, size.cx, size.cy, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    if changed {
        let hilite = place_marker(list.kind, hwnd, target, ghost, marker, hilite);
        if let Some(session) = list.session.borrow_mut().as_mut() {
            session.hilite = hilite;
        }
    }
    let blocked =
        inside && list.reorder.get() && target.is_none() && list.kind == ListKind::Tree && hovering_item(hwnd, point);
    // SAFETY: cursores compartilhados do sistema.
    unsafe {
        let cursor = if blocked || (!inside && !wants_out) { IDC_NO } else { IDC_ARROW };
        SetCursor(LoadCursorW(std::ptr::null_mut(), cursor as PCWSTR));
    }
}

pub(crate) fn hovering_item(hwnd: HWND, point: POINT) -> bool {
    !tree_hit(hwnd, point).is_null()
}

/// Cursor left control: converts to OLE dragging using `on_drag_out` content.
pub(crate) fn drag_out(core: &Rc<WidgetCore>, list: &Rc<ListDrag>) {
    let Some(provider) = list.drag_out.borrow().clone() else { return };
    let (from, grab, bitmap, size) = {
        let session = list.session.borrow();
        let Some(s) = session.as_ref() else { return };
        (s.from, s.grab, s.ghost_bitmap, s.ghost_size)
    };
    let content = DropTarget::guarded(None, || provider(from));
    let allowed = list.out_effects.get();
    let Some(content) = content.filter(|c| !c.is_empty()) else {
        if let Some(session) = list.session.borrow_mut().as_mut() {
            session.out_refused = true;
        }
        return;
    };
    // O bitmap do fantasma vira a imagem do arraste OLE.
    if let Some(session) = list.session.borrow_mut().as_mut() {
        session.ghost_bitmap = std::ptr::null_mut();
    }
    finish(core, list, false);
    let image = (!bitmap.is_null()).then_some(DragBitmap { bitmap, width: size.cx, height: size.cy, hotspot: grab });
    // Uma falha aqui (por exemplo, sem OLE) só significa que nada foi solto.
    let effect =
        do_drag(&content, if allowed == 0 { DROPEFFECT_COPY } else { allowed }, image).unwrap_or(DropEffect::None);
    let done = list.out_done.borrow().clone();
    if let Some(done) = done {
        DropTarget::guarded((), || done(from, effect));
    }
}

pub(crate) fn tick(core: &Rc<WidgetCore>, list: &Rc<ListDrag>) {
    let hwnd = core.hwnd();
    let (point, target) = match list.session.borrow().as_ref() {
        Some(session) => (session.last, session.target),
        None => return,
    };
    let client = native::client_rect(hwnd);
    let zone = to_pixels(18, native::dpi_for_window(hwnd));
    let inside_x = point.x >= client.left && point.x < client.right;
    let direction = if !inside_x || !list.reorder.get() {
        0
    } else if point.y < client.top + header_height(list.kind, hwnd) + zone {
        -1
    } else if point.y >= client.bottom - zone {
        1
    } else {
        0
    };
    if direction != 0 {
        hide_marker(list);
        scroll(list.kind, hwnd, direction);
        if let Some(session) = list.session.borrow_mut().as_mut() {
            session.target = None;
        }
        update(core, list, point);
        return;
    }
    // Árvore: parado sobre um nó fechado com filhos, ele se abre.
    if let Some(Target::Tree(0, item)) = target {
        let expand = {
            let mut session = list.session.borrow_mut();
            let Some(session) = session.as_mut() else { return };
            if session.hover.0 == item {
                session.hover.1 += 1;
            } else {
                session.hover = (item, 0);
            }
            session.hover.1 == EXPAND_TICKS
        };
        if expand && !treeview::is_expanded(hwnd, item as HTREEITEM) {
            hide_marker(list);
            native::send(hwnd, TVM_EXPAND, TVE_EXPAND, item);
            if let Some(session) = list.session.borrow_mut().as_mut() {
                session.target = None;
            }
            update(core, list, point);
        }
    } else if let Some(session) = list.session.borrow_mut().as_mut() {
        session.hover = (0, 0);
    }
}

pub(crate) fn hide_marker(list: &ListDrag) {
    let marker = list.session.borrow().as_ref().map(|s| s.marker);
    if let Some(marker) = marker {
        // SAFETY: janela própria viva.
        unsafe { ShowWindow(marker, SW_HIDE) };
    }
}

pub(crate) fn finish(core: &Rc<WidgetCore>, list: &Rc<ListDrag>, commit: bool) {
    let Some(session) = list.session.borrow_mut().take() else { return };
    let hwnd = core.hwnd();
    // SAFETY: desfaz temporizador, captura e janelas criadas em `begin`.
    unsafe {
        KillTimer(hwnd, SCROLL_TIMER);
        if GetCapture() == hwnd {
            ReleaseCapture();
        }
        destroy_visual(session.ghost);
        destroy_visual(session.marker);
        if !session.ghost_bitmap.is_null() {
            DeleteObject(session.ghost_bitmap);
        }
    }
    if session.hilite {
        native::send(hwnd, TVM_SELECTITEM, TVGN_DROPHILITE, 0);
    }
    if !commit {
        return;
    }
    match session.target {
        Some(Target::Slot(slot)) if session.group.len() > 1 => {
            if block_moves(list_count(list.kind, hwnd), &session.group, slot).is_empty() {
                return;
            }
            let allowed = match list.block_veto.borrow().clone() {
                Some(veto) => DropTarget::guarded(false, || veto(&session.group, slot)),
                None => true,
            };
            if allowed {
                if let Some(mover) = list.block_mover.borrow().clone() {
                    DropTarget::guarded((), || mover(&session.group, slot));
                }
            }
        }
        Some(Target::Slot(slot)) => {
            let to = slot_to_index(session.from, slot);
            if to == session.from {
                return;
            }
            let allowed = match list.veto.borrow().clone() {
                Some(veto) => veto(session.from, to),
                None => true,
            };
            if allowed {
                if let Some(mover) = list.mover.borrow().clone() {
                    mover(session.from, to);
                }
            }
        }
        Some(Target::Tree(kind, item)) => {
            let source = treeview::TreeItem::from_raw(session.from as isize);
            let position = treeview::TreeDropPosition::from_parts(kind, item);
            let allowed = match list.tree_veto.borrow().clone() {
                Some(veto) => veto(source, position),
                None => true,
            };
            if allowed {
                if let Some(moved) = treeview::move_subtree(hwnd, source, position) {
                    let callbacks = list.tree_moved.borrow().clone();
                    for callback in callbacks {
                        callback(moved);
                    }
                }
            }
        }
        None => {}
    }
}

/// Selected multiple-selection ListBox/ListView items.
pub(crate) fn selection_of(kind: ListKind, hwnd: HWND) -> Vec<usize> {
    match kind {
        ListKind::ListBox => {
            if native::style(hwnd) & (LBS_MULTIPLESEL | LBS_EXTENDEDSEL) == 0 {
                return Vec::new();
            }
            (0..list_count(kind, hwnd)).filter(|&i| native::send(hwnd, LB_GETSEL, i, 0) > 0).collect()
        }
        ListKind::ListView => crate::controls::listview::selected_rows(hwnd),
        ListKind::Tree => Vec::new(),
    }
}

/// Simple `(from, to)` moves placing `items` as a block in current order before `slot` among `count` items. Empty if
/// unchanged.
pub(crate) fn block_moves(count: usize, items: &[usize], slot: usize) -> Vec<(usize, usize)> {
    let mut picked: Vec<usize> = items.iter().copied().filter(|&i| i < count).collect();
    picked.sort_unstable();
    picked.dedup();
    let slot = slot.min(count);
    let rest: Vec<usize> = (0..count).filter(|i| picked.binary_search(i).is_err()).collect();
    let before = rest.iter().take_while(|&&i| i < slot).count();
    let mut wanted = rest[..before].to_vec();
    wanted.extend_from_slice(&picked);
    wanted.extend_from_slice(&rest[before..]);
    let mut current: Vec<usize> = (0..count).collect();
    let mut moves = Vec::new();
    for (position, item) in wanted.into_iter().enumerate() {
        let Some(from) = current.iter().position(|&i| i == item) else { continue };
        if from != position {
            let value = current.remove(from);
            current.insert(position, value);
            moves.push((from, position));
        }
    }
    moves
}

/// Final index when moving from `from` to insertion `slot`.
pub(crate) fn slot_to_index(from: usize, slot: usize) -> usize {
    if slot > from {
        slot - 1
    } else {
        slot
    }
}

pub(crate) fn compute_target(list: &ListDrag, hwnd: HWND, point: POINT) -> Option<Target> {
    let from = list.session.borrow().as_ref()?.from;
    match list.kind {
        ListKind::ListBox | ListKind::ListView => {
            let slot = list_slot(list.kind, hwnd, point)?;
            (slot != from && slot != from + 1).then_some(Target::Slot(slot))
        }
        ListKind::Tree => {
            let item = tree_hit(hwnd, point);
            if item.is_null() {
                // Abaixo do último nó: vai para o fim do primeiro nível.
                let last = treeview::last_root(hwnd)?;
                let rect = tree_item_rect(hwnd, last, false)?;
                let below = point.y >= tree_bottom(hwnd).unwrap_or(rect.bottom);
                return (below && valid_tree_target(hwnd, from as HTREEITEM, 2, last))
                    .then_some(Target::Tree(2, last as isize));
            }
            let rect = tree_item_rect(hwnd, item, false)?;
            let kind = tree_zone(point.y - rect.top, rect.bottom - rect.top);
            valid_tree_target(hwnd, from as HTREEITEM, kind, item).then_some(Target::Tree(kind, item as isize))
        }
    }
}

/// Node zone: top quarter = before (1), bottom = after (2), middle = inside (0).
pub(crate) fn tree_zone(offset: i32, height: i32) -> u8 {
    if height <= 0 || offset * 4 < height {
        1
    } else if offset * 4 >= height * 3 {
        2
    } else {
        0
    }
}

pub(crate) fn valid_tree_target(hwnd: HWND, source: HTREEITEM, kind: u8, item: HTREEITEM) -> bool {
    if item == source || treeview::is_ancestor(hwnd, source, item) {
        return false;
    }
    let next = native::send(hwnd, TVM_GETNEXTITEM, TVGN_NEXT, source as isize) as HTREEITEM;
    let previous = native::send(hwnd, TVM_GETNEXTITEM, TVGN_PREVIOUS, source as isize) as HTREEITEM;
    // Soltar logo antes do seguinte ou logo depois do anterior não muda nada.
    !(kind == 1 && item == next && !next.is_null() || kind == 2 && item == previous && !previous.is_null())
}

pub(crate) fn tree_bottom(hwnd: HWND) -> Option<i32> {
    // TVGN_LASTVISIBLE
    let last = native::send(hwnd, TVM_GETNEXTITEM, 0xA, 0) as HTREEITEM;
    (!last.is_null()).then(|| tree_item_rect(hwnd, last, false).map(|r| r.bottom)).flatten()
}

/// Positions insertion line/target node highlight; returns whether highlighted.
pub(crate) fn place_marker(
    kind: ListKind,
    hwnd: HWND,
    target: Option<Target>,
    ghost: HWND,
    marker: HWND,
    hilited: bool,
) -> bool {
    let dpi = native::dpi_for_window(hwnd);
    let thickness = to_pixels(6, dpi);
    let client = native::client_rect(hwnd);
    let header = header_height(kind, hwnd);
    let mut hilite: HTREEITEM = std::ptr::null_mut();
    let line = match target {
        Some(Target::Slot(slot)) => {
            let count = list_count(kind, hwnd);
            let y = if slot < count {
                item_rect(kind, hwnd, slot).map(|r| r.top)
            } else {
                count.checked_sub(1).and_then(|last| item_rect(kind, hwnd, last)).map(|r| r.bottom)
            };
            y.map(|y| (client.left + to_pixels(2, dpi), client.right - to_pixels(2, dpi), y))
        }
        Some(Target::Tree(0, item)) => {
            hilite = item as HTREEITEM;
            None
        }
        Some(Target::Tree(kind_code, item)) => {
            let item = item as HTREEITEM;
            let full = tree_item_rect(hwnd, item, false);
            let text = tree_item_rect(hwnd, item, true);
            match (full, text) {
                (Some(full), Some(text)) => {
                    let y = if kind_code == 1 { full.top } else { full.bottom };
                    Some(((text.left - to_pixels(20, dpi)).max(client.left), client.right - to_pixels(2, dpi), y))
                }
                _ => None,
            }
        }
        None => None,
    };
    if kind == ListKind::Tree && (hilited || !hilite.is_null()) {
        native::send(hwnd, TVM_SELECTITEM, TVGN_DROPHILITE, hilite as isize);
    }
    match line {
        Some((left, right, y)) if y >= client.top + header - 1 && y <= client.bottom + 1 && right > left => {
            let mut origin = POINT { x: left, y: y - thickness / 2 };
            // SAFETY: conversão de coordenadas e posicionamento de janelas vivas.
            unsafe {
                ClientToScreen(hwnd, &mut origin);
                let width = right - left;
                set_marker_shape(marker, width, thickness, dpi);
                SetWindowPos(
                    marker,
                    HWND_TOPMOST,
                    origin.x,
                    origin.y,
                    width,
                    thickness,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                // O fantasma fica por cima da linha.
                SetWindowPos(ghost, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
            }
        }
        // SAFETY: janela própria viva.
        _ => unsafe {
            ShowWindow(marker, SW_HIDE);
        },
    }
    kind == ListKind::Tree && !hilite.is_null()
}

/// Clips insertion window to a left dot plus 2 px line.
pub(crate) fn set_marker_shape(marker: HWND, width: i32, height: i32, dpi: u32) {
    let line = to_pixels(2, dpi).max(2);
    // SAFETY: regiões criadas aqui; SetWindowRgn assume a posse da região final e as
    // auxiliares são apagadas.
    unsafe {
        let shape = CreateEllipticRgn(0, 0, height + 1, height + 1);
        let bar = CreateRectRgn(height / 2, (height - line) / 2, width, (height - line) / 2 + line);
        CombineRgn(shape, shape, bar, RGN_OR);
        DeleteObject(bar);
        SetWindowRgn(marker, shape, TRUE);
    }
}

// ---------------------------------------------------------------------------
// Geometria dos controles
// ---------------------------------------------------------------------------

pub(crate) fn list_count(kind: ListKind, hwnd: HWND) -> usize {
    let message = if kind == ListKind::ListBox { LB_GETCOUNT } else { LVM_GETITEMCOUNT };
    native::send(hwnd, message, 0, 0).max(0) as usize
}

pub(crate) fn top_index(kind: ListKind, hwnd: HWND) -> usize {
    let message = if kind == ListKind::ListBox { LB_GETTOPINDEX } else { LVM_GETTOPINDEX };
    native::send(hwnd, message, 0, 0).max(0) as usize
}

pub(crate) fn header_height(kind: ListKind, hwnd: HWND) -> i32 {
    if kind != ListKind::ListView {
        return 0;
    }
    let header = native::send(hwnd, LVM_GETHEADER, 0, 0) as HWND;
    // SAFETY: consulta simples de uma janela possivelmente nula.
    if header.is_null() || unsafe { IsWindowVisible(header) } == 0 {
        return 0;
    }
    let rect = native::window_rect(header);
    rect.bottom - rect.top
}

pub(crate) fn item_rect(kind: ListKind, hwnd: HWND, item: usize) -> Option<RECT> {
    match kind {
        ListKind::ListBox => {
            let mut rect = RECT::default();
            (native::send_mut(hwnd, LB_GETITEMRECT, item, &mut rect) != -1).then_some(rect)
        }
        ListKind::ListView => {
            let mut rect = RECT { left: LVIR_BOUNDS, ..RECT::default() };
            (native::send_mut(hwnd, LVM_GETITEMRECT, item, &mut rect) != 0).then_some(rect)
        }
        ListKind::Tree => tree_item_rect(hwnd, item as HTREEITEM, true).map(|mut rect| {
            // Inclui o ícone à esquerda do texto.
            let icon = treeview::icon_width(hwnd);
            rect.left = (rect.left - icon).max(0);
            rect.right += 4;
            rect
        }),
    }
}

/// Insertion position (0..=count) at a point for uniformly tall rows.
pub(crate) fn list_slot(kind: ListKind, hwnd: HWND, point: POINT) -> Option<usize> {
    let count = list_count(kind, hwnd);
    if count == 0 {
        return Some(0);
    }
    let top = top_index(kind, hwnd).min(count - 1);
    let rect = item_rect(kind, hwnd, top)?;
    let height = (rect.bottom - rect.top).max(1);
    let offset = point.y - rect.top;
    let slot = top as i64 + (offset + height / 2).div_euclid(height) as i64;
    Some(slot.clamp(0, count as i64) as usize)
}

/// Item under a client pixel point.
pub(crate) fn list_index_at(kind: ListKind, hwnd: HWND, point: POINT) -> Option<usize> {
    let count = list_count(kind, hwnd);
    let client = native::client_rect(hwnd);
    if count == 0
        || point.x < 0
        || point.x >= client.right
        || point.y < header_height(kind, hwnd)
        || point.y >= client.bottom
    {
        return None;
    }
    let top = top_index(kind, hwnd).min(count - 1);
    let rect = item_rect(kind, hwnd, top)?;
    let height = (rect.bottom - rect.top).max(1);
    let offset = point.y - rect.top;
    if offset < 0 {
        return None;
    }
    let index = top + (offset / height) as usize;
    (index < count).then_some(index)
}

pub(crate) fn tree_hit(hwnd: HWND, point: POINT) -> HTREEITEM {
    let mut info = TVHITTESTINFO { pt: point, flags: 0, hItem: std::ptr::null_mut() };
    native::send_mut(hwnd, TVM_HITTEST, 0, &mut info) as HTREEITEM
}

pub(crate) fn tree_item_rect(hwnd: HWND, item: HTREEITEM, text_only: bool) -> Option<RECT> {
    let mut rect = RECT::default();
    // TVM_GETITEMRECT recebe o HTREEITEM nos primeiros bytes do RECT.
    // SAFETY: o RECT tem espaço para um ponteiro (16 bytes ≥ 8).
    unsafe { *(&mut rect as *mut RECT as *mut HTREEITEM) = item };
    (native::send_mut(hwnd, TVM_GETITEMRECT, text_only as usize, &mut rect) != 0).then_some(rect)
}

pub(crate) fn scroll(kind: ListKind, hwnd: HWND, direction: i32) {
    match kind {
        ListKind::ListBox => {
            let top = top_index(kind, hwnd) as i64 + direction as i64;
            native::send(hwnd, LB_SETTOPINDEX, top.max(0) as usize, 0);
        }
        ListKind::ListView => {
            let height = item_rect(kind, hwnd, top_index(kind, hwnd)).map(|r| r.bottom - r.top).unwrap_or(16);
            native::send(hwnd, LVM_SCROLL, 0, (height * direction) as isize);
        }
        ListKind::Tree => {
            let code = if direction < 0 { SB_LINEUP } else { SB_LINEDOWN };
            native::send(hwnd, WM_VSCROLL, code as usize, 0);
        }
    }
    // SAFETY: atualiza imediatamente a janela viva antes de reposicionar a linha.
    unsafe { UpdateWindow(hwnd) };
}

/// Copies visible item area into an opaque 32-bit DIB section.
pub(crate) fn snapshot(hwnd: HWND, rect: RECT) -> Option<HBITMAP> {
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    let (bitmap, bits) = dib_section(width, height).ok()?;
    // SAFETY: DCs criados e liberados aqui; a seção tem largura × altura × 4 bytes.
    unsafe {
        let screen = GetDC(hwnd);
        let memory = CreateCompatibleDC(screen);
        let old = SelectObject(memory, bitmap);
        BitBlt(memory, 0, 0, width, height, screen, rect.left, rect.top, SRCCOPY);
        SelectObject(memory, old);
        DeleteDC(memory);
        ReleaseDC(hwnd, screen);
        // O GDI zera o alfa: torna a imagem opaca.
        let pixels = std::slice::from_raw_parts_mut(bits, width as usize * height as usize * 4);
        pixels.chunks_exact_mut(4).for_each(|p| p[3] = 255);
    }
    Some(bitmap)
}

pub(crate) fn accent_for(hwnd: HWND) -> Color {
    let theme = lookup(native::root_of(hwnd)).and_then(|core| core.theme.borrow().clone());
    match theme {
        Some(theme) if !crate::system::accessibility::high_contrast() => theme.accent,
        // SAFETY: consulta de cor do sistema.
        _ => Color::from_colorref(unsafe { GetSysColor(COLOR_HIGHLIGHT) }),
    }
}

/// Whether running on Wine, where layered windows disappear without a compositor.
pub(crate) fn running_on_wine() -> bool {
    thread_local! {
        static WINE: bool = {
            let name = to_wide("ntdll.dll");
            // SAFETY: consulta de símbolo; o módulo ntdll está sempre carregado.
            unsafe {
                let module = GetModuleHandleW(name.as_ptr());
                !module.is_null() && !GetProcAddress(module, c"wine_get_version".as_ptr() as *const u8).is_null()
            }
        };
    }
    WINE.with(|wine| *wine)
}

// ---------------------------------------------------------------------------
// Janelas auxiliares: fantasma do item e linha de inserção
// ---------------------------------------------------------------------------

pub(crate) const VISUAL_CLASS: &str = "ArqRustUI.DragVisual";

#[derive(Clone, Copy)]
pub(crate) enum Visual {
    Ghost { bitmap: HBITMAP, accent: Color },
    Marker { accent: Color },
}

thread_local! {
    static VISUALS: RefCell<HashMap<isize, Visual>> = RefCell::new(HashMap::new());
    static VISUAL_CLASS_READY: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn ensure_visual_class() -> bool {
    if VISUAL_CLASS_READY.with(Cell::get) {
        return true;
    }
    let name = to_wide(VISUAL_CLASS);
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(visual_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: native::instance(),
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: name.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };
    // SAFETY: estrutura completa; o nome vive durante a chamada.
    let ok = unsafe { RegisterClassExW(&class) } != 0
        || unsafe { crate::ffi::kernel32::GetLastError() } == crate::ffi::kernel32::ERROR_CLASS_ALREADY_EXISTS;
    VISUAL_CLASS_READY.with(|c| c.set(ok));
    ok
}

pub(crate) fn visual_window(owner: HWND, visual: Visual, translucent: bool) -> HWND {
    if !ensure_visual_class() {
        return std::ptr::null_mut();
    }
    let class = to_wide(VISUAL_CLASS);
    let mut ex = WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TRANSPARENT;
    if translucent {
        ex |= WS_EX_LAYERED;
    }
    // SAFETY: classe registrada acima; janela popup própria, destruída em `finish`.
    let hwnd = unsafe {
        CreateWindowExW(
            ex,
            class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            owner,
            std::ptr::null_mut(),
            native::instance(),
            std::ptr::null_mut(),
        )
    };
    if !hwnd.is_null() {
        VISUALS.with(|v| v.borrow_mut().insert(hwnd as isize, visual));
        if translucent {
            // SAFETY: janela em camadas recém-criada.
            unsafe { SetLayeredWindowAttributes(hwnd, 0, 200, LWA_ALPHA) };
        }
    }
    hwnd
}

/// # Safety
/// `hwnd` must be null or a window created by `visual_window` on this thread.
pub(crate) unsafe fn destroy_visual(hwnd: HWND) {
    if !hwnd.is_null() {
        VISUALS.with(|v| v.borrow_mut().remove(&(hwnd as isize)));
        DestroyWindow(hwnd);
    }
}

/// Helper window procedure.
///
/// # Safety
/// Called only by Windows on the window's owning thread.
pub(crate) unsafe extern "system" fn visual_proc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    crate::core::ffi_guard::guard(0, move || match msg {
        WM_NCHITTEST => HTTRANSPARENT,
        WM_MOUSEACTIVATE => MA_NOACTIVATE,
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !hdc.is_null() {
                let client = native::client_rect(hwnd);
                match VISUALS.with(|v| v.borrow().get(&(hwnd as isize)).copied()) {
                    Some(Visual::Ghost { bitmap, accent }) => {
                        if !bitmap.is_null() {
                            let memory = CreateCompatibleDC(hdc);
                            let old = SelectObject(memory, bitmap);
                            BitBlt(hdc, 0, 0, client.right, client.bottom, memory, 0, 0, SRCCOPY);
                            SelectObject(memory, old);
                            DeleteDC(memory);
                        }
                        FrameRect(hdc, &client, app::brush_for(accent));
                    }
                    Some(Visual::Marker { accent }) => {
                        FillRect(hdc, &client, app::brush_for(accent));
                    }
                    None => {}
                }
            }
            EndPaint(hwnd, &ps);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    })
}
