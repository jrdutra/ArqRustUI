//! [`CommandBar`](super::CommandBar) drawing: buttons, menus, separators, labels, search and `»` in theme colors.

use super::*;

pub(super) fn paint(state: &BarState, canvas: &Canvas) {
    let style = state.style();
    let font = state.font();
    let size = canvas.size();
    canvas.fill_rect(Rect::new(0, 0, size.width, size.height), style.bar_background);
    if state.bottom_border.get() {
        canvas.fill_rect(Rect::new(0, size.height - 1, size.width, 1), style.separator);
    }
    let (hot, pressed, open) = (state.hot.get(), state.pressed.get(), state.open.get());
    let bar_focused = state.core().is_some_and(|c| focused() == c.hwnd());
    let focus = if bar_focused { state.focus.get() } else { None };
    let field = field_colors(state);
    let radius = style.corner_radius;
    let items = state.items.borrow();
    for item in items.iter() {
        let Some(r) = item.rect else { continue };
        match &item.kind {
            Kind::Separator => {
                let x = r.x + SEPARATOR / 2;
                canvas.fill_rect(Rect::new(x, r.y + 4, 1, r.height - 8), style.separator);
            }
            Kind::Spacer | Kind::Widget(_) => {}
            Kind::Label(text) => {
                let area = Rect::new(r.x + LABEL_PAD, r.y, r.width - 2 * LABEL_PAD, r.height);
                canvas.mnemonic_text(text, area, &font, style.bar_foreground, TextAlign::Left, false);
            }
            Kind::Search(h) => {
                let has_focus = focused() == h.core.hwnd();
                let has_text = !native::window_text(h.core.hwnd()).is_empty();
                let border = if has_focus { field.accent } else { style.popup_border };
                canvas.round_rect(r, radius, Some(&Pen::new(border, 1)), Some(field.background));
                if has_focus {
                    canvas.fill_rect(Rect::new(r.x + radius, r.bottom() - 2, r.width - 2 * radius, 2), field.accent);
                }
                draw_magnifier(canvas, r.x + 9, r.y + r.height / 2, style.shortcut_foreground);
                if has_text {
                    let clear = Rect::new(r.right() - SEARCH_CLEAR + 2, r.y + 4, SEARCH_CLEAR - 6, r.height - 8);
                    if hot == Some((item.id, Part::Clear)) {
                        canvas.round_rect(clear, radius, None, Some(field.background.mix(field.foreground, 0.1)));
                    }
                    draw_cross(canvas, clear.x + clear.width / 2, clear.y + clear.height / 2, field.foreground);
                }
            }
            Kind::Button { data, .. } => {
                let is_hot = hot.is_some_and(|(id, _)| id == item.id) && data.enabled;
                let is_open = open == Some(item.id);
                let split_at = r.right() - SPLIT_ARROW;
                let pressed_part = pressed.filter(|(id, _)| *id == item.id).map(|(_, p)| p);
                if data.checked && matches!(data.mode, Mode::Toggle) {
                    let fill = if is_hot { style.bar_open.mix(style.bar_foreground, 0.06) } else { style.bar_open };
                    canvas.round_rect(r, radius, Some(&Pen::new(style.separator, 1)), Some(fill));
                } else if is_hot || is_open || pressed_part.is_some() {
                    canvas.round_rect(r, radius, None, Some(style.bar_hover));
                }
                let strong = match (&data.mode, pressed_part, is_open) {
                    (Mode::Split(_), Some(Part::Main), _) => Some(Rect::new(r.x, r.y, split_at - r.x, r.height)),
                    (Mode::Split(_), _, true) | (Mode::Split(_), Some(Part::Arrow), _) => {
                        Some(Rect::new(split_at, r.y, SPLIT_ARROW, r.height))
                    }
                    (_, Some(_), _) | (_, _, true) => Some(r),
                    _ => None,
                };
                if let Some(area) = strong {
                    canvas.round_rect(area, radius, None, Some(style.bar_open));
                }
                if matches!(data.mode, Mode::Split(_)) && (is_hot || is_open) {
                    canvas.fill_rect(Rect::new(split_at, r.y + 5, 1, r.height - 10), style.separator);
                }
                let color = if !data.enabled {
                    style.disabled_foreground
                } else if is_hot || is_open {
                    style.bar_hover_foreground
                } else {
                    style.bar_foreground
                };
                let content_right = match data.mode {
                    Mode::Dropdown(_) => r.right() - ARROW,
                    Mode::Split(_) => split_at,
                    _ => r.right(),
                };
                let text_visible = data.show_text && !data.text.is_empty();
                let mut x = r.x + BUTTON_PAD;
                if let Some(icon) = &data.icon {
                    if !text_visible {
                        x = r.x + (content_right - r.x - ICON) / 2;
                    }
                    popup::draw_icon(canvas, icon.as_ref(), x, r.y + (r.height - ICON) / 2, data.enabled);
                    x += ICON + ICON_TEXT_GAP;
                }
                if text_visible {
                    let area = Rect::new(x, r.y, (content_right - BUTTON_PAD - x + 2).max(0), r.height);
                    canvas.mnemonic_text(&data.text, area, &font, color, TextAlign::Left, false);
                }
                match data.mode {
                    Mode::Dropdown(_) => draw_chevron(canvas, r.right() - ARROW / 2 - 4, r.y + r.height / 2, color),
                    Mode::Split(_) => draw_chevron(canvas, split_at + SPLIT_ARROW / 2, r.y + r.height / 2, color),
                    _ => {}
                }
            }
        }
        if focus == Some(item.id) {
            canvas.round_rect(r, radius, Some(&Pen::new(style.bar_foreground, 1)), None);
        }
    }
    drop(items);
    if let Some(Reorder { active: true, target: Some(target), .. }) = state.reorder.get() {
        let x = state.insertion_x(target);
        let accent = style.highlight;
        canvas.fill_rect(Rect::new(x - 1, 4, 2, size.height - 8), accent);
        canvas.fill_rect(Rect::new(x - 3, 3, 6, 2), accent);
        canvas.fill_rect(Rect::new(x - 3, size.height - 5, 6, 2), accent);
    }
    if let Some(r) = state.overflow.get() {
        let is_open = open == Some(OVERFLOW_ID);
        if is_open {
            canvas.round_rect(r, radius, None, Some(style.bar_open));
        } else if hot.is_some_and(|(id, _)| id == OVERFLOW_ID) {
            canvas.round_rect(r, radius, None, Some(style.bar_hover));
        }
        canvas.mnemonic_text("»", r, &font, style.bar_foreground, TextAlign::Center, false);
        if focus == Some(OVERFLOW_ID) {
            canvas.round_rect(r, radius, Some(&Pen::new(style.bar_foreground, 1)), None);
        }
    }
}

/// Search field colors (theme text field colors or system colors).
struct FieldColors {
    background: Color,
    foreground: Color,
    accent: Color,
}

fn field_colors(state: &BarState) -> FieldColors {
    // SAFETY: consultas de cores do sistema.
    let system = |index| Color::from_colorref(unsafe { GetSysColor(index) });
    let theme = state.core().and_then(|c| c.root()).and_then(|root| root.theme.borrow().clone());
    match theme {
        Some(theme) if !crate::system::accessibility::high_contrast() => {
            FieldColors { background: theme.input_background, foreground: theme.input_foreground, accent: theme.accent }
        }
        _ => FieldColors {
            background: system(COLOR_WINDOW),
            foreground: system(COLOR_WINDOWTEXT),
            accent: system(COLOR_HIGHLIGHT),
        },
    }
}

/// Window with keyboard focus on this thread.
pub(super) fn focused() -> HWND {
    // SAFETY: consulta simples.
    unsafe { GetFocus() }
}

fn draw_chevron(canvas: &Canvas, cx: i32, cy: i32, color: Color) {
    let pen = Pen::new(color, 1);
    canvas.polyline(&[Point::new(cx - 4, cy - 2), Point::new(cx, cy + 2), Point::new(cx + 4, cy - 2)], &pen);
}

fn draw_magnifier(canvas: &Canvas, x: i32, cy: i32, color: Color) {
    let pen = Pen::new(color, 1);
    canvas.ellipse(Rect::new(x, cy - 6, 10, 10), Some(&pen), None);
    canvas.line(Point::new(x + 8, cy + 3), Point::new(x + 12, cy + 7), &Pen::new(color, 2));
}

fn draw_cross(canvas: &Canvas, cx: i32, cy: i32, color: Color) {
    let pen = Pen::new(color, 1);
    canvas.line(Point::new(cx - 4, cy - 4), Point::new(cx + 5, cy + 5), &pen);
    canvas.line(Point::new(cx + 4, cy - 4), Point::new(cx - 5, cy + 5), &pen);
}
