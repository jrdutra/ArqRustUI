//! [`MenuStrip`]: library-drawn menu bar.
//!
//! Occupies nonclient area between title/client like Windows menu bar, preserving client layouts/control positions.
//! Popup windows in `popup` share its style.

use super::popup;
use super::{mnemonic_of, plain_text, Entry, EntryKind, Menu, MenuInner};
use crate::accessibility::tree::{role, Action, Node, TreeHost, VirtualTree};
use crate::core::application as app;
use crate::core::handles::{IconSource, WindowDc};
use crate::core::native;
use crate::core::widget::{lookup, Widget, WidgetCore};
use crate::core::{GuiError, GuiResult, Window};
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::graphics::{Canvas, Font, TextAlign};
use crate::input::{Key, Modifiers, Shortcut};
use crate::themes::{SystemTheme, Theme};
use crate::utilities::{to_logical, to_pixels, Color, Point, Rect, Size};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

mod bar;
mod messages;

pub(crate) use messages::{idle_translate, root_message};

thread_local! {
    /// Thread window bars by window `HWND`.
    static STRIPS: RefCell<HashMap<isize, Rc<StripInner>>> = RefCell::new(HashMap::new());
    /// Windows displaying native MenuBar as MenuStrip in dark theme.
    static AUTO: RefCell<std::collections::HashSet<isize>> = RefCell::new(std::collections::HashSet::new());
    /// Windows opting out of dark-theme native bar replacement.
    static KEEP_NATIVE: RefCell<std::collections::HashSet<isize>> = RefCell::new(std::collections::HashSet::new());
}

/// Dark theme replaces system-colored native MenuBar with themed [`MenuStrip`], restoring native bar in light theme.
/// Items, shortcuts, `on_open` and `on_highlight_changed` remain the same.
pub(crate) fn sync_native_bar(window: &Window, dark: bool) {
    let core = window.core();
    let hwnd = core.hwnd();
    let Some(bar) = core.menu_bar.borrow().clone() else { return };
    let key = hwnd as isize;
    let auto = AUTO.with(|a| a.borrow().contains(&key));
    let keep = KEEP_NATIVE.with(|k| k.borrow().contains(&key));
    let drawn = dark && !keep;
    if drawn && !auto && strip_of(hwnd).is_none() {
        let Ok(strip) = MenuStrip::new() else { return };
        for entry in bar.inner.entries() {
            if let EntryKind::Submenu(child) = entry.kind {
                strip.push(&entry.text, entry.icon.clone(), &Menu { inner: child });
            }
        }
        if window.set_menu_strip(&strip).is_ok() {
            AUTO.with(|a| a.borrow_mut().insert(key));
        }
    } else if !drawn && auto {
        AUTO.with(|a| a.borrow_mut().remove(&key));
        window.remove_menu_strip();
        let size = window.client_size();
        // SAFETY: devolve à janela viva o HMENU da barra, mantido vivo pelo núcleo.
        unsafe {
            SetMenu(hwnd, bar.raw());
            DrawMenuBar(hwnd);
        }
        let b = core.bounds();
        core.set_bounds(Rect::new(b.x, b.y, size.width, size.height));
    }
}

/// Reverts automatic replacement before setting a native bar (`set_menu`).
pub(crate) fn drop_auto_strip(window: &Window) {
    let key = window.core().hwnd() as isize;
    if AUTO.with(|a| a.borrow_mut().remove(&key)) {
        window.remove_menu_strip();
    }
}

/// Logical vertical text padding.
const BAR_PAD_Y: i32 = 4;
/// Logical horizontal bar-menu text padding.
const BAR_PAD_X: i32 = 10;
/// Logical margin before first menu.
const BAR_MARGIN: i32 = 4;
/// Logical bar/menu icon size.
pub(crate) const ICON: i32 = 16;
/// Logical bar icon/text spacing.
const BAR_ICON_GAP: i32 = 6;
/// Logical overflow `»` width.
const OVERFLOW_WIDTH: i32 = 28;

/// [`MenuStrip`] and drawn-menu colors.
///
/// Without explicit [`MenuStrip::set_style`], uses [`MenuStyle::from_theme`] and [`Theme::apply`] theme, or
/// [`MenuStyle::system`] without a theme. High contrast always overrides theme with system colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuStyle {
    /// Bar background.
    pub bar_background: Color,
    /// Bar menu text.
    pub bar_foreground: Color,
    /// Hovered/keyboard-selected bar menu background.
    pub bar_hover: Color,
    /// Open bar menu background.
    pub bar_open: Color,
    /// Hovered/selected/open bar menu text.
    pub bar_hover_foreground: Color,
    /// Popup background.
    pub popup_background: Color,
    /// Popup border.
    pub popup_border: Color,
    /// Item text.
    pub popup_foreground: Color,
    /// Selected item background.
    pub highlight: Color,
    /// Selected item text.
    pub highlight_foreground: Color,
    /// Disabled menu/item text.
    pub disabled_foreground: Color,
    /// Shortcut text.
    pub shortcut_foreground: Color,
    /// Separator line.
    pub separator: Color,
    /// Logical highlight corner radius.
    pub corner_radius: i32,
}

impl MenuStyle {
    /// Theme-derived style with matching background/text and contrast-preserving highlights/borders.
    pub fn from_theme(theme: &Theme) -> MenuStyle {
        if crate::system::accessibility::high_contrast() {
            return MenuStyle::high_contrast();
        }
        let (bg, fg) = (theme.background, theme.foreground);
        let dark = theme.dark;
        let popup = if dark { bg.mix(Color::WHITE, 0.07) } else { bg.mix(Color::WHITE, 0.6) };
        MenuStyle {
            bar_background: bg,
            bar_foreground: fg,
            bar_hover: bg.mix(fg, if dark { 0.12 } else { 0.08 }),
            bar_open: bg.mix(fg, if dark { 0.18 } else { 0.13 }),
            bar_hover_foreground: fg,
            popup_background: popup,
            popup_border: if dark { popup.mix(Color::WHITE, 0.16) } else { popup.mix(Color::BLACK, 0.17) },
            popup_foreground: fg,
            highlight: popup.mix(fg, if dark { 0.13 } else { 0.08 }),
            highlight_foreground: fg,
            disabled_foreground: fg.mix(popup, 0.55),
            shortcut_foreground: fg.mix(popup, 0.38),
            separator: popup.mix(fg, if dark { 0.18 } else { 0.13 }),
            corner_radius: 4,
        }
    }

    /// Light style ([`Theme::light`]).
    pub fn light() -> MenuStyle {
        MenuStyle::from_theme(&Theme::light())
    }

    /// Dark style ([`Theme::dark`]).
    pub fn dark() -> MenuStyle {
        MenuStyle::from_theme(&Theme::dark())
    }

    /// System menu colors (`COLOR_MENU`, `COLOR_MENUTEXT`, `COLOR_HIGHLIGHT`, etc.) for high contrast.
    pub fn high_contrast() -> MenuStyle {
        // SAFETY: consultas de cores do sistema.
        let color = |index| Color::from_colorref(unsafe { GetSysColor(index) });
        let (menu, text) = (color(COLOR_MENU), color(COLOR_MENUTEXT));
        let (highlight, highlight_text) = (color(COLOR_HIGHLIGHT), color(COLOR_HIGHLIGHTTEXT));
        MenuStyle {
            bar_background: color(COLOR_MENUBAR),
            bar_foreground: text,
            bar_hover: highlight,
            bar_open: highlight,
            bar_hover_foreground: highlight_text,
            popup_background: menu,
            popup_border: text,
            popup_foreground: text,
            highlight,
            highlight_foreground: highlight_text,
            disabled_foreground: color(COLOR_GRAYTEXT),
            shortcut_foreground: text,
            separator: text,
            corner_radius: 0,
        }
    }

    /// System-following dark/light/high-contrast style.
    pub fn system() -> MenuStyle {
        match SystemTheme::current() {
            SystemTheme::Dark => MenuStyle::dark(),
            SystemTheme::Light => MenuStyle::light(),
            SystemTheme::HighContrast => MenuStyle::high_contrast(),
        }
    }

    /// Same style with accent highlights and contrast-based black/white text.
    pub fn with_accent(mut self, accent: Color) -> MenuStyle {
        let text = if accent.luminance() > 0.6 { Color::BLACK } else { Color::WHITE };
        self.highlight = accent;
        self.highlight_foreground = text;
        self.bar_open = accent;
        self
    }
}

/// Top-level bar menu.
struct BarItem {
    text: String,
    icon: Option<Rc<dyn IconSource>>,
    menu: Menu,
    enabled: bool,
}

/// Visible strip position: menu or overflow button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotKind {
    Item(usize),
    Overflow,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Slot {
    pub kind: SlotKind,
    /// Logical strip-relative rectangle.
    pub rect: Rect,
}

/// Action on strip-position activation.
pub(crate) enum SlotMenu {
    Menu(Rc<MenuInner>),
    Overflow(Vec<Entry>),
}

#[derive(Clone, PartialEq)]
struct LayoutKey {
    width: i32,
    dpi: u32,
    font: Font,
    generation: u64,
}

pub(crate) struct StripInner {
    items: RefCell<Vec<BarItem>>,
    owner: Cell<HWND>,
    style: Cell<Option<MenuStyle>>,
    /// Hovered/keyboard-selected position.
    hot: Cell<Option<usize>>,
    /// Position with open list.
    open: Cell<Option<usize>>,
    /// Underlines visible while Alt is down.
    alt_cues: Cell<bool>,
    /// Alt pressed alone: release enters menu mode.
    alt_armed: Cell<bool>,
    tracking_leave: Cell<bool>,
    generation: Cell<u64>,
    layout: RefCell<Option<(LayoutKey, Vec<Slot>)>>,
}

/// Distributes width among menus, moving overflow to `»`.
fn layout_slots(widths: &[i32], available: i32, height: i32) -> Vec<Slot> {
    let mut slots = Vec::new();
    let total: i32 = widths.iter().sum::<i32>() + BAR_MARGIN;
    let limit = if total > available { available - OVERFLOW_WIDTH } else { available };
    let mut x = BAR_MARGIN;
    for (index, width) in widths.iter().enumerate() {
        if x + width > limit && total > available {
            break;
        }
        slots.push(Slot { kind: SlotKind::Item(index), rect: Rect::new(x, 0, *width, height) });
        x += width;
    }
    if slots.len() < widths.len() {
        slots.push(Slot { kind: SlotKind::Overflow, rect: Rect::new(x, 0, OVERFLOW_WIDTH, height) });
    }
    slots
}

/// Runs `f` with screen-DC measuring `Canvas` at given DPI.
pub(crate) fn with_measure<R>(dpi: u32, f: impl FnOnce(&Canvas) -> R) -> Option<R> {
    let screen = WindowDc::get(std::ptr::null_mut()).ok()?;
    let canvas = Canvas::for_hdc(screen.raw(), dpi, Size::new(1, 1));
    Some(f(&canvas))
}

thread_local! {
    static HEIGHTS: RefCell<HashMap<(Font, u32), i32>> = RefCell::new(HashMap::new());
}

/// Logical font line height, cached per font/DPI.
pub(crate) fn text_height(font: &Font, dpi: u32) -> i32 {
    let key = (font.clone(), dpi);
    if let Some(height) = HEIGHTS.with(|h| h.borrow().get(&key).copied()) {
        return height;
    }
    let height = with_measure(dpi, |canvas| canvas.measure_mnemonic("Ág", font).height).unwrap_or(16).max(1);
    HEIGHTS.with(|h| h.borrow_mut().insert(key, height));
    height
}

fn bar_height(font: &Font, dpi: u32) -> i32 {
    text_height(font, dpi).max(ICON) + 2 * BAR_PAD_Y
}

/// Drawn-menu style without a window MenuStrip, e.g. context menus.
pub(crate) fn style_for_window(hwnd: HWND) -> MenuStyle {
    if let Some(style) = strip_of(hwnd).and_then(|s| s.style.get()) {
        return style;
    }
    if crate::system::accessibility::high_contrast() {
        return MenuStyle::high_contrast();
    }
    let core = lookup(native::root_of(hwnd));
    if let Some(theme) = core.as_ref().and_then(|c| c.theme.borrow().clone()) {
        return MenuStyle::from_theme(&theme);
    }
    // Sem tema: menus claros sobre o fundo da própria janela.
    let mut style = MenuStyle::light();
    if let Some(background) = core.and_then(|c| c.effective_background()) {
        style.bar_background = background;
    } else {
        // SAFETY: consulta de cor do sistema.
        style.bar_background = Color::from_colorref(unsafe { GetSysColor(COLOR_BTNFACE) });
    }
    style
}

/// Window font: theme, `set_font` or system.
pub(crate) fn font_for_window(hwnd: HWND) -> Font {
    lookup(native::root_of(hwnd)).map(|core| core.font()).unwrap_or_else(Font::system)
}

/// Whether the system always underlines access keys.
pub(crate) fn system_keyboard_cues() -> bool {
    let mut on: BOOL = 0;
    // SAFETY: ponteiro de saída válido para um BOOL.
    unsafe { SystemParametersInfoW(SPI_GETKEYBOARDCUES, 0, &mut on as *mut BOOL as *mut _, 0) };
    on != 0
}

pub(crate) fn strip_of(hwnd: HWND) -> Option<Rc<StripInner>> {
    if hwnd.is_null() {
        return None;
    }
    STRIPS.with(|s| s.borrow().get(&(hwnd as isize)).cloned())
}

fn has_strips() -> bool {
    STRIPS.with(|s| !s.borrow().is_empty())
}

/// Extra strip height in pixels, 0 without MenuStrip.
pub(crate) fn band_height_for(hwnd: HWND) -> i32 {
    strip_of(hwnd).map(|s| s.height_px()).unwrap_or(0)
}

/// Library-drawn menu bar.
///
/// Uses the same [`Menu`]s as native [`MenuBar`](super::MenuBar), drawing bar/popups with:
///
/// * bundled icons ([`crate::icons`]) at exact DPI size, dimmed when disabled;
/// * window [`Theme::apply`] or custom [`MenuStyle`] colors, system colors in high contrast;
/// * full keyboard navigation: Alt/F10 enters, Alt+letter opens, arrows/Home/End/Enter/Esc/mnemonics navigate;
///   underlines appear with keyboard or always if configured;
/// * item shortcuts while window focused (disabled items do not execute), with [`Menu::on_open`] before opening;
/// * hover-opened submenus, scrolling for lists taller than screen and `»` overflow menus.
///
/// Nonclient strip below title bar preserves client size/control positions on attachment.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let file = Menu::new()?;
/// file.add_item_with_shortcut("&Open...", Shortcut::ctrl(Key(b'O' as u16)), || println!("open"))?
///     .set_icon(&icons::FOLDER_OPEN)?;
/// file.add_separator()?;
/// let target = window.clone();
/// file.add_item("E&xit", move || target.close())?;
///
/// let strip = MenuStrip::new()?;
/// strip.add_menu("&File", &file)?;
/// window.set_menu_strip(&strip)?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct MenuStrip {
    inner: Rc<StripInner>,
}

impl std::fmt::Debug for MenuStrip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuStrip").field("menus", &self.len()).finish()
    }
}

impl MenuStrip {
    /// Creates empty bar.
    pub fn new() -> GuiResult<MenuStrip> {
        app::require()?;
        Ok(MenuStrip {
            inner: Rc::new(StripInner {
                items: RefCell::new(Vec::new()),
                owner: Cell::new(std::ptr::null_mut()),
                style: Cell::new(None),
                hot: Cell::new(None),
                open: Cell::new(None),
                alt_cues: Cell::new(false),
                alt_armed: Cell::new(false),
                tracking_leave: Cell::new(false),
                generation: Cell::new(0),
                layout: RefCell::new(None),
            }),
        })
    }

    fn push(&self, text: &str, icon: Option<Rc<dyn IconSource>>, menu: &Menu) {
        popup::cancel_for(self.inner.owner());
        self.inner.items.borrow_mut().push(BarItem { text: text.to_string(), icon, menu: menu.clone(), enabled: true });
        self.inner.bump();
    }

    /// Adds top-level menu; `&` defines mnemonic opened by Alt+letter.
    pub fn add_menu(&self, text: &str, menu: &Menu) -> GuiResult<()> {
        self.push(text, None, menu);
        Ok(())
    }

    /// Adds top-level menu with adjacent icon.
    pub fn add_menu_with_icon(&self, text: &str, icon: &impl IconSource, menu: &Menu) -> GuiResult<()> {
        self.push(text, Some(icon.to_shared()), menu);
        Ok(())
    }

    fn with_item<R>(&self, index: usize, f: impl FnOnce(&mut BarItem) -> R) -> GuiResult<R> {
        let result = {
            let mut items = self.inner.items.borrow_mut();
            let item = items
                .get_mut(index)
                .ok_or_else(|| GuiError::InvalidArgument(format!("menu {index} does not exist in the bar")))?;
            f(item)
        };
        self.inner.bump();
        Ok(result)
    }

    /// Changes menu text at `index`.
    pub fn set_menu_text(&self, index: usize, text: &str) -> GuiResult<()> {
        self.with_item(index, |item| item.text = text.to_string())
    }

    /// Menu text at `index`.
    pub fn menu_text(&self, index: usize) -> Option<String> {
        self.inner.items.borrow().get(index).map(|item| item.text.clone())
    }

    /// Enables/disables menu at `index`; disabled menus are dimmed and cannot open.
    pub fn set_menu_enabled(&self, index: usize, enabled: bool) -> GuiResult<()> {
        popup::cancel_for(self.inner.owner());
        self.with_item(index, |item| item.enabled = enabled)
    }

    /// Whether menu at `index` is enabled.
    pub fn is_menu_enabled(&self, index: usize) -> bool {
        self.inner.items.borrow().get(index).is_some_and(|item| item.enabled)
    }

    /// Shows icon beside menu at `index`.
    pub fn set_menu_icon(&self, index: usize, icon: &impl IconSource) -> GuiResult<()> {
        let shared = icon.to_shared();
        self.with_item(index, |item| item.icon = Some(shared))
    }

    /// Removes menu icon at `index`.
    pub fn clear_menu_icon(&self, index: usize) -> GuiResult<()> {
        self.with_item(index, |item| item.icon = None)
    }

    /// Removes menu at `index`.
    pub fn remove_menu(&self, index: usize) -> GuiResult<()> {
        popup::cancel_for(self.inner.owner());
        {
            let mut items = self.inner.items.borrow_mut();
            if index >= items.len() {
                return Err(GuiError::InvalidArgument(format!("menu {index} does not exist in the bar")));
            }
            items.remove(index);
        }
        self.inner.bump();
        Ok(())
    }

    /// Removes all menus.
    pub fn clear(&self) {
        popup::cancel_for(self.inner.owner());
        self.inner.items.borrow_mut().clear();
        self.inner.bump();
    }

    /// Top-level menu count.
    pub fn len(&self) -> usize {
        self.inner.items.borrow().len()
    }

    /// Whether bar has no menus.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets colors; `None` follows window theme/system.
    pub fn set_style(&self, style: Option<MenuStyle>) {
        self.inner.style.set(style);
        self.inner.repaint();
    }

    /// Effective explicit, window theme or system style.
    pub fn style(&self) -> MenuStyle {
        self.inner.effective_style()
    }

    /// Logical strip height depending on window font.
    pub fn height(&self) -> i32 {
        if self.inner.owner().is_null() {
            return bar_height(&Font::system(), native::system_dpi());
        }
        self.inner.height()
    }

    /// Keyboard-opens menu `index` with first item selected; opens `»` if the menu overflows.
    pub fn open(&self, index: usize) -> GuiResult<()> {
        if self.inner.owner().is_null() {
            return Err(GuiError::InvalidArgument("the bar is not attached to a window".into()));
        }
        if index >= self.len() {
            return Err(GuiError::InvalidArgument(format!("menu {index} does not exist in the bar")));
        }
        let slots = self.inner.slots();
        if let Some(slot) = slots.iter().position(|s| s.kind == SlotKind::Item(index)) {
            popup::open_bar(self.inner.clone(), slot, true);
            return Ok(());
        }
        // Menu escondido no botão »: abre o » e, nele, o submenu correspondente.
        let slot = slots
            .iter()
            .position(|s| s.kind == SlotKind::Overflow)
            .ok_or(GuiError::Unsupported("the window does not show the menu bar"))?;
        let visible = slots.iter().filter(|s| matches!(s.kind, SlotKind::Item(_))).count();
        popup::open_bar(self.inner.clone(), slot, true);
        popup::open_child_of_first(index - visible);
        Ok(())
    }

    /// Closes lists and exits menu mode.
    pub fn close(&self) {
        popup::cancel_for(self.inner.owner());
    }

    /// Whether in menu mode (open list or keyboard selection).
    pub fn is_open(&self) -> bool {
        let owner = self.inner.owner();
        !owner.is_null() && popup::current_owner() == Some(owner)
    }

    /// Top-level menu count overflowing to `»`.
    pub fn overflow_count(&self) -> usize {
        let visible = self.inner.slots().iter().filter(|s| matches!(s.kind, SlotKind::Item(_))).count();
        if self.inner.owner().is_null() {
            0
        } else {
            self.len().saturating_sub(visible)
        }
    }
}

/// Strip UIA identifier and position identifier base.
const STRIP_BAR_ID: u32 = 1;
const STRIP_ITEM_BASE: u32 = 100;

/// Strip UIA tree: menu bar with one item per visible position.
struct StripTree {
    hwnd: HWND,
}

impl VirtualTree for StripTree {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn nodes(&self) -> Vec<Node> {
        let Some(strip) = strip_of(self.hwnd) else { return Vec::new() };
        let Some(band) = strip.band_screen_rect() else { return Vec::new() };
        let mut bar = Node::new(
            STRIP_BAR_ID,
            0,
            role::MENU_BAR,
            &crate::i18n::library_text(crate::i18n::LibraryText::MenuBarName),
        );
        bar.bounds = band;
        let mut nodes = vec![bar];
        let keyboard = popup::current().is_some_and(|t| t.owner() == self.hwnd && t.depth() == 0);
        let items = strip.items.borrow();
        for (slot, info) in strip.slots().iter().enumerate() {
            let (name, enabled, key) = match info.kind {
                SlotKind::Item(index) => match items.get(index) {
                    Some(item) => (plain_text(&item.text), item.enabled, mnemonic_of(&item.text)),
                    None => continue,
                },
                SlotKind::Overflow => (crate::i18n::library_text(crate::i18n::LibraryText::MoreMenus), true, None),
            };
            let mut node = Node::new(STRIP_ITEM_BASE + slot as u32, STRIP_BAR_ID, role::MENU_ITEM, &name);
            node.enabled = enabled;
            node.focusable = true;
            node.focused = keyboard && strip.hot() == Some(slot);
            node.expanded = Some(strip.open_slot() == Some(slot));
            node.access_key = key.map(|c| format!("Alt+{}", c.to_uppercase())).unwrap_or_default();
            node.bounds = strip.slot_screen_rect(slot).unwrap_or_default();
            nodes.push(node);
        }
        nodes
    }

    fn act(&self, id: u32, action: Action) -> bool {
        let Some(strip) = strip_of(self.hwnd) else { return false };
        let Some(slot) = id.checked_sub(STRIP_ITEM_BASE).map(|s| s as usize) else { return false };
        if slot >= strip.slot_count() {
            return false;
        }
        match action {
            Action::Expand | Action::Invoke => popup::open_bar(strip, slot, true),
            Action::Collapse => popup::cancel_for(self.hwnd),
            Action::Focus => {
                if popup::current_owner() != Some(self.hwnd) {
                    popup::enter_bar_keyboard_mode(strip.clone());
                }
                strip.set_state(Some(slot), None);
            }
            _ => return false,
        }
        true
    }
}

impl Window {
    /// Attaches [`MenuStrip`], preserving client size.
    ///
    /// Replaces previous native/drawn bar. A MenuStrip belongs to one window at a time.
    pub fn set_menu_strip(&self, strip: &MenuStrip) -> GuiResult<()> {
        let hwnd = self.core().require_alive()?;
        let current = strip.inner.owner();
        if !current.is_null() && current != hwnd {
            return Err(GuiError::InvalidArgument("this MenuStrip is already attached to another window".into()));
        }
        let size = self.client_size();
        // SAFETY: remove a barra nativa, se houver (o HMENU continua com seu dono em Rust).
        unsafe {
            if !GetMenu(hwnd).is_null() {
                SetMenu(hwnd, std::ptr::null_mut());
            }
        }
        if let Some(previous) = strip_of(hwnd) {
            previous.owner.set(std::ptr::null_mut());
        }
        strip.inner.owner.set(hwnd);
        strip.inner.layout.borrow_mut().take();
        STRIPS.with(|s| s.borrow_mut().insert(hwnd as isize, strip.inner.clone()));
        // Leitores de tela: a faixa vira uma barra de menus dentro da janela.
        let core = self.core();
        if core.accessible_tree.borrow().is_none() {
            *core.accessible_tree.borrow_mut() = Some(TreeHost::new(StripTree { hwnd }));
        }
        self.refresh_frame(size);
        Ok(())
    }

    /// Removes [`MenuStrip`], preserving client size.
    pub fn remove_menu_strip(&self) {
        let hwnd = self.core().hwnd();
        let Some(strip) = strip_of(hwnd) else { return };
        popup::cancel_for(hwnd);
        let size = self.client_size();
        STRIPS.with(|s| s.borrow_mut().remove(&(hwnd as isize)));
        strip.owner.set(std::ptr::null_mut());
        let tree = self.core().accessible_tree.borrow_mut().take();
        drop(tree);
        self.refresh_frame(size);
    }

    /// Dark theme renders native [`MenuBar`](super::MenuBar) as MenuStrip with identical menus because Windows does not
    /// allow recoloring. `false` retains native system colors even in dark mode.
    pub fn set_menu_bar_follows_theme(&self, follow: bool) {
        let key = self.core().hwnd() as isize;
        KEEP_NATIVE.with(|k| {
            if follow {
                k.borrow_mut().remove(&key);
            } else {
                k.borrow_mut().insert(key);
            }
        });
        let dark = crate::themes::dark::dark_theme(self.core()).is_some();
        sync_native_bar(self, dark);
    }

    /// Whether native bar is displayed as MenuStrip for dark theme.
    pub fn is_menu_bar_drawn(&self) -> bool {
        AUTO.with(|a| a.borrow().contains(&(self.core().hwnd() as isize)))
    }

    /// Attached [`MenuStrip`], if any.
    pub fn menu_strip(&self) -> Option<MenuStrip> {
        strip_of(self.core().hwnd()).map(|inner| MenuStrip { inner })
    }

    /// Recalculates nonclient area, restoring client size `client`.
    fn refresh_frame(&self, client: Size) {
        let hwnd = self.core().hwnd();
        // SAFETY: força um novo WM_NCCALCSIZE na janela viva.
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )
        };
        let bounds = self.core().bounds();
        self.core().set_bounds(Rect::new(bounds.x, bounds.y, client.width, client.height));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_overflow_when_the_band_is_narrow() {
        let wide = layout_slots(&[60, 70, 80], 400, 24);
        assert_eq!(wide.len(), 3);
        assert_eq!(wide[0].rect, Rect::new(BAR_MARGIN, 0, 60, 24));
        assert_eq!(wide[2].rect.x, BAR_MARGIN + 130);
        let narrow = layout_slots(&[60, 70, 80], 170, 24);
        assert_eq!(narrow.len(), 3, "dois menus e o botão »");
        assert_eq!(narrow[2].kind, SlotKind::Overflow);
        assert!(narrow[2].rect.right() <= 170);
        let tiny = layout_slots(&[60, 70], 20, 24);
        assert_eq!(tiny.len(), 1);
        assert_eq!(tiny[0].kind, SlotKind::Overflow);
        assert!(layout_slots(&[], 300, 24).is_empty());
    }

    #[test]
    fn styles_keep_text_readable() {
        for style in [MenuStyle::light(), MenuStyle::dark(), MenuStyle::light().with_accent(Color::rgb(0, 95, 184))] {
            let contrast = |a: Color, b: Color| (a.luminance() - b.luminance()).abs();
            assert!(contrast(style.bar_background, style.bar_foreground) > 0.4);
            assert!(contrast(style.popup_background, style.popup_foreground) > 0.4);
            assert!(contrast(style.highlight, style.highlight_foreground) > 0.3);
            assert!(contrast(style.popup_background, style.disabled_foreground) > 0.1);
            assert_ne!(style.popup_background, style.highlight);
            assert_ne!(style.bar_background, style.bar_hover);
        }
        assert!(MenuStyle::dark().popup_background.luminance() < 0.3);
        let _ = MenuStyle::system();
        let _ = MenuStyle::high_contrast();
    }

    #[test]
    fn mnemonics_follow_the_ampersand() {
        assert_eq!(mnemonic_of("&Arquivo"), Some('a'));
        assert_eq!(mnemonic_of("Sai&r"), Some('r'));
        assert_eq!(mnemonic_of("A && B"), None);
        assert_eq!(mnemonic_of("A && &B"), Some('b'));
        assert_eq!(mnemonic_of("&Édição"), Some('é'));
        assert_eq!(mnemonic_of("Fim&"), None);
    }
}
