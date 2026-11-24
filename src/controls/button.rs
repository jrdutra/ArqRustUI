//! Command buttons.

use super::common::{builder_common, ControlOptions};
use super::icon_support::{IconPosition, WidgetIcon, DEFAULT_ICON_SIZE};
use crate::core::handles::IconSource;
use crate::core::native;
use crate::core::widget::{Reflected, Widget, WidgetCore};
use crate::core::{Event, EventKind, GuiResult};
use crate::ffi::user32::*;
use crate::ffi::{LRESULT, RECT};
use crate::graphics::{Canvas, Pen, TextAlign};
use crate::themes::ButtonStyle;
use crate::utilities::{to_logical, to_pixels, Point, Rect, Size};
use std::rc::Rc;

/// Native command button with text and an optional icon.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let button = Button::builder()
///     .text("&Save")
///     .icon(&icons::SAVE)
///     .bounds(20, 20, 120, 32)
///     .parent(window)
///     .build()?;
/// button.on_click(|| MessageBox::info("Save", "Document saved."));
/// # Ok(()) }
/// ```
#[derive(Clone, Debug)]
pub struct Button {
    pub(crate) core: Rc<WidgetCore>,
}

impl Widget for Button {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

/// Builder for [`Button`].
#[derive(Clone, Debug)]
pub struct ButtonBuilder {
    opts: ControlOptions,
    default: bool,
    cancel: bool,
    style: Option<ButtonStyle>,
    icon: Option<Rc<dyn IconSource>>,
    icon_size: Option<i32>,
    icon_position: IconPosition,
    content_alignment: TextAlign,
}

impl ButtonBuilder {
    builder_common!();

    /// Aligns the icon and text group horizontally, preserving the inner margin. Centered by default; the icon's
    /// relative position is unchanged.
    pub fn content_alignment(mut self, alignment: TextAlign) -> Self {
        self.content_alignment = alignment;
        self
    }

    /// Makes this the default button: Enter invokes it from anywhere in the window.
    pub fn default(mut self, default: bool) -> Self {
        self.default = default;
        self
    }

    /// Makes this the cancel button: Esc invokes it.
    pub fn cancel(mut self, cancel: bool) -> Self {
        self.cancel = cancel;
        self
    }

    /// Custom appearance (state colors and rounded corners).
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Icon beside the text. By default its size is 60% of the button height; it forms a centered group with the text,
    /// with spacing between them.
    pub fn icon(mut self, icon: &impl IconSource) -> Self {
        self.icon = Some(icon.to_shared());
        self
    }

    /// Fixed logical icon size instead of the automatic size (60% of button height).
    pub fn icon_size(mut self, size: i32) -> Self {
        self.icon_size = Some(size);
        self
    }

    /// Icon position relative to text (default: left).
    pub fn icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// Creates the button.
    pub fn build(self) -> GuiResult<Button> {
        let style = if self.default { BS_DEFPUSHBUTTON } else { BS_PUSHBUTTON };
        // IDOK (1) e IDCANCEL (2) são os identificadores que a navegação de diálogo
        // do Windows aciona com Enter e Esc.
        let id = if self.default {
            Some(1)
        } else if self.cancel {
            Some(2)
        } else {
            None
        };
        let alignment_style = match self.content_alignment {
            TextAlign::Left => BS_LEFT,
            TextAlign::Center => BS_CENTER,
            TextAlign::Right => BS_RIGHT,
        };
        let core = self.opts.create("Button", "BUTTON", style | alignment_style, 0, true, id)?;
        core.button_style.set(self.style);
        core.button_content_alignment.set(self.content_alignment);
        core.set_reflect(reflect_button);
        // O estado "hover" do desenho personalizado depende de redesenhar ao entrar e sair.
        for kind in [EventKind::MouseEnter, EventKind::MouseLeave] {
            let target = Rc::downgrade(&core);
            core.on(kind, move |_| {
                if let Some(core) = target.upgrade() {
                    if is_owner_drawn(&core) {
                        core.invalidate();
                    }
                }
            });
        }
        refresh_draw_mode(&core);
        let button = Button { core };
        if let Some(icon) = self.icon {
            button.install_icon(icon, self.icon_size, self.icon_position)?;
        }
        Ok(button)
    }
}

/// Default ratio of icon size to button height (60%).
pub(crate) const ICON_HEIGHT_PERCENT: i32 = 60;
/// Minimum distance, in logical units, between the content and the button's sides.
pub(crate) const CONTENT_MARGIN: i32 = 8;

/// Automatic logical icon size: 60% of button height. Above or below text, limited to the space remaining after the
/// text.
pub(crate) fn auto_icon_size(height: i32, position: IconPosition, text_height: i32) -> i32 {
    let size = (height * ICON_HEIGHT_PERCENT + 50) / 100;
    let size = match position {
        IconPosition::Top | IconPosition::Bottom if text_height > 0 => {
            size.min(height - text_height - icon_gap(size) - CONTENT_MARGIN)
        }
        _ => size,
    };
    size.max(8)
}

/// Logical spacing between icon and text: roughly one third of the icon size (4 to 12).
pub(crate) fn icon_gap(icon: i32) -> i32 {
    ((icon + 1) / 3).clamp(4, 12)
}

/// Effective logical icon size in a button of `height` logical units.
fn icon_side(icon: &WidgetIcon, height: i32, text_height: i32) -> i32 {
    if icon.auto_size {
        auto_icon_size(height, icon.position, text_height)
    } else {
        icon.size
    }
}

/// Checks whether the native button uses Common Controls v6, which notifies its parent before drawing content
/// (`NM_CUSTOMDRAW`). Icon and text are drawn over the themed background at that stage. Without v6, the library draws
/// the button. `BCM_GETIDEALSIZE` exists only in v6.
pub(crate) fn apply_native_icon(core: &Rc<WidgetCore>) {
    let hwnd = core.hwnd();
    if hwnd.is_null() || core.icon.borrow().is_none() {
        return;
    }
    let mut ideal = crate::ffi::SIZE { cx: 0, cy: 0 };
    let supported = native::send_mut(hwnd, BCM_GETIDEALSIZE, 0, &mut ideal) != 0;
    if let Some(slot) = core.icon.borrow_mut().as_mut() {
        slot.native_accepted = supported;
    }
    refresh_draw_mode(core);
}

/// Draws a native button's icon and text over the themed background (`CDDS_PREPAINT` of `NM_CUSTOMDRAW`). Returns
/// `None` to let the button draw itself (no icon or other stages).
fn custom_draw(core: &Rc<WidgetCore>, lparam: isize) -> Option<LRESULT> {
    use crate::ffi::comctl32::*;
    use crate::ffi::gdi32::{SelectObject, SetBkMode, SetTextColor};
    use crate::ffi::uxtheme::*;
    // SAFETY: em NM_CUSTOMDRAW o LPARAM aponta para um NMCUSTOMDRAW válido durante a mensagem.
    let draw = unsafe { &*(lparam as *const NMCUSTOMDRAW) };
    if draw.dwDrawStage != CDDS_PREPAINT || is_owner_drawn(core) {
        return None;
    }
    let icon = core.icon.borrow().clone()?;
    let hwnd = core.hwnd();
    let hdc = draw.hdc;
    let dpi = core.dpi();
    let state = draw.uItemState;
    let disabled = state & CDIS_DISABLED != 0;
    let prefix = if state & CDIS_SHOWKEYBOARDCUES != 0 { 0 } else { DT_HIDEPREFIX };
    let text: Vec<u16> = native::window_text(hwnd).encode_utf16().collect();
    let font = native::send(hwnd, WM_GETFONT, 0, 0);
    // SAFETY: o HDC da notificação é válido durante a mensagem; a fonte anterior é
    // restaurada no fim.
    let previous = unsafe { SelectObject(hdc, font as _) };
    let mut measured = RECT::default();
    if !text.is_empty() {
        // SAFETY: texto e retângulo válidos durante a chamada.
        unsafe {
            DrawTextW(hdc, text.as_ptr(), text.len() as i32, &mut measured, DT_CALCRECT | DT_SINGLELINE | prefix)
        };
    }
    let text_size = Size::new(measured.right - measured.left, measured.bottom - measured.top);
    let rc = draw.rc;
    let bounds = Rect::new(rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top);
    let logical = icon_side(&icon, to_logical(bounds.height, dpi), to_logical(text_size.height, dpi));
    let side = to_pixels(logical, dpi);
    let margin = to_pixels(CONTENT_MARGIN, dpi);
    let (icon_at, text_at) = layout_icon_and_text_aligned(
        bounds,
        side,
        text_size,
        icon.position,
        to_pixels(icon_gap(logical), dpi),
        margin,
        core.button_content_alignment.get(),
    );
    // SAFETY: a classe de tema é aberta e fechada nesta chamada; `theme` pode ser nulo
    // (estilos visuais desligados), caso em que se usa o desenho clássico.
    let theme = unsafe { OpenThemeData(hwnd, crate::utilities::to_wide("BUTTON").as_ptr()) };
    // No visual clássico o conteúdo desce um pixel enquanto o botão está pressionado.
    let shift = if theme.is_null() && state & CDIS_SELECTED != 0 { 1 } else { 0 };
    let image = icon.source.icon_pixels(side).and_then(|image| if disabled { image.disabled() } else { Ok(image) });
    if let Ok(image) = image {
        // SAFETY: ícone válido enquanto `image` existir; HDC válido durante a mensagem.
        unsafe {
            DrawIconEx(
                hdc,
                icon_at.x + shift,
                icon_at.y + shift,
                image.raw(),
                side,
                side,
                0,
                std::ptr::null_mut(),
                DI_NORMAL,
            )
        };
    }
    if !text.is_empty() && icon.position != IconPosition::Center {
        let content_right = if icon.position == IconPosition::Right {
            icon_at.x - to_pixels(icon_gap(logical), dpi)
        } else {
            rc.right - margin
        };
        let right = (text_at.x + text_size.width).min(content_right).max(text_at.x);
        let area = RECT {
            left: text_at.x + shift,
            top: text_at.y + shift,
            right: right + shift,
            bottom: text_at.y + text_size.height + shift,
        };
        let flags = DT_SINGLELINE | DT_END_ELLIPSIS | prefix;
        // SAFETY: HDC, texto e retângulo válidos durante a chamada.
        unsafe {
            let themed = !theme.is_null() && {
                let part_state = if disabled {
                    PBS_DISABLED
                } else if state & CDIS_SELECTED != 0 {
                    PBS_PRESSED
                } else if state & CDIS_HOT != 0 {
                    PBS_HOT
                } else if state & CDIS_DEFAULT != 0 {
                    PBS_DEFAULTED
                } else {
                    PBS_NORMAL
                };
                DrawThemeText(theme, hdc, BP_PUSHBUTTON, part_state, text.as_ptr(), text.len() as i32, flags, 0, &area)
                    >= 0
            };
            if !themed {
                SetBkMode(hdc, crate::ffi::gdi32::TRANSPARENT);
                SetTextColor(hdc, GetSysColor(if disabled { COLOR_GRAYTEXT } else { COLOR_BTNTEXT }));
                let mut area = area;
                DrawTextW(hdc, text.as_ptr(), text.len() as i32, &mut area, flags);
            }
        }
    }
    // O desenho padrão (que inclui o retângulo de foco) é pulado: o foco é desenhado aqui.
    let focus_hidden = native::send(hwnd, WM_QUERYUISTATE, 0, 0) as u16 & UISF_HIDEFOCUS != 0;
    if state & CDIS_FOCUS != 0 && !focus_hidden {
        let inset = to_pixels(3, dpi);
        let ring =
            RECT { left: rc.left + inset, top: rc.top + inset, right: rc.right - inset, bottom: rc.bottom - inset };
        // SAFETY: HDC e retângulo válidos durante a chamada.
        unsafe { DrawFocusRect(hdc, &ring) };
    }
    // SAFETY: restaura a fonte e fecha a classe de tema abertas acima.
    unsafe {
        SelectObject(hdc, previous);
        if !theme.is_null() {
            CloseThemeData(theme);
        }
    }
    Some(CDRF_SKIPDEFAULT)
}

/// Effective style: the button's style, the window theme, or system colors for an icon button whose native control does
/// not support image lists.
fn effective_style(core: &Rc<WidgetCore>) -> Option<ButtonStyle> {
    core.button_style
        .get()
        .or_else(|| core.root().and_then(|r| r.theme.borrow().as_ref().and_then(|t| t.button)))
        .or_else(|| {
            let fallback = core.icon.borrow().as_ref().is_some_and(|icon| !icon.native_accepted);
            fallback.then(ButtonStyle::system)
        })
}

fn is_owner_drawn(core: &WidgetCore) -> bool {
    native::style(core.hwnd()) & BS_TYPEMASK == BS_OWNERDRAW
}

/// Switches between native and library drawing (`BS_OWNERDRAW`) according to whether an effective style exists.
pub(crate) fn refresh_draw_mode(core: &Rc<WidgetCore>) {
    let hwnd = core.hwnd();
    if hwnd.is_null() {
        return;
    }
    let current = native::style(hwnd);
    let wanted = match effective_style(core) {
        Some(_) => BS_OWNERDRAW,
        // O identificador 1 (IDOK) marca o botão padrão criado com `.default(true)`.
        // SAFETY: consulta simples do identificador do controle.
        None if unsafe { GetDlgCtrlID(hwnd) } == 1 => BS_DEFPUSHBUTTON,
        None => BS_PUSHBUTTON,
    };
    if current & BS_TYPEMASK != wanted {
        native::send(hwnd, BM_SETSTYLE, ((current & !BS_TYPEMASK) | wanted) as usize & 0xFFFF, 1);
    }
    core.invalidate();
}

/// Handles command button notifications.
fn reflect_button(core: &Rc<WidgetCore>, reflected: &Reflected) -> Option<LRESULT> {
    match reflected {
        Reflected::Command { code, .. } if *code == BN_CLICKED => {
            core.emit(&Event::Click);
            Some(0)
        }
        Reflected::Command { code, .. } if *code == BN_DBLCLK => {
            core.emit(&Event::DoubleClick);
            Some(0)
        }
        Reflected::Notify { code, lparam } if *code == crate::ffi::comctl32::NM_CUSTOMDRAW => {
            custom_draw(core, *lparam)
        }
        Reflected::DrawItem { lparam } => {
            let style = effective_style(core)?;
            // SAFETY: em WM_DRAWITEM o LPARAM aponta para um DRAWITEMSTRUCT válido
            // durante a mensagem.
            let draw = unsafe { &*(*lparam as *const DRAWITEMSTRUCT) };
            paint_button(core, draw, &style);
            Some(1)
        }
        _ => None,
    }
}

fn paint_button(core: &Rc<WidgetCore>, draw: &DRAWITEMSTRUCT, style: &ButtonStyle) {
    let dpi = core.dpi();
    let pixel = Size::new(draw.rcItem.right - draw.rcItem.left, draw.rcItem.bottom - draw.rcItem.top);
    let canvas = Canvas::for_hdc(draw.hDC, dpi, pixel);
    let bounds = Rect::new(0, 0, to_logical(pixel.width, dpi), to_logical(pixel.height, dpi));
    let state = draw.itemState;
    let colors = if state & ODS_DISABLED != 0 {
        style.disabled
    } else if state & ODS_SELECTED != 0 {
        style.pressed
    } else if core.is_hovered() {
        style.hover
    } else {
        style.normal
    };
    if let Some(background) = core.effective_background().or_else(|| parent_background(core)) {
        canvas.fill_rect(bounds, background);
    }
    let inner = Rect::new(0, 0, bounds.width - 1, bounds.height - 1);
    canvas.round_rect(inner, style.corner_radius, Some(&Pen::new(colors.border, 1)), Some(colors.background));
    let text = native::window_text(core.hwnd()).replace('&', "");
    let icon = core.icon.borrow().clone();
    match icon {
        Some(icon) => {
            let text_size = if text.is_empty() { Size::default() } else { canvas.measure_text(&text, &core.font()) };
            let side = icon_side(&icon, bounds.height, text_size.height);
            let image = icon.source.icon_pixels(to_pixels(side, dpi)).and_then(|image| {
                if state & ODS_DISABLED != 0 {
                    image.disabled()
                } else {
                    Ok(image)
                }
            });
            let (icon_at, text_at) = layout_icon_and_text_aligned(
                bounds,
                side,
                text_size,
                icon.position,
                icon_gap(side),
                CONTENT_MARGIN,
                core.button_content_alignment.get(),
            );
            if let Ok(image) = image {
                canvas.draw_icon(&image, icon_at, side);
            }
            if !text.is_empty() && icon.position != IconPosition::Center {
                let right = if icon.position == IconPosition::Right {
                    icon_at.x - icon_gap(side)
                } else {
                    bounds.right() - CONTENT_MARGIN
                };
                let width = text_size.width.min(right - text_at.x).max(0);
                let area = Rect::new(text_at.x, text_at.y, width, text_size.height);
                canvas.text_line(&text, area, &core.font(), colors.foreground);
            }
        }
        None => canvas.mnemonic_text(
            &native::window_text(core.hwnd()),
            Rect::new(CONTENT_MARGIN, 0, (bounds.width - 2 * CONTENT_MARGIN).max(0), bounds.height),
            &core.font(),
            colors.foreground,
            core.button_content_alignment.get(),
            true,
        ),
    }
    if state & ODS_FOCUS != 0 {
        let ring = Rect::new(2, 2, bounds.width - 5, bounds.height - 5);
        canvas.round_rect(ring, (style.corner_radius - 1).max(0), Some(&Pen::new(style.focus_ring, 1)), None);
    }
}

/// Compatibility for controls using the centered group.
pub(crate) fn layout_icon_and_text(
    bounds: Rect,
    icon: i32,
    text: Size,
    position: IconPosition,
    gap: i32,
    margin: i32,
) -> (Point, Point) {
    layout_icon_and_text_aligned(bounds, icon, text, position, gap, margin, TextAlign::Center)
}

/// Icon and text positions as an aligned group within margins. If content does not fit, reserves the icon and reduces
/// available text width.
pub(crate) fn layout_icon_and_text_aligned(
    bounds: Rect,
    icon: i32,
    text: Size,
    position: IconPosition,
    gap: i32,
    margin: i32,
    alignment: TextAlign,
) -> (Point, Point) {
    let gap = if text.width == 0 { 0 } else { gap };
    let margin = margin.min(bounds.width / 4).max(0);
    let available = (bounds.width - 2 * margin).max(0);
    let text = Size::new(
        text.width.min(if matches!(position, IconPosition::Left | IconPosition::Right) {
            (available - icon - gap).max(0)
        } else {
            available
        }),
        text.height,
    );
    let align_x = |width: i32| match alignment {
        TextAlign::Left => bounds.x + margin,
        TextAlign::Center => (bounds.x + (bounds.width - width) / 2).max(bounds.x + margin),
        TextAlign::Right => (bounds.right() - margin - width).max(bounds.x + margin),
    };
    let center_y = |height: i32| bounds.y + (bounds.height - height) / 2;
    match position {
        IconPosition::Left | IconPosition::Right => {
            let total = icon + gap + text.width;
            let start = align_x(total);
            let (icon_x, text_x) = if position == IconPosition::Left {
                (start, start + icon + gap)
            } else {
                (start + text.width + gap, start)
            };
            (Point::new(icon_x, center_y(icon)), Point::new(text_x, center_y(text.height)))
        }
        IconPosition::Top | IconPosition::Bottom => {
            let total = icon + gap + text.height;
            let start = center_y(total);
            let (icon_y, text_y) = if position == IconPosition::Top {
                (start, start + icon + gap)
            } else {
                (start + text.height + gap, start)
            };
            (Point::new(align_x(icon), icon_y), Point::new(align_x(text.width), text_y))
        }
        IconPosition::Center => {
            (Point::new(align_x(icon), center_y(icon)), Point::new(align_x(text.width), center_y(text.height)))
        }
    }
}

fn parent_background(core: &Rc<WidgetCore>) -> Option<crate::utilities::Color> {
    // SAFETY: consulta simples do pai.
    let parent = unsafe { GetParent(core.hwnd()) };
    crate::core::widget::lookup(parent).and_then(|p| p.effective_background())
}

impl Button {
    /// Starts building a button (default size 96 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
    pub fn builder() -> ButtonBuilder {
        ButtonBuilder {
            opts: ControlOptions::new(96, super::DEFAULT_CONTROL_HEIGHT),
            default: false,
            cancel: false,
            style: None,
            icon: None,
            icon_size: None,
            icon_position: IconPosition::Left,
            content_alignment: TextAlign::Center,
        }
    }

    fn install_icon(&self, source: Rc<dyn IconSource>, size: Option<i32>, position: IconPosition) -> GuiResult<()> {
        self.core.require_alive()?;
        // Carrega uma vez agora para que um ícone inválido vire erro aqui, e não silêncio.
        source.icon_pixels(to_pixels(size.unwrap_or(DEFAULT_ICON_SIZE).max(1), self.core.dpi()))?;
        let previous = self.core.icon.borrow_mut().replace(WidgetIcon {
            source,
            size: size.unwrap_or(DEFAULT_ICON_SIZE).max(1),
            auto_size: size.is_none(),
            position,
            native_accepted: true,
            saved_style: 0,
        });
        apply_native_icon(&self.core);
        drop(previous);
        Ok(())
    }

    /// Shows an icon beside the text (replaces the previous icon).
    pub fn set_icon(&self, icon: &impl IconSource) -> GuiResult<()> {
        let (size, position) = self.icon_layout();
        self.install_icon(icon.to_shared(), size, position)
    }

    /// Removes the icon.
    pub fn clear_icon(&self) {
        let previous = self.core.icon.borrow_mut().take();
        drop(previous);
        refresh_draw_mode(&self.core);
    }

    /// `true` if the icon button remains native (Windows themed background, border and states, with icon and text drawn
    /// during the Common Controls v6 `NM_CUSTOMDRAW` notification); `false` if the library draws the entire button
    /// (custom style, window theme or no v6).
    pub fn is_icon_native(&self) -> bool {
        self.core.icon.borrow().as_ref().is_some_and(|icon| icon.native_accepted) && !is_owner_drawn(&self.core)
    }

    /// `true` if the button displays an icon.
    pub fn has_icon(&self) -> bool {
        self.core.icon.borrow().is_some()
    }

    /// Changes the icon position relative to the text.
    pub fn set_icon_position(&self, position: IconPosition) {
        let source = self
            .core
            .icon
            .borrow()
            .as_ref()
            .map(|icon| (icon.source.clone(), if icon.auto_size { None } else { Some(icon.size) }));
        if let Some((source, size)) = source {
            let _ = self.install_icon(source, size, position);
        }
    }

    /// Sets a fixed logical icon size (`None` restores automatic sizing: 60% of button height).
    pub fn set_icon_size(&self, size: impl Into<Option<i32>>) {
        let source = self.core.icon.borrow().as_ref().map(|icon| (icon.source.clone(), icon.position));
        if let Some((source, position)) = source {
            let _ = self.install_icon(source, size.into(), position);
        }
    }

    /// Current logical drawing size of the icon (`None` without an icon).
    pub fn icon_size(&self) -> Option<i32> {
        let icon = self.core.icon.borrow().clone()?;
        let dpi = self.core.dpi();
        let height = to_logical(native::client_rect(self.core.hwnd()).bottom, dpi);
        let text = if native::window_text(self.core.hwnd()).is_empty() {
            0
        } else {
            to_logical(super::icon_support::text_height(self.core.hwnd()), dpi)
        };
        Some(icon_side(&icon, height, text))
    }

    fn icon_layout(&self) -> (Option<i32>, IconPosition) {
        self.core
            .icon
            .borrow()
            .as_ref()
            .map_or((None, IconPosition::Left), |i| (if i.auto_size { None } else { Some(i.size) }, i.position))
    }

    /// Current horizontal content alignment.
    pub fn content_alignment(&self) -> TextAlign {
        self.core.button_content_alignment.get()
    }

    /// Changes horizontal alignment of the icon and text group.
    pub fn set_content_alignment(&self, alignment: TextAlign) {
        self.core.button_content_alignment.set(alignment);
        let flag = match alignment {
            TextAlign::Left => BS_LEFT,
            TextAlign::Center => BS_CENTER,
            TextAlign::Right => BS_RIGHT,
        };
        let style = native::style(self.core.hwnd());
        native::send(self.core.hwnd(), BM_SETSTYLE, ((style & !BS_CENTER) | flag) as usize & 0xffff, 1);
        self.core.invalidate();
    }

    /// Simulates a user click (fires `on_click`).
    pub fn click(&self) {
        native::send(self.core.hwnd(), BM_CLICK, 0, 0);
    }

    /// Sets or removes (`None`) the custom appearance.
    pub fn set_style(&self, style: Option<ButtonStyle>) {
        self.core.button_style.set(style);
        refresh_draw_mode(&self.core);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_icon_follows_the_button_height() {
        assert_eq!(auto_icon_size(28, IconPosition::Left, 15), 17);
        assert_eq!(auto_icon_size(40, IconPosition::Left, 15), 24);
        assert_eq!(auto_icon_size(10, IconPosition::Left, 15), 8);
        // acima do texto: o ícone cede espaço ao texto
        assert_eq!(auto_icon_size(64, IconPosition::Top, 15), 29);
        assert_eq!((icon_gap(17), icon_gap(24), icon_gap(8), icon_gap(60)), (6, 8, 4, 12));
    }

    #[test]
    fn icon_and_text_are_centered_together_and_keep_the_margin() {
        let bounds = Rect::new(0, 0, 120, 28);
        let (icon, text) =
            layout_icon_and_text_aligned(bounds, 17, Size::new(40, 15), IconPosition::Left, 6, 8, TextAlign::Center);
        assert_eq!((icon, text), (Point::new(28, 5), Point::new(51, 6)));
        // não cabe: o grupo encosta na margem em vez de sair pela esquerda
        let (icon, _) = layout_icon_and_text_aligned(
            Rect::new(0, 0, 60, 28),
            17,
            Size::new(80, 15),
            IconPosition::Left,
            6,
            8,
            TextAlign::Center,
        );
        assert_eq!(icon.x, 8);
    }

    #[test]
    fn right_alignment_keeps_both_margins_and_icon_order() {
        let bounds = Rect::new(10, 0, 200, 60);
        let (icon, text) =
            layout_icon_and_text_aligned(bounds, 48, Size::new(70, 16), IconPosition::Left, 12, 8, TextAlign::Right);
        assert_eq!(text.x + 70, bounds.right() - 8);
        assert_eq!(text.x - icon.x, 60);
        assert_eq!(icon.y, 6);
        for alignment in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
            let (icon, text) =
                layout_icon_and_text_aligned(bounds, 48, Size::new(500, 16), IconPosition::Right, 12, 8, alignment);
            assert_eq!(text.x, bounds.x + 8);
            assert_eq!(icon.x + 48, bounds.right() - 8);
        }
    }
}
