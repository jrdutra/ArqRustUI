//! DWM effects: dark title, corners, borders, shadow and backdrop materials.
//!
//! Checks Windows version, returning [`GuiError::Unsupported`] for absent features rather than silently failing.

use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult, Window};
use crate::ffi::dwmapi::*;
use crate::ffi::BOOL;
use crate::system::WindowsVersion;
use crate::utilities::{Color, Thickness};

/// Window corner style (Windows 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CornerStyle {
    /// System choice.
    Default,
    /// Square corners.
    Square,
    /// Rounded corners.
    Round,
    /// Slightly rounded corners.
    RoundSmall,
}

/// Window backdrop material (Windows 11 22H2+).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Backdrop {
    /// No material.
    None,
    /// Mica.
    Mica,
    /// Acrylic.
    Acrylic,
    /// Alternative Mica for tabs.
    MicaAlt,
}

fn set_attribute<T>(window: &Window, attribute: u32, value: &T, context: &'static str) -> GuiResult<()> {
    let hwnd = window.core().require_alive()?;
    // SAFETY: `value` é uma referência válida do tamanho informado.
    let hr = unsafe {
        DwmSetWindowAttribute(hwnd, attribute, value as *const T as *const _, std::mem::size_of::<T>() as u32)
    };
    GuiError::check_hresult(context, hr)
}

impl Window {
    /// Toggles dark title bar (Windows 10 20H1+).
    pub fn set_dark_title_bar(&self, dark: bool) -> GuiResult<()> {
        if !WindowsVersion::current().supports_dark_title_bar() {
            return Err(GuiError::Unsupported("dark title bar requires Windows 10 build 19041+"));
        }
        let value: BOOL = dark as BOOL;
        set_attribute(self, DWMWA_USE_IMMERSIVE_DARK_MODE, &value, "DwmSetWindowAttribute(dark)")
    }

    /// Sets rounded corners (Windows 11).
    pub fn set_corner_style(&self, style: CornerStyle) -> GuiResult<()> {
        if !WindowsVersion::current().supports_window_corners() {
            return Err(GuiError::Unsupported("configurable window corners require Windows 11"));
        }
        let value: u32 = match style {
            CornerStyle::Default => 0,
            CornerStyle::Square => 1,
            CornerStyle::Round => 2,
            CornerStyle::RoundSmall => 3,
        };
        set_attribute(self, DWMWA_WINDOW_CORNER_PREFERENCE, &value, "DwmSetWindowAttribute(cantos)")
    }

    /// Sets border color; `None` restores default (Windows 11).
    pub fn set_border_color(&self, color: Option<Color>) -> GuiResult<()> {
        self.set_dwm_color(DWMWA_BORDER_COLOR, color, "DwmSetWindowAttribute(borda)")
    }

    /// Sets title bar color; `None` restores default (Windows 11).
    pub fn set_caption_color(&self, color: Option<Color>) -> GuiResult<()> {
        self.set_dwm_color(DWMWA_CAPTION_COLOR, color, "DwmSetWindowAttribute(caption)")
    }

    /// Sets title text color; `None` restores default (Windows 11).
    pub fn set_title_text_color(&self, color: Option<Color>) -> GuiResult<()> {
        self.set_dwm_color(DWMWA_TEXT_COLOR, color, "DwmSetWindowAttribute(text)")
    }

    fn set_dwm_color(&self, attribute: u32, color: Option<Color>, context: &'static str) -> GuiResult<()> {
        if !WindowsVersion::current().supports_window_corners() {
            return Err(GuiError::Unsupported("DWM frame colors require Windows 11"));
        }
        const DWMWA_COLOR_DEFAULT: u32 = 0xFFFF_FFFF;
        let value = color.map(|c| c.to_colorref()).unwrap_or(DWMWA_COLOR_DEFAULT);
        set_attribute(self, attribute, &value, context)
    }

    /// Sets Mica/Acrylic backdrop (Windows 11 22H2+).
    pub fn set_backdrop(&self, backdrop: Backdrop) -> GuiResult<()> {
        let version = WindowsVersion::current();
        if version.major < 10 || version.build < 22621 {
            return Err(GuiError::Unsupported("backdrop materials require Windows 11 22H2 (build 22621)+"));
        }
        let value: u32 = match backdrop {
            Backdrop::None => 1,
            Backdrop::Mica => 2,
            Backdrop::Acrylic => 3,
            Backdrop::MicaAlt => 4,
        };
        set_attribute(self, DWMWA_SYSTEMBACKDROP_TYPE, &value, "DwmSetWindowAttribute(backdrop)")
    }

    /// Extends DWM frame into client area.
    ///
    /// `Thickness::all(1)` restores system shadow on borderless windows.
    pub fn extend_frame_into_client(&self, margins: Thickness) -> GuiResult<()> {
        let hwnd = self.core().require_alive()?;
        let native = MARGINS {
            cxLeftWidth: margins.left,
            cxRightWidth: margins.right,
            cyTopHeight: margins.top,
            cyBottomHeight: margins.bottom,
        };
        // SAFETY: estrutura local completa.
        let hr = unsafe { DwmExtendFrameIntoClientArea(hwnd, &native) };
        GuiError::check_hresult("DwmExtendFrameIntoClientArea", hr)
    }

    /// Whether DWM composition (shadows/transparency) is active.
    pub fn composition_enabled() -> bool {
        let mut enabled: BOOL = 0;
        // SAFETY: ponteiro de saída válido.
        unsafe { DwmIsCompositionEnabled(&mut enabled) >= 0 && enabled != 0 }
    }
}
