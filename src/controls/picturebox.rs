//! Picture box: displays a scaled icon/image (own implementation).

use super::common::{builder_common, ControlOptions};
use crate::core::handles::IconSource;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{GuiResult, Icon};
use crate::graphics::{Canvas, PlusImage, SvgImage};
use crate::utilities::{to_pixels, Color, Point, Rect, Size};
use std::cell::RefCell;
use std::rc::Rc;

/// Above this side length in pixels, prefers high-resolution image over ICO.
const LARGEST_ICO: i32 = 256;

#[derive(Default)]
struct PictureState {
    source: Option<Rc<dyn IconSource>>,
    image: Option<Rc<PlusImage>>,
    svg: Option<Rc<SvgImage>>,
    size: Option<i32>,
}

/// Displays an icon/image centered and scaled to fit, preserving aspect ratio.
///
/// Own implementation (custom-drawn panel). Bundled icons ([`crate::icons`]) use exact-size ICO images up to 256 px and
/// high-resolution PNG above that.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let picture = PictureBox::builder().icon(&icons::CLOUD_UPLOAD).bounds(10, 10, 128, 128).parent(window).build()?;
/// picture.set_icon(&icons::CLOUD_DOWNLOAD);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct PictureBox {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<RefCell<PictureState>>,
}

impl Widget for PictureBox {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for PictureBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PictureBox").field("core", &self.core).finish()
    }
}

/// Builder for [`PictureBox`].
#[derive(Clone, Debug)]
pub struct PictureBoxBuilder {
    opts: ControlOptions,
    source: Option<Rc<dyn IconSource>>,
    size: Option<i32>,
    background: Option<Color>,
}

impl PictureBoxBuilder {
    builder_common!();

    /// Displayed icon.
    pub fn icon(mut self, icon: &impl IconSource) -> Self {
        self.source = Some(icon.to_shared());
        self
    }

    /// Fixed logical icon size (default: largest square fitting the control).
    pub fn icon_size(mut self, size: i32) -> Self {
        self.size = Some(size);
        self
    }

    /// Background color (default: container color).
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Creates the picture box.
    pub fn build(self) -> GuiResult<PictureBox> {
        let core = self.opts.create("PictureBox", PANEL_CLASS, 0, 0, false, None)?;
        core.background.set(self.background);
        let state =
            Rc::new(RefCell::new(PictureState { source: self.source, image: None, svg: None, size: self.size }));
        let painter = Rc::downgrade(&state);
        let target = Rc::downgrade(&core);
        *core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(state), Some(core)) = (painter.upgrade(), target.upgrade()) {
                paint(&core, &state.borrow(), canvas);
            }
        }));
        Ok(PictureBox { core, state })
    }
}

/// Largest rectangle with `content` aspect ratio fitting centered in `area`.
fn fit(area: Rect, content: Size) -> Rect {
    if content.width <= 0 || content.height <= 0 || area.width <= 0 || area.height <= 0 {
        return Rect::new(area.x, area.y, 0, 0);
    }
    let scale = (area.width as f64 / content.width as f64).min(area.height as f64 / content.height as f64);
    let width = (content.width as f64 * scale).round() as i32;
    let height = (content.height as f64 * scale).round() as i32;
    Rect::new(area.x + (area.width - width) / 2, area.y + (area.height - height) / 2, width, height)
}

fn paint(core: &WidgetCore, state: &PictureState, canvas: &Canvas) {
    let area = Rect::new(0, 0, canvas.size().width, canvas.size().height);
    let enabled = core.is_alive() && crate::core::native::is_enabled(core.hwnd());
    if let Some(svg) = &state.svg {
        let _ = canvas.draw_svg(svg, fit(area, svg.size()));
        return;
    }
    if let Some(image) = &state.image {
        let dest = fit(area, image.size());
        let _ = canvas.with_gdiplus(|g| g.draw_image(image, dest));
        return;
    }
    let Some(source) = &state.source else { return };
    let side = state.size.unwrap_or(area.width.min(area.height)).max(0);
    if side == 0 {
        return;
    }
    let pixels = to_pixels(side, canvas.dpi());
    let origin = Point::new((area.width - side) / 2, (area.height - side) / 2);
    if pixels > LARGEST_ICO && enabled {
        if let Some(Ok(image)) = source.large_image() {
            let dest = fit(Rect::new(origin.x, origin.y, side, side), image.size());
            let _ = canvas.with_gdiplus(|g| g.draw_image(&image, dest));
            return;
        }
    }
    let icon = source.icon_pixels(pixels).and_then(|icon| if enabled { Ok(icon) } else { icon.disabled() });
    if let Ok(icon) = icon {
        canvas.draw_icon(&icon, origin, side);
    }
}

impl PictureBox {
    /// Starts building (default size 64×64).
    pub fn builder() -> PictureBoxBuilder {
        PictureBoxBuilder { opts: ControlOptions::new(64, 64), source: None, size: None, background: None }
    }

    /// Displays an icon, replacing the current image.
    pub fn set_icon(&self, icon: &impl IconSource) {
        {
            let mut state = self.state.borrow_mut();
            state.source = Some(icon.to_shared());
            state.image = None;
            state.svg = None;
        }
        self.core.invalidate();
    }

    /// Displays a GDI+-decoded image, replacing the current icon.
    pub fn set_image(&self, image: PlusImage) {
        {
            let mut state = self.state.borrow_mut();
            state.image = Some(Rc::new(image));
            state.source = None;
            state.svg = None;
        }
        self.core.invalidate();
    }

    /// Displays SVG at the control's exact size (sharp at any DPI), replacing the current icon/image.
    pub fn set_svg(&self, svg: SvgImage) {
        {
            let mut state = self.state.borrow_mut();
            state.svg = Some(Rc::new(svg));
            state.image = None;
            state.source = None;
        }
        self.core.invalidate();
    }

    /// Removes the icon/image.
    pub fn clear(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.image = None;
            state.source = None;
            state.svg = None;
        }
        self.core.invalidate();
    }

    /// Sets (`Some`) or releases (`None`) fixed logical icon size.
    pub fn set_icon_size(&self, size: Option<i32>) {
        self.state.borrow_mut().size = size;
        self.core.invalidate();
    }

    /// `true` if an icon/image is available.
    pub fn has_content(&self) -> bool {
        let state = self.state.borrow();
        state.source.is_some() || state.image.is_some() || state.svg.is_some()
    }

    /// Current icon loaded at drawing size (testing/debugging API).
    pub fn current_icon(&self) -> Option<Icon> {
        let state = self.state.borrow();
        let source = state.source.as_ref()?;
        let size = self.client_size();
        let side = state.size.unwrap_or(size.width.min(size.height));
        source.icon_pixels(to_pixels(side.max(1), self.core.dpi())).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_aspect_ratio_and_centers() {
        assert_eq!(fit(Rect::new(0, 0, 200, 100), Size::new(512, 512)), Rect::new(50, 0, 100, 100));
        assert_eq!(fit(Rect::new(0, 0, 100, 200), Size::new(200, 100)), Rect::new(0, 75, 100, 50));
        assert_eq!(fit(Rect::new(0, 0, 0, 10), Size::new(10, 10)).width, 0);
    }
}
