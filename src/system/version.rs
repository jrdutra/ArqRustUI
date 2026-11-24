//! Windows version/capability detection.

use crate::ffi::kernel32::{RtlGetVersion, OSVERSIONINFOW};
use crate::ffi::user32::{GetSystemMetrics, SM_DIGITIZER, SM_MAXIMUMTOUCHES};

/// Operating system version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct WindowsVersion {
    /// Major version (10 for Windows 10/11).
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Build number.
    pub build: u32,
}

impl WindowsVersion {
    /// Running OS version unaffected by compatibility manifests.
    pub fn current() -> WindowsVersion {
        // SAFETY: OSVERSIONINFOW contém apenas inteiros; zero é válido.
        let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
        info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        // SAFETY: estrutura corretamente dimensionada; a função sempre tem sucesso.
        unsafe { RtlGetVersion(&mut info) };
        WindowsVersion { major: info.dwMajorVersion, minor: info.dwMinorVersion, build: info.dwBuildNumber }
    }

    /// Windows 10 or later.
    pub fn is_windows_10_or_later(&self) -> bool {
        self.major >= 10
    }

    /// Windows 11 build 22000 or later.
    pub fn is_windows_11_or_later(&self) -> bool {
        self.major > 10 || (self.major == 10 && self.build >= 22000)
    }

    /// DWM dark title bar from Windows 10 20H1 build 19041.
    pub fn supports_dark_title_bar(&self) -> bool {
        self.major > 10 || (self.major == 10 && self.build >= 19041)
    }

    /// Rounded corners, border/title colors and Mica (Windows 11).
    pub fn supports_window_corners(&self) -> bool {
        self.is_windows_11_or_later()
    }
}

/// Whether a touch digitizer is ready.
pub fn touch_available() -> bool {
    const NID_READY: i32 = 0x80;
    // SAFETY: consulta de métrica do sistema.
    unsafe { GetSystemMetrics(SM_DIGITIZER) & NID_READY != 0 }
}

/// Maximum simultaneous touch contacts, 0 without touch.
pub fn max_touch_points() -> u32 {
    // SAFETY: consulta de métrica do sistema.
    unsafe { GetSystemMetrics(SM_MAXIMUMTOUCHES).max(0) as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_plausible_and_ordered() {
        let version = WindowsVersion::current();
        assert!(version.major >= 6, "versão inesperada: {version:?}");
        let ten = WindowsVersion { major: 10, minor: 0, build: 19045 };
        let eleven = WindowsVersion { major: 10, minor: 0, build: 22631 };
        assert!(ten < eleven);
        assert!(ten.supports_dark_title_bar() && !ten.supports_window_corners());
        assert!(eleven.is_windows_11_or_later() && eleven.supports_window_corners());
        let _ = (touch_available(), max_touch_points());
    }
}
