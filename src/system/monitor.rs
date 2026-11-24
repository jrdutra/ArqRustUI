//! Monitor/resolution/DPI information.

use crate::core::native;
use crate::ffi::dynamic::optional;
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::utilities::{from_wide, Point, Rect};

/// Connected monitor description.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Monitor {
    /// Device name, e.g. `\\.\DISPLAY1`.
    pub name: String,
    /// Full monitor area in virtual-desktop pixels.
    pub bounds: Rect,
    /// Work area excluding taskbar in virtual-desktop pixels.
    pub work_area: Rect,
    /// Effective monitor DPI.
    pub dpi: u32,
    /// `true` for primary monitor.
    pub primary: bool,
}

impl Monitor {
    /// Scale factor (`dpi / 96`).
    pub fn scale_factor(&self) -> f32 {
        self.dpi as f32 / 96.0
    }

    fn from_handle(handle: HMONITOR) -> Option<Monitor> {
        if handle.is_null() {
            return None;
        }
        // SAFETY: MONITORINFOEXW é composto de inteiros; zero é válido.
        let mut info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        // SAFETY: estrutura corretamente dimensionada.
        if unsafe { GetMonitorInfoW(handle, &mut info) } == 0 {
            return None;
        }
        let mut dpi = native::system_dpi();
        if let Some(get) = optional().GetDpiForMonitor {
            let (mut x, mut y) = (0u32, 0u32);
            // SAFETY: ponteiros de saída válidos; 0 = MDT_EFFECTIVE_DPI.
            if unsafe { get(handle, 0, &mut x, &mut y) } >= 0 && x != 0 {
                dpi = x;
            }
        }
        Some(Monitor {
            name: from_wide(&info.szDevice),
            bounds: Rect::from_native(info.rcMonitor),
            work_area: Rect::from_native(info.rcWork),
            dpi,
            primary: info.dwFlags & MONITORINFOF_PRIMARY != 0,
        })
    }

    /// Lists connected monitors.
    pub fn all() -> Vec<Monitor> {
        unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
            crate::core::ffi_guard::guard(0, move || {
                // SAFETY: `data` é o ponteiro para o Vec local passado a EnumDisplayMonitors,
                // válido durante toda a enumeração síncrona.
                let list = &mut *(data as *mut Vec<HMONITOR>);
                list.push(monitor);
                TRUE
            })
        }
        let mut handles: Vec<HMONITOR> = Vec::new();
        // SAFETY: o callback só é chamado durante esta chamada, com o ponteiro para `handles`.
        unsafe {
            EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), collect, &mut handles as *mut _ as LPARAM)
        };
        handles.into_iter().filter_map(Monitor::from_handle).collect()
    }

    /// Primary monitor.
    pub fn primary() -> Option<Monitor> {
        Monitor::all().into_iter().find(|m| m.primary)
    }

    /// Monitor containing/nearest a virtual-desktop point.
    pub fn from_point(point: Point) -> Option<Monitor> {
        // SAFETY: consulta simples.
        let handle = unsafe { MonitorFromPoint(POINT { x: point.x, y: point.y }, MONITOR_DEFAULTTONEAREST) };
        Monitor::from_handle(handle)
    }

    pub(crate) fn from_window_handle(hwnd: HWND) -> Option<Monitor> {
        if hwnd.is_null() {
            return None;
        }
        // SAFETY: consulta simples.
        let handle = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        Monitor::from_handle(handle)
    }

    /// Virtual desktop union rectangle in pixels.
    pub fn virtual_screen() -> Rect {
        // SAFETY: consultas de métricas do sistema.
        unsafe {
            Rect::new(
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        }
    }
}
