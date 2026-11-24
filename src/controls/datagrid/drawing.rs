//! [`DataGrid`](super::DataGrid) drawing: colors, fonts, cell custom drawing, check boxes and group headers.

use super::*;

impl DataGrid {
    fn font_handle(&self, dpi: u32, bold: bool, italic: bool) -> Option<Rc<GdiObject>> {
        if let Some((_, _, _, handle)) =
            self.model.borrow().fonts.iter().find(|(d, b, i, _)| *d == dpi && *b == bold && *i == italic)
        {
            return Some(handle.clone());
        }
        let mut font: Font = self.core.font();
        if bold {
            font = font.bold();
        }
        if italic {
            font = font.italic();
        }
        let handle = font.handle_for_dpi(dpi).ok()?;
        let mut model = self.model.borrow_mut();
        model.fonts.retain(|(d, ..)| *d == dpi);
        model.fonts.push((dpi, bold, italic, handle.clone()));
        Some(handle)
    }

    fn list_colors(&self) -> (Color, Color) {
        let hwnd = self.core.hwnd();
        let fix = |value: isize, fallback: i32| {
            let value = value as u32;
            if value == CLR_DEFAULT || value == 0xFFFF_FFFF {
                // SAFETY: consulta de cor do sistema.
                Color::from_colorref(unsafe { GetSysColor(fallback) })
            } else {
                Color::from_colorref(value)
            }
        };
        (
            fix(native::send(hwnd, LVM_GETBKCOLOR, 0, 0), COLOR_WINDOW),
            fix(native::send(hwnd, LVM_GETTEXTCOLOR, 0, 0), COLOR_WINDOWTEXT),
        )
    }

    pub(super) fn custom_draw(&self, lparam: isize) -> Option<isize> {
        // SAFETY: em NM_CUSTOMDRAW de um ListView o LPARAM aponta para um NMLVCUSTOMDRAW
        // gravável, válido durante a notificação.
        let draw = unsafe { &mut *(lparam as *mut NMLVCUSTOMDRAW) };
        let stage = draw.nmcd.dwDrawStage;
        let (grouped, checks, styles) = {
            let model = self.model.borrow();
            (model.grouped(), model.has_checkboxes(), model.has_styles())
        };
        if !grouped && !checks && !styles {
            return None;
        }
        let item = draw.nmcd.dwItemSpec;
        match stage {
            CDDS_PREPAINT => Some(CDRF_NOTIFYITEMDRAW),
            CDDS_ITEMPREPAINT => {
                let target = self.model.borrow().item(item);
                if let Some(Item::Group(group)) = target {
                    self.paint_group(draw.nmcd.hdc, item, group, draw.nmcd.uItemState);
                    return Some(CDRF_SKIPDEFAULT);
                }
                let mut result = CDRF_DODEFAULT;
                if styles {
                    result |= CDRF_NOTIFYSUBITEMDRAW;
                }
                if checks {
                    result |= CDRF_NOTIFYPOSTPAINT;
                }
                Some(result)
            }
            s if s == CDDS_ITEMPREPAINT | CDDS_SUBITEM => {
                let Some(row) = self.model.borrow().item_to_row(item) else { return Some(CDRF_DODEFAULT) };
                let column = draw.iSubItem.max(0) as usize;
                let mut style = self.model.borrow().static_style(row, column);
                let provider = self.model.borrow().style_provider.clone();
                if let Some(provider) = provider {
                    let text = self.cell_value(row, column);
                    if let Some(extra) = provider(row, column, &text) {
                        style = style.merged(extra);
                    }
                }
                draw.clrText = style.foreground.map_or(CLR_DEFAULT, |c| c.to_colorref());
                draw.clrTextBk = style.background.map_or(CLR_DEFAULT, |c| c.to_colorref());
                let dpi = self.core.dpi();
                if let Some(font) = self.font_handle(dpi, style.bold, style.italic) {
                    // SAFETY: seleciona no DC do desenho uma fonte mantida viva pelo modelo.
                    unsafe { SelectObject(draw.nmcd.hdc, font.raw()) };
                    return Some(CDRF_NEWFONT);
                }
                Some(CDRF_DODEFAULT)
            }
            CDDS_ITEMPOSTPAINT => {
                if let Some(row) = self.model.borrow().item_to_row(item) {
                    self.paint_checks(draw.nmcd.hdc, item, row, draw.nmcd.uItemState & CDIS_SELECTED != 0);
                }
                Some(CDRF_DODEFAULT)
            }
            _ => None,
        }
    }

    /// A row's `CheckBox` column check boxes (client pixels).
    fn paint_checks(&self, hdc: crate::ffi::HDC, item: usize, row: usize, _selected: bool) {
        let columns: Vec<usize> = {
            let model = self.model.borrow();
            (0..self.column_count()).filter(|&c| model.kind(c) == ColumnKind::CheckBox).collect()
        };
        let dpi = self.core.dpi();
        let (_, text) = self.list_colors();
        let accent = drag::accent_for(self.core.hwnd());
        let canvas = Canvas::for_hdc(hdc, 96, Size::new(0, 0));
        for column in columns {
            let rect = self.subitem_rect(item, column);
            let checked = parse_check(&self.cell_value(row, column));
            let side = to_pixels(14, dpi);
            let x = rect.left + (rect.right - rect.left - side) / 2;
            let y = rect.top + (rect.bottom - rect.top - side) / 2;
            let frame = Rect::new(x, y, side, side);
            if checked {
                canvas.round_rect(frame, to_pixels(3, dpi), None, Some(accent));
                let mark = if accent.luminance() > 0.55 { Color::BLACK } else { Color::WHITE };
                let pen = Pen::new(mark, to_pixels(2, dpi).max(1));
                let p = |fx: f32, fy: f32| Point::new(x + (side as f32 * fx) as i32, y + (side as f32 * fy) as i32);
                canvas.line(p(0.22, 0.52), p(0.42, 0.72), &pen);
                canvas.line(p(0.42, 0.72), p(0.78, 0.3), &pen);
            } else {
                canvas.round_rect(frame, to_pixels(3, dpi), Some(&Pen::new(text.mix(Color::GRAY, 0.4), 1)), None);
            }
        }
    }

    /// Group header: full row with arrow, bold key and count.
    fn paint_group(&self, hdc: crate::ffi::HDC, item: usize, group: usize, state: u32) {
        let hwnd = self.core.hwnd();
        let mut bounds = RECT { left: LVIR_BOUNDS, ..RECT::default() };
        native::send_mut(hwnd, LVM_GETITEMRECT, item, &mut bounds);
        let client = native::client_rect(hwnd);
        bounds.right = bounds.right.max(client.right);
        let (background, foreground) = self.list_colors();
        let accent = drag::accent_for(hwnd);
        let dpi = self.core.dpi();
        let canvas = Canvas::for_hdc(hdc, 96, Size::new(0, 0));
        let area = Rect::new(bounds.left, bounds.top, bounds.right - bounds.left, bounds.bottom - bounds.top);
        let fill =
            if state & CDIS_SELECTED != 0 { background.mix(accent, 0.18) } else { background.mix(foreground, 0.05) };
        canvas.fill_rect(area, fill);
        canvas.line(
            Point::new(area.x, area.bottom() - 1),
            Point::new(area.right(), area.bottom() - 1),
            &Pen::new(background.mix(accent, 0.6), 1),
        );
        let caption = self.group_caption(group);
        let pad = to_pixels(6, dpi);
        let font = self.core.font().bold();
        // A fonte é escalada pelo DPI do Canvas (96 aqui): passa o tamanho já em pixels.
        let scaled = font.clone().with_size(font.size() * dpi as f32 / 96.0);
        canvas.text_in_rect(
            &caption,
            Rect::new(area.x + pad, area.y, area.width - pad * 2, area.height),
            &scaled,
            foreground,
            TextAlign::Left,
        );
        if state & CDIS_FOCUS != 0 && native::send(hwnd, WM_GETFONT, 0, 0) != -1 {
            // SAFETY: consulta simples.
            if unsafe { GetFocus() } == hwnd {
                canvas.focus_rect(area);
            }
        }
    }

    pub(super) fn subitem_rect(&self, item: usize, column: usize) -> RECT {
        let mut rect =
            RECT { left: if column == 0 { LVIR_LABEL } else { LVIR_BOUNDS }, top: column as i32, right: 0, bottom: 0 };
        native::send_mut(self.core.hwnd(), LVM_GETSUBITEMRECT, item, &mut rect);
        if column == 0 {
            // Com várias colunas, LVIR_BOUNDS da coluna 0 cobre a linha inteira: limita à coluna.
            let width = native::send(self.core.hwnd(), LVM_GETCOLUMNWIDTH, 0, 0) as i32;
            let mut full = RECT { left: LVIR_BOUNDS, top: 0, right: 0, bottom: 0 };
            native::send_mut(self.core.hwnd(), LVM_GETSUBITEMRECT, item, &mut full);
            rect.left = full.left;
            rect.right = full.left + width;
        }
        rect
    }
}
