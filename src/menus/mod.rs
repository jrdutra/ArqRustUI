//! Menus: bars, submenus, context/system menus.
//!
//! Two ways to display the same [`Menu`]s:
//!
//! * [`MenuBar`] — Windows menu bar (`HMENU` + `SetMenu`) with system appearance;
//! * [`MenuStrip`] — library-drawn nonclient bar with custom popup windows, bundled icons at any DPI,
//!   light/dark/high-contrast themes, keyboard navigation and overflow (`»`).
//!
//! Build [`Menu`] items once for either bar and context menus; [`ContextMenu::set_drawn`] uses MenuStrip drawing.
//!
//! ```no_run
//! # use arqrustui::prelude::*;
//! # fn demo(window: &Window) -> GuiResult<()> {
//! let file = Menu::new()?;
//! file.add_item_with_shortcut("&Open...", Shortcut::ctrl(Key(b'O' as u16)), || println!("open"))?;
//! file.add_separator()?;
//! let wrap = file.add_check_item("&Word wrap", true, |on| println!("wrap: {on}"))?;
//! wrap.set_enabled(false);
//!
//! let bar = MenuBar::new()?;
//! bar.add_menu("&File", &file)?;
//! window.set_menu(&bar)?;
//! # Ok(()) }
//! ```

pub(crate) mod popup;
pub mod strip;

pub use strip::{MenuStrip, MenuStyle};

#[doc(hidden)]
pub use popup::debug_state;

use crate::core::application as app;
use crate::core::handles::IconSource;
use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult, Window};
use crate::ffi::user32::*;
use crate::ffi::{HMENU, HWND, POINT, TRUE};
use crate::graphics::Bitmap;
use crate::input::Shortcut;
use crate::utilities::{from_wide, to_pixels, to_wide, Point};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

thread_local! {
    /// Live thread menus by `HMENU` for `WM_INITMENUPOPUP`.
    static MENUS: RefCell<HashMap<isize, Weak<MenuInner>>> = RefCell::new(HashMap::new());
}

type OpenFn = Rc<dyn Fn(&Menu)>;
type HighlightFn = Rc<dyn Fn(Option<MenuItem>)>;

/// Last highlight's notified menus and highlighted (menu, id).
type Highlighted = (Vec<Weak<MenuInner>>, Option<(isize, u32)>);

thread_local! {
    /// Last notified menus, to receive `None` on closing.
    static HIGHLIGHTED: RefCell<Highlighted> = const { RefCell::new((Vec::new(), None)) };
}
/// Item shortcut: key, handler and identifier.
type ItemShortcut = (Shortcut, Rc<dyn Fn()>, u32);

pub(crate) struct MenuInner {
    handle: HMENU,
    /// `true` when Windows owns `HMENU` destruction: attached submenu/bar or system menu.
    system_owned: Cell<bool>,
    ids: RefCell<Vec<u32>>,
    children: RefCell<Vec<Rc<MenuInner>>>,
    shortcuts: RefCell<Vec<ItemShortcut>>,
    bitmaps: RefCell<Vec<Bitmap>>,
    /// Original command item icons by identifier, reloaded at exact DPI by drawn bars.
    icons: RefCell<HashMap<u32, Rc<dyn IconSource>>>,
    /// Submenu-opening item icons by submenu `HMENU`.
    submenu_icons: RefCell<HashMap<isize, Rc<dyn IconSource>>>,
    on_open: RefCell<Option<OpenFn>>,
    on_highlight: RefCell<Option<HighlightFn>>,
}

/// Parsed menu entry type.
pub(crate) enum EntryKind {
    Command(u32),
    Submenu(Rc<MenuInner>),
    Separator,
}

/// Menu entry snapshot at opening.
pub(crate) struct Entry {
    pub kind: EntryKind,
    /// Mnemonic `&` text without shortcut.
    pub text: String,
    /// Shortcut text after `\t`.
    pub shortcut: String,
    pub enabled: bool,
    pub checked: bool,
    pub radio: bool,
    pub default: bool,
    pub icon: Option<Rc<dyn IconSource>>,
}

impl Entry {
    pub(crate) fn is_separator(&self) -> bool {
        matches!(self.kind, EntryKind::Separator)
    }

    /// Lowercase mnemonic letter if text has `&`.
    pub(crate) fn mnemonic(&self) -> Option<char> {
        mnemonic_of(&self.text)
    }
}

/// Letter after first isolated `&` (`&&` is literal). Removes mnemonic markers (`&File` → `File`, `&&` → `&`).
pub(crate) fn plain_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                out.push('&');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

pub(crate) fn mnemonic_of(text: &str) -> Option<char> {
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.next() {
                Some('&') => continue,
                Some(next) => return next.to_lowercase().next(),
                None => return None,
            }
        }
    }
    None
}

impl MenuInner {
    fn create(popup: bool) -> GuiResult<Rc<MenuInner>> {
        app::require()?;
        // SAFETY: criação simples de menu; falha é sinalizada por retorno nulo.
        let handle = unsafe {
            if popup {
                CreatePopupMenu()
            } else {
                CreateMenu()
            }
        };
        if handle.is_null() {
            return Err(GuiError::last_os_error("CreateMenu"));
        }
        Ok(MenuInner::wrap(handle, false))
    }

    fn wrap(handle: HMENU, system_owned: bool) -> Rc<MenuInner> {
        let inner = Rc::new(MenuInner {
            handle,
            system_owned: Cell::new(system_owned),
            ids: RefCell::new(Vec::new()),
            children: RefCell::new(Vec::new()),
            shortcuts: RefCell::new(Vec::new()),
            bitmaps: RefCell::new(Vec::new()),
            icons: RefCell::new(HashMap::new()),
            submenu_icons: RefCell::new(HashMap::new()),
            on_open: RefCell::new(None),
            on_highlight: RefCell::new(None),
        });
        MENUS.with(|m| m.borrow_mut().insert(handle as isize, Rc::downgrade(&inner)));
        inner
    }

    /// Finds item with `shortcut` in this menu/submenus.
    pub(crate) fn find_shortcut(self: &Rc<Self>, shortcut: Shortcut) -> Option<(Rc<MenuInner>, u32)> {
        if let Some(id) = self.shortcuts.borrow().iter().find(|(s, _, _)| *s == shortcut).map(|(_, _, id)| *id) {
            return Some((self.clone(), id));
        }
        let children = self.children.borrow().clone();
        children.iter().find_map(|child| child.find_shortcut(shortcut))
    }

    /// Whether command `id` is enabled.
    pub(crate) fn is_enabled(&self, id: u32) -> bool {
        // SAFETY: consulta simples a um menu vivo.
        let state = unsafe { GetMenuState(self.handle, id, MF_BYCOMMAND) };
        state != u32::MAX && state & MF_GRAYED == 0
    }

    /// Calls registered [`Menu::on_open`] handler, if any.
    pub(crate) fn notify_open(self: &Rc<Self>) {
        let handler = self.on_open.borrow().clone();
        if let Some(handler) = handler {
            handler(&Menu { inner: self.clone() });
        }
    }

    /// Reads current entries: text, state, submenus, icons.
    pub(crate) fn entries(self: &Rc<Self>) -> Vec<Entry> {
        // SAFETY: consulta simples a um menu vivo.
        let count = unsafe { GetMenuItemCount(self.handle) }.max(0) as u32;
        (0..count).filter_map(|position| self.entry_at(position)).collect()
    }

    fn entry_at(self: &Rc<Self>, position: u32) -> Option<Entry> {
        let mut info = empty_item_info(MIIM_FTYPE | MIIM_STATE | MIIM_ID | MIIM_SUBMENU | MIIM_STRING);
        // SAFETY: primeira chamada com buffer nulo: só informa o tamanho do texto em `cch`.
        if unsafe { GetMenuItemInfoW(self.handle, position, TRUE, &mut info) } == 0 {
            return None;
        }
        let mut buffer = vec![0u16; info.cch as usize + 1];
        if info.cch > 0 {
            info.dwTypeData = buffer.as_mut_ptr();
            info.cch += 1;
            // SAFETY: o buffer tem `cch` unidades e vive durante a chamada.
            if unsafe { GetMenuItemInfoW(self.handle, position, TRUE, &mut info) } == 0 {
                return None;
            }
        }
        let full = from_wide(&buffer);
        let (text, shortcut) = match full.split_once('\t') {
            Some((text, shortcut)) => (text.to_string(), shortcut.to_string()),
            None => (full, String::new()),
        };
        let kind = if info.fType & MFT_SEPARATOR != 0 {
            EntryKind::Separator
        } else if !info.hSubMenu.is_null() {
            let child = self.children.borrow().iter().find(|c| c.handle == info.hSubMenu).cloned();
            EntryKind::Submenu(child.unwrap_or_else(|| MenuInner::wrap(info.hSubMenu, true)))
        } else {
            EntryKind::Command(info.wID)
        };
        let icon = match &kind {
            EntryKind::Command(id) => self.icons.borrow().get(id).cloned(),
            EntryKind::Submenu(child) => self.submenu_icons.borrow().get(&(child.handle as isize)).cloned(),
            EntryKind::Separator => None,
        };
        Some(Entry {
            kind,
            text,
            shortcut,
            enabled: info.fState & MFS_GRAYED == 0,
            checked: info.fState & MFS_CHECKED != 0,
            radio: info.fType & MFT_RADIOCHECK != 0,
            default: info.fState & MFS_DEFAULT != 0,
            icon,
        })
    }

    /// Command `id` position/index in this menu.
    fn position_of(&self, id: u32) -> Option<u32> {
        // SAFETY: consultas simples a um menu vivo.
        let count = unsafe { GetMenuItemCount(self.handle) }.max(0);
        (0..count).find(|&p| unsafe { GetMenuItemID(self.handle, p) } == id).map(|p| p as u32)
    }

    /// Whether entry is radio (`MFT_RADIOCHECK`).
    fn is_radio_at(&self, position: u32) -> bool {
        let mut info = empty_item_info(MIIM_FTYPE | MIIM_SUBMENU);
        // SAFETY: estrutura local corretamente dimensionada, sem buffer de texto.
        let found = unsafe { GetMenuItemInfoW(self.handle, position, TRUE, &mut info) } != 0;
        found && info.fType & MFT_RADIOCHECK != 0 && info.hSubMenu.is_null()
    }

    /// Checks radio `id`, unchecking consecutive radio peers bounded by separators/other item types.
    fn select_radio(&self, id: u32) {
        let Some(position) = self.position_of(id) else { return };
        // SAFETY: consulta simples.
        let count = unsafe { GetMenuItemCount(self.handle) }.max(0) as u32;
        let mut first = position;
        while first > 0 && self.is_radio_at(first - 1) {
            first -= 1;
        }
        let mut last = position;
        while last + 1 < count && self.is_radio_at(last + 1) {
            last += 1;
        }
        // SAFETY: posições válidas dentro do menu vivo.
        unsafe { CheckMenuRadioItem(self.handle, first, last, position, MF_BYPOSITION) };
    }
}

/// Empty `MENUITEMINFOW` with given mask.
fn empty_item_info(mask: u32) -> MENUITEMINFOW {
    MENUITEMINFOW {
        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
        fMask: mask,
        fType: 0,
        fState: 0,
        wID: 0,
        hSubMenu: std::ptr::null_mut(),
        hbmpChecked: std::ptr::null_mut(),
        hbmpUnchecked: std::ptr::null_mut(),
        dwItemData: 0,
        dwTypeData: std::ptr::null_mut(),
        cch: 0,
        hbmpItem: std::ptr::null_mut(),
    }
}

/// Icon bitmap at system small-icon size for native menus.
fn menu_bitmap(icon: &dyn IconSource) -> GuiResult<Bitmap> {
    // SAFETY: consulta de métrica do sistema.
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16);
    Bitmap::from_icon(&icon.icon_pixels(size)?, size)
}

/// Live menu by `HMENU`.
pub(crate) fn menu_of(handle: HMENU) -> Option<Rc<MenuInner>> {
    MENUS.with(|m| m.borrow().get(&(handle as isize)).and_then(Weak::upgrade))
}

/// Menu and ancestors from innermost outward.
fn chain_of(menu: &Rc<MenuInner>) -> Vec<Rc<MenuInner>> {
    let all: Vec<Rc<MenuInner>> = MENUS.with(|m| m.borrow().values().filter_map(Weak::upgrade).collect());
    let mut chain = vec![menu.clone()];
    while let Some(parent) = {
        let last = chain.last().map(|m| m.handle);
        all.iter().find(|candidate| {
            candidate.children.borrow().iter().any(|c| Some(c.handle) == last)
                && !chain.iter().any(|m| Rc::ptr_eq(m, candidate))
        })
    } {
        chain.push(parent.clone());
    }
    chain
}

/// Notifies menu/ancestor `on_highlight_changed` of `id`; `None` for separator, submenu or nothing.
pub(crate) fn notify_highlight(menu: &Rc<MenuInner>, id: Option<u32>) {
    let key = id.map(|id| (menu.handle as isize, id));
    let repeated = HIGHLIGHTED.with(|h| h.borrow().1 == key && key.is_some());
    if repeated {
        return;
    }
    let chain = chain_of(menu);
    HIGHLIGHTED.with(|h| *h.borrow_mut() = (chain.iter().map(Rc::downgrade).collect(), key));
    for owner in chain {
        let handler = owner.on_highlight.borrow().clone();
        if let Some(handler) = handler {
            handler(id.map(|id| MenuItem { menu: menu.clone(), id }));
        }
    }
}

/// On closing, previously notified menus receive `None`.
pub(crate) fn notify_menu_closed() {
    let (menus, last) = HIGHLIGHTED.with(|h| std::mem::take(&mut *h.borrow_mut()));
    if last.is_none() && menus.is_empty() {
        return;
    }
    for menu in menus.iter().filter_map(Weak::upgrade) {
        let handler = menu.on_highlight.borrow().clone();
        if let Some(handler) = handler {
            handler(None);
        }
    }
}

/// Native menu `WM_MENUSELECT`.
pub(crate) fn native_menu_select(wp: usize, lp: isize) {
    let flags = (wp >> 16) as u32 & 0xFFFF;
    if flags == 0xFFFF && lp == 0 {
        notify_menu_closed();
        return;
    }
    let Some(menu) = menu_of(lp as HMENU) else { return };
    let id = (flags & (MF_POPUP | MF_SEPARATOR) == 0).then_some((wp & 0xFFFF) as u32);
    notify_highlight(&menu, id);
}

/// Called in `WM_INITMENUPOPUP` before native menu display.
pub(crate) fn notify_native_open(handle: HMENU) {
    let menu = MENUS.with(|m| m.borrow().get(&(handle as isize)).and_then(Weak::upgrade));
    if let Some(menu) = menu {
        menu.notify_open();
    }
}

impl Drop for MenuInner {
    fn drop(&mut self) {
        let key = self.handle as isize;
        let _ = MENUS.try_with(|m| {
            let mut map = m.borrow_mut();
            if map.get(&key).is_some_and(|w| w.strong_count() == 0) {
                map.remove(&key);
            }
        });
        for id in self.ids.borrow().iter() {
            app::unregister_menu_handler(*id);
        }
        if !self.system_owned.get() {
            // SAFETY: o menu foi criado por este objeto e nunca foi entregue ao sistema.
            unsafe { DestroyMenu(self.handle) };
        }
    }
}

/// Popup menu for [`MenuBar`]/`Menu` submenus or [`ContextMenu`] base.
#[derive(Clone)]
pub struct Menu {
    pub(crate) inner: Rc<MenuInner>,
}

/// Menu item reference for later changes.
#[derive(Clone)]
pub struct MenuItem {
    menu: Rc<MenuInner>,
    id: u32,
}

impl PartialEq for MenuItem {
    fn eq(&self, other: &MenuItem) -> bool {
        self.id == other.id && Rc::ptr_eq(&self.menu, &other.menu)
    }
}

impl Eq for MenuItem {}

impl std::fmt::Debug for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuItem").field("id", &self.id).field("text", &self.text()).finish()
    }
}

impl Menu {
    /// Creates empty popup menu.
    pub fn new() -> GuiResult<Menu> {
        Ok(Menu { inner: MenuInner::create(true)? })
    }

    /// Native `HMENU` (advanced API).
    pub fn handle(&self) -> isize {
        self.inner.handle as isize
    }

    /// Called when mouse/keyboard highlight changes in this menu/submenus. Receives command item, or `None` for
    /// separators, submenu openers and closing. Works with native bar, [`MenuStrip`] and context menus, e.g. status-bar
    /// command descriptions.
    pub fn on_highlight_changed(&self, f: impl Fn(Option<MenuItem>) + 'static) {
        *self.inner.on_highlight.borrow_mut() = Some(Rc::new(f));
    }

    /// Former name of [`on_highlight_changed`](Self::on_highlight_changed), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_highlight_changed`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_highlight(&self, f: impl Fn(Option<MenuItem>) + 'static) {
        self.on_highlight_changed(f)
    }

    /// Runs before menu display (native bar, [`MenuStrip`], context) and before checking its MenuStrip shortcuts. Use
    /// to enable/check/rebuild according to application state.
    pub fn on_open(&self, f: impl Fn(&Menu) + 'static) {
        *self.inner.on_open.borrow_mut() = Some(Rc::new(f));
    }

    fn append(&self, text: &str, flags: u32, handler: Rc<dyn Fn()>) -> GuiResult<MenuItem> {
        let id = app::register_menu_handler(handler)?;
        let wide = to_wide(text);
        // SAFETY: menu válido e string terminada em NUL.
        if unsafe { AppendMenuW(self.inner.handle, MF_STRING | flags, id as usize, wide.as_ptr()) } == 0 {
            app::unregister_menu_handler(id);
            return Err(GuiError::last_os_error("AppendMenuW"));
        }
        self.inner.ids.borrow_mut().push(id);
        Ok(MenuItem { menu: self.inner.clone(), id })
    }

    /// Adds command item; `&` before a letter sets mnemonic.
    pub fn add_item(&self, text: &str, on_select: impl Fn() + 'static) -> GuiResult<MenuItem> {
        self.append(text, 0, Rc::new(on_select))
    }

    /// Adds shortcut item: shortcut text appears right and becomes active in the bar's window.
    pub fn add_item_with_shortcut(
        &self,
        text: &str,
        shortcut: Shortcut,
        on_select: impl Fn() + 'static,
    ) -> GuiResult<MenuItem> {
        let handler: Rc<dyn Fn()> = Rc::new(on_select);
        let label = format!("{text}\t{}", shortcut.label());
        let item = self.append(&label, 0, handler.clone())?;
        self.inner.shortcuts.borrow_mut().push((shortcut, handler, item.id));
        Ok(item)
    }

    /// Adds checkable item, toggled on selection, delivering new state to `on_toggle`.
    pub fn add_check_item(&self, text: &str, checked: bool, on_toggle: impl Fn(bool) + 'static) -> GuiResult<MenuItem> {
        // O manipulador guarda apenas uma referência fraca ao menu, evitando um ciclo
        // (o registro global de manipuladores vive até o menu ser destruído).
        let weak = Rc::downgrade(&self.inner);
        let id_slot = Rc::new(Cell::new(0u32));
        let captured = id_slot.clone();
        let item = self.append(
            text,
            if checked { MF_CHECKED } else { MF_UNCHECKED },
            Rc::new(move || {
                if let Some(menu) = weak.upgrade() {
                    let item = MenuItem { menu, id: captured.get() };
                    let now = !item.is_checked();
                    item.set_checked(now);
                    on_toggle(now);
                }
            }),
        )?;
        id_slot.set(item.id);
        Ok(item)
    }

    /// Adds radio item (dot). Consecutive radio items form an exclusive group; choosing checks it, clears peers and
    /// calls `on_select`. Separators/other types end the group.
    pub fn add_radio_item(&self, text: &str, checked: bool, on_select: impl Fn() + 'static) -> GuiResult<MenuItem> {
        let weak = Rc::downgrade(&self.inner);
        let id_slot = Rc::new(Cell::new(0u32));
        let captured = id_slot.clone();
        let item = self.append(
            text,
            0,
            Rc::new(move || {
                if let Some(menu) = weak.upgrade() {
                    menu.select_radio(captured.get());
                    on_select();
                }
            }),
        )?;
        id_slot.set(item.id);
        let mut info = empty_item_info(MIIM_FTYPE);
        info.fType = MFT_RADIOCHECK;
        // SAFETY: menu vivo; a estrutura só altera o tipo do item recém-criado.
        unsafe { SetMenuItemInfoW(self.inner.handle, item.id, 0, &info) };
        if checked {
            self.inner.select_radio(item.id);
        }
        Ok(item)
    }

    /// Adds a separator.
    pub fn add_separator(&self) -> GuiResult<()> {
        // SAFETY: menu válido; separadores não usam texto.
        if unsafe { AppendMenuW(self.inner.handle, MF_SEPARATOR, 0, std::ptr::null()) } == 0 {
            return Err(GuiError::last_os_error("AppendMenuW(separador)"));
        }
        Ok(())
    }

    /// Adds submenu.
    pub fn add_submenu(&self, text: &str, submenu: &Menu) -> GuiResult<()> {
        attach_submenu(&self.inner, text, submenu)
    }

    /// Adds submenu with text-adjacent icon.
    pub fn add_submenu_with_icon(&self, text: &str, icon: &impl IconSource, submenu: &Menu) -> GuiResult<()> {
        attach_submenu(&self.inner, text, submenu)?;
        let key = submenu.inner.handle as isize;
        self.inner.submenu_icons.borrow_mut().insert(key, icon.to_shared());
        let bitmap = menu_bitmap(icon)?;
        // SAFETY: consulta simples.
        let position = unsafe { GetMenuItemCount(self.inner.handle) } - 1;
        let mut info = empty_item_info(MIIM_BITMAP);
        info.hbmpItem = bitmap.raw();
        // SAFETY: estrutura completa; o bitmap é mantido vivo pelo menu.
        unsafe { SetMenuItemInfoW(self.inner.handle, position.max(0) as u32, TRUE, &info) };
        self.inner.bitmaps.borrow_mut().push(bitmap);
        Ok(())
    }

    /// Removes menu item.
    pub fn remove(&self, item: &MenuItem) {
        // SAFETY: menu válido; identificador inexistente apenas faz a chamada falhar.
        unsafe { DeleteMenu(self.inner.handle, item.id, MF_BYCOMMAND) };
        self.inner.ids.borrow_mut().retain(|id| *id != item.id);
        self.inner.shortcuts.borrow_mut().retain(|(_, _, id)| *id != item.id);
        self.inner.icons.borrow_mut().remove(&item.id);
        app::unregister_menu_handler(item.id);
    }

    /// Removes all items for dynamic rebuilding.
    pub fn clear(&self) {
        // SAFETY: menu válido; remove posições até esvaziar. RemoveMenu não destrói
        // submenus, que continuam pertencendo aos respectivos objetos `Menu`.
        unsafe {
            while GetMenuItemCount(self.inner.handle) > 0 {
                if RemoveMenu(self.inner.handle, 0, MF_BYPOSITION) == 0 {
                    break;
                }
            }
        }
        for id in self.inner.ids.borrow_mut().drain(..) {
            app::unregister_menu_handler(id);
        }
        for child in self.inner.children.borrow_mut().drain(..) {
            child.system_owned.set(false);
        }
        self.inner.shortcuts.borrow_mut().clear();
        self.inner.icons.borrow_mut().clear();
        self.inner.submenu_icons.borrow_mut().clear();
    }

    /// Entry count including items, separators and submenus.
    pub fn len(&self) -> usize {
        // SAFETY: consulta simples.
        unsafe { GetMenuItemCount(self.inner.handle).max(0) as usize }
    }

    /// Whether menu is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Shows at given screen position, executing chosen item.
    pub(crate) fn show_at_screen(&self, owner: HWND, screen: Point) {
        // SAFETY: `owner` é uma janela desta thread; com TPM_RETURNCMD a função devolve
        // o identificador escolhido (ou 0) em vez de postar WM_COMMAND.
        let chosen = unsafe {
            SetForegroundWindow(owner);
            let id = TrackPopupMenu(
                self.inner.handle,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                screen.x,
                screen.y,
                0,
                owner,
                std::ptr::null(),
            );
            PostMessageW(owner, WM_NULL, 0, 0);
            id as u32
        };
        if chosen != 0 {
            app::invoke_menu(chosen);
        }
    }
}

fn attach_submenu(parent: &Rc<MenuInner>, text: &str, submenu: &Menu) -> GuiResult<()> {
    if submenu.inner.system_owned.get() {
        return Err(GuiError::InvalidArgument("this menu is already attached to another menu".into()));
    }
    let wide = to_wide(text);
    // SAFETY: ambos os menus são válidos; a string termina em NUL.
    let ok = unsafe { AppendMenuW(parent.handle, MF_POPUP, submenu.inner.handle as usize, wide.as_ptr()) };
    if ok == 0 {
        return Err(GuiError::last_os_error("AppendMenuW(submenu)"));
    }
    // A partir daqui o pai destrói o submenu junto com ele.
    submenu.inner.system_owned.set(true);
    parent.children.borrow_mut().push(submenu.inner.clone());
    Ok(())
}

impl MenuItem {
    /// Enables/disables item.
    pub fn set_enabled(&self, enabled: bool) {
        let flags = MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_GRAYED };
        // SAFETY: menu mantido vivo por `self.menu`.
        unsafe { EnableMenuItem(self.menu.handle, self.id, flags) };
    }

    /// Whether item is enabled.
    pub fn is_enabled(&self) -> bool {
        // SAFETY: consulta simples.
        let state = unsafe { GetMenuState(self.menu.handle, self.id, MF_BYCOMMAND) };
        state != u32::MAX && state & MF_GRAYED == 0
    }

    /// Checks/unchecks item.
    pub fn set_checked(&self, checked: bool) {
        let flags = MF_BYCOMMAND | if checked { MF_CHECKED } else { MF_UNCHECKED };
        // SAFETY: menu mantido vivo por `self.menu`.
        unsafe { CheckMenuItem(self.menu.handle, self.id, flags) };
    }

    /// Whether item is checked.
    pub fn is_checked(&self) -> bool {
        // SAFETY: consulta simples.
        let state = unsafe { GetMenuState(self.menu.handle, self.id, MF_BYCOMMAND) };
        state != u32::MAX && state & MF_CHECKED != 0
    }

    /// Changes item text.
    pub fn set_text(&self, text: &str) {
        let wide = to_wide(text);
        // SAFETY: menu válido e string terminada em NUL; o identificador é preservado.
        unsafe { ModifyMenuW(self.menu.handle, self.id, MF_BYCOMMAND | MF_STRING, self.id as usize, wide.as_ptr()) };
    }

    /// Shows icon beside item text.
    ///
    /// Accepts [`Icon`](crate::core::Icon) or bundled icon (`&icons::SAVE`) at system small-icon size.
    ///
    /// [`MenuStrip`] and library-drawn context menus reload at exact window DPI size.
    pub fn set_icon(&self, icon: &impl IconSource) -> GuiResult<()> {
        let bitmap = menu_bitmap(icon)?;
        let mut info = empty_item_info(MIIM_BITMAP);
        info.hbmpItem = bitmap.raw();
        // SAFETY: estrutura completa; o bitmap é mantido vivo pelo menu.
        if unsafe { SetMenuItemInfoW(self.menu.handle, self.id, 0, &info) } == 0 {
            return Err(GuiError::last_os_error("SetMenuItemInfoW"));
        }
        self.menu.bitmaps.borrow_mut().push(bitmap);
        self.menu.icons.borrow_mut().insert(self.id, icon.to_shared());
        Ok(())
    }

    /// Current text including mnemonic `&` and optional `\t` shortcut.
    pub fn text(&self) -> String {
        self.menu
            .position_of(self.id)
            .and_then(|position| self.menu.entry_at(position))
            .map(
                |entry| {
                    if entry.shortcut.is_empty() {
                        entry.text
                    } else {
                        format!("{}\t{}", entry.text, entry.shortcut)
                    }
                },
            )
            .unwrap_or_default()
    }

    /// Command identifier (`WM_COMMAND`, advanced API).
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Executes handler as if selected by user.
    pub fn trigger(&self) {
        app::invoke_menu(self.id);
    }
}

/// Window menu bar; attach via [`Window::set_menu`].
#[derive(Clone)]
pub struct MenuBar {
    inner: Rc<MenuInner>,
}

impl MenuBar {
    /// Creates empty menu bar.
    pub fn new() -> GuiResult<MenuBar> {
        Ok(MenuBar { inner: MenuInner::create(false)? })
    }

    /// Adds top-level menu. Build before window attachment to register shortcuts.
    pub fn add_menu(&self, text: &str, menu: &Menu) -> GuiResult<()> {
        attach_submenu(&self.inner, text, menu)
    }

    pub(crate) fn raw(&self) -> HMENU {
        // A janela passa a ser a dona do HMENU e o destruirá junto com ela.
        self.inner.system_owned.set(true);
        self.inner.handle
    }

    /// Enabled item matching `shortcut`, queried per key, including items added after
    /// [`Window::set_menu`](crate::core::Window::set_menu).
    pub(crate) fn shortcut_item(&self, shortcut: Shortcut) -> Option<u32> {
        let (menu, id) = self.inner.find_shortcut(shortcut)?;
        menu.is_enabled(id).then_some(id)
    }
}

/// Right-click context menu: [`Menu`] with display methods.
///
/// Attach via [`Widget::set_context_menu`] or show via [`ContextMenu::show`].
///
/// Defaults to native popup; [`set_drawn(true)`](ContextMenu::set_drawn) uses MenuStrip-style themed drawing, any-DPI
/// icons and keyboard navigation.
#[derive(Clone)]
pub struct ContextMenu {
    menu: Menu,
    drawn: Rc<Cell<bool>>,
}

impl ContextMenu {
    /// Creates empty context menu.
    pub fn new() -> GuiResult<ContextMenu> {
        Ok(ContextMenu { menu: Menu::new()?, drawn: Rc::new(Cell::new(false)) })
    }

    /// Chooses native (`false`, default) or library-drawn menu.
    ///
    /// Drawn menus use window MenuStrip [`MenuStyle`] or window theme. Nonblocking: [`show`](ContextMenu::show) returns
    /// immediately and executes chosen item later.
    pub fn set_drawn(&self, drawn: bool) {
        self.drawn.set(drawn);
    }

    /// Whether library-drawn.
    pub fn is_drawn(&self) -> bool {
        self.drawn.get()
    }

    /// Closes an open drawn menu; native menus close themselves.
    pub fn close(&self) {
        popup::close_menu(&self.menu.inner);
    }

    /// Shows at widget-relative logical `at`, executing chosen item.
    pub fn show(&self, widget: &impl Widget, at: Point) {
        let hwnd = widget.core().hwnd();
        if hwnd.is_null() {
            return;
        }
        let dpi = widget.dpi();
        let mut point = POINT { x: to_pixels(at.x, dpi), y: to_pixels(at.y, dpi) };
        // SAFETY: ponteiro válido para um POINT local.
        unsafe { ClientToScreen(hwnd, &mut point) };
        self.show_at_screen(hwnd, Point::new(point.x, point.y));
    }

    pub(crate) fn show_at_screen(&self, owner: HWND, screen: Point) {
        if self.drawn.get() {
            popup::open_context(owner, self.menu.inner.clone(), screen);
        } else {
            self.menu.show_at_screen(owner, screen);
        }
    }
}

impl std::ops::Deref for ContextMenu {
    type Target = Menu;

    fn deref(&self) -> &Menu {
        &self.menu
    }
}

impl Window {
    /// Window system menu, opened from title icon.
    ///
    /// Added items integrate with native menu and invoke handlers normally.
    pub fn system_menu(&self) -> GuiResult<Menu> {
        let hwnd = self.core().require_alive()?;
        // SAFETY: obtém o menu de sistema (pertencente à janela) sem revertê-lo.
        let handle = unsafe { GetSystemMenu(hwnd, 0) };
        if handle.is_null() {
            return Err(GuiError::Unsupported("the window has no system menu"));
        }
        let menu = Menu { inner: MenuInner::wrap(handle, true) };
        self.core().keep_alive(menu.clone());
        Ok(menu)
    }
}
