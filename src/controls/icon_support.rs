//! Shared infrastructure for controls displaying icons.
//!
//! Two mechanisms:
//!
//! * **control icon** ([`WidgetIcon`]) — buttons and labels store the source and reload it at current DPI when drawing;
//! * **managed image list** ([`ManagedImages`]) — trees, lists, grids, tabs and drop-downs accept per-item icon
//!   sources; each distinct source occupies one slot in an `ImageList` created at the control's DPI and rebuilt on DPI
//!   changes.

use crate::core::handles::{IconSource, ImageList};
use crate::core::native;
use crate::core::widget::WidgetCore;
use crate::core::{GuiError, GuiResult};
use crate::ffi::gdi32::{SelectObject, SetBkMode, SetTextColor};
use crate::ffi::user32::*;
use crate::ffi::{HWND, RECT, UINT};
use crate::utilities::{to_pixels, Color};
use std::collections::HashMap;
use std::rc::Rc;

/// Default logical control icon size (Windows small icon).
pub const DEFAULT_ICON_SIZE: i32 = 16;

/// Logical content inset (icon or text) in library-drawn list items.
const LIST_PADDING: i32 = 4;

/// Icon position relative to button/label text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum IconPosition {
    /// Left of text (default).
    #[default]
    Left,
    /// Right of text.
    Right,
    /// Above text.
    Top,
    /// Below text.
    Bottom,
    /// Centered without adjacent text (icon-only buttons/labels).
    Center,
}

/// Icon displayed by the control itself.
#[derive(Clone)]
pub(crate) struct WidgetIcon {
    pub source: Rc<dyn IconSource>,
    /// Icon side length in logical units.
    pub size: i32,
    /// Automatic button sizing: follows control height and ignores `size`.
    pub auto_size: bool,
    pub position: IconPosition,
    /// `false` if the native control lacks custom content drawing (no Common Controls v6); the library draws the entire
    /// button.
    pub native_accepted: bool,
    /// Original style bits replaced by the icon (labels).
    pub saved_style: u32,
}

/// Sends the image list to the native control.
pub(crate) type Attach = fn(HWND, &ImageList);

/// Library-managed image list for a control.
pub(crate) struct ManagedImages {
    list: Rc<ImageList>,
    logical: i32,
    sources: Vec<Rc<dyn IconSource>>,
    indices: HashMap<usize, i32>,
    attach: Attach,
}

impl ManagedImages {
    /// Index of `source` in the control's list, creating the list or adding the icon as needed.
    pub(crate) fn index_of(
        core: &Rc<WidgetCore>,
        source: &dyn IconSource,
        logical: i32,
        attach: Attach,
    ) -> GuiResult<i32> {
        let hwnd = core.require_alive()?;
        let pixels = to_pixels(logical, core.dpi());
        let mut slot = core.managed_images.borrow_mut();
        let stale = slot.as_ref().is_some_and(|m| m.list.icon_size() != pixels || m.logical != logical);
        if stale {
            let previous = slot.take().expect("checked above");
            *slot = Some(Self::rebuild(hwnd, previous.sources, logical, pixels, attach)?);
        }
        if slot.is_none() {
            let list = ImageList::new(pixels)?;
            attach(hwnd, &list);
            *slot = Some(ManagedImages { list, logical, sources: Vec::new(), indices: HashMap::new(), attach });
        }
        let images = slot.as_mut().expect("created above");
        if let Some(index) = images.indices.get(&source.source_key()) {
            return Ok(*index);
        }
        let index = images.list.add(&source)?;
        images.indices.insert(source.source_key(), index);
        images.sources.push(source.to_shared());
        Ok(index)
    }

    fn rebuild(
        hwnd: HWND,
        sources: Vec<Rc<dyn IconSource>>,
        logical: i32,
        pixels: i32,
        attach: Attach,
    ) -> GuiResult<ManagedImages> {
        let list = ImageList::new(pixels)?;
        let mut indices = HashMap::new();
        for source in &sources {
            let index = list.add(source)?;
            indices.insert(source.source_key(), index);
        }
        attach(hwnd, &list);
        Ok(ManagedImages { list, logical, sources, indices, attach })
    }

    /// Ensures the control has a managed list (empty if newly created).
    pub(crate) fn ensure(core: &Rc<WidgetCore>, logical: i32, attach: Attach) -> GuiResult<()> {
        let hwnd = core.require_alive()?;
        let mut slot = core.managed_images.borrow_mut();
        if slot.is_none() {
            let list = ImageList::new(to_pixels(logical, core.dpi()))?;
            attach(hwnd, &list);
            *slot = Some(ManagedImages { list, logical, sources: Vec::new(), indices: HashMap::new(), attach });
        }
        Ok(())
    }

    /// Changes logical image size, reloading all sources.
    pub(crate) fn resize(core: &Rc<WidgetCore>, logical: i32, attach: Attach) -> GuiResult<()> {
        let hwnd = core.require_alive()?;
        let mut slot = core.managed_images.borrow_mut();
        let sources = slot.take().map(|m| m.sources).unwrap_or_default();
        *slot = Some(Self::rebuild(hwnd, sources, logical, to_pixels(logical, core.dpi()), attach)?);
        Ok(())
    }

    /// Source registered at `index`, if any.
    pub(crate) fn source_at(core: &WidgetCore, index: i32) -> Option<Rc<dyn IconSource>> {
        let slot = core.managed_images.borrow();
        let images = slot.as_ref()?;
        usize::try_from(index).ok().and_then(|i| images.sources.get(i)).cloned()
    }

    /// Forgets the managed list when the application supplies its own.
    pub(crate) fn release(core: &WidgetCore) {
        let previous = core.managed_images.borrow_mut().take();
        drop(previous);
    }

    /// Logical image size of the control's list, if any.
    pub(crate) fn logical_size(core: &WidgetCore) -> Option<i32> {
        core.managed_images.borrow().as_ref().map(|m| m.logical)
    }
}

/// Reloads control icons after a DPI change.
pub(crate) fn refresh_after_dpi_change(core: &Rc<WidgetCore>) {
    let hwnd = core.hwnd();
    if hwnd.is_null() {
        return;
    }
    let mut slot = core.managed_images.borrow_mut();
    if let Some(images) = slot.take() {
        let pixels = to_pixels(images.logical, core.dpi());
        *slot = if images.list.icon_size() == pixels {
            Some(images)
        } else {
            // Se a recriação falhar, o controle continua com a lista antiga (no tamanho anterior).
            match ManagedImages::rebuild(hwnd, images.sources.clone(), images.logical, pixels, images.attach) {
                Ok(rebuilt) => Some(rebuilt),
                Err(_) => Some(images),
            }
        };
    }
    drop(slot);
    if core.icon.borrow().is_some() {
        if core.kind == "Button" {
            super::button::apply_native_icon(core);
        }
        core.invalidate();
    }
}

// ---- Listas desenhadas pela biblioteca (ComboBox e ListBox com ícones) --------

/// Native messages for a text-item list.
#[derive(Clone, Copy)]
pub(crate) struct ListMessages {
    pub text_length: UINT,
    pub text: UINT,
    pub set_data: UINT,
    pub get_data: UINT,
    pub set_height: UINT,
    /// `true` for ComboBox, whose selection field has a separate height.
    pub selection_field: bool,
}

/// Ensures the list was created owner-drawn (`with_icons(true)`).
pub(crate) fn require_owner_drawn(core: &WidgetCore, owner_draw_bit: u32) -> GuiResult<HWND> {
    let hwnd = core.require_alive()?;
    if native::style(hwnd) & owner_draw_bit == 0 {
        return Err(GuiError::Unsupported("item icons require the control to be created with `.with_icons(true)`"));
    }
    Ok(hwnd)
}

/// Sets/removes (`None`) an item icon: item data stores the source's managed-list index plus one.
pub(crate) fn set_item_icon(
    core: &Rc<WidgetCore>,
    owner_draw_bit: u32,
    messages: ListMessages,
    index: usize,
    icon: Option<&dyn IconSource>,
) -> GuiResult<()> {
    let hwnd = require_owner_drawn(core, owner_draw_bit)?;
    let data = match icon {
        Some(icon) => ManagedImages::index_of(core, &icon, DEFAULT_ICON_SIZE, |_, _| {})? as isize + 1,
        None => 0,
    };
    // LB_ERR / CB_ERR (-1) indicam índice inexistente.
    if native::send(hwnd, messages.set_data, index, data) == -1 {
        return Err(GuiError::InvalidArgument(format!("item {index} does not exist")));
    }
    core.invalidate();
    Ok(())
}

/// Item icon source index (`None` without an icon).
pub(crate) fn item_image(core: &WidgetCore, messages: ListMessages, index: usize) -> Option<i32> {
    let data = native::send(core.hwnd(), messages.get_data, index, 0);
    (data > 0).then(|| (data - 1) as i32)
}

/// Adjusts item height to fit the larger of text and icon.
pub(crate) fn fit_item_height(core: &WidgetCore, messages: ListMessages) {
    let hwnd = core.hwnd();
    if hwnd.is_null() {
        return;
    }
    let dpi = core.dpi();
    let text = text_height(hwnd);
    let height = text.max(to_pixels(DEFAULT_ICON_SIZE, dpi)) + to_pixels(4, dpi);
    if messages.selection_field {
        // wParam -1: campo de seleção da ComboBox.
        native::send(hwnd, messages.set_height, usize::MAX, height as isize);
    }
    native::send(hwnd, messages.set_height, 0, height as isize);
}

/// Text line height in pixels for the control's current font.
pub(crate) fn text_height(hwnd: HWND) -> i32 {
    let Ok(dc) = crate::core::WindowDc::get(hwnd) else { return 16 };
    let font = native::send(hwnd, WM_GETFONT, 0, 0);
    let sample: Vec<u16> = "ÁgÇ".encode_utf16().collect();
    let mut rect = RECT::default();
    // SAFETY: DC válido enquanto `dc` existir; a fonte anterior é restaurada.
    unsafe {
        let previous = SelectObject(dc.raw(), font as _);
        DrawTextW(dc.raw(), sample.as_ptr(), sample.len() as i32, &mut rect, DT_CALCRECT | DT_SINGLELINE);
        SelectObject(dc.raw(), previous);
    }
    (rect.bottom - rect.top).max(1)
}

/// Draws a ComboBox/ListBox item with icon and text.
pub(crate) fn paint_list_item(core: &Rc<WidgetCore>, draw: &DRAWITEMSTRUCT, messages: ListMessages) {
    let hwnd = core.hwnd();
    let hdc = draw.hDC;
    let rect = draw.rcItem;
    let dpi = core.dpi();
    let state = draw.itemState;
    // SAFETY: consultas de cores do sistema.
    let system = |index| Color::from_colorref(unsafe { GetSysColor(index) });
    let theme = core.root().and_then(|root| root.theme.borrow().clone());
    let selected = state & ODS_SELECTED != 0;
    let disabled = state & ODS_DISABLED != 0;
    let (background, foreground) = if selected && !disabled {
        match &theme {
            Some(theme) => (theme.accent, Color::WHITE),
            None => (system(COLOR_HIGHLIGHT), system(COLOR_HIGHLIGHTTEXT)),
        }
    } else {
        let background = core
            .background
            .get()
            .or(theme.as_ref().map(|t| t.input_background))
            .unwrap_or_else(|| system(COLOR_WINDOW));
        let foreground = if disabled {
            system(COLOR_GRAYTEXT)
        } else {
            core.foreground
                .get()
                .or(theme.as_ref().map(|t| t.input_foreground))
                .unwrap_or_else(|| system(COLOR_WINDOWTEXT))
        };
        (background, foreground)
    };
    // SAFETY: DC e retângulo fornecidos pelo sistema em WM_DRAWITEM; pincel do cache.
    unsafe { FillRect(hdc, &rect, crate::core::application::brush_for(background)) };
    if draw.itemID == u32::MAX {
        return; // lista vazia: só o fundo (e o foco)
    }
    let index = draw.itemID as usize;
    let padding = to_pixels(LIST_PADDING, dpi);
    let icon_side = to_pixels(DEFAULT_ICON_SIZE, dpi);
    // A coluna do ícone só existe se a lista tem ícones: assim os textos ficam alinhados
    // entre si, e uma lista sem ícones não ganha um recuo vazio.
    let has_icons = ManagedImages::logical_size(core).is_some();
    let height = rect.bottom - rect.top;
    if let Some(source) = item_image(core, messages, index).and_then(|i| ManagedImages::source_at(core, i)) {
        let icon = source.icon_pixels(icon_side).and_then(|icon| if disabled { icon.disabled() } else { Ok(icon) });
        if let Ok(icon) = icon {
            // SAFETY: DC válido; o ícone é mantido vivo por `icon` durante a chamada.
            unsafe {
                DrawIconEx(
                    hdc,
                    rect.left + padding,
                    rect.top + (height - icon_side) / 2,
                    icon.raw(),
                    icon_side,
                    icon_side,
                    0,
                    std::ptr::null_mut(),
                    DI_NORMAL,
                )
            };
        }
    }
    let text = native::read_text(hwnd, messages.text_length, messages.text, index).unwrap_or_default();
    let wide: Vec<u16> = text.encode_utf16().collect();
    let indent = if has_icons { icon_side + to_pixels(super::button::icon_gap(DEFAULT_ICON_SIZE), dpi) } else { 0 };
    let mut text_rect = RECT { left: rect.left + padding + indent, right: rect.right - padding, ..rect };
    let font = native::send(hwnd, WM_GETFONT, 0, 0);
    // SAFETY: DC válido; fonte e modo de fundo são restaurados ao final.
    unsafe {
        let previous = SelectObject(hdc, font as _);
        SetBkMode(hdc, crate::ffi::gdi32::TRANSPARENT);
        SetTextColor(hdc, foreground.to_colorref());
        DrawTextW(
            hdc,
            wide.as_ptr(),
            wide.len() as i32,
            &mut text_rect,
            DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS | DT_LEFT,
        );
        SelectObject(hdc, previous);
        if state & ODS_FOCUS != 0 && state & ODS_NOFOCUSRECT == 0 {
            DrawFocusRect(hdc, &rect);
        }
    }
}
