//! Print preview: [`PrintDocument`] pages at the chosen scale, keyboard/wheel navigation and a ready-made
//! [`PrintPreviewDialog`] printing the selected range.

use super::common::{builder_common, ControlOptions};
use crate::core::widget::{Widget, WidgetCore};
use crate::core::window::PANEL_CLASS;
use crate::core::{Event, EventKind, GuiResult, HandlerId, Window};
use crate::graphics::{Bitmap, Canvas, Pen};
use crate::i18n::{library_format, library_text, LibraryText};
use crate::input::Key;
use crate::system::{PrintDocument, PrintOptions};
use crate::utilities::{Color, Rect, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Logical page margin.
const MARGIN: i32 = 16;
/// Zoom percentage steps for `zoom_in`/`zoom_out` and Ctrl+wheel.
const ZOOM_STEPS: [u16; 10] = [10, 25, 50, 75, 100, 125, 150, 200, 300, 400];
/// Scroll per wheel tick or arrow, in logical units.
const SCROLL_STEP: i32 = 48;

/// Preview page scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PreviewZoom {
    /// Fits the entire page.
    #[default]
    FitPage,
    /// Fits page width (scrolls vertically).
    FitWidth,
    /// Actual-size percentage (100 = paper size on screen).
    Percent(u16),
}

/// Scale (1.0 = actual size) for `page` in a `client` control.
pub(crate) fn scale_for(client: Size, page: Size, zoom: PreviewZoom) -> f64 {
    let available = |side: i32| (side - 2 * MARGIN).max(1) as f64;
    let (pw, ph) = (page.width.max(1) as f64, page.height.max(1) as f64);
    let scale = match zoom {
        PreviewZoom::FitPage => (available(client.width) / pw).min(available(client.height) / ph),
        PreviewZoom::FitWidth => available(client.width) / pw,
        PreviewZoom::Percent(percent) => percent.max(1) as f64 / 100.0,
    };
    scale.clamp(0.02, 8.0)
}

/// Logical page rectangle in the control (centered if fitting, scroll-offset otherwise) and maximum scroll `(x, y)`.
pub(crate) fn page_rect(client: Size, page: Size, scale: f64, scroll: (i32, i32)) -> (Rect, (i32, i32)) {
    let width = (page.width as f64 * scale).round() as i32;
    let height = (page.height as f64 * scale).round() as i32;
    let max = ((width + 2 * MARGIN - client.width).max(0), (height + 2 * MARGIN - client.height).max(0));
    let (sx, sy) = (scroll.0.clamp(0, max.0), scroll.1.clamp(0, max.1));
    let x = if max.0 > 0 { MARGIN - sx } else { (client.width - width) / 2 };
    let y = if max.1 > 0 { MARGIN - sy } else { (client.height - height) / 2 };
    (Rect::new(x, y, width, height), max)
}

/// Next zoom step from current scale.
pub(crate) fn zoom_step(scale: f64, larger: bool) -> u16 {
    let percent = scale * 100.0;
    if larger {
        ZOOM_STEPS.iter().copied().find(|&p| p as f64 > percent + 0.5).unwrap_or(ZOOM_STEPS[ZOOM_STEPS.len() - 1])
    } else {
        ZOOM_STEPS.iter().rev().copied().find(|&p| (p as f64) < percent - 0.5).unwrap_or(ZOOM_STEPS[0])
    }
}

/// Last drawn page: (page, DPI, page size, bitmap).
type CachedPage = (usize, u32, Size, Rc<Bitmap>);

struct PreviewState {
    document: RefCell<Option<PrintDocument>>,
    page: Cell<usize>,
    zoom: Cell<PreviewZoom>,
    scroll: Cell<(i32, i32)>,
    cache: RefCell<Option<CachedPage>>,
    background: Cell<Color>,
}

impl PreviewState {
    fn document(&self) -> Option<PrintDocument> {
        self.document.borrow().clone()
    }
}

/// [`PrintDocument`] page preview (custom-drawn control).
///
/// Keys: Page Down/→ and Page Up/← change page, Home/End go first/last, ↑/↓ scroll, Ctrl + `+`/`-` zoom, Ctrl+0 fits
/// page. Wheel scrolls (changes page if fitting); Ctrl+wheel zooms.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window, doc: PrintDocument) -> GuiResult<()> {
/// let preview = PrintPreview::builder().document(doc).bounds(0, 0, 600, 500).parent(window).build()?;
/// preview.on_page_changed(|page| println!("page {}", page + 1));
/// preview.set_zoom(PreviewZoom::FitWidth);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct PrintPreview {
    pub(crate) core: Rc<WidgetCore>,
    state: Rc<PreviewState>,
}

impl Widget for PrintPreview {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

impl std::fmt::Debug for PrintPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrintPreview").field("core", &self.core).field("page", &self.state.page.get()).finish()
    }
}

/// Builder for [`PrintPreview`].
#[derive(Clone, Debug)]
pub struct PrintPreviewBuilder {
    opts: ControlOptions,
    document: Option<PrintDocument>,
    zoom: PreviewZoom,
    background: Color,
}

impl PrintPreviewBuilder {
    builder_common!();

    /// Displayed document.
    pub fn document(mut self, document: PrintDocument) -> Self {
        self.document = Some(document);
        self
    }

    /// Initial scale (default: entire page).
    pub fn zoom(mut self, zoom: PreviewZoom) -> Self {
        self.zoom = zoom;
        self
    }

    /// Surrounding background (default: medium gray).
    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Creates the preview.
    pub fn build(self) -> GuiResult<PrintPreview> {
        let core = self.opts.create("PrintPreview", PANEL_CLASS, 0, 0, true, None)?;
        core.background.set(Some(self.background));
        let state = Rc::new(PreviewState {
            document: RefCell::new(self.document),
            page: Cell::new(0),
            zoom: Cell::new(self.zoom),
            scroll: Cell::new((0, 0)),
            cache: RefCell::new(None),
            background: Cell::new(self.background),
        });
        let preview = PrintPreview { core, state };
        preview.install();
        preview.describe();
        Ok(preview)
    }
}

impl PrintPreview {
    /// Starts building (default size 400×500).
    pub fn builder() -> PrintPreviewBuilder {
        PrintPreviewBuilder {
            opts: ControlOptions::new(400, 500),
            document: None,
            zoom: PreviewZoom::FitPage,
            background: Color::rgb(128, 128, 128),
        }
    }

    fn install(&self) {
        let state = Rc::downgrade(&self.state);
        let target = Rc::downgrade(&self.core);
        *self.core.paint.borrow_mut() = Some(Rc::new(move |canvas: &Canvas| {
            if let (Some(state), Some(core)) = (state.upgrade(), target.upgrade()) {
                paint(&core, &state, canvas);
            }
        }));
        let me = self.weak();
        self.core.on(EventKind::KeyDown, move |event| {
            if let (Some(preview), Event::KeyDown(key)) = (me.upgrade(), event) {
                preview.key(key.key, key.modifiers.ctrl);
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseWheel, move |event| {
            if let (Some(preview), Event::MouseWheel(mouse)) = (me.upgrade(), event) {
                preview.wheel(mouse.wheel_delta, mouse.modifiers.ctrl, mouse.modifiers.shift);
            }
        });
        let me = self.weak();
        self.core.on(EventKind::MouseDown, move |_| {
            if let Some(preview) = me.upgrade() {
                preview.focus();
            }
        });
        let me = self.weak();
        self.core.on(EventKind::Resize, move |_| {
            if let Some(preview) = me.upgrade() {
                preview.clamp_scroll();
                preview.core.invalidate();
            }
        });
    }

    fn weak(&self) -> WeakPreview {
        WeakPreview { core: Rc::downgrade(&self.core), state: Rc::downgrade(&self.state) }
    }

    fn key(&self, key: Key, ctrl: bool) {
        const OEM_PLUS: u16 = 0xBB;
        const OEM_MINUS: u16 = 0xBD;
        const ADD: u16 = 0x6B;
        const SUBTRACT: u16 = 0x6D;
        match key {
            Key::PAGE_DOWN | Key::RIGHT => self.next_page(),
            Key::PAGE_UP | Key::LEFT => self.previous_page(),
            Key::HOME => self.set_page(0),
            Key::END => self.set_page(self.page_count().saturating_sub(1)),
            Key::DOWN => self.scroll_by(0, SCROLL_STEP),
            Key::UP => self.scroll_by(0, -SCROLL_STEP),
            Key(OEM_PLUS) | Key(ADD) if ctrl => self.zoom_in(),
            Key(OEM_MINUS) | Key(SUBTRACT) if ctrl => self.zoom_out(),
            Key(0x30) if ctrl => self.set_zoom(PreviewZoom::FitPage),
            _ => {}
        }
    }

    fn wheel(&self, delta: i32, ctrl: bool, shift: bool) {
        if delta == 0 {
            return;
        }
        if ctrl {
            if delta > 0 {
                self.zoom_in()
            } else {
                self.zoom_out()
            }
            return;
        }
        let (_, max) = self.geometry();
        if shift && max.0 > 0 {
            self.scroll_by(-delta * SCROLL_STEP, 0);
        } else if max.1 > 0 {
            let before = self.state.scroll.get().1;
            self.scroll_by(0, -delta * SCROLL_STEP);
            // No fim (ou no começo) da página, a roda passa para a página seguinte.
            if self.state.scroll.get().1 == before {
                let old = self.page();
                if delta < 0 && old + 1 < self.page_count() {
                    self.set_page(old + 1);
                } else if delta > 0 && old > 0 {
                    self.set_page(old - 1);
                    self.state.scroll.set((self.state.scroll.get().0, i32::MAX));
                    self.clamp_scroll();
                }
            }
        } else if delta < 0 {
            self.next_page();
        } else {
            self.previous_page();
        }
    }

    fn geometry(&self) -> (Rect, (i32, i32)) {
        let client = self.client_size();
        let page = self.state.document().map_or(crate::system::A4, |d| d.page_size());
        let scale = scale_for(client, page, self.state.zoom.get());
        page_rect(client, page, scale, self.state.scroll.get())
    }

    fn clamp_scroll(&self) {
        let (_, max) = self.geometry();
        let (x, y) = self.state.scroll.get();
        self.state.scroll.set((x.clamp(0, max.0), y.clamp(0, max.1)));
    }

    fn scroll_by(&self, dx: i32, dy: i32) {
        let (x, y) = self.state.scroll.get();
        self.state.scroll.set((x.saturating_add(dx), y.saturating_add(dy)));
        self.clamp_scroll();
        self.core.invalidate();
    }

    fn describe(&self) {
        let count = self.page_count();
        let text = library_format(
            LibraryText::PageOf,
            &[("page", &(self.page() + 1).min(count.max(1)).to_string()), ("count", &count.max(1).to_string())],
        );
        let _ = self.set_accessible_name(&library_text(LibraryText::PrintPreview));
        let _ = self.set_accessible_description(&text);
    }

    /// Displayed document.
    pub fn document(&self) -> Option<PrintDocument> {
        self.state.document()
    }

    /// Changes document, returning to the first page.
    pub fn set_document(&self, document: Option<PrintDocument>) {
        *self.state.document.borrow_mut() = document;
        self.state.page.set(0);
        self.state.scroll.set((0, 0));
        self.refresh();
        self.core.emit(&Event::ValueChanged);
    }

    /// Redraws pages; call after changing document drawing.
    pub fn refresh(&self) {
        self.state.cache.borrow_mut().take();
        let count = self.page_count();
        if self.page() >= count.max(1) {
            self.state.page.set(count.saturating_sub(1));
        }
        self.clamp_scroll();
        self.describe();
        self.core.invalidate();
    }

    /// Document page count.
    pub fn page_count(&self) -> usize {
        self.state.document().map_or(0, |d| d.page_count())
    }

    /// Displayed page (zero-based).
    pub fn page(&self) -> usize {
        self.state.page.get()
    }

    /// Shows page `index` (zero-based; clamped to existing pages).
    pub fn set_page(&self, index: usize) {
        let index = index.min(self.page_count().saturating_sub(1));
        if index == self.page() {
            return;
        }
        self.state.page.set(index);
        self.state.scroll.set((self.state.scroll.get().0, 0));
        self.describe();
        self.core.invalidate();
        self.core.emit(&Event::ValueChanged);
    }

    /// Next page.
    pub fn next_page(&self) {
        self.set_page(self.page() + 1);
    }

    /// Previous page.
    pub fn previous_page(&self) {
        self.set_page(self.page().saturating_sub(1));
    }

    /// Current scale.
    pub fn zoom(&self) -> PreviewZoom {
        self.state.zoom.get()
    }

    /// Changes scale.
    pub fn set_zoom(&self, zoom: PreviewZoom) {
        self.state.zoom.set(zoom);
        self.clamp_scroll();
        self.core.invalidate();
    }

    /// Effective scale (1.0 = paper size) at current control size.
    pub fn effective_scale(&self) -> f64 {
        let page = self.state.document().map_or(crate::system::A4, |d| d.page_size());
        scale_for(self.client_size(), page, self.state.zoom.get())
    }

    /// Zooms to the next step (10% … 400%).
    pub fn zoom_in(&self) {
        self.set_zoom(PreviewZoom::Percent(zoom_step(self.effective_scale(), true)));
    }

    /// Zooms out to the previous step.
    pub fn zoom_out(&self) {
        self.set_zoom(PreviewZoom::Percent(zoom_step(self.effective_scale(), false)));
    }

    /// Logical page drawing rectangle within the control.
    pub fn page_bounds(&self) -> Rect {
        self.geometry().0
    }

    /// Called when the displayed page or document changes; receives the zero-based page.
    pub fn on_page_changed(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        let state = Rc::downgrade(&self.state);
        self.core.on(EventKind::ValueChanged, move |_| {
            if let Some(state) = state.upgrade() {
                f(state.page.get())
            }
        })
    }

    /// Background surrounding the page.
    pub fn set_page_background(&self, color: Color) {
        self.state.background.set(color);
        self.core.background.set(Some(color));
        self.core.invalidate();
    }
}

#[derive(Clone)]
struct WeakPreview {
    core: std::rc::Weak<WidgetCore>,
    state: std::rc::Weak<PreviewState>,
}

impl WeakPreview {
    fn upgrade(&self) -> Option<PrintPreview> {
        Some(PrintPreview { core: self.core.upgrade()?, state: self.state.upgrade()? })
    }
}

fn paint(core: &WidgetCore, state: &PreviewState, canvas: &Canvas) {
    let client = canvas.size();
    canvas.clear(state.background.get());
    let Some(document) = state.document() else { return };
    if document.page_count() == 0 {
        return;
    }
    let page_size = document.page_size();
    let scale = scale_for(client, page_size, state.zoom.get());
    let (rect, _) = page_rect(client, page_size, scale, state.scroll.get());
    let index = state.page.get().min(document.page_count() - 1);
    let dpi = ((canvas.dpi() as f64) * scale).round().max(1.0) as u32;
    let cached = state
        .cache
        .borrow()
        .as_ref()
        .filter(|(page, at, size, _)| *page == index && *at == dpi && *size == page_size)
        .map(|(_, _, _, bitmap)| bitmap.clone());
    // O desenho da página roda sem nenhum empréstimo do estado: a aplicação pode
    // consultar a pré-visualização dentro dele.
    let bitmap = match cached {
        Some(bitmap) => Some(bitmap),
        None => match document.render_bitmap(index, dpi) {
            Ok((bitmap, _)) => {
                let bitmap = Rc::new(bitmap);
                *state.cache.borrow_mut() = Some((index, dpi, page_size, bitmap.clone()));
                Some(bitmap)
            }
            Err(_) => None,
        },
    };
    let shadow = Rect::new(rect.x + 3, rect.y + 3, rect.width, rect.height);
    canvas.fill_rect(shadow, Color::rgb(64, 64, 64));
    match bitmap {
        Some(bitmap) => canvas.draw_bitmap(&bitmap, rect),
        None => canvas.fill_rect(rect, Color::WHITE),
    }
    if core.is_alive() && has_focus(core.hwnd()) {
        let outline = Rect::new(rect.x - 2, rect.y - 2, rect.width + 4, rect.height + 4);
        canvas.rect(outline, Some(&Pen::new(Color::rgb(0, 120, 215), 1)), None);
    }
}

fn has_focus(hwnd: crate::ffi::HWND) -> bool {
    // SAFETY: consulta simples do foco desta thread.
    !hwnd.is_null() && unsafe { crate::ffi::user32::GetFocus() } == hwnd
}

/// Ready-made preview window: toolbar with Print, Previous/Next, "Page X of N", Zoom Out/In and Close, with preview
/// filling the rest.
///
/// Print opens [`PrintDialog`](crate::dialogs::PrintDialog) with the document's page range and prints selected pages.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window, doc: PrintDocument) -> GuiResult<()> {
/// let printed = PrintPreviewDialog::new(doc).show(window)?;
/// # Ok(()) }
/// ```
#[derive(Clone, Debug)]
pub struct PrintPreviewDialog {
    document: PrintDocument,
    title: Option<String>,
    size: Size,
    options: PrintOptions,
}

/// Preview dialog assembled by [`PrintPreviewDialog::create`], exposing window and preview before display to add
/// controls or show nonmodally.
#[derive(Clone, Debug)]
pub struct PrintPreviewWindow {
    dialog: crate::dialogs::Dialog,
    preview: PrintPreview,
    label: super::Label,
    printed: Rc<Cell<Option<usize>>>,
}

impl PrintPreviewWindow {
    /// Dialog window.
    pub fn window(&self) -> &Window {
        self.dialog.window()
    }

    /// Preview inside the dialog.
    pub fn preview(&self) -> &PrintPreview {
        &self.preview
    }

    /// Current "Page X of N" text.
    pub fn page_text(&self) -> String {
        self.label.text()
    }

    /// Pages sent to the printer by Print (`None` until anything is printed).
    pub fn printed_pages(&self) -> Option<usize> {
        self.printed.get()
    }

    /// Shows modally; returns [`printed_pages`](Self::printed_pages) on closing.
    pub fn show_modal(&self) -> GuiResult<Option<usize>> {
        self.dialog.show_modal()?;
        Ok(self.printed.get())
    }

    /// Shows nonmodally.
    pub fn show(&self) {
        self.dialog.window().show();
    }

    /// Closes the dialog.
    pub fn close(&self) {
        self.dialog.close_with(crate::dialogs::DialogResult::Cancel);
    }
}

impl PrintPreviewDialog {
    /// Dialog for the document.
    pub fn new(document: PrintDocument) -> PrintPreviewDialog {
        PrintPreviewDialog { document, title: None, size: Size::new(720, 760), options: PrintOptions::new() }
    }

    /// Window title (default: "Print preview — document name").
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Logical client size (default 720×760).
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.size = Size::new(width.max(320), height.max(240));
        self
    }

    /// Extra print dialog options (selection, copies); page count comes from the document.
    pub fn print_options(mut self, options: PrintOptions) -> Self {
        self.options = options;
        self
    }

    /// Assembles the dialog without showing it.
    pub fn create(&self, owner: &Window) -> GuiResult<PrintPreviewWindow> {
        use super::{Button, Label};
        use crate::layout::{LayoutParams, StackLayout};
        let title = self
            .title
            .clone()
            .unwrap_or_else(|| format!("{} — {}", library_text(LibraryText::PrintPreview), self.document.name()));
        let dialog = crate::dialogs::Dialog::from_builder(
            Window::builder()
                .title(&title)
                .size(self.size.width, self.size.height)
                .resizable(true)
                .minimizable(false)
                .close_on_escape(true)
                .owner(owner),
            Some(owner),
        )?;
        let parent = dialog.window();
        let print = Button::builder().text(&library_text(LibraryText::Print)).parent(parent).build()?;
        let previous =
            Button::builder().text(&format!("◀ {}", library_text(LibraryText::Previous))).parent(parent).build()?;
        let label = Label::builder().text("").parent(parent).build()?;
        let next = Button::builder().text(&format!("{} ▶", library_text(LibraryText::Next))).parent(parent).build()?;
        let zoom_out =
            Button::builder().text("−").tooltip(&library_text(LibraryText::ZoomOut)).parent(parent).build()?;
        let zoom_in = Button::builder().text("+").tooltip(&library_text(LibraryText::ZoomIn)).parent(parent).build()?;
        let close = Button::builder().text(&library_text(LibraryText::Close)).parent(parent).build()?;
        let _ = zoom_out.set_accessible_name(&library_text(LibraryText::ZoomOut));
        let _ = zoom_in.set_accessible_name(&library_text(LibraryText::ZoomIn));
        let preview = PrintPreview::builder().document(self.document.clone()).parent(parent).build()?;

        let bar = StackLayout::horizontal().spacing(6);
        bar.add_with(&print, LayoutParams::new().width(96));
        bar.add_with(&previous, LayoutParams::new().width(96));
        bar.add_with(&label, LayoutParams::new().weight(1.0).margin(crate::utilities::Thickness::new(6, 6, 6, 0)));
        bar.add_with(&next, LayoutParams::new().width(96));
        bar.add_with(&zoom_out, LayoutParams::new().width(32));
        bar.add_with(&zoom_in, LayoutParams::new().width(32));
        bar.add_with(&close, LayoutParams::new().width(88));
        let root = StackLayout::vertical().spacing(8).padding(crate::utilities::Thickness::all(8));
        root.add_with(bar, LayoutParams::new().height(30));
        root.add_with(&preview, LayoutParams::new().weight(1.0));
        parent.set_layout(root);

        let update = {
            let (preview, label, previous, next) = (preview.clone(), label.clone(), previous.clone(), next.clone());
            move || {
                let count = preview.page_count().max(1);
                label.set_text(&library_format(
                    LibraryText::PageOf,
                    &[("page", &(preview.page() + 1).to_string()), ("count", &count.to_string())],
                ));
                previous.set_enabled(preview.page() > 0);
                next.set_enabled(preview.page() + 1 < preview.page_count());
            }
        };
        update();
        preview.on_page_changed(move |_| update());
        let target = preview.clone();
        previous.on_click(move || target.previous_page());
        let target = preview.clone();
        next.on_click(move || target.next_page());
        let target = preview.clone();
        zoom_in.on_click(move || target.zoom_in());
        let target = preview.clone();
        zoom_out.on_click(move || target.zoom_out());
        let handle = dialog.clone();
        close.on_click(move || handle.close_with(crate::dialogs::DialogResult::Cancel));

        let printed = Rc::new(Cell::new(None));
        let (handle, document, options, result) =
            (dialog.clone(), self.document.clone(), self.options, printed.clone());
        print.on_click(move || {
            let options = options.page_count(document.page_count().max(1) as u32);
            let outcome = crate::dialogs::PrintDialog::new()
                .show_with(handle.window(), &options)
                .and_then(|printer| printer.map(|printer| printer.print_document(&document)).transpose());
            match outcome {
                Ok(Some(pages)) => {
                    result.set(Some(pages));
                    handle.close_with(crate::dialogs::DialogResult::Ok);
                }
                Ok(None) => {}
                Err(error) => {
                    use crate::dialogs::{MessageBox, MessageButtons, MessageIcon};
                    let title = library_text(LibraryText::Print);
                    MessageBox::show(
                        handle.window(),
                        &title,
                        &error.to_string(),
                        MessageButtons::Ok,
                        MessageIcon::Error,
                    );
                }
            }
        });
        preview.focus();
        Ok(PrintPreviewWindow { dialog, preview, label, printed })
    }

    /// Shows modally. Returns pages sent to the printer (`None` if closed without printing).
    pub fn show(&self, owner: &Window) -> GuiResult<Option<usize>> {
        self.create(owner)?.show_modal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_modes_and_percent() {
        let page = Size::new(800, 1000);
        let client = Size::new(532, 632);
        assert!((scale_for(client, page, PreviewZoom::FitPage) - 0.6).abs() < 1e-9);
        assert!((scale_for(client, page, PreviewZoom::FitWidth) - 0.625).abs() < 1e-9);
        assert!((scale_for(client, page, PreviewZoom::Percent(150)) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn page_is_centered_or_scrolled() {
        let (rect, max) = page_rect(Size::new(500, 500), Size::new(200, 100), 1.0, (40, 40));
        assert_eq!((rect, max), (Rect::new(150, 200, 200, 100), (0, 0)), "cabe: centraliza e ignora a rolagem");
        let (rect, max) = page_rect(Size::new(300, 400), Size::new(268, 1000), 1.0, (0, 50));
        assert_eq!(max, (0, 632));
        assert_eq!(rect, Rect::new(16, 16 - 50, 268, 1000));
        let (rect, _) = page_rect(Size::new(300, 400), Size::new(268, 1000), 1.0, (0, 9999));
        assert_eq!(rect.y, 16 - 632, "rolagem limitada ao fim da página");
    }

    #[test]
    fn zoom_steps() {
        assert_eq!(zoom_step(0.6, true), 75);
        assert_eq!(zoom_step(0.6, false), 50);
        assert_eq!(zoom_step(1.0, true), 125);
        assert_eq!(zoom_step(1.0, false), 75);
        assert_eq!(zoom_step(4.0, true), 400);
        assert_eq!(zoom_step(0.1, false), 10);
    }
}
