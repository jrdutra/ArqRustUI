//! Window content dynamically reserving native status bar space.
use super::Layout;
use crate::controls::StatusBar;
use crate::core::{Widget, Window};
use crate::utilities::{Rect, Size};
use std::rc::Rc;

/// Lays content above [`StatusBar`], measuring actual bar height. Content/footer spacing belongs to inner layout
/// padding. No application-specific messages, icons or controls.
#[derive(Clone)]
pub struct StatusBarLayout {
    content: Rc<dyn Layout>,
    status: StatusBar,
    minimum_width: i32,
    fixed_height: bool,
}

impl StatusBarLayout {
    /// Attaches existing content/bar sharing a parent window.
    pub fn new(content: impl Layout + 'static, status: &StatusBar) -> Self {
        Self { content: Rc::new(content), status: status.clone(), minimum_width: 0, fixed_height: false }
    }

    /// Minimum logical width for [`fit_height`](Self::fit_height).
    pub fn minimum_width(mut self, width: i32) -> Self {
        self.minimum_width = width.max(0);
        self
    }

    /// If `true`, also limits maximum height to preferred content.
    pub fn fixed_height(mut self, fixed: bool) -> Self {
        self.fixed_height = fixed;
        self
    }

    /// Current logical native footer height.
    pub fn footer_height(&self) -> i32 {
        self.status.height().max(1)
    }

    /// Fits client height to visible content/footer, preserving width. Call after visibility changes. Minimum height
    /// follows content; flexible controls can grow. Maximized/minimized/fullscreen geometry stays unchanged: updates
    /// only limits/layout. Height fitting resumes when called in normal state.
    pub fn fit_height(&self, window: &Window) {
        if !window.is_alive() || !self.status.is_alive() {
            return;
        }
        // O primeiro WM_SIZE também permite à barra confirmar sua altura nativa.
        for _ in 0..2 {
            let height = self.preferred_size().height;
            window.set_min_size(self.minimum_width, height);
            if self.fixed_height {
                window.set_max_size(100_000, height);
            }
            let size = window.client_size();
            // SetWindowPos (usado por set_size) altera o retângulo mesmo com
            // WS_MAXIMIZE ativo: a janela encolheria sem sair do estado maximizado.
            // A geometria desses estados pertence ao Windows, não ao conteúdo.
            if !window.is_maximized() && !window.is_minimized() && !window.is_fullscreen() && size.height != height {
                window.set_size(size.width, height);
            }
        }
        window.relayout();
        window.invalidate();
    }
}

impl Layout for StatusBarLayout {
    fn arrange(&self, area: Rect) {
        self.content.arrange(Rect::new(area.x, area.y, area.width, (area.height - self.footer_height()).max(0)));
    }
    fn preferred_size(&self) -> Size {
        let size = self.content.preferred_size();
        Size::new(size.width, size.height + self.footer_height())
    }
}
