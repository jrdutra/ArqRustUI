//! Tool bar.

use super::icon_support::{ManagedImages, DEFAULT_ICON_SIZE};
use crate::core::handles::{IconSource, ImageList};
use crate::core::native;
use crate::core::widget::{Reflected, Widget, WidgetCore};
use crate::core::{EventKind, GuiResult};
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;
use crate::utilities::{to_logical, to_wide, Rect};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type Handlers = Rc<RefCell<HashMap<u16, Rc<dyn Fn()>>>>;

/// Toolbar anchored to window top.
///
/// Resizes with the window. Reserve [`height`](ToolBar::height) units at the top in layouts.
#[derive(Clone)]
pub struct ToolBar {
    pub(crate) core: Rc<WidgetCore>,
    handlers: Handlers,
    strings: Rc<RefCell<Vec<Vec<u16>>>>,
}

impl Widget for ToolBar {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl ToolBar {
    /// Creates a toolbar at the top of `parent`.
    pub fn new(parent: &impl Widget) -> GuiResult<ToolBar> {
        let core = WidgetCore::create_child(
            "ToolBar",
            parent.core(),
            "ToolbarWindow32",
            "",
            WS_VISIBLE | TBSTYLE_FLAT | TBSTYLE_LIST | TBSTYLE_TOOLTIPS,
            0,
            Rect::default(),
            None,
        )?;
        let hwnd = core.hwnd();
        native::send(hwnd, TB_BUTTONSTRUCTSIZE, std::mem::size_of::<TBBUTTON>(), 0);
        native::send(hwnd, TB_SETEXTENDEDSTYLE, 0, TBSTYLE_EX_MIXEDBUTTONS as isize);
        let handlers: Handlers = Rc::new(RefCell::new(HashMap::new()));
        let captured = handlers.clone();
        core.set_reflect(move |core, reflected| match reflected {
            Reflected::Command { id, .. } => {
                let handler = captured.borrow().get(id).cloned();
                handler.map(|handler| {
                    handler();
                    0
                })
            }
            Reflected::Notify { code, lparam } if *code == NM_CUSTOMDRAW => {
                crate::themes::dark::toolbar_custom_draw(core, *lparam)
            }
            _ => None,
        });
        let raw = hwnd as isize;
        parent.core().on(EventKind::Resize, move |_| {
            native::send(raw as _, TB_AUTOSIZE, 0, 0);
        });
        Ok(ToolBar { core, handlers, strings: Rc::new(RefCell::new(Vec::new())) })
    }

    fn add(
        &self,
        text: &str,
        icon: Option<&dyn IconSource>,
        style: u8,
        handler: Option<Rc<dyn Fn()>>,
    ) -> GuiResult<u16> {
        let hwnd = self.core.require_alive()?;
        let image = match icon {
            Some(icon) => ManagedImages::index_of(&self.core, &icon, DEFAULT_ICON_SIZE, attach_images)?,
            None => I_IMAGENONE,
        };
        let id = crate::core::application::next_control_id();
        // O controle guarda o ponteiro do texto: a string precisa viver tanto quanto a barra.
        let wide = to_wide(text);
        let pointer = wide.as_ptr() as isize;
        self.strings.borrow_mut().push(wide);
        let button = TBBUTTON {
            iBitmap: image,
            idCommand: id as i32,
            fsState: TBSTATE_ENABLED,
            fsStyle: style,
            bReserved: Default::default(),
            dwData: 0,
            iString: if text.is_empty() { 0 } else { pointer },
        };
        native::send_ref(hwnd, TB_ADDBUTTONSW, 1, &button);
        native::send(hwnd, TB_AUTOSIZE, 0, 0);
        if let Some(handler) = handler {
            self.handlers.borrow_mut().insert(id, handler);
        }
        Ok(id)
    }

    /// Adds a text button with optional icon; returns its identifier.
    ///
    /// Accepts an [`Icon`](crate::core::Icon) or bundled icon (`Some(&icons::SAVE)`); loads at the correct DPI size.
    pub fn add_button(
        &self,
        text: &str,
        icon: Option<&dyn IconSource>,
        on_click: impl Fn() + 'static,
    ) -> GuiResult<u16> {
        self.add(text, icon, BTNS_BUTTON | BTNS_AUTOSIZE | BTNS_SHOWTEXT, Some(Rc::new(on_click)))
    }

    /// Adds a button toggling pressed/released state.
    pub fn add_toggle_button(
        &self,
        text: &str,
        icon: Option<&dyn IconSource>,
        on_toggle: impl Fn(bool) + 'static,
    ) -> GuiResult<u16> {
        let id = self.add(text, icon, BTNS_CHECK | BTNS_AUTOSIZE | BTNS_SHOWTEXT, None)?;
        let core = self.core.clone();
        self.handlers
            .borrow_mut()
            .insert(id, Rc::new(move || on_toggle(native::send(core.hwnd(), TB_ISBUTTONCHECKED, id as usize, 0) != 0)));
        Ok(id)
    }

    /// Adds a separator.
    pub fn add_separator(&self) -> GuiResult<()> {
        self.add("", None, BTNS_SEP, None).map(|_| ())
    }

    /// Enables/disables a button by identifier.
    pub fn set_button_enabled(&self, id: u16, enabled: bool) {
        native::send(self.core.hwnd(), TB_ENABLEBUTTON, id as usize, enabled as isize);
    }

    /// Whether a button is enabled.
    pub fn is_button_enabled(&self, id: u16) -> bool {
        native::send(self.core.hwnd(), TB_ISBUTTONENABLED, id as usize, 0) != 0
    }

    /// Sets toggle button state.
    pub fn set_button_checked(&self, id: u16, checked: bool) {
        native::send(self.core.hwnd(), TB_CHECKBUTTON, id as usize, checked as isize);
    }

    /// Whether a toggle button is pressed.
    pub fn is_button_checked(&self, id: u16) -> bool {
        native::send(self.core.hwnd(), TB_ISBUTTONCHECKED, id as usize, 0) != 0
    }

    /// Number of buttons and separators.
    pub fn button_count(&self) -> usize {
        native::send(self.core.hwnd(), TB_BUTTONCOUNT, 0, 0).max(0) as usize
    }

    /// Executes a button handler as if clicked.
    pub fn trigger(&self, id: u16) {
        let handler = self.handlers.borrow().get(&id).cloned();
        if let Some(handler) = handler {
            handler();
        }
    }

    /// Logical bar height.
    pub fn height(&self) -> i32 {
        let rect = native::window_rect(self.core.hwnd());
        to_logical(rect.bottom - rect.top, self.core.dpi())
    }
}

fn attach_images(hwnd: crate::ffi::HWND, list: &ImageList) {
    native::send(hwnd, TB_SETIMAGELIST, 0, list.raw() as isize);
    native::send(hwnd, TB_AUTOSIZE, 0, 0);
}
