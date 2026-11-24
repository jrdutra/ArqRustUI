//! Native tooltips.

use crate::core::native;
use crate::core::widget::{CreateParams, Widget, WidgetCore};
use crate::core::GuiResult;
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;
use crate::ffi::{HWND, RECT};
use crate::utilities::{to_pixels, to_wide};
use std::rc::Rc;

/// Gets or creates the root window's shared tooltip control.
fn tooltip_window(root: &Rc<WidgetCore>) -> GuiResult<HWND> {
    let existing = root.tooltip.get();
    if !existing.is_null() {
        return Ok(existing);
    }
    let tooltip = WidgetCore::create(CreateParams {
        kind: "Tooltip",
        class: "tooltips_class32",
        text: "",
        style: WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
        ex_style: WS_EX_TOPMOST,
        parent: root.hwnd(),
        pixel_bounds: RECT::default(),
        id: 0,
        top_level: false,
        subclass: true,
    })?;
    let hwnd = tooltip.hwnd();
    // Largura máxima habilita quebra de linha em dicas longas.
    native::send(hwnd, TTM_SETMAXTIPWIDTH, 0, to_pixels(400, root.dpi()) as isize);
    root.tooltip.set(hwnd);
    Ok(hwnd)
}

/// Sets a control tooltip; empty text removes it.
pub(crate) fn set_for(core: &Rc<WidgetCore>, text: &str) -> GuiResult<()> {
    let target = core.require_alive()?;
    let root = core.root().unwrap_or_else(|| core.clone());
    let tooltip = tooltip_window(&root)?;
    let mut wide = to_wide(text);
    let mut info = TTTOOLINFOW {
        cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
        uFlags: TTF_IDISHWND | TTF_SUBCLASS,
        hwnd: root.hwnd(),
        uId: target as usize,
        rect: RECT::default(),
        hinst: std::ptr::null_mut(),
        lpszText: wide.as_mut_ptr(),
        lParam: 0,
        lpReserved: std::ptr::null_mut(),
    };
    native::send_mut(tooltip, TTM_DELTOOLW, 0, &mut info);
    if !text.is_empty() {
        native::send_mut(tooltip, TTM_ADDTOOLW, 0, &mut info);
    }
    Ok(())
}

/// Replaces region tooltips of a library-drawn control: (identifier, client pixel rectangle, text). `previous` lists
/// old identifiers removed here.
pub(crate) fn set_regions(
    core: &Rc<WidgetCore>,
    previous: &[usize],
    regions: &[(usize, RECT, String)],
) -> GuiResult<()> {
    let target = core.require_alive()?;
    let root = core.root().unwrap_or_else(|| core.clone());
    let tooltip = tooltip_window(&root)?;
    let mut info = TTTOOLINFOW {
        cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
        uFlags: TTF_SUBCLASS,
        hwnd: target,
        uId: 0,
        rect: RECT::default(),
        hinst: std::ptr::null_mut(),
        lpszText: std::ptr::null_mut(),
        lParam: 0,
        lpReserved: std::ptr::null_mut(),
    };
    for id in previous {
        info.uId = *id;
        native::send_mut(tooltip, TTM_DELTOOLW, 0, &mut info);
    }
    for (id, rect, text) in regions {
        if text.is_empty() {
            continue;
        }
        let mut wide = to_wide(text);
        info.uId = *id;
        info.rect = *rect;
        info.lpszText = wide.as_mut_ptr();
        native::send_mut(tooltip, TTM_ADDTOOLW, 0, &mut info);
    }
    Ok(())
}

/// Tooltip operations.
///
/// Usually [`Widget::set_tooltip`] or builder `.tooltip("...")` suffices.
pub struct Tooltip;

impl Tooltip {
    /// Attaches a tooltip to a control.
    pub fn set(widget: &impl Widget, text: &str) -> GuiResult<()> {
        set_for(widget.core(), text)
    }

    /// Removes a control tooltip.
    pub fn remove(widget: &impl Widget) -> GuiResult<()> {
        set_for(widget.core(), "")
    }

    /// Enables/disables all tooltips in the widget's containing window.
    pub fn set_enabled(widget: &impl Widget, enabled: bool) -> GuiResult<()> {
        let root = widget.core().root().unwrap_or_else(|| widget.core().clone());
        let tooltip = tooltip_window(&root)?;
        native::send(tooltip, TTM_ACTIVATE, enabled as usize, 0);
        Ok(())
    }
}
