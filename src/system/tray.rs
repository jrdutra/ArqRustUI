//! System tray and Windows notifications.

use crate::core::application::{self as app, WM_TRAY};
use crate::core::handles::{Icon, IconSource};
use crate::core::{GuiError, GuiResult};
use crate::ffi::shell32::*;
use crate::ffi::user32::{WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP};
use crate::ffi::{GUID, HWND};
use crate::menus::ContextMenu;
use crate::system::Cursor;
use crate::utilities::strings::copy_to_buffer;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Notification severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotificationKind {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

type Callback = RefCell<Option<Rc<dyn Fn()>>>;

struct TrayInner {
    id: u32,
    hwnd: HWND,
    icon: RefCell<Icon>,
    removed: Cell<bool>,
    on_click: Callback,
    on_double_click: Callback,
    on_notification_click: Callback,
    menu: RefCell<Option<ContextMenu>>,
}

impl TrayInner {
    fn data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: self.id,
            uFlags: 0,
            uCallbackMessage: WM_TRAY,
            hIcon: self.icon.borrow().raw(),
            szTip: [0; 128],
            dwState: 0,
            dwStateMask: 0,
            szInfo: [0; 256],
            uVersion: 0,
            szInfoTitle: [0; 64],
            dwInfoFlags: 0,
            guidItem: GUID::default(),
            hBalloonIcon: std::ptr::null_mut(),
        }
    }

    fn notify(&self, message: u32, data: &NOTIFYICONDATAW, context: &'static str) -> GuiResult<()> {
        if self.removed.get() {
            return Err(GuiError::WidgetDestroyed);
        }
        // SAFETY: estrutura completa e corretamente dimensionada.
        if unsafe { Shell_NotifyIconW(message, data) } == 0 {
            return Err(GuiError::Unsupported(context));
        }
        Ok(())
    }

    fn handle(&self, event: u32) {
        let pick = |slot: &Callback| slot.borrow().clone();
        match event {
            WM_LBUTTONUP => {
                if let Some(f) = pick(&self.on_click) {
                    f()
                }
            }
            WM_LBUTTONDBLCLK => {
                if let Some(f) = pick(&self.on_double_click) {
                    f()
                }
            }
            WM_RBUTTONUP | WM_CONTEXTMENU => {
                let menu = self.menu.borrow().clone();
                if let Some(menu) = menu {
                    menu.show_at_screen(self.hwnd, Cursor::position());
                }
            }
            NIN_BALLOONUSERCLICK => {
                if let Some(f) = pick(&self.on_notification_click) {
                    f()
                }
            }
            _ => {}
        }
    }

    fn remove(&self) {
        if !self.removed.replace(true) {
            let data = self.data();
            // SAFETY: remove o ícone registrado por este objeto.
            unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
            app::unregister_message(WM_TRAY, self.id as usize);
        }
    }
}

impl Drop for TrayInner {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Tray icon removed when last clone drops.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(app: &Application) -> GuiResult<()> {
/// app.set_quit_on_last_window_closed(false);
/// let tray = TrayIcon::new(&Icon::system(SystemIcon::Application)?, "My application")?;
/// let menu = ContextMenu::new()?;
/// menu.add_item("Exit", Application::quit)?;
/// tray.set_menu(&menu);
/// tray.show_notification("Pronto", "The application remains in the tray.", NotificationKind::Info)?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct TrayIcon {
    inner: Rc<TrayInner>,
}

impl TrayIcon {
    /// Adds tray icon with given tooltip.
    ///
    /// Accepts [`Icon`] or bundled `&icons::CLOUD_SYNC` at system small-icon size.
    pub fn new(icon: &impl IconSource, tooltip: &str) -> GuiResult<TrayIcon> {
        let state = app::require()?;
        let icon = icon.icon_pixels(tray_icon_size())?;
        thread_local! {
            static NEXT_ID: Cell<u32> = const { Cell::new(1) };
        }
        let id = NEXT_ID.with(|n| n.replace(n.get() + 1));
        let inner = Rc::new(TrayInner {
            id,
            hwnd: state.dispatch_hwnd,
            icon: RefCell::new(icon),
            removed: Cell::new(false),
            on_click: RefCell::new(None),
            on_double_click: RefCell::new(None),
            on_notification_click: RefCell::new(None),
            menu: RefCell::new(None),
        });
        let mut data = inner.data();
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        copy_to_buffer(tooltip, &mut data.szTip);
        // SAFETY: estrutura completa e corretamente dimensionada.
        if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
            inner.removed.set(true);
            return Err(GuiError::Unsupported("notification area not available (Shell_NotifyIconW failed)"));
        }
        let weak = Rc::downgrade(&inner);
        app::register_message(
            WM_TRAY,
            id as usize,
            Rc::new(move |_, lparam| {
                if let Some(inner) = weak.upgrade() {
                    inner.handle(lparam as u32 & 0xFFFF);
                }
            }),
        );
        Ok(TrayIcon { inner })
    }

    /// Changes tooltip text.
    pub fn set_tooltip(&self, tooltip: &str) -> GuiResult<()> {
        let mut data = self.inner.data();
        data.uFlags = NIF_TIP;
        copy_to_buffer(tooltip, &mut data.szTip);
        self.inner.notify(NIM_MODIFY, &data, "Shell_NotifyIconW(dica)")
    }

    /// Changes icon.
    pub fn set_icon(&self, icon: &impl IconSource) -> GuiResult<()> {
        *self.inner.icon.borrow_mut() = icon.icon_pixels(tray_icon_size())?;
        let mut data = self.inner.data();
        data.uFlags = NIF_ICON;
        self.inner.notify(NIM_MODIFY, &data, "Shell_NotifyIconW(icon)")
    }

    /// Shows Windows notification associated with icon.
    pub fn show_notification(&self, title: &str, text: &str, kind: NotificationKind) -> GuiResult<()> {
        let mut data = self.inner.data();
        data.uFlags = NIF_INFO;
        copy_to_buffer(title, &mut data.szInfoTitle);
        copy_to_buffer(text, &mut data.szInfo);
        data.dwInfoFlags = match kind {
            NotificationKind::Info => NIIF_INFO,
            NotificationKind::Warning => NIIF_WARNING,
            NotificationKind::Error => NIIF_ERROR,
        };
        self.inner.notify(NIM_MODIFY, &data, "Shell_NotifyIconW(notification)")
    }

    /// Left click on icon.
    pub fn on_click(&self, f: impl Fn() + 'static) {
        *self.inner.on_click.borrow_mut() = Some(Rc::new(f));
    }

    /// Double click on icon.
    pub fn on_double_click(&self, f: impl Fn() + 'static) {
        *self.inner.on_double_click.borrow_mut() = Some(Rc::new(f));
    }

    /// User clicked displayed notification.
    pub fn on_notification_click(&self, f: impl Fn() + 'static) {
        *self.inner.on_notification_click.borrow_mut() = Some(Rc::new(f));
    }

    /// Right-click menu.
    pub fn set_menu(&self, menu: &ContextMenu) {
        *self.inner.menu.borrow_mut() = Some(menu.clone());
    }

    /// Removes tray icon immediately.
    pub fn remove(&self) {
        self.inner.remove();
    }
}

/// Tray icon size: system small-icon size at primary DPI.
fn tray_icon_size() -> i32 {
    crate::core::native::system_metric(crate::ffi::user32::SM_CXSMICON, crate::core::native::system_dpi()).max(16)
}
