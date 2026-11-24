//! [`MenuStrip`](super::MenuStrip) host messages: nonclient, mouse, Alt/F10/mnemonics and message-loop filter.

use super::*;

/// Owner messages for nonclient strip/menu cancellation.
pub(crate) fn root_message(core: &WidgetCore, hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_ACTIVATE if loword(wp) == WA_INACTIVE => popup::cancel_for(hwnd),
        WM_CANCELMODE | WM_ENTERSIZEMOVE | WM_SIZE | WM_DESTROY | WM_DPICHANGED => popup::cancel_for(hwnd),
        _ => {}
    }
    if !has_strips() {
        return None;
    }
    let strip = strip_of(hwnd)?;
    match msg {
        WM_NCCALCSIZE => {
            let result = core.default_proc(hwnd, msg, wp, lp);
            let height = strip.height_px();
            // SAFETY: com WPARAM verdadeiro o LPARAM aponta para NCCALCSIZE_PARAMS; com
            // falso, para um RECT. Ambos graváveis durante a mensagem.
            unsafe {
                let rect =
                    if wp != 0 { &mut (*(lp as *mut NCCALCSIZE_PARAMS)).rgrc[0] } else { &mut *(lp as *mut RECT) };
                rect.top = (rect.top + height).min(rect.bottom);
            }
            Some(result)
        }
        WM_NCPAINT => {
            core.default_proc(hwnd, msg, wp, lp);
            strip.repaint();
            Some(0)
        }
        WM_NCACTIVATE => {
            let result = core.default_proc(hwnd, msg, wp, lp);
            strip.repaint();
            Some(result)
        }
        WM_NCHITTEST => {
            let point = POINT { x: x_lparam(lp), y: y_lparam(lp) };
            strip.band_contains(point).then_some(HTMENU as LRESULT)
        }
        WM_NCMOUSEMOVE => {
            if wp as u16 == HTMENU {
                if popup::current_owner() != Some(hwnd) {
                    let slot = strip.slot_at_screen(POINT { x: x_lparam(lp), y: y_lparam(lp) });
                    strip.set_state(slot, None);
                }
                if !strip.tracking_leave.get() {
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE | TME_NONCLIENT,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: estrutura local corretamente dimensionada.
                    unsafe { TrackMouseEvent(&mut track) };
                    strip.tracking_leave.set(true);
                }
                return Some(0);
            }
            if popup::current_owner() != Some(hwnd) && strip.hot().is_some() {
                strip.set_state(None, None);
            }
            None
        }
        WM_NCMOUSELEAVE => {
            strip.tracking_leave.set(false);
            if popup::current_owner() != Some(hwnd) {
                strip.set_state(None, None);
            }
            None
        }
        WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK if wp as u16 == HTMENU => {
            if let Some(slot) = strip.slot_at_screen(POINT { x: x_lparam(lp), y: y_lparam(lp) }) {
                popup::open_bar(strip.clone(), slot, false);
            }
            Some(0)
        }
        WM_NCLBUTTONUP | WM_NCRBUTTONDOWN | WM_NCRBUTTONUP | WM_NCRBUTTONDBLCLK | WM_NCMBUTTONDOWN | WM_NCMBUTTONUP
            if wp as u16 == HTMENU =>
        {
            Some(0)
        }
        // Caminho de reserva: Alt, F10 e Alt+letra que chegarem ao DefWindowProc.
        WM_SYSCOMMAND if wp & 0xFFF0 == SC_KEYMENU => {
            if lp == 0 {
                strip.enter_keyboard_mode();
                return Some(0);
            }
            let c = char::from_u32(lp as u32)?;
            let slot = strip.slot_by_mnemonic(c)?;
            popup::open_bar(strip.clone(), slot, true);
            Some(0)
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED => {
            strip.bump();
            None
        }
        WM_NCDESTROY => {
            STRIPS.with(|s| s.borrow_mut().remove(&(hwnd as isize)));
            strip.owner.set(std::ptr::null_mut());
            None
        }
        _ => None,
    }
}

/// Keyboard outside menu mode: Alt, F10, Alt+letter, shortcuts.
pub(crate) fn idle_translate(msg: &MSG) -> bool {
    if !has_strips() {
        return false;
    }
    if !matches!(msg.message, WM_KEYDOWN | WM_SYSKEYDOWN | WM_SYSKEYUP | WM_SYSCHAR) {
        return false;
    }
    let root = native::root_of(msg.hwnd);
    let Some(strip) = strip_of(root) else { return false };
    let vk = msg.wParam as i32;
    match msg.message {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if vk == VK_MENU {
                // Bit 30: a tecla já estava pressionada (repetição automática).
                if msg.lParam & (1 << 30) == 0 {
                    strip.alt_armed.set(true);
                    strip.set_alt_cues(true);
                }
                return false;
            }
            strip.alt_armed.set(false);
            if vk == VK_F10 && !native::key_down(VK_SHIFT) && !native::key_down(VK_CONTROL) {
                popup::suppress_key_up(VK_F10);
                strip.enter_keyboard_mode();
                return true;
            }
            strip.try_shortcut(Shortcut { key: Key(vk as u16), modifiers: Modifiers::current() })
        }
        WM_SYSCHAR => {
            let Some(c) = char::from_u32(msg.wParam as u32) else { return false };
            match strip.slot_by_mnemonic(c) {
                Some(slot) => {
                    strip.alt_armed.set(false);
                    popup::open_bar(strip.clone(), slot, true);
                    true
                }
                None => false,
            }
        }
        WM_SYSKEYUP => {
            if popup::take_suppressed_key_up(vk) {
                strip.set_alt_cues(false);
                return true;
            }
            if vk == VK_MENU {
                if strip.alt_armed.replace(false) {
                    strip.enter_keyboard_mode();
                    return true;
                }
                strip.set_alt_cues(false);
            }
            false
        }
        _ => false,
    }
}
