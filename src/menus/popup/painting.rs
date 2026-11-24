//! Drawn menu measurement/rendering: column widths, icons, checks, arrows, shortcuts and separators in theme colors.

use super::*;

/// Measures entries and calculates list layout.
pub(super) fn measure(entries: &[Entry], font: &Font, dpi: u32, min_width: i32) -> Layout {
    let bold = font.clone().bold();
    let measured: Vec<Measured> = strip::with_measure(dpi, |canvas| {
        entries
            .iter()
            .map(|e| {
                if e.is_separator() {
                    return Measured { separator: true, ..Measured::default() };
                }
                Measured {
                    separator: false,
                    text: canvas.measure_mnemonic(&e.text, if e.default { &bold } else { font }).width,
                    shortcut: if e.shortcut.is_empty() { 0 } else { canvas.measure_mnemonic(&e.shortcut, font).width },
                    submenu: matches!(e.kind, EntryKind::Submenu(_)),
                }
            })
            .collect()
    })
    .unwrap_or_default();
    let icon_column = entries.iter().any(|e| e.icon.is_some() || e.checked || e.radio);
    let row_height = (strip::text_height(font, dpi) + 12).max(ICON + 12);
    compute_layout(&measured, row_height, icon_column, min_width)
}

/// Draws logical [`ICON`]-sized icon, dimmed if not `enabled`.
pub(crate) fn draw_icon(canvas: &Canvas, icon: &dyn IconSource, x: i32, y: i32, enabled: bool) {
    let Ok(pixels) = icon.icon_pixels(to_pixels(ICON, canvas.dpi())) else { return };
    let pixels = if enabled { pixels } else { pixels.disabled().unwrap_or(pixels) };
    canvas.draw_icon(&pixels, Point::new(x, y), ICON);
}

pub(super) fn paint_level(level: &Level, canvas: &Canvas, cues: bool) {
    let style = &level.style;
    let size = canvas.size();
    let layout = &level.layout;
    let scroll = level.scroll.get();
    let bold = level.font.clone().bold();
    let has_submenu = level.entries.iter().any(|e| matches!(e.kind, EntryKind::Submenu(_)));
    let shortcut_right = size.width - INSET_X - if has_submenu { ARROW_COLUMN } else { RIGHT_PAD };
    for (index, entry) in level.entries.iter().enumerate() {
        let (top, height) = layout.rows[index];
        let y = top - scroll;
        if y + height < 0 || y > size.height {
            continue;
        }
        if entry.is_separator() {
            let line = Rect::new(INSET_X + ICON_PAD, y + height / 2, size.width - 2 * (INSET_X + ICON_PAD), 1);
            canvas.fill_rect(line, style.separator);
            continue;
        }
        let hot = level.hot.get() == Some(index);
        if hot {
            let back = Rect::new(INSET_X, y + 1, size.width - 2 * INSET_X, height - 2);
            let fill = if entry.enabled { style.highlight } else { style.highlight.mix(style.popup_background, 0.5) };
            canvas.round_rect(back, style.corner_radius, None, Some(fill));
        }
        let color = if !entry.enabled {
            style.disabled_foreground
        } else if hot {
            style.highlight_foreground
        } else {
            style.popup_foreground
        };
        if layout.icon_column {
            let (ix, iy) = (INSET_X + ICON_PAD, y + (height - ICON) / 2);
            match &entry.icon {
                Some(icon) => {
                    if entry.checked {
                        let frame = Rect::new(ix - 3, iy - 3, ICON + 6, ICON + 6);
                        let pen = Pen::new(color.mix(style.popup_background, 0.5), 1);
                        canvas.round_rect(frame, 3, Some(&pen), Some(style.highlight));
                    }
                    draw_icon(canvas, icon.as_ref(), ix, iy, entry.enabled);
                }
                None if entry.checked => draw_mark(canvas, ix + ICON / 2, y + height / 2, entry.radio, color),
                None => {}
            }
        }
        let font = if entry.default { &bold } else { &level.font };
        let text_right = if entry.shortcut.is_empty() { shortcut_right } else { shortcut_right - SHORTCUT_GAP };
        let text = Rect::new(layout.text_x, y, (text_right - layout.text_x).max(0), height);
        canvas.mnemonic_text(&entry.text, text, font, color, TextAlign::Left, cues);
        if !entry.shortcut.is_empty() {
            let shortcut_color = if entry.enabled && !hot { style.shortcut_foreground } else { color };
            let area = Rect::new(layout.text_x, y, shortcut_right - layout.text_x, height);
            canvas.mnemonic_text(&entry.shortcut, area, &level.font, shortcut_color, TextAlign::Right, false);
        }
        if matches!(entry.kind, EntryKind::Submenu(_)) {
            let (cx, cy) = (size.width - INSET_X - ARROW_COLUMN / 2, y + height / 2);
            let pen = Pen::new(color, 1);
            canvas.polyline(&[Point::new(cx - 2, cy - 4), Point::new(cx + 2, cy), Point::new(cx - 2, cy + 4)], &pen);
        }
    }
    if level.scrollable() {
        let view = level.view_height.get();
        let pen = Pen::new(style.popup_foreground, 1);
        let cx = size.width / 2;
        if scroll > 0 {
            canvas.fill_rect(Rect::new(BORDER, BORDER, size.width - 2 * BORDER, SCROLL_BAND), style.popup_background);
            let cy = BORDER + SCROLL_BAND / 2;
            canvas.polyline(&[Point::new(cx - 4, cy + 2), Point::new(cx, cy - 2), Point::new(cx + 4, cy + 2)], &pen);
        }
        if scroll < level.max_scroll() {
            let top = view - BORDER - SCROLL_BAND;
            canvas.fill_rect(Rect::new(BORDER, top, size.width - 2 * BORDER, SCROLL_BAND), style.popup_background);
            let cy = top + SCROLL_BAND / 2;
            canvas.polyline(&[Point::new(cx - 4, cy - 2), Point::new(cx, cy + 2), Point::new(cx + 4, cy - 2)], &pen);
        }
    }
    // Borda própria em todas as versões; no Windows 11 o DWM arredonda os cantos e
    // desenha a borda dele por cima, na mesma cor.
    let border = style.popup_border;
    canvas.fill_rect(Rect::new(0, 0, size.width, BORDER), border);
    canvas.fill_rect(Rect::new(0, size.height - BORDER, size.width, BORDER), border);
    canvas.fill_rect(Rect::new(0, 0, BORDER, size.height), border);
    canvas.fill_rect(Rect::new(size.width - BORDER, 0, BORDER, size.height), border);
}

/// Check ✓ or radio ● centered at (`cx`, `cy`).
pub(super) fn draw_mark(canvas: &Canvas, cx: i32, cy: i32, radio: bool, color: Color) {
    if radio {
        canvas.ellipse(Rect::new(cx - 3, cy - 3, 7, 7), None, Some(color));
    } else {
        let pen = Pen::new(color, 2);
        canvas.polyline(&[Point::new(cx - 5, cy), Point::new(cx - 2, cy + 3), Point::new(cx + 5, cy - 4)], &pen);
    }
}
