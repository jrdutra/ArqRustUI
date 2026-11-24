//! Detachable tabs: dragging outside or [`TabControl::detach_tab`] moves content into its own window; closing restores
//! the tab.

use super::TabControl;
use crate::controls::dock::reparent;
use crate::controls::Panel;
use crate::core::native;
use crate::core::widget::{lookup, Widget};
use crate::core::{Event, EventKind, GuiError, GuiResult, Window};
use crate::ffi::comctl32::*;
use crate::ffi::{POINT, RECT};
use crate::utilities::{to_logical, to_pixels, Point, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Logical distance outside the tab strip triggering detachment.
const DETACH_DISTANCE: i32 = 30;

/// Detached tab: title, content and window.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DetachedTab {
    /// Tab title.
    pub title: String,
    /// Content panel (same as before).
    pub page: Panel,
    /// Separate window.
    pub window: Window,
}

type DetachedFn = Rc<dyn Fn(&DetachedTab)>;

#[derive(Default)]
pub(crate) struct DetachState {
    enabled: Cell<bool>,
    dragging: Cell<Option<usize>>,
    detached: RefCell<Vec<DetachedTab>>,
    on_detached: RefCell<Vec<DetachedFn>>,
    on_attached: RefCell<Vec<DetachedFn>>,
}

impl std::fmt::Debug for DetachState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetachState").field("detached", &self.detached.borrow().len()).finish()
    }
}

#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
struct TCHITTESTINFO {
    pt: POINT,
    flags: u32,
}

const TCM_HITTEST: u32 = 0x130D;

impl TabControl {
    pub(super) fn install_detach(&self) {
        let me = self.clone();
        self.core.on(EventKind::MouseDown, move |event| {
            let Event::MouseDown(m) = event else { return };
            if !me.detach.enabled.get() || m.button != crate::input::MouseButton::Left {
                return;
            }
            if let Some(index) = me.tab_at(m.position) {
                me.detach.dragging.set(Some(index));
                // SAFETY: captura o mouse para o controle vivo desta thread durante o arrasto.
                unsafe { crate::ffi::user32::SetCapture(me.core.hwnd()) };
            }
        });
        let me = self.clone();
        self.core.on(EventKind::MouseMove, move |event| {
            let Event::MouseMove(m) = event else { return };
            let Some(index) = me.detach.dragging.get() else { return };
            let strip = me.display_rect().y;
            let size = me.client_size();
            let outside = m.position.y > strip + DETACH_DISTANCE
                || m.position.y < -DETACH_DISTANCE
                || m.position.x < -DETACH_DISTANCE
                || m.position.x > size.width + DETACH_DISTANCE;
            if outside {
                me.detach.dragging.set(None);
                // SAFETY: devolve a captura pega no MouseDown.
                unsafe { crate::ffi::user32::ReleaseCapture() };
                if let Ok(tab) = me.detach_tab(index) {
                    // A janela aparece sob o cursor, já com o título "na mão".
                    let mut point =
                        POINT { x: to_pixels(m.position.x, me.core.dpi()), y: to_pixels(m.position.y, me.core.dpi()) };
                    // SAFETY: converte coordenadas do controle vivo.
                    unsafe { crate::ffi::user32::ClientToScreen(me.core.hwnd(), &mut point) };
                    tab.window.set_bounds(Rect::new(
                        point.x - 60,
                        point.y - 12,
                        tab.window.bounds().width,
                        tab.window.bounds().height,
                    ));
                }
            }
        });
        let me = self.clone();
        self.core.on(EventKind::MouseUp, move |_| {
            if me.detach.dragging.replace(None).is_some() {
                // SAFETY: devolve a captura pega no MouseDown.
                unsafe { crate::ffi::user32::ReleaseCapture() };
            }
        });
    }

    /// Tab index at a logical point.
    pub fn tab_at(&self, at: Point) -> Option<usize> {
        let dpi = self.core.dpi();
        let mut info = TCHITTESTINFO { pt: POINT { x: to_pixels(at.x, dpi), y: to_pixels(at.y, dpi) }, flags: 0 };
        let index = native::send_mut(self.core.hwnd(), TCM_HITTEST, 0, &mut info);
        (index >= 0).then_some(index as usize)
    }

    /// Logical tab header rectangle.
    pub fn tab_bounds(&self, index: usize) -> Option<Rect> {
        let mut rect = RECT::default();
        if native::send_mut(self.core.hwnd(), TCM_GETITEMRECT, index, &mut rect) == 0 {
            return None;
        }
        let dpi = self.core.dpi();
        Some(Rect::new(
            to_logical(rect.left, dpi),
            to_logical(rect.top, dpi),
            to_logical(rect.right - rect.left, dpi),
            to_logical(rect.bottom - rect.top, dpi),
        ))
    }

    /// Allows dragging tabs outside (default: no).
    pub fn set_detachable(&self, detachable: bool) {
        self.detach.enabled.set(detachable);
    }

    /// `true` if tabs can be dragged outside.
    pub fn is_detachable(&self) -> bool {
        self.detach.enabled.get()
    }

    /// Moves tab `index` (title/content) to a separate window owned by the control's window. Closing restores it at the
    /// strip's end.
    pub fn detach_tab(&self, index: usize) -> GuiResult<DetachedTab> {
        if index >= self.tab_count() {
            return Err(GuiError::InvalidArgument(format!("tab {index} does not exist")));
        }
        let root = lookup(native::root_of(self.core.require_alive()?)).ok_or(GuiError::WidgetDestroyed)?;
        let owner = Window { core: root };
        let title = self.tab_title(index);
        let page = self.pages.borrow()[index].clone();
        let size = page.client_size();
        let window =
            Window::builder().title(&title).size(size.width.max(200), size.height.max(120)).owner(&owner).build()?;
        // Tira a aba da faixa sem destruir o conteúdo.
        self.pages.borrow_mut().remove(index);
        native::send(self.core.hwnd(), TCM_DELETEITEM, index, 0);
        if self.selected_index().is_none() && !self.pages.borrow().is_empty() {
            native::send(self.core.hwnd(), TCM_SETCURSEL, index.min(self.pages.borrow().len() - 1), 0);
        }
        self.show_selected_page();
        reparent(page.core(), window.core(), Rect::new(0, 0, window.client_size().width, window.client_size().height));
        page.set_visible(true);
        let tab = DetachedTab { title, page: page.clone(), window: window.clone() };
        self.detach.detached.borrow_mut().push(tab.clone());
        let me = self.clone();
        let handle = window.clone();
        window.on_close_request(move || {
            me.attach_window(&handle);
            true
        });
        let content = page.clone();
        window.core().on(EventKind::Resize, move |event| {
            if let Event::Resize(size) = event {
                if content.core().is_alive() {
                    content.set_bounds(Rect::new(0, 0, size.width, size.height));
                }
            }
        });
        window.show();
        self.core.emit(&Event::SelectionChanged);
        let callbacks = self.detach.on_detached.borrow().clone();
        for callback in callbacks {
            callback(&tab);
        }
        Ok(tab)
    }

    /// Restores the tab in `window` without closing the window. Returns its new index.
    fn attach_window(&self, window: &Window) -> Option<usize> {
        let position = self.detach.detached.borrow().iter().position(|t| t.window.handle() == window.handle())?;
        let tab = self.detach.detached.borrow_mut().remove(position);
        if !self.core.is_alive() || !tab.page.core().is_alive() {
            return None;
        }
        let index = self.tab_count();
        let mut wide = crate::utilities::to_wide(&tab.title);
        let item = TCITEMW {
            mask: TCIF_TEXT,
            dwState: 0,
            dwStateMask: 0,
            pszText: wide.as_mut_ptr(),
            cchTextMax: 0,
            iImage: -1,
            lParam: 0,
        };
        native::send_ref(self.core.hwnd(), TCM_INSERTITEMW, index, &item);
        reparent(tab.page.core(), &self.core, self.display_rect());
        self.pages.borrow_mut().push(tab.page.clone());
        native::send(self.core.hwnd(), TCM_SETCURSEL, index, 0);
        self.arrange_pages();
        self.show_selected_page();
        self.core.emit(&Event::SelectionChanged);
        let callbacks = self.detach.on_attached.borrow().clone();
        for callback in callbacks {
            callback(&tab);
        }
        Some(index)
    }

    /// Restores the detached tab in `window` and closes that window. Returns the tab index.
    pub fn attach_tab(&self, window: &Window) -> Option<usize> {
        let index = self.attach_window(window)?;
        window.on_close_request(|| true);
        window.destroy();
        Some(index)
    }

    /// Currently detached tabs.
    pub fn detached_tabs(&self) -> Vec<DetachedTab> {
        self.detach.detached.borrow().clone()
    }

    /// Called on tab detachment by dragging or code.
    pub fn on_tab_detached(&self, f: impl Fn(&DetachedTab) + 'static) {
        self.detach.on_detached.borrow_mut().push(Rc::new(f));
    }

    /// Called when a detached tab returns.
    pub fn on_tab_attached(&self, f: impl Fn(&DetachedTab) + 'static) {
        self.detach.on_attached.borrow_mut().push(Rc::new(f));
    }
}
