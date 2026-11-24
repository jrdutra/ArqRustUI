//! Dark theme support for native controls ignoring theme colors.
//!
//! Tabs (`SysTabControl32`), status bars (`msctls_statusbar32`) and headers (`SysHeader32`) lack configurable colors
//! with visual styles. In dark theme the library paints from native state via WM_PAINT; native
//! items/text/icons/selection, mouse/keyboard/accessibility remain intact. Light theme leaves painting native. Toolbar
//! uses NM_CUSTOMDRAW; calendar, progress and rebar use native color messages in classic mode.

use super::Theme;
use crate::core::native;
use crate::core::widget::WidgetCore;
use crate::ffi::comctl32::*;
use crate::ffi::gdi32::*;
use crate::ffi::user32::*;
use crate::ffi::uxtheme::SetWindowTheme;
use crate::ffi::*;
use crate::utilities::{to_pixels, Color};
use std::rc::Rc;

/// Theme-derived colors for library-painted controls.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Palette {
    pub background: Color,
    pub foreground: Color,
    pub surface: Color,
    pub border: Color,
    pub muted: Color,
    pub hover: Color,
    pub pressed: Color,
    pub accent: Color,
    pub input: Color,
}

impl Palette {
    pub(crate) fn from_theme(theme: &Theme) -> Palette {
        let (bg, fg) = (theme.background, theme.foreground);
        Palette {
            background: bg,
            foreground: fg,
            surface: bg.mix(Color::WHITE, 0.05),
            border: bg.mix(fg, 0.2),
            muted: fg.mix(bg, 0.4),
            hover: bg.mix(fg, 0.12),
            pressed: bg.mix(fg, 0.2),
            accent: theme.accent,
            input: theme.input_background,
        }
    }
}

/// Control's dark window theme, if any and not high contrast.
pub(crate) fn dark_theme(core: &WidgetCore) -> Option<Rc<Theme>> {
    let root = core.root()?;
    let theme = root.theme.borrow().clone()?;
    (theme.dark && !crate::system::accessibility::high_contrast()).then_some(theme)
}

/// Recolors light native drawing: low-saturation pixels map by luminance (white → field background, black → text);
/// saturated accent/slider pixels and already-dark `keep` background remain.
pub(crate) fn recolor(pixels: &mut [u8], foreground: Color, background: Color, keep: Color) {
    for px in pixels.chunks_exact_mut(4) {
        let (b, g, r) = (px[0], px[1], px[2]);
        let close = |a: u8, b: u8| a.abs_diff(b) <= 3;
        if close(r, keep.r) && close(g, keep.g) && close(b, keep.b) {
            continue;
        }
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        if max - min > 48 {
            continue;
        }
        let light = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;
        let mapped = foreground.mix(background, light);
        px[0] = mapped.b;
        px[1] = mapped.g;
        px[2] = mapped.r;
    }
}

/// Dark DatePicker/TimePicker/Slider render into memory (`WM_PAINT` with DC in WPARAM, supported by Common Controls),
/// then library recolors before display.
fn handle_recolored(core: &WidgetCore, hwnd: HWND, msg: UINT, wp: WPARAM) -> Option<LRESULT> {
    if !matches!(msg, WM_PAINT | WM_PRINTCLIENT | WM_ERASEBKGND) {
        return None;
    }
    let theme = dark_theme(core)?;
    if msg == WM_ERASEBKGND {
        return Some(1);
    }
    let palette = Palette::from_theme(&theme);
    let field = core.kind != "Slider";
    let render = |target: HDC| {
        let client = native::client_rect(hwnd);
        let (width, height) = (client.right - client.left, client.bottom - client.top);
        if width <= 0 || height <= 0 {
            return;
        }
        let Ok((bitmap, bits)) = crate::input::drag::dib_section(width, height) else { return };
        // SAFETY: DC de memória e seção DIB criados e liberados aqui; a seção tem
        // largura × altura × 4 bytes; o controle desenha nela de forma síncrona.
        unsafe {
            let memory = CreateCompatibleDC(target);
            let previous = SelectObject(memory, bitmap);
            let base = if field { Color::WHITE } else { palette.background };
            FillRect(memory, &client, crate::core::application::brush_for(base));
            core.default_proc(hwnd, WM_PAINT, memory as WPARAM, 0);
            let pixels = std::slice::from_raw_parts_mut(bits, width as usize * height as usize * 4);
            let surface = if field { palette.input } else { palette.background.mix(palette.foreground, 0.18) };
            recolor(pixels, palette.foreground, surface, palette.background);
            BitBlt(target, 0, 0, width, height, memory, 0, 0, SRCCOPY);
            SelectObject(memory, previous);
            DeleteDC(memory);
            DeleteObject(bitmap);
        }
    };
    if msg == WM_PRINTCLIENT {
        render(wp as HDC);
        return Some(0);
    }
    // SAFETY: par BeginPaint/EndPaint da própria janela durante WM_PAINT.
    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        if !hdc.is_null() {
            render(hdc);
        }
        EndPaint(hwnd, &ps);
    }
    Some(0)
}

/// Custom painting of listed controls in dark theme.
pub(crate) fn handle(core: &WidgetCore, hwnd: HWND, msg: UINT, wp: WPARAM) -> Option<LRESULT> {
    if matches!(core.kind, "DatePicker" | "TimePicker" | "Slider") {
        return handle_recolored(core, hwnd, msg, wp);
    }
    if !matches!(core.kind, "TabControl" | "StatusBar" | "Header" | "HeaderControl" | "GroupBox") {
        return None;
    }
    if !matches!(msg, WM_PAINT | WM_PRINTCLIENT | WM_ERASEBKGND) {
        return None;
    }
    let theme = dark_theme(core)?;
    if core.kind == "TabControl" && native::style(hwnd) & (TCS_BOTTOM | TCS_VERTICAL | TCS_BUTTONS) != 0 {
        return None;
    }
    if msg == WM_ERASEBKGND {
        return Some(1);
    }
    let palette = Palette::from_theme(&theme);
    let paint = |hdc: HDC| {
        let client = native::client_rect(hwnd);
        let dpi = core.dpi();
        crate::graphics::gdi::paint_buffered(hdc, client, 96, Some(palette.background), |canvas| {
            let target = canvas.hdc();
            let painter = Painter { hdc: target, font: font_of(hwnd), dpi };
            match core.kind {
                "TabControl" => paint_tabs(hwnd, &painter, &palette, client),
                "StatusBar" => paint_status(hwnd, &painter, &palette, client),
                "GroupBox" => paint_group(hwnd, &painter, &palette, client),
                _ => paint_header(hwnd, &painter, &palette, client),
            }
        });
    };
    if msg == WM_PRINTCLIENT {
        paint(wp as HDC);
        return Some(0);
    }
    // SAFETY: par BeginPaint/EndPaint da própria janela durante WM_PAINT.
    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        if !hdc.is_null() {
            paint(hdc);
        }
        EndPaint(hwnd, &ps);
    }
    Some(0)
}

fn font_of(hwnd: HWND) -> HFONT {
    native::send(hwnd, WM_GETFONT, 0, 0) as HFONT
}

/// Physical-pixel DC drawing.
struct Painter {
    hdc: HDC,
    font: HFONT,
    dpi: u32,
}

impl Painter {
    fn px(&self, logical: i32) -> i32 {
        to_pixels(logical, self.dpi).max(1)
    }

    fn fill(&self, rect: RECT, color: Color) {
        if rect.right > rect.left && rect.bottom > rect.top {
            // SAFETY: DC válido durante a pintura; pincel em cache da aplicação.
            unsafe { FillRect(self.hdc, &rect, crate::core::application::brush_for(color)) };
        }
    }

    fn text(&self, text: &str, rect: RECT, color: Color, flags: u32) {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut area = rect;
        // SAFETY: DC válido; a fonte do controle continua selecionada só durante o desenho.
        unsafe {
            let previous = (!self.font.is_null()).then(|| SelectObject(self.hdc, self.font as _));
            SetBkMode(self.hdc, TRANSPARENT);
            SetTextColor(self.hdc, color.to_colorref());
            DrawTextW(
                self.hdc,
                wide.as_ptr(),
                wide.len() as i32,
                &mut area,
                flags | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            if let Some(previous) = previous {
                SelectObject(self.hdc, previous);
            }
        }
    }

    /// Arrow (`^`/`v`) centered at (`cx`, `cy`).
    fn chevron(&self, cx: i32, cy: i32, up: bool, color: Color) {
        let size = self.px(3);
        for step in 0..=size {
            let y = if up { cy - size / 2 + step } else { cy + size / 2 - step };
            self.fill(RECT { left: cx - step, top: y, right: cx + step + 1, bottom: y + 1 }, color);
        }
    }
}

fn item_rect(hwnd: HWND, message: UINT, index: usize) -> RECT {
    let mut rect = RECT::default();
    native::send_mut(hwnd, message, index, &mut rect);
    rect
}

fn paint_tabs(hwnd: HWND, p: &Painter, c: &Palette, client: RECT) {
    let count = native::send(hwnd, TCM_GETITEMCOUNT, 0, 0).max(0) as usize;
    let selected = native::send(hwnd, TCM_GETCURSEL, 0, 0);
    let rects: Vec<RECT> = (0..count).map(|i| item_rect(hwnd, TCM_GETITEMRECT, i)).collect();
    let strip = rects.iter().map(|r| r.bottom).max().unwrap_or(0);
    let page = RECT { left: client.left, top: strip, right: client.right, bottom: client.bottom };
    p.fill(page, c.background);
    // Borda da área das páginas.
    let line = p.px(1);
    p.fill(RECT { bottom: page.top + line, ..page }, c.border);
    p.fill(RECT { top: page.bottom - line, ..page }, c.border);
    p.fill(RECT { right: page.left + line, ..page }, c.border);
    p.fill(RECT { left: page.right - line, ..page }, c.border);
    let images = native::send(hwnd, TCM_GETIMAGELIST, 0, 0) as HIMAGELIST;
    let functions = functions();
    for (index, rect) in rects.iter().enumerate() {
        let active = index as isize == selected;
        let mut r = *rect;
        if active {
            r.top -= p.px(2);
            r.bottom += line;
            p.fill(r, c.background);
            p.fill(RECT { right: r.left + line, ..r }, c.border);
            p.fill(RECT { left: r.right - line, ..r }, c.border);
            p.fill(RECT { bottom: r.top + p.px(2), ..r }, c.accent);
        }
        let mut buffer = [0u16; 260];
        let mut item = TCITEMW {
            mask: TCIF_TEXT | TCIF_IMAGE,
            dwState: 0,
            dwStateMask: 0,
            pszText: buffer.as_mut_ptr(),
            cchTextMax: buffer.len() as i32,
            iImage: -1,
            lParam: 0,
        };
        native::send_mut(hwnd, TCM_GETITEMW, index, &mut item);
        let text = crate::utilities::from_wide(&buffer);
        let mut area = RECT { left: r.left + p.px(6), right: r.right - p.px(6), ..r };
        if item.iImage >= 0 && !images.is_null() {
            if let Some(functions) = functions {
                let (mut w, mut h) = (0, 0);
                // SAFETY: lista de imagens viva do controle; DC válido.
                unsafe {
                    (functions.ImageList_GetIconSize)(images, &mut w, &mut h);
                    let y = area.top + (area.bottom - area.top - h) / 2;
                    (functions.ImageList_Draw)(images, item.iImage, p.hdc, area.left, y, ILD_TRANSPARENT);
                }
                area.left += w + p.px(4);
            }
        }
        p.text(&text, area, if active { c.foreground } else { c.muted }, DT_CENTER);
    }
}

fn paint_status(hwnd: HWND, p: &Painter, c: &Palette, client: RECT) {
    p.fill(client, c.surface);
    let line = p.px(1);
    p.fill(RECT { bottom: client.top + line, ..client }, c.border);
    let simple = native::send(hwnd, SB_ISSIMPLE, 0, 0) != 0;
    let parts = if simple { 1 } else { native::send(hwnd, SB_GETPARTS, 0, 0).max(0) as usize };
    let icon_size = p.px(16);
    for part in 0..parts {
        let index = if simple { 255 } else { part };
        let mut rect = if simple { client } else { item_rect(hwnd, SB_GETRECT, part) };
        if rect.right < 0 || rect.right > client.right {
            rect.right = client.right;
        }
        let info = native::send(hwnd, SB_GETTEXTLENGTHW, index, 0) as usize;
        let length = info & 0xFFFF;
        let mut area = RECT { left: rect.left + p.px(6), right: rect.right - p.px(6), ..rect };
        let icon = native::send(hwnd, SB_GETICON, index, 0) as HICON;
        if !icon.is_null() {
            let y = rect.top + (rect.bottom - rect.top - icon_size) / 2;
            // SAFETY: ícone vivo da barra; DC válido.
            unsafe { DrawIconEx(p.hdc, area.left, y, icon, icon_size, icon_size, 0, std::ptr::null_mut(), DI_NORMAL) };
            area.left += icon_size + p.px(4);
        }
        if info & SBT_OWNERDRAW == 0 && length > 0 {
            let mut buffer = vec![0u16; length + 1];
            native::send(hwnd, SB_GETTEXTW, index, buffer.as_mut_ptr() as LPARAM);
            p.text(&crate::utilities::from_wide(&buffer), area, c.foreground, DT_LEFT);
        }
        if part + 1 < parts {
            let x = rect.right - line;
            p.fill(RECT { left: x, right: x + line, top: rect.top + p.px(4), bottom: rect.bottom - p.px(3) }, c.border);
        }
    }
    // Alça de redimensionamento: seis pontos no canto, como a do sistema.
    // SAFETY: consulta simples.
    let maximized = unsafe { IsZoomed(native::root_of(hwnd)) } != 0;
    if native::style(hwnd) & SBARS_SIZEGRIP != 0 && !maximized {
        let dot = p.px(2);
        let step = p.px(4);
        for (col, row) in [(0, 0), (1, 0), (2, 0), (1, 1), (2, 1), (2, 2)] {
            let x = client.right - p.px(4) - step * col - dot;
            let y = client.bottom - p.px(4) - step * row - dot;
            p.fill(RECT { left: x, top: y, right: x + dot, bottom: y + dot }, c.muted);
        }
    }
}

fn paint_header(hwnd: HWND, p: &Painter, c: &Palette, client: RECT) {
    p.fill(client, c.input.mix(Color::WHITE, 0.05));
    let line = p.px(1);
    p.fill(RECT { top: client.bottom - line, ..client }, c.border);
    let count = native::send(hwnd, HDM_GETITEMCOUNT, 0, 0).max(0) as usize;
    for index in 0..count {
        let rect = item_rect(hwnd, HDM_GETITEMRECT, index);
        let mut buffer = [0u16; 260];
        let mut item = HDITEMW {
            mask: HDI_TEXT | HDI_FORMAT,
            cxy: 0,
            pszText: buffer.as_mut_ptr(),
            hbm: std::ptr::null_mut(),
            cchTextMax: buffer.len() as i32,
            fmt: 0,
            lParam: 0,
            iImage: 0,
            iOrder: 0,
            r#type: 0,
            pvFilter: std::ptr::null_mut(),
            state: 0,
        };
        native::send_mut(hwnd, HDM_GETITEMW, index, &mut item);
        let sorted = item.fmt & (HDF_SORTUP | HDF_SORTDOWN) != 0;
        let arrow = if sorted { p.px(14) } else { 0 };
        let area = RECT { left: rect.left + p.px(6), right: rect.right - p.px(6) - arrow, ..rect };
        let align = match item.fmt & HDF_JUSTIFYMASK {
            HDF_RIGHT => DT_RIGHT,
            HDF_CENTER => DT_CENTER,
            _ => DT_LEFT,
        };
        p.text(&crate::utilities::from_wide(&buffer), area, c.foreground, align);
        if sorted {
            let cy = (rect.top + rect.bottom) / 2;
            p.chevron(rect.right - p.px(12), cy, item.fmt & HDF_SORTUP != 0, c.muted);
        }
        let x = rect.right - line;
        p.fill(RECT { left: x, right: x + line, top: rect.top + p.px(4), bottom: rect.bottom - p.px(4) }, c.border);
    }
}

fn paint_group(hwnd: HWND, p: &Painter, c: &Palette, client: RECT) {
    p.fill(client, c.background);
    let text = native::window_text(hwnd);
    let wide: Vec<u16> = text.encode_utf16().collect();
    let mut size = RECT::default();
    // SAFETY: DC válido; a fonte do controle fica selecionada só durante a medição.
    unsafe {
        let previous = (!p.font.is_null()).then(|| SelectObject(p.hdc, p.font as _));
        DrawTextW(p.hdc, wide.as_ptr(), wide.len() as i32, &mut size, DT_SINGLELINE | DT_CALCRECT);
        if let Some(previous) = previous {
            SelectObject(p.hdc, previous);
        }
    }
    let text_height = (size.bottom - size.top).max(p.px(14));
    let line = p.px(1);
    let frame = RECT { top: client.top + text_height / 2, ..client };
    p.fill(RECT { bottom: frame.top + line, ..frame }, c.border);
    p.fill(RECT { top: frame.bottom - line, ..frame }, c.border);
    p.fill(RECT { right: frame.left + line, ..frame }, c.border);
    p.fill(RECT { left: frame.right - line, ..frame }, c.border);
    if !text.is_empty() {
        let left = client.left + p.px(8);
        let width = size.right - size.left;
        p.fill(
            RECT { left, top: client.top, right: left + width + p.px(6), bottom: client.top + text_height },
            c.background,
        );
        let area = RECT {
            left: left + p.px(3),
            top: client.top,
            right: left + width + p.px(4),
            bottom: client.top + text_height,
        };
        // SAFETY: consulta simples.
        let enabled = unsafe { IsWindowEnabled(hwnd) } != 0;
        p.text(&text, area, if enabled { c.foreground } else { c.muted }, DT_LEFT | DT_HIDEPREFIX);
    }
}

/// Whether control's 3D `WS_EX_CLIENTEDGE` should paint dark.
pub(crate) fn wants_dark_edge(core: &WidgetCore, hwnd: HWND) -> bool {
    !core.is_top_level() && native::ex_style(hwnd) & WS_EX_CLIENTEDGE != 0 && dark_theme(core).is_some()
}

/// Paints dark 3D border after default WM_NCPAINT, which already drew scrollbars.
pub(crate) fn paint_edge(core: &WidgetCore, hwnd: HWND) {
    let Some(theme) = dark_theme(core) else { return };
    let palette = Palette::from_theme(&theme);
    let outer = native::window_rect(hwnd);
    let (w, h) = (outer.right - outer.left, outer.bottom - outer.top);
    // SAFETY: consulta de métrica do sistema.
    let edge = unsafe { GetSystemMetrics(SM_CXEDGE) }.max(1);
    // SAFETY: DC da janela inteira, liberado logo após o desenho.
    unsafe {
        let hdc = GetWindowDC(hwnd);
        if hdc.is_null() {
            return;
        }
        let p = Painter { hdc, font: std::ptr::null_mut(), dpi: core.dpi() };
        for ring in 0..edge {
            let color = if ring == 0 { palette.border } else { palette.input };
            let r = RECT { left: ring, top: ring, right: w - ring, bottom: h - ring };
            p.fill(RECT { bottom: r.top + 1, ..r }, color);
            p.fill(RECT { top: r.bottom - 1, ..r }, color);
            p.fill(RECT { right: r.left + 1, ..r }, color);
            p.fill(RECT { left: r.right - 1, ..r }, color);
        }
        ReleaseDC(hwnd, hdc);
    }
}

/// Dark toolbar NM_CUSTOMDRAW: background, button highlight and text. `None` in light theme for native drawing.
pub(crate) fn toolbar_custom_draw(core: &Rc<WidgetCore>, lparam: LPARAM) -> Option<LRESULT> {
    let theme = dark_theme(core)?;
    let palette = Palette::from_theme(&theme);
    // SAFETY: em NM_CUSTOMDRAW de uma barra de ferramentas o LPARAM aponta para um
    // NMTBCUSTOMDRAW válido e gravável durante a notificação.
    let draw = unsafe { &mut *(lparam as *mut NMTBCUSTOMDRAW) };
    let painter = Painter { hdc: draw.nmcd.hdc, font: std::ptr::null_mut(), dpi: core.dpi() };
    match draw.nmcd.dwDrawStage {
        CDDS_PREPAINT => {
            painter.fill(draw.nmcd.rc, palette.background);
            Some(CDRF_NOTIFYITEMDRAW)
        }
        CDDS_ITEMPREPAINT => {
            let state = draw.nmcd.uItemState;
            let disabled = state & CDIS_DISABLED != 0;
            let fill = if disabled {
                None
            } else if state & CDIS_SELECTED != 0 {
                Some(palette.pressed)
            } else if state & CDIS_CHECKED != 0 {
                Some(palette.pressed.mix(palette.accent, 0.25))
            } else if state & CDIS_HOT != 0 {
                Some(palette.hover)
            } else {
                None
            };
            painter.fill(draw.nmcd.rc, palette.background);
            if let Some(color) = fill {
                let r = draw.nmcd.rc;
                let inset = painter.px(1);
                painter.fill(
                    RECT { left: r.left + inset, top: r.top + inset, right: r.right - inset, bottom: r.bottom - inset },
                    color,
                );
            }
            let text = if disabled { palette.muted } else { palette.foreground };
            draw.clrText = text.to_colorref();
            draw.clrTextHighlight = text.to_colorref();
            draw.clrBtnFace = palette.background.to_colorref();
            draw.clrBtnHighlight = palette.hover.to_colorref();
            draw.clrHighlightHotTrack = palette.hover.to_colorref();
            draw.nStringBkMode = TRANSPARENT;
            draw.nHLStringBkMode = TRANSPARENT;
            Some(
                CDRF_DODEFAULT
                    | TBCDRF_NOEDGES
                    | TBCDRF_NOBACKGROUND
                    | TBCDRF_USECDCOLORS
                    | TBCDRF_NOMARK
                    | TBCDRF_NOOFFSET
                    | TBCDRF_NOETCHEDEFFECT,
            )
        }
        _ => None,
    }
}

/// SysMonthCal32 colors, standalone or picker drop-down.
fn month_colors(hwnd: HWND, message: UINT, theme: Option<&Theme>) {
    let colors: [(usize, u32); 6] = match theme {
        Some(theme) => {
            let palette = Palette::from_theme(theme);
            [
                (MCSC_BACKGROUND, palette.input.to_colorref()),
                (MCSC_MONTHBK, palette.input.to_colorref()),
                (MCSC_TEXT, theme.input_foreground.to_colorref()),
                (MCSC_TITLEBK, palette.accent.to_colorref()),
                (MCSC_TITLETEXT, readable_on(palette.accent).to_colorref()),
                (MCSC_TRAILINGTEXT, palette.muted.to_colorref()),
            ]
        }
        None => {
            // SAFETY: consultas de cores do sistema.
            let system = |index| unsafe { GetSysColor(index) };
            [
                (MCSC_BACKGROUND, system(COLOR_WINDOW)),
                (MCSC_MONTHBK, system(COLOR_WINDOW)),
                (MCSC_TEXT, system(COLOR_WINDOWTEXT)),
                (MCSC_TITLEBK, system(COLOR_ACTIVECAPTION)),
                (MCSC_TITLETEXT, system(COLOR_CAPTIONTEXT)),
                (MCSC_TRAILINGTEXT, system(COLOR_GRAYTEXT)),
            ]
        }
    };
    for (index, color) in colors {
        native::send(hwnd, message, index, color as isize);
    }
}

fn readable_on(color: Color) -> Color {
    if color.luminance() > 0.6 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}

/// Window visual class (`Some("")` = classic, `None` = default).
fn visual_class(hwnd: HWND, class: Option<&str>) {
    let empty = [0u16];
    let wide = class.map(crate::utilities::to_wide);
    // SAFETY: strings terminadas em NUL válidas durante a chamada.
    unsafe {
        match (&wide, class) {
            (Some(_), Some("")) => SetWindowTheme(hwnd, empty.as_ptr(), empty.as_ptr()),
            (Some(name), Some(_)) => SetWindowTheme(hwnd, name.as_ptr(), std::ptr::null()),
            _ => SetWindowTheme(hwnd, std::ptr::null(), std::ptr::null()),
        };
    }
}

/// Applies/reverts message-based dark colors; called per control by [`Theme::apply`](super::Theme::apply).
pub(crate) fn style_native(core: &Rc<WidgetCore>, theme: &Theme) {
    let hwnd = core.hwnd();
    let dark = theme.dark && !crate::system::accessibility::high_contrast();
    let palette = Palette::from_theme(theme);
    match core.kind {
        "ToolBar" => visual_class(hwnd, dark.then_some("")),
        "MonthCalendar" => {
            visual_class(hwnd, dark.then_some(""));
            month_colors(hwnd, MCM_SETCOLOR, dark.then_some(theme));
        }
        "DatePicker" | "TimePicker" => month_colors(hwnd, DTM_SETMCCOLOR, dark.then_some(theme)),
        "ProgressBar" => {
            visual_class(hwnd, dark.then_some(""));
            let (bar, back) = if dark {
                (palette.accent.to_colorref(), palette.input.to_colorref())
            } else {
                (CLR_DEFAULT, CLR_DEFAULT)
            };
            native::send(hwnd, PBM_SETBARCOLOR, 0, bar as isize);
            native::send(hwnd, PBM_SETBKCOLOR, 0, back as isize);
        }
        "ScrollBar" => visual_class(hwnd, dark.then_some("DarkMode_Explorer")),
        "Rebar" => {
            let (back, text) = if dark {
                (palette.background.to_colorref(), palette.foreground.to_colorref())
            } else {
                (CLR_DEFAULT, CLR_DEFAULT)
            };
            native::send(hwnd, RB_SETBKCOLOR, 0, back as isize);
            native::send(hwnd, RB_SETTEXTCOLOR, 0, text as isize);
        }
        "ListView" | "DataGrid" | "PropertyGrid" => {
            // O cabeçalho é criado pelo próprio ListView: passa a ser um núcleo da
            // biblioteca para receber a pintura escura.
            let header = native::send(hwnd, LVM_GETHEADER, 0, 0) as HWND;
            if let Some(header) = WidgetCore::adopt(header, "Header") {
                visual_class(header.hwnd(), dark.then_some("DarkMode_ItemsView"));
                header.invalidate();
            }
        }
        _ => {}
    }
    if matches!(core.kind, "TabControl" | "StatusBar" | "HeaderControl" | "ToolBar") {
        core.invalidate();
    }
}

/// Date picker DTN_DROPDOWN: applies dark calendar colors.
pub(crate) fn date_dropdown(core: &Rc<WidgetCore>) {
    let Some(theme) = dark_theme(core) else { return };
    let calendar = native::send(core.hwnd(), DTM_GETMONTHCAL, 0, 0) as HWND;
    if calendar.is_null() {
        return;
    }
    visual_class(calendar, Some(""));
    month_colors(calendar, MCM_SETCOLOR, Some(&theme));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recolor_maps_light_tones_and_keeps_accents_and_background() {
        let fg = Color::rgb(240, 240, 240);
        let bg = Color::rgb(40, 40, 40);
        let keep = Color::rgb(32, 32, 32);
        // branco, preto, azul forte, fundo já escuro (BGRA)
        let mut pixels = vec![255, 255, 255, 255, 0, 0, 0, 255, 215, 120, 0, 255, 32, 32, 32, 255];
        recolor(&mut pixels, fg, bg, keep);
        assert_eq!(&pixels[0..3], &[40, 40, 40], "branco vira o fundo do campo");
        assert_eq!(&pixels[4..7], &[240, 240, 240], "preto vira o texto");
        assert_eq!(&pixels[8..11], &[215, 120, 0], "cor de destaque fica");
        assert_eq!(&pixels[12..15], &[32, 32, 32], "fundo escuro do próprio controle fica");
    }
}
