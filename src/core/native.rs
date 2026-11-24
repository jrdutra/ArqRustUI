//! Low-level abstractions for recurring Win32 calls.
//!
//! Together with `ffi`, concentrates library `unsafe`: helpers encapsulate message pointers, text and style operations
//! behind safe signatures, avoiding per-control unsafe blocks.

use crate::ffi::dynamic::optional;
use crate::ffi::gdi32::{GetDeviceCaps, LOGPIXELSX};
use crate::ffi::kernel32::GetModuleHandleW;
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::utilities::{from_wide, to_wide};

/// Sends a message with simple value parameters, no pointers.
pub(crate) fn send(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: SendMessageW tolera HWND inválido (retorna 0). Esta função só é usada
    // com mensagens cujos parâmetros não são interpretados como ponteiros.
    unsafe { SendMessageW(hwnd, msg, wp, lp) }
}

/// Sends a message with read-only `data` pointer in `LPARAM`.
pub(crate) fn send_ref<T>(hwnd: HWND, msg: UINT, wp: WPARAM, data: &T) -> LRESULT {
    // SAFETY: a referência é válida durante toda a chamada síncrona; o chamador
    // garante que `T` é a estrutura esperada pela mensagem.
    unsafe { SendMessageW(hwnd, msg, wp, data as *const T as LPARAM) }
}

/// Sends a message with read/write `data` pointer in `LPARAM`.
pub(crate) fn send_mut<T>(hwnd: HWND, msg: UINT, wp: WPARAM, data: &mut T) -> LRESULT {
    // SAFETY: a referência exclusiva é válida durante toda a chamada síncrona; o
    // chamador garante que `T` é a estrutura esperada pela mensagem.
    unsafe { SendMessageW(hwnd, msg, wp, data as *mut T as LPARAM) }
}

/// Sends a message with NUL-terminated UTF-16 `LPARAM` string.
pub(crate) fn send_text(hwnd: HWND, msg: UINT, wp: WPARAM, text: &str) -> LRESULT {
    let wide = to_wide(text);
    // SAFETY: `wide` termina em NUL e vive até o fim da chamada síncrona.
    unsafe { SendMessageW(hwnd, msg, wp, wide.as_ptr() as LPARAM) }
}

/// Reads text using length/content message pairs, e.g. `LB_GETTEXTLEN`/`LB_GETTEXT`.
pub(crate) fn read_text(hwnd: HWND, len_msg: UINT, text_msg: UINT, index: WPARAM) -> Option<String> {
    let len = send(hwnd, len_msg, index, 0);
    if len < 0 {
        return None;
    }
    let mut buffer = vec![0u16; len as usize + 2];
    // SAFETY: o buffer comporta o tamanho informado pelo controle mais o terminador.
    let copied = unsafe { SendMessageW(hwnd, text_msg, index, buffer.as_mut_ptr() as LPARAM) };
    if copied < 0 {
        return None;
    }
    Some(from_wide(&buffer))
}

/// Window text (`GetWindowTextW`), empty if absent.
pub(crate) fn window_text(hwnd: HWND) -> String {
    // SAFETY: funções tolerantes a HWND inválido; o buffer tem o tamanho informado.
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; len as usize + 1];
        let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        buffer.truncate(copied.max(0) as usize);
        String::from_utf16_lossy(&buffer)
    }
}

/// Sets window text (`SetWindowTextW`).
pub(crate) fn set_window_text(hwnd: HWND, text: &str) {
    let wide = to_wide(text);
    // SAFETY: string válida terminada em NUL; HWND inválido apenas faz a chamada falhar.
    unsafe { SetWindowTextW(hwnd, wide.as_ptr()) };
}

/// Window style (`GWL_STYLE`). System metric (`SM_*`) at the given DPI.
pub(crate) fn system_metric(index: i32, dpi: u32) -> i32 {
    if let Some(get) = crate::ffi::dynamic::optional().GetSystemMetricsForDpi {
        // SAFETY: função documentada, sem ponteiros.
        return unsafe { get(index, dpi) };
    }
    // SAFETY: consulta simples de métrica.
    let value = unsafe { crate::ffi::user32::GetSystemMetrics(index) };
    crate::utilities::to_pixels(crate::utilities::to_logical(value, system_dpi()), dpi)
}

/// `true` if the window exists and accepts input.
pub(crate) fn is_enabled(hwnd: HWND) -> bool {
    // SAFETY: consulta simples; HWND inválido devolve falso.
    unsafe { crate::ffi::user32::IsWindowEnabled(hwnd) != 0 }
}

pub(crate) fn style(hwnd: HWND) -> u32 {
    // SAFETY: leitura simples; HWND inválido retorna 0.
    unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 }
}

/// Extended window style (`GWL_EXSTYLE`).
pub(crate) fn ex_style(hwnd: HWND) -> u32 {
    // SAFETY: leitura simples; HWND inválido retorna 0.
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 }
}

/// Toggles style bits and forces frame refresh.
pub(crate) fn set_style_bits(hwnd: HWND, index: i32, bits: u32, on: bool) {
    // SAFETY: leitura e escrita de estilos de uma janela; HWND inválido falha sem efeito.
    unsafe {
        let current = GetWindowLongPtrW(hwnd, index) as u32;
        let updated = if on { current | bits } else { current & !bits };
        if updated != current {
            SetWindowLongPtrW(hwnd, index, updated as isize);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

/// Client rectangle in pixels.
pub(crate) fn client_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    // SAFETY: ponteiro de saída válido.
    unsafe { GetClientRect(hwnd, &mut rect) };
    rect
}

/// Outer window rectangle in screen pixels.
pub(crate) fn window_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    // SAFETY: ponteiro de saída válido.
    unsafe { GetWindowRect(hwnd, &mut rect) };
    rect
}

/// Child rectangle in pixels relative to parent client area.
pub(crate) fn rect_in_parent(hwnd: HWND) -> RECT {
    let rect = window_rect(hwnd);
    let mut points = [POINT { x: rect.left, y: rect.top }, POINT { x: rect.right, y: rect.bottom }];
    // SAFETY: `points` tem exatamente 2 elementos; HWND nulo em `from` significa tela.
    unsafe {
        let parent = GetParent(hwnd);
        MapWindowPoints(std::ptr::null_mut(), parent, points.as_mut_ptr(), 2);
    }
    RECT { left: points[0].x, top: points[0].y, right: points[1].x, bottom: points[1].y }
}

/// Current executable module handle.
pub(crate) fn instance() -> HINSTANCE {
    // SAFETY: com nome nulo devolve o módulo do processo; nunca falha.
    unsafe { GetModuleHandleW(std::ptr::null()) }
}

/// System DPI (primary monitor).
pub(crate) fn system_dpi() -> u32 {
    if let Some(get) = optional().GetDpiForSystem {
        // SAFETY: função sem parâmetros.
        let dpi = unsafe { get() };
        if dpi != 0 {
            return dpi;
        }
    }
    let Ok(screen) = crate::core::handles::WindowDc::get(std::ptr::null_mut()) else {
        return 96;
    };
    // SAFETY: consulta de capacidade de um DC válido mantido por `screen`.
    let dpi = unsafe { GetDeviceCaps(screen.raw(), LOGPIXELSX) };
    if dpi > 0 {
        dpi as u32
    } else {
        96
    }
}

/// Effective window DPI, per monitor when supported.
pub(crate) fn dpi_for_window(hwnd: HWND) -> u32 {
    if let Some(get) = optional().GetDpiForWindow {
        // SAFETY: HWND inválido devolve 0, tratado abaixo.
        let dpi = unsafe { get(hwnd) };
        if dpi != 0 {
            return dpi;
        }
    }
    system_dpi()
}

/// Top-level window containing `hwnd`.
pub(crate) fn root_of(hwnd: HWND) -> HWND {
    // SAFETY: consulta simples; HWND inválido devolve nulo.
    unsafe { GetAncestor(hwnd, GA_ROOT) }
}

/// Current modifier key state.
pub(crate) fn key_down(vk: i32) -> bool {
    // SAFETY: consulta de estado sem pré-condições.
    unsafe { GetKeyState(vk) < 0 }
}
