//! Static text labels with optional icons.

use super::button::{icon_gap, layout_icon_and_text};
use super::common::{builder_common, widget_type, ControlOptions};
use super::icon_support::{IconPosition, WidgetIcon, DEFAULT_ICON_SIZE};
use crate::core::handles::IconSource;
use crate::core::native;
use crate::core::widget::{Reflected, WidgetCore};
use crate::core::{Event, GuiResult};
use crate::ffi::user32::*;
use crate::graphics::{Canvas, TextAlign};
use crate::utilities::{to_logical, to_pixels, Color, Rect, Size};
use std::rc::Rc;

widget_type!(
    /// Static text label. `&` before a letter creates a mnemonic focusing the next control in tab order.
    ///
    /// With an icon ([`LabelBuilder::icon`] or [`Label::set_icon`]), the library draws the label (`SS_OWNERDRAW`): icon
    /// and text on one line with the chosen alignment.
    ///
    /// ```no_run
    /// # use arqrustui::prelude::*;
    /// # fn demo(window: &Window) -> GuiResult<()> {
    /// Label::builder().text("Secure connection").icon(&icons::SHIELD_CHECK).parent(window).build()?;
    /// # Ok(()) }
    /// ```
    Label
);

/// Builder for [`Label`].
#[derive(Clone, Debug)]
pub struct LabelBuilder {
    opts: ControlOptions,
    align: TextAlign,
    no_prefix: bool,
    single_line: bool,
    icon: Option<Rc<dyn IconSource>>,
    icon_size: i32,
    icon_position: IconPosition,
}

impl LabelBuilder {
    builder_common!();

    /// Horizontal text alignment (icon/text group when an icon exists).
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Single line, vertically centered without wrapping. Use beside fields, buttons and combo boxes at the same height
    /// ([`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)) to align text baselines.
    pub fn single_line(mut self, single_line: bool) -> Self {
        self.single_line = single_line;
        self
    }

    /// Displays `&` literally instead of as a mnemonic.
    pub fn no_prefix(mut self, no_prefix: bool) -> Self {
        self.no_prefix = no_prefix;
        self
    }

    /// Icon displayed with text.
    pub fn icon(mut self, icon: &impl IconSource) -> Self {
        self.icon = Some(icon.to_shared());
        self
    }

    /// Logical icon size (default: 16).
    pub fn icon_size(mut self, size: i32) -> Self {
        self.icon_size = size;
        self
    }

    /// Icon position relative to text (default: left).
    pub fn icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// Creates the label.
    pub fn build(self) -> GuiResult<Label> {
        let mut style = SS_NOTIFY
            | match self.align {
                TextAlign::Left => SS_LEFT,
                TextAlign::Center => SS_CENTER,
                TextAlign::Right => SS_RIGHT,
            };
        if self.no_prefix {
            style |= SS_NOPREFIX;
        }
        if self.single_line {
            style |= SS_CENTERIMAGE;
        }
        let core = self.opts.create("Label", "STATIC", style, 0, false, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Command { code, .. } if *code == STN_CLICKED => {
                core.emit(&Event::Click);
                Some(0)
            }
            Reflected::Command { code, .. } if *code == STN_DBLCLK => {
                core.emit(&Event::DoubleClick);
                Some(0)
            }
            Reflected::DrawItem { lparam } => {
                // SAFETY: em WM_DRAWITEM o LPARAM aponta para um DRAWITEMSTRUCT válido
                // durante a mensagem.
                let draw = unsafe { &*(*lparam as *const DRAWITEMSTRUCT) };
                paint_label(core, draw);
                Some(1)
            }
            _ => None,
        });
        let label = Label { core };
        if let Some(icon) = self.icon {
            label.install_icon(icon, self.icon_size, self.icon_position)?;
        }
        Ok(label)
    }
}

impl Label {
    /// Starts building (default size 120×20).
    pub fn builder() -> LabelBuilder {
        LabelBuilder {
            opts: ControlOptions::new(120, 20),
            align: TextAlign::Left,
            no_prefix: false,
            single_line: false,
            icon: None,
            icon_size: DEFAULT_ICON_SIZE,
            icon_position: IconPosition::Left,
        }
    }

    fn install_icon(&self, source: Rc<dyn IconSource>, size: i32, position: IconPosition) -> GuiResult<()> {
        let hwnd = self.core.require_alive()?;
        source.icon_pixels(to_pixels(size.max(1), self.core.dpi()))?;
        let current = native::style(hwnd);
        let saved_style = match self.core.icon.borrow().as_ref() {
            Some(icon) => icon.saved_style,
            None => current & SS_TYPEMASK,
        };
        let previous = self.core.icon.borrow_mut().replace(WidgetIcon {
            source,
            size: size.max(1),
            position,
            auto_size: false,
            native_accepted: false,
            saved_style,
        });
        set_type(hwnd, SS_OWNERDRAW);
        drop(previous);
        self.core.invalidate();
        Ok(())
    }

    /// Shows an icon with text (replaces the previous icon).
    pub fn set_icon(&self, icon: &impl IconSource) -> GuiResult<()> {
        let (size, position) =
            self.core.icon.borrow().as_ref().map_or((DEFAULT_ICON_SIZE, IconPosition::Left), |i| (i.size, i.position));
        self.install_icon(icon.to_shared(), size, position)
    }

    /// Removes the icon and restores native drawing.
    pub fn clear_icon(&self) {
        let previous = self.core.icon.borrow_mut().take();
        if let Some(icon) = previous {
            set_type(self.core.hwnd(), icon.saved_style);
            self.core.invalidate();
        }
    }

    /// `true` if the label displays an icon.
    pub fn has_icon(&self) -> bool {
        self.core.icon.borrow().is_some()
    }
}

/// Changes static control type (`SS_TYPEMASK` bits), preserving other bits.
fn set_type(hwnd: crate::ffi::HWND, kind: u32) {
    if hwnd.is_null() {
        return;
    }
    let style = (native::style(hwnd) & !SS_TYPEMASK) | (kind & SS_TYPEMASK);
    // SAFETY: altera o estilo de um controle vivo desta thread; o controle estático
    // consulta o tipo a cada pintura.
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize) };
}

/// Displayed text without mnemonic markers (`&&` becomes `&`).
fn display_text(text: &str, no_prefix: bool) -> String {
    if no_prefix {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                out.push('&');
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn paint_label(core: &Rc<WidgetCore>, draw: &DRAWITEMSTRUCT) {
    let Some(icon) = core.icon.borrow().clone() else { return };
    let dpi = core.dpi();
    let pixel = Size::new(draw.rcItem.right - draw.rcItem.left, draw.rcItem.bottom - draw.rcItem.top);
    let canvas = Canvas::for_hdc(draw.hDC, dpi, pixel);
    let bounds = Rect::new(0, 0, to_logical(pixel.width, dpi), to_logical(pixel.height, dpi));
    // SAFETY: consultas de cores do sistema.
    let system = |index| Color::from_colorref(unsafe { GetSysColor(index) });
    let background = core.effective_background().unwrap_or_else(|| system(COLOR_BTNFACE));
    canvas.fill_rect(bounds, background);
    let enabled = draw.itemState & ODS_DISABLED == 0;
    let theme = core.root().and_then(|root| root.theme.borrow().clone());
    let foreground = if enabled {
        core.foreground.get().or(theme.map(|t| t.foreground)).unwrap_or_else(|| system(COLOR_WINDOWTEXT))
    } else {
        system(COLOR_GRAYTEXT)
    };
    let style = native::style(core.hwnd());
    let text = display_text(&native::window_text(core.hwnd()), style & SS_NOPREFIX != 0);
    let font = core.font();
    let text_size = if text.is_empty() { Size::default() } else { canvas.measure_text(&text, &font) };
    // O conjunto ícone + texto respeita o alinhamento original do rótulo.
    let content = if icon.position == IconPosition::Left || icon.position == IconPosition::Right {
        Size::new(icon.size + if text.is_empty() { 0 } else { icon_gap(icon.size) } + text_size.width, bounds.height)
    } else {
        Size::new(icon.size.max(text_size.width), bounds.height)
    };
    let area = match icon.saved_style {
        SS_CENTER => bounds,
        SS_RIGHT => Rect::new(bounds.width - content.width, 0, content.width, bounds.height),
        _ => Rect::new(0, 0, content.width, bounds.height),
    };
    let (icon_at, text_at) = layout_icon_and_text(area, icon.size, text_size, icon.position, icon_gap(icon.size), 0);
    let image =
        icon.source.icon_pixels(to_pixels(icon.size, dpi)).and_then(
            |image| {
                if enabled {
                    Ok(image)
                } else {
                    image.disabled()
                }
            },
        );
    if let Ok(image) = image {
        canvas.draw_icon(&image, icon_at, icon.size);
    }
    if !text.is_empty() && icon.position != IconPosition::Center {
        canvas.text(&text, text_at, &font, foreground);
    }
}

#[cfg(test)]
mod tests {
    use super::display_text;

    #[test]
    fn mnemonic_markers_are_removed() {
        assert_eq!(display_text("&Arquivo", false), "Arquivo");
        assert_eq!(display_text("A && B", false), "A & B");
        assert_eq!(display_text("A && B", true), "A && B");
    }
}
