//! System accessibility settings.

use crate::ffi::kernel32::{RegGetValueW, ERROR_SUCCESS, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use crate::ffi::user32::*;
use crate::utilities::to_wide;

/// Reads DWORD from `HKEY_CURRENT_USER`.
pub(crate) fn read_user_dword(subkey: &str, value: &str) -> Option<u32> {
    let (subkey, value) = (to_wide(subkey), to_wide(value));
    let mut data = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: strings terminadas em NUL; buffer de saída de 4 bytes com o tamanho informado.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut _,
            &mut size,
        )
    };
    (status as u32 == ERROR_SUCCESS).then_some(data)
}

fn system_bool(action: u32) -> bool {
    let mut value = 0i32;
    // SAFETY: para estas ações o parâmetro é um ponteiro para BOOL.
    unsafe { SystemParametersInfoW(action, 0, &mut value as *mut i32 as *mut _, 0) != 0 && value != 0 }
}

/// Settings → Accessibility text scaling (1.0 to 2.25).
pub fn text_scale_factor() -> f32 {
    read_user_dword("SOFTWARE\\Microsoft\\Accessibility", "TextScaleFactor")
        .map(|percent| (percent.clamp(100, 225)) as f32 / 100.0)
        .unwrap_or(1.0)
}

/// Whether high contrast is active.
pub fn high_contrast() -> bool {
    let mut info = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        dwFlags: 0,
        lpszDefaultScheme: std::ptr::null_mut(),
    };
    // SAFETY: estrutura corretamente dimensionada.
    let ok = unsafe { SystemParametersInfoW(SPI_GETHIGHCONTRAST, info.cbSize, &mut info as *mut _ as *mut _, 0) };
    ok != 0 && info.dwFlags & HCF_HIGHCONTRASTON != 0
}

/// Whether a screen reader reports running.
pub fn screen_reader_active() -> bool {
    system_bool(SPI_GETSCREENREADER)
}

/// Whether UI animations are enabled.
pub fn animations_enabled() -> bool {
    system_bool(SPI_GETCLIENTAREAANIMATION)
}

/// Whether mnemonic underlines/focus rectangles should always show.
pub fn keyboard_cues_always_visible() -> bool {
    system_bool(SPI_GETKEYBOARDCUES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_do_not_fail() {
        let scale = text_scale_factor();
        assert!((1.0..=2.25).contains(&scale));
        let _ = (high_contrast(), screen_reader_active(), animations_enabled(), keyboard_cues_always_visible());
        assert_eq!(read_user_dword("SOFTWARE\\ArqRustUI\\Inexistente", "Nada"), None);
    }
}
