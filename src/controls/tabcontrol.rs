//! Tab control with tabs detachable into separate windows.

mod detach;

pub use detach::DetachedTab;

use super::icon_support::{ManagedImages, DEFAULT_ICON_SIZE};
use super::panel::Panel;
use crate::core::handles::{IconSource, ImageList};
use crate::core::native;
use crate::core::widget::{Reflected, Widget, WidgetCore};
use crate::core::{Event, EventKind, GuiError, GuiResult, HandlerId};
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;
use crate::ffi::RECT;
use crate::utilities::{to_logical, to_wide, Rect};
use std::cell::RefCell;
use std::rc::Rc;

/// Tab control. Each tab has a content-hosting [`Panel`].
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let tabs = TabControl::builder().bounds(10, 10, 400, 300).parent(window).build()?;
/// let general = tabs.add_tab("General")?;
/// Label::builder().text("Tab content").position(12, 12).parent(&general).build()?;
/// tabs.add_tab("Advanced")?;
/// # Ok(()) }
/// ```
#[derive(Clone, Debug)]
pub struct TabControl {
    pub(crate) core: Rc<WidgetCore>,
    pages: Rc<RefCell<Vec<Panel>>>,
    detach: Rc<detach::DetachState>,
}

impl Widget for TabControl {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

super::common::plain_builder!(
    /// Builder for [`TabControl`].
    TabControlBuilder
);

impl TabControlBuilder {
    /// Creates the tab control.
    pub fn build(self) -> GuiResult<TabControl> {
        let core = self.opts.create(
            "TabControl",
            "SysTabControl32",
            WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            WS_EX_CONTROLPARENT,
            true,
            None,
        )?;
        let tabs = TabControl { core, pages: Rc::new(RefCell::new(Vec::new())), detach: Rc::default() };
        tabs.install_detach();
        let on_select = tabs.clone();
        tabs.core.set_reflect(move |core, reflected| match reflected {
            Reflected::Notify { code, .. } if *code == TCN_SELCHANGE => {
                on_select.show_selected_page();
                core.emit(&Event::SelectionChanged);
                Some(0)
            }
            _ => None,
        });
        let on_resize = tabs.clone();
        tabs.core.on(EventKind::Resize, move |_| on_resize.arrange_pages());
        Ok(tabs)
    }
}

impl TabControl {
    /// Starts building (default size 320×240).
    pub fn builder() -> TabControlBuilder {
        TabControlBuilder { opts: super::common::ControlOptions::new(320, 240) }
    }

    /// Logical tab content area.
    fn display_rect(&self) -> Rect {
        let hwnd = self.core.hwnd();
        let mut rect: RECT = native::client_rect(hwnd);
        native::send_mut(hwnd, TCM_ADJUSTRECT, 0, &mut rect);
        let dpi = self.core.dpi();
        Rect::new(
            to_logical(rect.left, dpi),
            to_logical(rect.top, dpi),
            to_logical((rect.right - rect.left).max(0), dpi),
            to_logical((rect.bottom - rect.top).max(0), dpi),
        )
    }

    fn arrange_pages(&self) {
        let area = self.display_rect();
        for page in self.pages.borrow().iter() {
            page.set_bounds(area);
        }
    }

    fn show_selected_page(&self) {
        let selected = self.selected_index();
        for (index, page) in self.pages.borrow().iter().enumerate() {
            page.set_visible(Some(index) == selected);
        }
    }

    /// Adds a tab and returns its content panel.
    pub fn add_tab(&self, title: &str) -> GuiResult<Panel> {
        let hwnd = self.core.require_alive()?;
        let mut wide = to_wide(title);
        let item = TCITEMW {
            mask: TCIF_TEXT,
            dwState: 0,
            dwStateMask: 0,
            pszText: wide.as_mut_ptr(),
            cchTextMax: 0,
            iImage: -1,
            lParam: 0,
        };
        let index = self.pages.borrow().len();
        if native::send_ref(hwnd, TCM_INSERTITEMW, index, &item) < 0 {
            return Err(GuiError::InvalidArgument("could not insert the tab".into()));
        }
        let page = Panel::builder().parent(self).visible(false).build()?;
        self.pages.borrow_mut().push(page.clone());
        self.arrange_pages();
        if index == 0 {
            native::send(hwnd, TCM_SETCURSEL, 0, 0);
        }
        self.show_selected_page();
        Ok(page)
    }

    /// Removes a tab and destroys its content.
    pub fn remove_tab(&self, index: usize) {
        if index >= self.pages.borrow().len() {
            return;
        }
        let page = self.pages.borrow_mut().remove(index);
        page.destroy();
        native::send(self.core.hwnd(), TCM_DELETEITEM, index, 0);
        if self.selected_index().is_none() && !self.pages.borrow().is_empty() {
            self.set_selected_index(index.min(self.pages.borrow().len() - 1));
        }
        self.arrange_pages();
        self.show_selected_page();
    }

    /// Tab count.
    pub fn tab_count(&self) -> usize {
        native::send(self.core.hwnd(), TCM_GETITEMCOUNT, 0, 0).max(0) as usize
    }

    /// Tab content panel.
    pub fn page(&self, index: usize) -> Option<Panel> {
        self.pages.borrow().get(index).cloned()
    }

    /// Changes a tab title.
    pub fn set_tab_title(&self, index: usize, title: &str) {
        let mut wide = to_wide(title);
        let item = TCITEMW {
            mask: TCIF_TEXT,
            dwState: 0,
            dwStateMask: 0,
            pszText: wide.as_mut_ptr(),
            cchTextMax: 0,
            iImage: -1,
            lParam: 0,
        };
        native::send_ref(self.core.hwnd(), TCM_SETITEMW, index, &item);
    }

    /// Shows an icon before a tab title.
    pub fn set_tab_icon(&self, index: usize, icon: &impl IconSource) -> GuiResult<()> {
        if index >= self.tab_count() {
            return Err(GuiError::InvalidArgument(format!("tab {index} does not exist")));
        }
        let image = ManagedImages::index_of(&self.core, icon, DEFAULT_ICON_SIZE, attach_images)?;
        self.set_tab_image(index, image);
        Ok(())
    }

    /// Removes a tab icon.
    pub fn clear_tab_icon(&self, index: usize) {
        self.set_tab_image(index, -1);
    }

    /// Tab image index (`None` without an icon).
    pub fn tab_image(&self, index: usize) -> Option<i32> {
        let mut item = TCITEMW {
            mask: TCIF_IMAGE,
            dwState: 0,
            dwStateMask: 0,
            pszText: std::ptr::null_mut(),
            cchTextMax: 0,
            iImage: -1,
            lParam: 0,
        };
        native::send_mut(self.core.hwnd(), TCM_GETITEMW, index, &mut item);
        (item.iImage >= 0).then_some(item.iImage)
    }

    fn set_tab_image(&self, index: usize, image: i32) {
        let item = TCITEMW {
            mask: TCIF_IMAGE,
            dwState: 0,
            dwStateMask: 0,
            pszText: std::ptr::null_mut(),
            cchTextMax: 0,
            iImage: image,
            lParam: 0,
        };
        native::send_ref(self.core.hwnd(), TCM_SETITEMW, index, &item);
        // A altura das abas pode mudar com o ícone: reposiciona o conteúdo.
        self.arrange_pages();
    }

    /// Tab title.
    pub fn tab_title(&self, index: usize) -> String {
        let mut buffer = vec![0u16; 256];
        let mut item = TCITEMW {
            mask: TCIF_TEXT,
            dwState: 0,
            dwStateMask: 0,
            pszText: buffer.as_mut_ptr(),
            cchTextMax: buffer.len() as i32,
            iImage: -1,
            lParam: 0,
        };
        native::send_mut(self.core.hwnd(), TCM_GETITEMW, index, &mut item);
        crate::utilities::from_wide(&buffer)
    }

    /// Selected tab index.
    pub fn selected_index(&self) -> Option<usize> {
        let index = native::send(self.core.hwnd(), TCM_GETCURSEL, 0, 0);
        (index >= 0).then_some(index as usize)
    }

    /// Selects a tab programmatically without firing `on_selection_changed`.
    pub fn set_selected_index(&self, index: usize) {
        native::send(self.core.hwnd(), TCM_SETCURSEL, index, 0);
        self.show_selected_page();
    }

    /// The user changed tabs; receives the new index.
    pub fn on_selection_changed(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::SelectionChanged, move |_| {
            if let Some(index) = this.selected_index() {
                f(index)
            }
        })
    }
}

fn attach_images(hwnd: crate::ffi::HWND, list: &ImageList) {
    native::send(hwnd, TCM_SETIMAGELIST, 0, list.raw() as isize);
}
