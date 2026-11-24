//! [`CommandBar`]: library-drawn command bar.
//!
//! A strip at the top of the client area with icon/text buttons, toggles, menu buttons (whole or split), separators,
//! flexible space, labels, search field and arbitrary library controls. Overflow goes into `»`. Menus share
//! [`MenuStrip`](crate::menus::MenuStrip) drawing and follow the window theme.

use super::common::{builder_common, ControlOptions};
use super::textbox::TextBox;
use crate::accessibility::tree::{role, Action, Node, TreeHost, VirtualTree};
use crate::core::application as app;
use crate::core::handles::IconSource;
use crate::core::native;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::{Event, EventKind, GuiError, GuiResult};
use crate::ffi::user32::*;
use crate::ffi::{HWND, POINT, RECT};
use crate::graphics::{Canvas, Font, Pen, TextAlign};
use crate::input::{Key, MouseButton};
use crate::menus::popup::{self, DropSource};
use crate::menus::strip::{self as menu_strip, ICON};
use crate::menus::{plain_text, Entry, EntryKind, Menu, MenuStyle};
use crate::utilities::{to_logical, to_pixels, Color, Point, Rect};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

mod access;
mod customize;
mod flyout;
mod layout;
mod painting;

use access::*;
pub use customize::CommandBarLayout;
use flyout::*;
use painting::*;

/// Default logical bar height (28-high buttons with 4 units above and below).
pub const COMMAND_BAR_HEIGHT: i32 = 36;

/// Logical button and search field height.
const ITEM_HEIGHT: i32 = 28;
/// Horizontal bar margin.
const PAD_X: i32 = 6;
/// Item spacing.
const GAP: i32 = 2;
/// Horizontal button padding.
const BUTTON_PAD: i32 = 8;
/// Icon-to-text spacing.
const ICON_TEXT_GAP: i32 = 6;
/// Width added by a menu button's arrow.
const ARROW: i32 = 12;
/// Width of a split button's arrow section.
const SPLIT_ARROW: i32 = 20;
/// Separator width.
const SEPARATOR: i32 = 9;
/// Overflow button `»` width.
const OVERFLOW: i32 = 28;
/// Horizontal label padding.
const LABEL_PAD: i32 = 6;
/// Search field magnifier area.
const SEARCH_ICON: i32 = 26;
/// Search field clear button area.
const SEARCH_CLEAR: i32 = 24;
/// Overflow button identifier.
const OVERFLOW_ID: u32 = u32::MAX;

/// Bar button behavior.
#[derive(Clone)]
enum Mode {
    /// Executes `on_click`.
    Normal,
    /// Toggles checked state; calls `on_toggle`.
    Toggle,
    /// Opens the menu.
    Dropdown(Menu),
    /// Main section executes `on_click`; arrow opens the menu.
    Split(Menu),
}

/// Button description for [`CommandBar::add_button`].
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(bar: &CommandBar, tools: &Menu) -> GuiResult<()> {
/// bar.add_button(CommandButton::new("&Save").icon(&icons::SAVE).on_click(|| println!("save")))?;
/// bar.add_button(CommandButton::new("Refresh").icon(&icons::REFRESH).icon_only().on_click(|| {}))?;
/// bar.add_button(CommandButton::new("Bold").icon(&icons::EDIT).toggle(false).on_toggled(|on| println!("{on}")))?;
/// bar.add_button(CommandButton::new("Tools").icon(&icons::TOOLS).menu(tools))?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct CommandButton {
    text: String,
    icon: Option<Rc<dyn IconSource>>,
    tooltip: Option<String>,
    show_text: bool,
    mode: Mode,
    checked: bool,
    enabled: bool,
    on_click: Option<Rc<dyn Fn()>>,
    on_toggle: Option<Rc<dyn Fn(bool)>>,
}

impl std::fmt::Debug for CommandButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandButton").field("text", &self.text).finish_non_exhaustive()
    }
}

impl CommandButton {
    /// Button with the given text. Text also appears in the `»` menu and becomes the tooltip for icon-only buttons.
    pub fn new(text: &str) -> CommandButton {
        CommandButton {
            text: text.to_string(),
            icon: None,
            tooltip: None,
            show_text: true,
            mode: Mode::Normal,
            checked: false,
            enabled: true,
            on_click: None,
            on_toggle: None,
        }
    }

    /// Icon left of text (loaded at the exact DPI size).
    pub fn icon(mut self, icon: &impl IconSource) -> CommandButton {
        self.icon = Some(icon.to_shared());
        self
    }

    /// Shows only the icon; text becomes the tooltip unless a custom tooltip exists.
    pub fn icon_only(mut self) -> CommandButton {
        self.show_text = false;
        self
    }

    /// Tooltip shown when hovering over the button.
    pub fn tooltip(mut self, text: &str) -> CommandButton {
        self.tooltip = Some(text.to_string());
        self
    }

    /// Callback for clicks (or a split button's main section).
    pub fn on_click(mut self, f: impl Fn() + 'static) -> CommandButton {
        self.on_click = Some(Rc::new(f));
        self
    }

    /// Makes this a toggle button with the given initial state.
    pub fn toggle(mut self, checked: bool) -> CommandButton {
        self.mode = Mode::Toggle;
        self.checked = checked;
        self
    }

    /// Callback when the user toggles the button (receives the new state).
    pub fn on_toggled(mut self, f: impl Fn(bool) + 'static) -> CommandButton {
        self.on_toggle = Some(Rc::new(f));
        self
    }

    /// Former name of [`on_toggled`](Self::on_toggled), retained for existing code.
    #[deprecated(since = "0.1.0", note = "use `on_toggled`: events follow the naming convention in docs/EVENTS.md")]
    pub fn on_toggle(self, f: impl Fn(bool) + 'static) -> CommandButton {
        self.on_toggled(f)
    }

    /// Click opens `menu` below the button (`▾` arrow beside content).
    pub fn menu(mut self, menu: &Menu) -> CommandButton {
        self.mode = Mode::Dropdown(menu.clone());
        self
    }

    /// Split button: main section executes [`on_click`](CommandButton::on_click), arrow opens `menu`.
    pub fn split(mut self, menu: &Menu) -> CommandButton {
        self.mode = Mode::Split(menu.clone());
        self
    }

    /// Initial enabled state (default: enabled).
    pub fn enabled(mut self, enabled: bool) -> CommandButton {
        self.enabled = enabled;
        self
    }

    fn menu_of(&self) -> Option<&Menu> {
        match &self.mode {
            Mode::Dropdown(menu) | Mode::Split(menu) => Some(menu),
            _ => None,
        }
    }

    fn tooltip_text(&self) -> String {
        match &self.tooltip {
            Some(text) => text.clone(),
            None if !self.show_text => self.text.replace('&', ""),
            None => String::new(),
        }
    }
}

/// Hosted item: a library control or the search field.
struct Hosted {
    core: Rc<WidgetCore>,
    width: i32,
    /// Name shown in `»` and customization.
    name: String,
    /// `»` action: opens the control in a floating window below the button.
    menu_id: u32,
}

/// Dragging a button to reorder the bar.
#[derive(Clone, Copy, Debug)]
struct Reorder {
    id: u32,
    start: Point,
    active: bool,
    /// Insertion position among visible items (index into `items`).
    target: Option<usize>,
}

enum Kind {
    Button { data: CommandButton, menu_id: u32 },
    Separator,
    Spacer,
    Label(String),
    Widget(Hosted),
    Search(Hosted),
}

struct Item {
    id: u32,
    kind: Kind,
    visible: bool,
    /// Position calculated by the last layout (`None` = in `»` or invisible).
    rect: Option<Rect>,
}

/// Item section under the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Main,
    Arrow,
    Clear,
}

/// Item type for layout calculations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    Fixed(i32),
    Separator,
    Spacer,
}

/// Layout result: item positions and widths (`None` if moved to `»`) and overflow button position if needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Placement {
    pub items: Vec<Option<(i32, i32)>>,
    pub overflow: Option<i32>,
}

/// Distributes items across the available width (logical units).
pub(crate) fn place(slots: &[Slot], available: i32) -> Placement {
    let width_of = |slot: &Slot| match slot {
        Slot::Fixed(w) => *w,
        Slot::Separator => SEPARATOR,
        Slot::Spacer => 0,
    };
    let mut included: Vec<bool> = vec![true; slots.len()];
    let mut overflow = false;
    let total = |included: &[bool], overflow: bool| {
        let chosen: Vec<&Slot> = slots.iter().zip(included).filter(|(_, &on)| on).map(|(s, _)| s).collect();
        let count = chosen.iter().filter(|s| !matches!(s, Slot::Spacer)).count() as i32;
        let widths: i32 = chosen.iter().map(|s| width_of(s)).sum();
        let extra = if overflow { GAP + OVERFLOW } else { 0 };
        2 * PAD_X + widths + GAP * (count - 1).max(0) + extra
    };
    while total(&included, overflow) > available {
        // Retira o último item de largura fixa ainda presente.
        let Some(last) = (0..slots.len()).rev().find(|&i| included[i] && !matches!(slots[i], Slot::Spacer)) else {
            break;
        };
        included[last] = false;
        overflow = true;
    }
    if overflow {
        // Itens depois do primeiro retirado também vão para o » (a ordem se mantém).
        if let Some(first_out) = (0..slots.len()).find(|&i| !included[i] && !matches!(slots[i], Slot::Spacer)) {
            for flag in included.iter_mut().skip(first_out) {
                *flag = false;
            }
        }
    }
    // Separadores nas pontas não separam nada.
    let order: Vec<usize> = (0..slots.len()).filter(|&i| included[i] && !matches!(slots[i], Slot::Spacer)).collect();
    if let Some(&first) = order.first() {
        if slots[first] == Slot::Separator {
            included[first] = false;
        }
    }
    if let Some(&last) = order.last() {
        if slots[last] == Slot::Separator {
            included[last] = false;
        }
    }
    let spacers = (0..slots.len()).filter(|&i| included[i] && slots[i] == Slot::Spacer).count() as i32;
    let free = if overflow || spacers == 0 { 0 } else { (available - total(&included, false)).max(0) };
    let mut x = PAD_X;
    let mut items = vec![None; slots.len()];
    let mut spacer_index = 0;
    let mut first = true;
    for (i, slot) in slots.iter().enumerate() {
        if !included[i] {
            continue;
        }
        if let Slot::Spacer = slot {
            let share = free / spacers + i32::from(spacer_index < free % spacers.max(1));
            spacer_index += 1;
            items[i] = Some((x, share));
            x += share;
            continue;
        }
        if !first {
            x += GAP;
        }
        first = false;
        let w = width_of(slot);
        items[i] = Some((x, w));
        x += w;
    }
    let overflow = overflow.then(|| (available - PAD_X - OVERFLOW).max(x + GAP));
    Placement { items, overflow }
}

struct BarState {
    core: Weak<WidgetCore>,
    items: RefCell<Vec<Item>>,
    next_id: Cell<u32>,
    style: Cell<Option<MenuStyle>>,
    bottom_border: Cell<bool>,
    hot: Cell<Option<(u32, Part)>>,
    pressed: Cell<Option<(u32, Part)>>,
    /// Item with an open menu.
    open: Cell<Option<u32>>,
    /// Item with keyboard focus (when the bar has focus).
    focus: Cell<Option<u32>>,
    overflow: Cell<Option<Rect>>,
    tooltip_ids: RefCell<Vec<usize>>,
    reorderable: Cell<bool>,
    reorder: Cell<Option<Reorder>>,
    reordered: RefCell<Vec<Rc<dyn Fn()>>>,
    /// Floating window for a hosted control that was in `»`.
    flyout: RefCell<Option<(crate::core::Window, u32)>>,
    /// Reference order and visibility for "Restore defaults".
    defaults: RefCell<Option<CommandBarLayout>>,
    /// Separators added by the user during customization.
    added_separators: RefCell<std::collections::HashSet<u32>>,
}

/// Executes button `id` (click or toggle).
fn activate(state: &Rc<BarState>, id: u32) {
    let Some((data, _)) = state.button(id) else { return };
    if !data.enabled {
        return;
    }
    match data.mode {
        Mode::Toggle => {
            let checked = !data.checked;
            set_button(state, id, |b| b.checked = checked);
            if let Some(f) = data.on_toggle {
                f(checked);
            }
        }
        Mode::Dropdown(_) => open_menu(state, id, false),
        Mode::Normal | Mode::Split(_) => {
            if let Some(f) = data.on_click {
                f();
            }
        }
    }
}

/// Opens the menu for button `id` (or `»`).
fn open_menu(state: &Rc<BarState>, id: u32, keyboard: bool) {
    let Some(anchor) = state.screen_rect(id) else { return };
    let source = if id == OVERFLOW_ID {
        DropSource::Entries(state.overflow_entries())
    } else {
        match state.button(id) {
            Some((data, _)) if data.enabled => match data.menu_of() {
                Some(menu) => DropSource::Menu(menu.inner.clone()),
                None => return,
            },
            _ => return,
        }
    };
    let weak = Rc::downgrade(state);
    let on_end: Box<dyn FnOnce()> = Box::new(move || {
        if let Some(state) = weak.upgrade() {
            if state.open.get() == Some(id) {
                state.open.set(None);
                state.pressed.set(None);
            }
            // Enquanto o menu esteve aberto a barra não recebeu o mouse.
            state.hot.set(state.hit_at_cursor());
            state.invalidate();
        }
    });
    let weak = Rc::downgrade(state);
    let switch: popup::SwitchFn = Rc::new(move |point: POINT| {
        let Some(state) = weak.upgrade() else { return false };
        let Some(core) = state.core() else { return false };
        let mut local = point;
        // SAFETY: ponteiro válido para um POINT local.
        unsafe { ScreenToClient(core.hwnd(), &mut local) };
        let dpi = core.dpi();
        let logical = Point::new(to_logical(local.x, dpi), to_logical(local.y, dpi));
        match state.hit(logical) {
            Some((other, part)) if Some(other) != state.open.get() && opens_menu(&state, other, part) => {
                open_menu(&state, other, false);
                true
            }
            _ => false,
        }
    });
    if popup::open_dropdown(state.hwnd(), source, anchor, keyboard, on_end, Some(switch)) {
        state.open.set(Some(id));
        state.invalidate();
    }
}

/// Whether clicking section `part` of item `id` opens a menu.
fn opens_menu(state: &BarState, id: u32, part: Part) -> bool {
    if id == OVERFLOW_ID {
        return true;
    }
    match state.button(id) {
        Some((data, _)) if data.enabled => match data.mode {
            Mode::Dropdown(_) => true,
            Mode::Split(_) => part == Part::Arrow,
            _ => false,
        },
        _ => false,
    }
}

fn set_button(state: &BarState, id: u32, f: impl FnOnce(&mut CommandButton)) -> bool {
    let changed = {
        let mut items = state.items.borrow_mut();
        match items.iter_mut().find(|i| i.id == id).map(|i| &mut i.kind) {
            Some(Kind::Button { data, .. }) => {
                f(data);
                true
            }
            _ => false,
        }
    };
    if changed {
        state.relayout();
    }
    changed
}

/// Library-drawn command bar for the top of a window or panel.
///
/// Contains, from left to right:
///
/// * buttons ([`CommandButton`]) with icon, text or both; toggles; menu buttons (`▾`) or split action/menu buttons;
/// * separators ([`add_separator`](CommandBar::add_separator)) and flexible spaces
///   ([`add_spacer`](CommandBar::add_spacer), pushing following items right);
/// * labels, a search field ([`add_search`](CommandBar::add_search)) and any library control
///   ([`add_widget`](CommandBar::add_widget));
/// * the `»` button containing items that do not fit the current width.
///
/// Button menus are drawn like [`MenuStrip`](crate::menus::MenuStrip) menus: same theme, icons at any DPI, submenus and
/// full keyboard support. Hovering over another menu button switches lists. When the bar has focus (Tab), arrows select
/// buttons, Enter/Space executes them, and Down or F4 opens menus.
///
/// This is a regular control: position it with layouts, reserving [`height`](CommandBar::height).
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window, page: &Menu) -> GuiResult<()> {
/// let bar = CommandBar::builder().parent(window).build()?;
/// bar.add_button(CommandButton::new("Back").icon(&icons::BACK).icon_only().on_click(|| {}))?;
/// bar.add_button(CommandButton::new("Forward").icon(&icons::FORWARD).icon_only().on_click(|| {}))?;
/// bar.add_separator()?;
/// bar.add_button(CommandButton::new("Page").icon(&icons::FILE).menu(page))?;
/// bar.add_spacer()?;
/// let search = bar.add_search("Search", 220, |text| println!("search {text}"))?;
///
/// let root = StackLayout::vertical();
/// root.add_with(&bar, LayoutParams::new().height(bar.height()));
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct CommandBar {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<BarState>,
}

impl std::fmt::Debug for CommandBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBar").field("items", &self.state.items.borrow().len()).finish()
    }
}

impl Widget for CommandBar {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

/// Builder for [`CommandBar`].
#[derive(Clone, Debug)]
pub struct CommandBarBuilder {
    opts: ControlOptions,
    style: Option<MenuStyle>,
    bottom_border: bool,
}

impl CommandBarBuilder {
    builder_common!();

    /// Custom colors (default: window theme, as with `MenuStrip`).
    pub fn style(mut self, style: MenuStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Thin bottom border separating the bar from content (default: enabled).
    pub fn bottom_border(mut self, border: bool) -> Self {
        self.bottom_border = border;
        self
    }

    /// Creates the bar.
    pub fn build(self) -> GuiResult<CommandBar> {
        let core = self.opts.create(
            "CommandBar",
            crate::core::window::PANEL_CLASS,
            WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            WS_EX_CONTROLPARENT,
            true,
            None,
        )?;
        let state = Rc::new(BarState {
            core: Rc::downgrade(&core),
            items: RefCell::new(Vec::new()),
            next_id: Cell::new(1),
            style: Cell::new(self.style),
            bottom_border: Cell::new(self.bottom_border),
            hot: Cell::new(None),
            pressed: Cell::new(None),
            open: Cell::new(None),
            focus: Cell::new(None),
            overflow: Cell::new(None),
            tooltip_ids: RefCell::new(Vec::new()),
            reorderable: Cell::new(false),
            reorder: Cell::new(None),
            reordered: RefCell::new(Vec::new()),
            flyout: RefCell::new(None),
            defaults: RefCell::new(None),
            added_separators: RefCell::new(std::collections::HashSet::new()),
        });
        *core.accessible_tree.borrow_mut() =
            Some(TreeHost::new(BarTree { state: Rc::downgrade(&state), hwnd: core.hwnd() }));
        let bar = CommandBar { core: core.clone(), state: state.clone() };
        bar.wire();
        Ok(bar)
    }
}

impl CommandBar {
    /// Starts building a bar (default size 400×[`COMMAND_BAR_HEIGHT`]).
    pub fn builder() -> CommandBarBuilder {
        CommandBarBuilder { opts: ControlOptions::new(400, COMMAND_BAR_HEIGHT), style: None, bottom_border: true }
    }

    /// Connects drawing, mouse and keyboard to bar state.
    fn wire(&self) {
        let core = &self.core;
        let weak = Rc::downgrade(&self.state);
        *core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let Some(state) = weak.upgrade() {
                paint(&state, canvas);
            }
        }));
        let state = self.state.clone();
        core.on(EventKind::Resize, move |_| state.relayout());
        let state = self.state.clone();
        core.on(EventKind::DpiChanged, move |_| state.relayout());
        let state = self.state.clone();
        core.on(EventKind::ThemeChanged, move |_| state.relayout());
        let state = self.state.clone();
        core.on(EventKind::MouseMove, move |event| {
            let Event::MouseMove(e) = event else { return };
            if let Some(mut drag) = state.reorder.get() {
                let (dx, dy) = ((e.position.x - drag.start.x).abs(), (e.position.y - drag.start.y).abs());
                if !drag.active && (dx >= 4 || dy >= 4) {
                    drag.active = true;
                }
                if drag.active {
                    drag.target = state.insertion_at(e.position.x);
                    state.reorder.set(Some(drag));
                    state.invalidate();
                    return;
                }
            }
            let hit = state.hit(e.position);
            if state.hot.replace(hit) != hit {
                state.invalidate();
            }
        });
        let state = self.state.clone();
        core.on(EventKind::MouseLeave, move |_| {
            if state.hot.replace(None).is_some() {
                state.invalidate();
            }
        });
        let state = self.state.clone();
        core.on(EventKind::MouseDown, move |event| {
            let Event::MouseDown(e) = event else { return };
            if e.button != MouseButton::Left {
                return;
            }
            let Some((id, part)) = state.hit(e.position) else { return };
            if opens_menu(&state, id, part) {
                open_menu(&state, id, false);
                return;
            }
            if let Some(search) = search_of(&state, id) {
                if part == Part::Clear {
                    TextBox { core: search.clone() }.set_text("");
                }
                // SAFETY: foco em uma janela filha viva desta thread.
                unsafe { SetFocus(search.hwnd()) };
                state.invalidate();
                return;
            }
            if state.reorderable.get() && state.button(id).is_some() {
                state.reorder.set(Some(Reorder { id, start: e.position, active: false, target: None }));
            }
            if state.button(id).is_some_and(|(d, _)| d.enabled) || state.reorder.get().is_some() {
                state.pressed.set(Some((id, part)));
                // SAFETY: captura do mouse pela própria barra até soltar o botão.
                unsafe { SetCapture(state.hwnd()) };
                state.invalidate();
            }
        });
        let state = self.state.clone();
        core.on(EventKind::MouseUp, move |event| {
            let Event::MouseUp(e) = event else { return };
            let Some(pressed) = state.pressed.take() else { return };
            // SAFETY: libera a captura obtida no MouseDown.
            unsafe { ReleaseCapture() };
            state.invalidate();
            if let Some(drag) = state.reorder.take() {
                if drag.active {
                    if let Some(target) = drag.target {
                        state.move_item(drag.id, target);
                    }
                    return;
                }
            }
            if state.hit(e.position) == Some(pressed) && state.button(pressed.0).is_some_and(|(d, _)| d.enabled) {
                activate(&state, pressed.0);
            }
        });
        let state = self.state.clone();
        core.on(EventKind::KeyDown, move |event| {
            let Event::KeyDown(e) = event else { return };
            on_key(&state, e.key);
        });
        let state = self.state.clone();
        core.on(EventKind::Focus, move |_| {
            if state.focus.get().is_none() {
                state.focus.set(focus_order(&state).first().copied());
            }
            state.invalidate();
            announce_focus(&state);
        });
        let state = self.state.clone();
        core.on(EventKind::Blur, move |_| state.invalidate());
        let state = self.state.clone();
        core.on(EventKind::Destroyed, move |_| {
            let items = std::mem::take(&mut *state.items.borrow_mut());
            for item in &items {
                match &item.kind {
                    Kind::Button { menu_id, .. } => app::unregister_menu_handler(*menu_id),
                    Kind::Widget(h) | Kind::Search(h) => app::unregister_menu_handler(h.menu_id),
                    _ => {}
                }
            }
            drop(items);
            let flyout = state.flyout.borrow_mut().take();
            if let Some((window, _)) = flyout {
                window.core().destroy();
            }
            let released = std::mem::take(&mut *state.reordered.borrow_mut());
            drop(released);
        });
        // Enter chega à barra (sem ela, o gerenciador de diálogo o trataria).
        core.set_raw_hook(|raw| {
            if raw.message != WM_GETDLGCODE {
                return None;
            }
            Some(DLGC_WANTARROWS | DLGC_WANTCHARS | if wants_enter(raw.lparam) { DLGC_WANTALLKEYS } else { 0 })
        });
    }

    fn push(&self, kind: Kind) -> GuiResult<CommandItem> {
        self.core.require_alive()?;
        let id = self.state.next_id.get();
        self.state.next_id.set(id + 1);
        self.state.items.borrow_mut().push(Item { id, kind, visible: true, rect: None });
        self.state.relayout();
        Ok(CommandItem { state: Rc::downgrade(&self.state), id })
    }

    /// Adds a button.
    pub fn add_button(&self, button: CommandButton) -> GuiResult<CommandItem> {
        let weak = Rc::downgrade(&self.state);
        let id = self.state.next_id.get();
        // Ação usada quando o botão está no menu `»`.
        let menu_id = app::register_menu_handler(Rc::new(move || {
            if let Some(state) = weak.upgrade() {
                activate(&state, id);
            }
        }))?;
        self.push(Kind::Button { data: button, menu_id })
    }

    /// Adds a vertical separator.
    pub fn add_separator(&self) -> GuiResult<CommandItem> {
        self.push(Kind::Separator)
    }

    /// Adds flexible space: following items align right (multiple spaces share the remainder equally).
    pub fn add_spacer(&self) -> GuiResult<CommandItem> {
        self.push(Kind::Spacer)
    }

    /// Adds static text.
    pub fn add_label(&self, text: &str) -> GuiResult<CommandItem> {
        self.push(Kind::Label(text.to_string()))
    }

    /// Hosts a library control created with `.parent(&bar)`, with logical width `width`; its own height is preserved
    /// and vertically centered.
    pub fn add_widget(&self, widget: &impl Widget, width: i32) -> GuiResult<CommandItem> {
        let child = widget.core();
        let hwnd = child.require_alive()?;
        // SAFETY: consulta simples.
        if unsafe { GetParent(hwnd) } != self.core.hwnd() {
            return Err(GuiError::InvalidArgument("the control must be created with .parent(&bar)".into()));
        }
        let name = match native::window_text(hwnd) {
            text if !text.is_empty() => plain_text(&text),
            _ => child.kind.to_string(),
        };
        let menu_id = self.flyout_handler()?;
        self.push(Kind::Widget(Hosted { core: child.clone(), width: width.max(1), name, menu_id }))
    }

    /// `»` action for a hosted control (id belongs to the next item).
    fn flyout_handler(&self) -> GuiResult<u32> {
        let weak = Rc::downgrade(&self.state);
        let id = self.state.next_id.get();
        app::register_menu_handler(Rc::new(move || {
            if let Some(state) = weak.upgrade() {
                // Adiado: a lista do `»` termina de fechar antes de a janela abrir.
                let _ = crate::core::defer(move || open_flyout(&state, id));
            }
        }))
    }

    /// Adds a search field with magnifier, `placeholder` text and `×` clear button. Enter calls `on_search` with the
    /// text; Esc clears it. Returns the field's [`TextBox`] (for text access, `on_text_changed`, focus, etc.).
    pub fn add_search(&self, placeholder: &str, width: i32, on_search: impl Fn(&str) + 'static) -> GuiResult<TextBox> {
        let edit = WidgetCore::create_child(
            "TextBox",
            &self.core,
            "EDIT",
            "",
            WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL,
            0,
            Rect::new(0, 0, 10, 10),
            None,
        )?;
        let search = TextBox { core: edit.clone() };
        search.set_placeholder(placeholder);
        let on_search: Rc<dyn Fn(&str)> = Rc::new(on_search);
        let weak = Rc::downgrade(&edit);
        edit.set_raw_hook(move |raw| {
            let core = weak.upgrade()?;
            let field = TextBox { core };
            match raw.message {
                WM_GETDLGCODE if wants_enter(raw.lparam) || wants_escape(raw.lparam) => {
                    Some(DLGC_WANTALLKEYS | DLGC_WANTCHARS | DLGC_HASSETSEL | DLGC_WANTARROWS)
                }
                WM_KEYDOWN if raw.wparam as i32 == VK_RETURN => {
                    on_search(&field.text());
                    Some(0)
                }
                WM_KEYDOWN if raw.wparam as i32 == VK_ESCAPE && !field.text().is_empty() => {
                    field.set_text("");
                    Some(0)
                }
                // Enter e Esc já tratados: sem o bipe do campo de uma linha.
                WM_CHAR if raw.wparam == 0x0D || raw.wparam == 0x1B => Some(0),
                _ => None,
            }
        });
        let weak = Rc::downgrade(&self.state);
        let repaint = move |_: &Event| {
            if let Some(state) = weak.upgrade() {
                state.invalidate();
            }
        };
        edit.on(EventKind::TextChanged, repaint.clone());
        edit.on(EventKind::Focus, repaint.clone());
        edit.on(EventKind::Blur, repaint);
        let menu_id = self.flyout_handler()?;
        self.push(Kind::Search(Hosted { core: edit, width: width.max(80), name: placeholder.to_string(), menu_id }))?;
        Ok(search)
    }

    /// Number of items, including separators and spaces.
    pub fn len(&self) -> usize {
        self.state.items.borrow().len()
    }

    /// Whether the bar has no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Removes all items (hosted controls are destroyed).
    pub fn clear(&self) {
        popup::cancel_for(native::root_of(self.core.hwnd()));
        let items = std::mem::take(&mut *self.state.items.borrow_mut());
        for item in items {
            match item.kind {
                Kind::Button { menu_id, .. } => app::unregister_menu_handler(menu_id),
                Kind::Widget(h) | Kind::Search(h) => {
                    app::unregister_menu_handler(h.menu_id);
                    h.core.destroy();
                }
                _ => {}
            }
        }
        self.state.relayout();
    }

    /// Current logical height (default [`COMMAND_BAR_HEIGHT`]); reserve it in layouts.
    pub fn height(&self) -> i32 {
        self.core.bounds().height
    }

    /// Sets colors (`None` follows the window theme again).
    pub fn set_style(&self, style: Option<MenuStyle>) {
        self.state.style.set(style);
        self.state.invalidate();
    }

    /// Effective style.
    pub fn style(&self) -> MenuStyle {
        self.state.style()
    }

    /// Number of items overflowing into `»` at the current width.
    pub fn overflow_count(&self) -> usize {
        self.state
            .items
            .borrow()
            .iter()
            .filter(|i| i.visible && i.rect.is_none() && !matches!(i.kind, Kind::Spacer | Kind::Separator))
            .count()
    }

    /// Opens the `»` menu if nonempty, as with the keyboard.
    pub fn open_overflow(&self) {
        if self.state.overflow.get().is_some() {
            open_menu(&self.state, OVERFLOW_ID, true);
        }
    }
}

fn wants_enter(lparam: isize) -> bool {
    keydown_of(lparam) == Some(VK_RETURN)
}

fn wants_escape(lparam: isize) -> bool {
    keydown_of(lparam) == Some(VK_ESCAPE)
}

fn keydown_of(lparam: isize) -> Option<i32> {
    if lparam == 0 {
        return None;
    }
    // SAFETY: em WM_GETDLGCODE o LPARAM, quando não nulo, aponta para o MSG que o
    // gerenciador de diálogo está avaliando, válido durante a chamada.
    let msg = unsafe { &*(lparam as *const MSG) };
    (msg.message == WM_KEYDOWN).then_some(msg.wParam as i32)
}

fn search_of(state: &BarState, id: u32) -> Option<Rc<WidgetCore>> {
    state.items.borrow().iter().find(|i| i.id == id).and_then(|item| match &item.kind {
        Kind::Search(h) => Some(h.core.clone()),
        _ => None,
    })
}

/// A [`CommandBar`] item, for changes after creation.
///
/// Button methods (`set_text`, `set_checked`, etc.) do nothing on separators, spaces and hosted controls.
#[derive(Clone)]
pub struct CommandItem {
    state: Weak<BarState>,
    id: u32,
}

impl std::fmt::Debug for CommandItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandItem").field("id", &self.id).finish()
    }
}

impl CommandItem {
    fn update(&self, f: impl FnOnce(&mut CommandButton)) {
        if let Some(state) = self.state.upgrade() {
            set_button(&state, self.id, f);
        }
    }

    fn read<R>(&self, f: impl FnOnce(&CommandButton) -> R) -> Option<R> {
        let state = self.state.upgrade()?;
        let (data, _) = state.button(self.id)?;
        Some(f(&data))
    }

    /// Enables or disables the button.
    pub fn set_enabled(&self, enabled: bool) {
        self.update(|b| b.enabled = enabled);
    }

    /// Whether the button is enabled.
    pub fn is_enabled(&self) -> bool {
        self.read(|b| b.enabled).unwrap_or(false)
    }

    /// Checks or unchecks a toggle button without calling `on_toggle`.
    pub fn set_checked(&self, checked: bool) {
        self.update(|b| b.checked = checked);
    }

    /// Toggle button state.
    pub fn is_checked(&self) -> bool {
        self.read(|b| b.checked).unwrap_or(false)
    }

    /// Changes button/label text or a hosted control's name (shown in `»` and customization).
    pub fn set_text(&self, text: &str) {
        let Some(state) = self.state.upgrade() else { return };
        let is_label = {
            let mut items = state.items.borrow_mut();
            match items.iter_mut().find(|i| i.id == self.id).map(|i| &mut i.kind) {
                Some(Kind::Label(current)) => {
                    *current = text.to_string();
                    true
                }
                Some(Kind::Widget(h)) | Some(Kind::Search(h)) => {
                    h.name = text.to_string();
                    true
                }
                _ => false,
            }
        };
        if is_label {
            state.relayout();
        } else {
            self.update(|b| b.text = text.to_string());
        }
    }

    /// Button/label text or hosted control name.
    pub fn text(&self) -> String {
        if let Some(text) = self.read(|b| b.text.clone()) {
            return text;
        }
        let Some(state) = self.state.upgrade() else { return String::new() };
        let items = state.items.borrow();
        match items.iter().find(|i| i.id == self.id).map(|i| &i.kind) {
            Some(Kind::Label(text)) => text.clone(),
            Some(Kind::Widget(h)) | Some(Kind::Search(h)) => h.name.clone(),
            _ => String::new(),
        }
    }

    /// Changes the button icon.
    pub fn set_icon(&self, icon: &impl IconSource) {
        let shared = icon.to_shared();
        self.update(|b| b.icon = Some(shared));
    }

    /// Removes the button icon.
    pub fn clear_icon(&self) {
        self.update(|b| b.icon = None);
    }

    /// Changes the button tooltip.
    pub fn set_tooltip(&self, text: &str) {
        self.update(|b| b.tooltip = Some(text.to_string()));
    }

    /// Shows or hides the item (redistributing its space).
    pub fn set_visible(&self, visible: bool) {
        let Some(state) = self.state.upgrade() else { return };
        let changed = {
            let mut items = state.items.borrow_mut();
            match items.iter_mut().find(|i| i.id == self.id) {
                Some(item) if item.visible != visible => {
                    item.visible = visible;
                    true
                }
                _ => false,
            }
        };
        if changed {
            state.relayout();
        }
    }

    /// Whether the item is visible (it may still be in `»`).
    pub fn is_visible(&self) -> bool {
        self.state
            .upgrade()
            .and_then(|s| s.items.borrow().iter().find(|i| i.id == self.id).map(|i| i.visible))
            .unwrap_or(false)
    }

    /// Whether the item overflowed into `»`.
    pub fn is_in_overflow(&self) -> bool {
        self.state
            .upgrade()
            .and_then(|s| s.items.borrow().iter().find(|i| i.id == self.id).map(|i| i.visible && i.rect.is_none()))
            .unwrap_or(false)
    }

    /// Executes the button as if clicked (toggles, executes or opens its menu).
    pub fn trigger(&self) {
        if let Some(state) = self.state.upgrade() {
            activate(&state, self.id);
        }
    }

    /// Opens a menu/split button's menu as with the keyboard.
    pub fn open_menu(&self) {
        if let Some(state) = self.state.upgrade() {
            if state.button(self.id).is_some_and(|(d, _)| d.menu_of().is_some()) {
                open_menu(&state, self.id, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_fill_the_bar_and_spacers_push_to_the_right() {
        let slots = [Slot::Fixed(40), Slot::Separator, Slot::Fixed(60), Slot::Spacer, Slot::Fixed(100)];
        let placed = place(&slots, 400);
        assert_eq!(placed.overflow, None);
        assert_eq!(placed.items[0], Some((PAD_X, 40)));
        assert_eq!(placed.items[1], Some((PAD_X + 40 + GAP, SEPARATOR)));
        let (last_x, last_w) = placed.items[4].unwrap();
        assert_eq!(last_x + last_w, 400 - PAD_X, "o último item encosta na margem direita");
    }

    #[test]
    fn what_does_not_fit_goes_to_overflow() {
        let slots = [Slot::Fixed(80), Slot::Separator, Slot::Fixed(80), Slot::Spacer, Slot::Fixed(80)];
        let placed = place(&slots, 200);
        assert_eq!(placed.items[0], Some((PAD_X, 80)));
        assert_eq!(placed.items[1], None, "separador sem nada à direita some");
        assert!(placed.items[2].is_none() && placed.items[4].is_none());
        let x = placed.overflow.expect("botão »");
        assert!(x + OVERFLOW <= 200);
        let tiny = place(&[Slot::Fixed(80)], 30);
        assert_eq!((tiny.items[0], tiny.overflow.is_some()), (None, true));
    }

    #[test]
    fn spacers_share_the_free_width() {
        let placed = place(&[Slot::Spacer, Slot::Fixed(50), Slot::Spacer], 162);
        let free = 162 - 2 * PAD_X - 50;
        assert_eq!(placed.items[0], Some((PAD_X, free / 2)));
        assert_eq!(placed.items[2].unwrap().1, free - free / 2);
        assert_eq!(place(&[], 100), Placement { items: vec![], overflow: None });
    }
}
