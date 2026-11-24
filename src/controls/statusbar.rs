//! Status bar.

use super::common::widget_type;
use super::icon_support::{ManagedImages, DEFAULT_ICON_SIZE};
use crate::core::handles::IconSource;
use crate::core::native;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::{EventKind, GuiResult};
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;
use crate::utilities::{to_logical, to_pixels, Rect};

widget_type!(
    /// Status bar anchored to window bottom with one or more text parts.
    ///
    /// Repositions automatically on window resizing. In layouts reserve [`height`](StatusBar::height) units at the
    /// bottom, e.g. padding `Thickness::new(0, 0, 0, status.height())`.
    StatusBar
);

impl StatusBar {
    /// Creates a status bar at the bottom of `parent`.
    pub fn new(parent: &impl Widget) -> GuiResult<StatusBar> {
        let core = WidgetCore::create_child(
            "StatusBar",
            parent.core(),
            "msctls_statusbar32",
            "",
            WS_VISIBLE | SBARS_SIZEGRIP,
            0,
            Rect::default(),
            None,
        )?;
        // A barra calcula a própria posição ao receber WM_SIZE.
        let hwnd = core.hwnd() as isize;
        parent.core().on(EventKind::Resize, move |_| {
            native::send(hwnd as _, WM_SIZE, 0, 0);
        });
        Ok(StatusBar { core })
    }

    /// Divides the bar into parts with given logical widths; `-1` on the last fills the remainder.
    pub fn set_parts(&self, widths: &[i32]) {
        let dpi = self.core.dpi();
        let mut edge = 0;
        let mut edges: Vec<i32> = widths
            .iter()
            .map(|&width| {
                if width < 0 {
                    -1
                } else {
                    edge += to_pixels(width, dpi);
                    edge
                }
            })
            .collect();
        if edges.is_empty() {
            edges.push(-1);
        }
        // SAFETY: SB_SETPARTS lê `wParam` inteiros do array apontado por `lParam`.
        unsafe { SendMessageW(self.core.hwnd(), SB_SETPARTS, edges.len(), edges.as_ptr() as isize) };
    }

    /// Sets part text (part 0 is the only part by default).
    pub fn set_part_text(&self, part: usize, text: &str) {
        native::send_text(self.core.hwnd(), SB_SETTEXTW, part, text);
    }

    /// Part text.
    pub fn part_text(&self, part: usize) -> String {
        let hwnd = self.core.hwnd();
        let len = (native::send(hwnd, SB_GETTEXTLENGTHW, part, 0) & 0xFFFF) as usize;
        let mut buffer = vec![0u16; len + 2];
        // SAFETY: o buffer comporta o tamanho informado pelo controle mais o terminador.
        unsafe { SendMessageW(hwnd, SB_GETTEXTW, part, buffer.as_mut_ptr() as isize) };
        crate::utilities::from_wide(&buffer)
    }

    /// Shows a small icon at the start of a part.
    pub fn set_part_icon(&self, part: usize, icon: &impl IconSource) -> GuiResult<()> {
        // O controle não copia o HICON: a origem fica registrada na lista gerenciada,
        // que a mantém viva (um `Icon` próprio) — ícones do pacote vivem no cache.
        ManagedImages::index_of(&self.core, icon, DEFAULT_ICON_SIZE, |_, _| {})?;
        let handle = icon.icon_pixels(to_pixels(DEFAULT_ICON_SIZE, self.core.dpi()))?;
        native::send(self.core.hwnd(), SB_SETICON, part, handle.raw() as isize);
        Ok(())
    }

    /// Removes a part's icon.
    pub fn clear_part_icon(&self, part: usize) {
        native::send(self.core.hwnd(), SB_SETICON, part, 0);
    }

    /// `true` if the part displays an icon.
    pub fn has_part_icon(&self, part: usize) -> bool {
        native::send(self.core.hwnd(), SB_GETICON, part, 0) != 0
    }

    /// Logical bar height.
    pub fn height(&self) -> i32 {
        let rect = native::window_rect(self.core.hwnd());
        to_logical(rect.bottom - rect.top, self.core.dpi())
    }
}
