//! Hierarchical tree (TreeView).

use super::common::{builder_common, widget_type, ControlOptions};
use super::icon_support::{ManagedImages, DEFAULT_ICON_SIZE};
use crate::core::handles::IconSource;
use crate::core::handles::ImageList;
use crate::core::native;
use crate::core::widget::Reflected;
use crate::core::{Event, EventKind, GuiError, GuiResult, HandlerId};
use crate::ffi::comctl32::*;
use crate::ffi::user32::WS_EX_CLIENTEDGE;
use crate::ffi::{HIMAGELIST, HTREEITEM, HWND, POINT};
use crate::input::drag::{self, ListKind};
use crate::system::ClipboardContent;
use crate::utilities::{from_wide, to_wide};
use crate::utilities::{to_pixels, Point};
use std::rc::Rc;

/// [`TreeView`] node identifier, valid only in its originating tree while the node exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TreeItem(isize);

impl TreeItem {
    fn raw(self) -> HTREEITEM {
        self.0 as HTREEITEM
    }

    fn from_result(value: isize) -> Option<TreeItem> {
        (value != 0).then_some(TreeItem(value))
    }

    /// Raw `HTREEITEM` for direct Win32 calls.
    pub fn handle(self) -> isize {
        self.0
    }

    pub(crate) fn from_raw(value: isize) -> TreeItem {
        TreeItem(value)
    }
}

/// Node destination when dragging or using [`TreeView::move_item`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeDropPosition {
    /// As the node's last child.
    Inside(TreeItem),
    /// As a sibling immediately before the node.
    Before(TreeItem),
    /// As a sibling immediately after the node.
    After(TreeItem),
}

impl TreeDropPosition {
    pub(crate) fn from_parts(kind: u8, item: isize) -> TreeDropPosition {
        match kind {
            1 => TreeDropPosition::Before(TreeItem(item)),
            2 => TreeDropPosition::After(TreeItem(item)),
            _ => TreeDropPosition::Inside(TreeItem(item)),
        }
    }

    /// Position reference node.
    pub fn item(self) -> TreeItem {
        match self {
            TreeDropPosition::Inside(item) | TreeDropPosition::Before(item) | TreeDropPosition::After(item) => item,
        }
    }
}

const TVIS_BOLD: u32 = 0x10;

fn next_of(hwnd: HWND, relation: usize, item: HTREEITEM) -> HTREEITEM {
    native::send(hwnd, TVM_GETNEXTITEM, relation, item as isize) as HTREEITEM
}

/// Whether `ancestor` is an ancestor of `item`.
pub(crate) fn is_ancestor(hwnd: HWND, ancestor: HTREEITEM, item: HTREEITEM) -> bool {
    let mut current = next_of(hwnd, TVGN_PARENT, item);
    while !current.is_null() {
        if current == ancestor {
            return true;
        }
        current = next_of(hwnd, TVGN_PARENT, current);
    }
    false
}

pub(crate) fn is_expanded(hwnd: HWND, item: HTREEITEM) -> bool {
    native::send(hwnd, TVM_GETITEMSTATE, item as usize, TVIS_EXPANDED as isize) as u32 & TVIS_EXPANDED != 0
}

/// Last top-level node.
pub(crate) fn last_root(hwnd: HWND) -> Option<HTREEITEM> {
    let mut current = next_of(hwnd, TVGN_ROOT, std::ptr::null_mut());
    let mut last = None;
    while !current.is_null() {
        last = Some(current);
        current = next_of(hwnd, TVGN_NEXT, current);
    }
    last
}

/// Pixels reserved for node icons plus text spacing.
pub(crate) fn icon_width(hwnd: HWND) -> i32 {
    let images = native::send(hwnd, TVM_GETIMAGELIST, TVSIL_NORMAL, 0) as HIMAGELIST;
    if images.is_null() {
        return 0;
    }
    let Some(functions) = functions() else { return 0 };
    let (mut width, mut height) = (0, 0);
    // SAFETY: lista de imagens viva da árvore; saídas locais.
    unsafe { (functions.ImageList_GetIconSize)(images, &mut width, &mut height) };
    width + 3
}

/// Copies `source` and its subtree to the requested position.
fn copy_subtree(hwnd: HWND, source: HTREEITEM, parent: HTREEITEM, after: HTREEITEM) -> HTREEITEM {
    let mut buffer = vec![0u16; 1024];
    let mut item = blank_item();
    item.mask = TVIF_TEXT | TVIF_IMAGE | TVIF_SELECTEDIMAGE | TVIF_STATE | TVIF_PARAM | TVIF_HANDLE;
    item.hItem = source;
    item.stateMask = TVIS_STATEIMAGEMASK | TVIS_BOLD | TVIS_EXPANDED;
    item.pszText = buffer.as_mut_ptr();
    item.cchTextMax = buffer.len() as i32;
    native::send_mut(hwnd, TVM_GETITEMW, 0, &mut item);
    let expanded = item.state & TVIS_EXPANDED != 0;
    let mut base = item;
    base.mask = TVIF_TEXT | TVIF_IMAGE | TVIF_SELECTEDIMAGE | TVIF_STATE | TVIF_PARAM;
    base.hItem = std::ptr::null_mut();
    base.stateMask = TVIS_STATEIMAGEMASK | TVIS_BOLD;
    base.state &= TVIS_STATEIMAGEMASK | TVIS_BOLD;
    let insert = TVINSERTSTRUCTW {
        hParent: parent,
        hInsertAfter: after,
        item: TVITEMEXW {
            base,
            iIntegral: 0,
            uStateEx: 0,
            hwnd: std::ptr::null_mut(),
            iExpandedImage: 0,
            iReserved: 0,
        },
    };
    let copy = native::send_ref(hwnd, TVM_INSERTITEMW, 0, &insert) as HTREEITEM;
    if copy.is_null() {
        return copy;
    }
    let mut child = next_of(hwnd, TVGN_CHILD, source);
    while !child.is_null() {
        copy_subtree(hwnd, child, copy, TVI_LAST);
        child = next_of(hwnd, TVGN_NEXT, child);
    }
    if expanded {
        native::send(hwnd, TVM_EXPAND, TVE_EXPAND, copy as isize);
    }
    copy
}

/// Moves `source` and descendants to `position`; returns the new identifier.
pub(crate) fn move_subtree(hwnd: HWND, source: TreeItem, position: TreeDropPosition) -> Option<TreeItem> {
    let source = source.raw();
    let target = position.item().raw();
    if source.is_null() || target.is_null() || source == target || is_ancestor(hwnd, source, target) {
        return None;
    }
    let (parent, after) = match position {
        TreeDropPosition::Inside(_) => (target, TVI_LAST),
        TreeDropPosition::After(_) => (next_of(hwnd, TVGN_PARENT, target), target),
        TreeDropPosition::Before(_) => {
            let previous = next_of(hwnd, TVGN_PREVIOUS, target);
            (next_of(hwnd, TVGN_PARENT, target), if previous.is_null() { TVI_FIRST } else { previous })
        }
    };
    let parent = if parent.is_null() { TVI_ROOT } else { parent };
    // Se o "anterior" é o próprio nó movido, insere depois do anterior dele.
    let after = if after == source {
        let previous = next_of(hwnd, TVGN_PREVIOUS, source);
        if previous.is_null() {
            TVI_FIRST
        } else {
            previous
        }
    } else {
        after
    };
    let selected = next_of(hwnd, TVGN_CARET, std::ptr::null_mut());
    let was_selected = selected == source || (!selected.is_null() && is_ancestor(hwnd, source, selected));
    let copy = copy_subtree(hwnd, source, parent, after);
    if copy.is_null() {
        return None;
    }
    native::send(hwnd, TVM_DELETEITEM, 0, source as isize);
    if was_selected {
        native::send(hwnd, TVM_SELECTITEM, TVGN_CARET, copy as isize);
    }
    if parent != TVI_ROOT {
        native::send(hwnd, TVM_EXPAND, TVE_EXPAND, parent as isize);
    }
    native::send(hwnd, TVM_ENSUREVISIBLE, 0, copy as isize);
    Some(TreeItem(copy as isize))
}

widget_type!(
    /// Hierarchical tree of expandable items.
    ///
    /// ```no_run
    /// # use arqrustui::prelude::*;
    /// # fn demo(window: &Window) -> GuiResult<()> {
    /// let tree = TreeView::builder().bounds(10, 10, 240, 300).parent(window).build()?;
    /// let root = tree.add_root("Project")?;
    /// let src = tree.add_child(root, "src")?;
    /// tree.add_child(src, "main.rs")?;
    /// tree.expand(root);
    /// # Ok(()) }
    /// ```
    TreeView
);

/// Builder for [`TreeView`].
#[derive(Clone, Debug)]
pub struct TreeViewBuilder {
    opts: ControlOptions,
    checkboxes: bool,
}

impl TreeViewBuilder {
    builder_common!();

    /// Shows a check box on each node.
    pub fn checkboxes(mut self, checkboxes: bool) -> Self {
        self.checkboxes = checkboxes;
        self
    }

    /// Creates the tree.
    pub fn build(self) -> GuiResult<TreeView> {
        let mut style = TVS_HASBUTTONS | TVS_HASLINES | TVS_LINESATROOT | TVS_SHOWSELALWAYS;
        if self.checkboxes {
            style |= TVS_CHECKBOXES;
        }
        let core = self.opts.create("TreeView", "SysTreeView32", style, WS_EX_CLIENTEDGE, true, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Notify { code, .. } if *code == TVN_SELCHANGEDW => {
                core.emit(&Event::SelectionChanged);
                Some(0)
            }
            Reflected::Notify { code, .. } if *code == TVN_ITEMEXPANDEDW => {
                core.emit(&Event::ValueChanged);
                Some(0)
            }
            Reflected::Notify { code, .. } if *code == NM_DBLCLK => {
                core.emit(&Event::DoubleClick);
                None
            }
            _ => None,
        });
        Ok(TreeView { core })
    }
}

fn blank_item() -> TVITEMW {
    TVITEMW {
        mask: 0,
        hItem: std::ptr::null_mut(),
        state: 0,
        stateMask: 0,
        pszText: std::ptr::null_mut(),
        cchTextMax: 0,
        iImage: 0,
        iSelectedImage: 0,
        cChildren: 0,
        lParam: 0,
    }
}

impl TreeView {
    /// Starts building (default size 220×260).
    pub fn builder() -> TreeViewBuilder {
        TreeViewBuilder { opts: ControlOptions::new(220, 260), checkboxes: false }
    }

    fn insert(&self, parent: HTREEITEM, text: &str) -> GuiResult<TreeItem> {
        let hwnd = self.core.require_alive()?;
        let mut wide = to_wide(text);
        let mut base = blank_item();
        // Sem imagem até que uma seja atribuída (senão o nó mostraria a imagem 0 da lista).
        base.mask = TVIF_TEXT | TVIF_IMAGE | TVIF_SELECTEDIMAGE;
        base.iImage = I_IMAGENONE;
        base.iSelectedImage = I_IMAGENONE;
        base.pszText = wide.as_mut_ptr();
        let insert = TVINSERTSTRUCTW {
            hParent: parent,
            hInsertAfter: TVI_LAST,
            item: TVITEMEXW {
                base,
                iIntegral: 0,
                uStateEx: 0,
                hwnd: std::ptr::null_mut(),
                iExpandedImage: 0,
                iReserved: 0,
            },
        };
        TreeItem::from_result(native::send_ref(hwnd, TVM_INSERTITEMW, 0, &insert))
            .ok_or_else(|| GuiError::InvalidArgument("could not insert the node into the tree".into()))
    }

    /// Adds a top-level node.
    pub fn add_root(&self, text: &str) -> GuiResult<TreeItem> {
        self.insert(TVI_ROOT, text)
    }

    /// Adds a child to the given node.
    pub fn add_child(&self, parent: TreeItem, text: &str) -> GuiResult<TreeItem> {
        self.insert(parent.raw(), text)
    }

    /// Removes a node and all descendants.
    pub fn remove(&self, item: TreeItem) {
        native::send(self.core.hwnd(), TVM_DELETEITEM, 0, item.0);
    }

    /// Removes all nodes.
    pub fn clear(&self) {
        native::send(self.core.hwnd(), TVM_DELETEITEM, 0, TVI_ROOT as isize);
    }

    /// Total node count.
    pub fn count(&self) -> usize {
        native::send(self.core.hwnd(), TVM_GETCOUNT, 0, 0).max(0) as usize
    }

    fn next(&self, relation: usize, item: isize) -> Option<TreeItem> {
        TreeItem::from_result(native::send(self.core.hwnd(), TVM_GETNEXTITEM, relation, item))
    }

    /// Top-level nodes.
    pub fn roots(&self) -> Vec<TreeItem> {
        self.siblings_from(self.next(TVGN_ROOT, 0))
    }

    /// Direct children of a node.
    pub fn children(&self, item: TreeItem) -> Vec<TreeItem> {
        self.siblings_from(self.next(TVGN_CHILD, item.0))
    }

    fn siblings_from(&self, first: Option<TreeItem>) -> Vec<TreeItem> {
        let mut items = Vec::new();
        let mut current = first;
        while let Some(item) = current {
            items.push(item);
            current = self.next(TVGN_NEXT, item.0);
        }
        items
    }

    /// Node parent (`None` for top-level nodes).
    pub fn parent_of(&self, item: TreeItem) -> Option<TreeItem> {
        self.next(TVGN_PARENT, item.0)
    }

    /// Node text.
    pub fn item_text(&self, item: TreeItem) -> String {
        let mut buffer = vec![0u16; 512];
        let mut native_item = blank_item();
        native_item.mask = TVIF_TEXT | TVIF_HANDLE;
        native_item.hItem = item.raw();
        native_item.pszText = buffer.as_mut_ptr();
        native_item.cchTextMax = buffer.len() as i32;
        native::send_mut(self.core.hwnd(), TVM_GETITEMW, 0, &mut native_item);
        from_wide(&buffer)
    }

    /// Changes node text.
    pub fn set_item_text(&self, item: TreeItem, text: &str) {
        let mut wide = to_wide(text);
        let mut native_item = blank_item();
        native_item.mask = TVIF_TEXT | TVIF_HANDLE;
        native_item.hItem = item.raw();
        native_item.pszText = wide.as_mut_ptr();
        native::send_ref(self.core.hwnd(), TVM_SETITEMW, 0, &native_item);
    }

    /// Text path from root to node (e.g. `["Project", "src", "main.rs"]`).
    pub fn item_path(&self, item: TreeItem) -> Vec<String> {
        let mut path = vec![self.item_text(item)];
        let mut current = item;
        while let Some(parent) = self.parent_of(current) {
            path.push(self.item_text(parent));
            current = parent;
        }
        path.reverse();
        path
    }

    /// Expands a node.
    pub fn expand(&self, item: TreeItem) {
        native::send(self.core.hwnd(), TVM_EXPAND, TVE_EXPAND, item.0);
    }

    /// Collapses a node.
    pub fn collapse(&self, item: TreeItem) {
        native::send(self.core.hwnd(), TVM_EXPAND, TVE_COLLAPSE, item.0);
    }

    /// Selected node, if any.
    pub fn selected(&self) -> Option<TreeItem> {
        self.next(TVGN_CARET, 0)
    }

    /// Selects a node and makes it visible (`None` clears selection).
    pub fn select(&self, item: Option<TreeItem>) {
        let raw = item.map(|i| i.0).unwrap_or(0);
        native::send(self.core.hwnd(), TVM_SELECTITEM, TVGN_CARET, raw);
        if raw != 0 {
            native::send(self.core.hwnd(), TVM_ENSUREVISIBLE, 0, raw);
        }
    }

    /// Whether the node check box is checked (`checkboxes` trees).
    pub fn is_checked(&self, item: TreeItem) -> bool {
        let mut native_item = blank_item();
        native_item.mask = TVIF_STATE | TVIF_HANDLE;
        native_item.hItem = item.raw();
        native_item.stateMask = TVIS_STATEIMAGEMASK;
        native::send_mut(self.core.hwnd(), TVM_GETITEMW, 0, &mut native_item);
        (native_item.state >> 12) == 2
    }

    /// Checks/unchecks a node check box.
    pub fn set_checked(&self, item: TreeItem, checked: bool) {
        let mut native_item = blank_item();
        native_item.mask = TVIF_STATE | TVIF_HANDLE;
        native_item.hItem = item.raw();
        native_item.stateMask = TVIS_STATEIMAGEMASK;
        native_item.state = if checked { 2 << 12 } else { 1 << 12 };
        native::send_ref(self.core.hwnd(), TVM_SETITEMW, 0, &native_item);
    }

    /// Attaches a custom image list, replacing the list managed by [`set_item_icon`](Self::set_item_icon).
    pub fn set_image_list(&self, images: &Rc<ImageList>) {
        ManagedImages::release(&self.core);
        native::send(self.core.hwnd(), TVM_SETIMAGELIST, TVSIL_NORMAL, images.raw() as isize);
        self.core.keep_alive(images.clone());
    }

    /// Sets a node image (image list index).
    pub fn set_item_image(&self, item: TreeItem, image: i32) {
        let mut native_item = blank_item();
        native_item.mask = TVIF_IMAGE | TVIF_SELECTEDIMAGE | TVIF_HANDLE;
        native_item.hItem = item.raw();
        native_item.iImage = image;
        native_item.iSelectedImage = image;
        native::send_ref(self.core.hwnd(), TVM_SETITEMW, 0, &native_item);
    }

    /// Shows a node icon.
    ///
    /// The tree maintains a DPI-sized image list with one slot per distinct icon.
    pub fn set_item_icon(&self, item: TreeItem, icon: &impl IconSource) -> GuiResult<()> {
        let index = self.icon_index(icon)?;
        self.set_item_image_pair(item, index, index);
        Ok(())
    }

    /// Shows different normal/selected icons, such as closed/open folders.
    pub fn set_item_icons(
        &self,
        item: TreeItem,
        normal: &impl IconSource,
        selected: &impl IconSource,
    ) -> GuiResult<()> {
        let normal = self.icon_index(normal)?;
        let selected = self.icon_index(selected)?;
        self.set_item_image_pair(item, normal, selected);
        Ok(())
    }

    /// Removes a node icon.
    pub fn clear_item_icon(&self, item: TreeItem) {
        self.set_item_image_pair(item, I_IMAGENONE, I_IMAGENONE);
    }

    /// Logical node icon size (default 16). Reloads used icons.
    pub fn set_icon_size(&self, size: i32) -> GuiResult<()> {
        ManagedImages::resize(&self.core, size.max(1), attach_images)
    }

    fn icon_index(&self, icon: &impl IconSource) -> GuiResult<i32> {
        let size = ManagedImages::logical_size(&self.core).unwrap_or(DEFAULT_ICON_SIZE);
        ManagedImages::index_of(&self.core, icon, size, attach_images)
    }

    fn set_item_image_pair(&self, item: TreeItem, normal: i32, selected: i32) {
        let mut native_item = blank_item();
        native_item.mask = TVIF_IMAGE | TVIF_SELECTEDIMAGE | TVIF_HANDLE;
        native_item.hItem = item.raw();
        native_item.iImage = normal;
        native_item.iSelectedImage = selected;
        native::send_ref(self.core.hwnd(), TVM_SETITEMW, 0, &native_item);
    }

    /// Node image index (`None` without an image).
    pub fn item_image(&self, item: TreeItem) -> Option<i32> {
        let mut native_item = blank_item();
        native_item.mask = TVIF_IMAGE | TVIF_HANDLE;
        native_item.hItem = item.raw();
        native::send_mut(self.core.hwnd(), TVM_GETITEMW, 0, &mut native_item);
        (native_item.iImage >= 0).then_some(native_item.iImage)
    }

    /// Moves a node and descendants and returns its new identifier; the old identifier becomes invalid.
    ///
    /// Fails if the destination is the node itself or its descendant.
    pub fn move_item(&self, item: TreeItem, position: TreeDropPosition) -> GuiResult<TreeItem> {
        let hwnd = self.core.require_alive()?;
        move_subtree(hwnd, item, position)
            .ok_or_else(|| GuiError::InvalidArgument("a node cannot be moved into itself".into()))
    }

    /// Enables/disables mouse drag-reordering of nodes.
    ///
    /// The node follows the cursor; dropping in a node's middle makes it a child (highlighted target), near top/bottom
    /// inserts before/after (insertion line). Hovering expands closed nodes; edges scroll; Esc cancels.
    pub fn set_drag_enabled(&self, enabled: bool) {
        let config = drag::list_config(&self.core, ListKind::Tree);
        config.reorder.set(enabled);
    }

    /// Called before moving a dropped node; return `false` to reject, e.g. to accept only folders as targets.
    pub fn on_item_drop_request(&self, f: impl Fn(TreeItem, TreeDropPosition) -> bool + 'static) {
        *drag::list_config(&self.core, ListKind::Tree).tree_veto.borrow_mut() = Some(Rc::new(f));
    }

    /// Former name of [`on_item_drop_request`](Self::on_item_drop_request), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_item_drop_request`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_item_drop(&self, f: impl Fn(TreeItem, TreeDropPosition) -> bool + 'static) {
        self.on_item_drop_request(f)
    }

    /// Called after a drag moves a node; receives its new identifier.
    pub fn on_item_moved(&self, f: impl Fn(TreeItem) + 'static) {
        drag::list_config(&self.core, ListKind::Tree).tree_moved.borrow_mut().push(Rc::new(f));
    }

    /// Allows dragging nodes to another control/program.
    ///
    /// On leaving the tree, `f` receives the node and returns drag content (`None` keeps internal dragging).
    pub fn on_drag_out(&self, f: impl Fn(TreeItem) -> Option<ClipboardContent> + 'static) {
        let provider = move |raw: usize| f(TreeItem(raw as isize));
        *drag::list_config(&self.core, ListKind::Tree).drag_out.borrow_mut() = Some(Rc::new(provider));
    }

    /// Node at a logical client point, e.g. [`DropData::position`](crate::input::DropData::position).
    pub fn item_at(&self, point: Point) -> Option<TreeItem> {
        let hwnd = self.core.hwnd();
        let dpi = native::dpi_for_window(hwnd);
        let pixels = POINT { x: to_pixels(point.x, dpi), y: to_pixels(point.y, dpi) };
        TreeItem::from_result(drag::tree_hit(hwnd, pixels) as isize)
    }

    /// Selection changed; receives the selected node.
    pub fn on_selection_changed(&self, f: impl Fn(Option<TreeItem>) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::SelectionChanged, move |_| f(this.selected()))
    }
}

fn attach_images(hwnd: crate::ffi::HWND, list: &ImageList) {
    native::send(hwnd, TVM_SETIMAGELIST, TVSIL_NORMAL, list.raw() as isize);
}
