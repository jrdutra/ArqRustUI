//! Printing with screen [`Canvas`], multipage [`PrintDocument`], [`PrintRange`] and
//! [`PrintPreview`](crate::controls::PrintPreview).

use crate::core::{GuiError, GuiResult};
use crate::ffi::gdi32::*;
use crate::ffi::HDC;
use crate::graphics::{Bitmap, Canvas, PlusImage};
use crate::utilities::{to_logical, to_pixels, to_wide, Color, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Pages selected for printing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrintRange {
    /// All pages.
    #[default]
    All,
    /// One-based inclusive page range.
    Pages {
        /// First page (1 = document start).
        first: u32,
        /// Last page.
        last: u32,
    },
    /// Current application selection; application decides what to print.
    Selection,
}

impl PrintRange {
    /// Zero-based indices for `total` pages. [`Selection`](Self::Selection) returns all; application must build a
    /// selection-only document.
    pub fn pages(&self, total: usize) -> Vec<usize> {
        match *self {
            PrintRange::All | PrintRange::Selection => (0..total).collect(),
            PrintRange::Pages { first, last } => {
                let (first, last) = (first.min(last).max(1) as usize, first.max(last).max(1) as usize);
                (first - 1..last.min(total)).collect()
            }
        }
    }

    /// Whether zero-based page `index` is included.
    pub fn contains(&self, index: usize) -> bool {
        match *self {
            PrintRange::All | PrintRange::Selection => true,
            PrintRange::Pages { first, last } => {
                let page = index as u32 + 1;
                page >= first.min(last) && page <= first.max(last)
            }
        }
    }
}

/// [`PrintDialog`](crate::dialogs::PrintDialog) options: existing pages, initial range and Selection availability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct PrintOptions {
    pub(crate) page_count: Option<u32>,
    pub(crate) range: PrintRange,
    pub(crate) selection: bool,
    pub(crate) copies: u16,
}

impl Default for PrintOptions {
    fn default() -> Self {
        PrintOptions { page_count: None, range: PrintRange::All, selection: false, copies: 1 }
    }
}

impl PrintOptions {
    /// Defaults: no range, no selection, one copy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Document has `count` pages; enables range 1..=count.
    pub fn page_count(mut self, count: u32) -> Self {
        self.page_count = Some(count.max(1));
        self
    }

    /// Initially selected range.
    pub fn range(mut self, range: PrintRange) -> Self {
        self.range = range;
        self
    }

    /// Offers Selection.
    pub fn allow_selection(mut self, allow: bool) -> Self {
        self.selection = allow;
        self
    }

    /// Initial copy count.
    pub fn copies(mut self, copies: u16) -> Self {
        self.copies = copies.max(1);
        self
    }

    /// `PRINTDLGW` fields `(Flags, nFromPage, nToPage, nMinPage, nMaxPage, nCopies)`.
    pub(crate) fn dialog_fields(&self) -> (u32, u16, u16, u16, u16, u16) {
        use crate::ffi::comdlg32::*;
        let mut flags = PD_RETURNDC | PD_USEDEVMODECOPIESANDCOLLATE;
        if !self.selection {
            flags |= PD_NOSELECTION;
        }
        let max = self.page_count.map_or(1, |c| c.min(u16::MAX as u32) as u16);
        let (mut from, mut to) = (1, max);
        match self.range {
            PrintRange::Pages { first, last } if self.page_count.is_some() => {
                flags |= PD_PAGENUMS;
                from = (first.min(last).max(1) as u16).min(max);
                to = (first.max(last).max(1) as u16).min(max);
            }
            PrintRange::Selection if self.selection => flags |= PD_SELECTION,
            _ => {}
        }
        if self.page_count.is_none() {
            flags |= PD_NOPAGENUMS;
        }
        (flags, from, to, 1, max, self.copies)
    }

    /// Selected range from `PrintDlgW` result fields.
    pub(crate) fn range_from_dialog(flags: u32, from: u16, to: u16) -> PrintRange {
        use crate::ffi::comdlg32::*;
        if flags & PD_SELECTION != 0 {
            PrintRange::Selection
        } else if flags & PD_PAGENUMS != 0 {
            PrintRange::Pages { first: from.max(1) as u32, last: to.max(from).max(1) as u32 }
        } else {
            PrintRange::All
        }
    }
}

/// Portrait A4 size in logical 1/96-inch units.
pub const A4: Size = Size { width: 794, height: 1123 };
/// Portrait Letter size in logical 1/96-inch units.
pub const LETTER: Size = Size { width: 816, height: 1056 };

type PageFn = dyn Fn(usize, &Canvas);

struct DocumentState {
    name: RefCell<String>,
    page_size: Cell<Size>,
    page_count: Cell<usize>,
    draw: Rc<PageFn>,
}

/// Multipage document: name, page count and drawing function. The same document supports
/// [`PrinterContext::print_document`] and [`PrintPreview`](crate::controls::PrintPreview) at any scale.
///
/// Callback receives zero-based page index and [`Canvas`] whose `size()` is logical page area: printable paper area on
/// printer, [`page_size`](Self::page_size) in preview.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// let doc = PrintDocument::new("Report", 3, |page, canvas| {
///     let font = Font::new("Segoe UI", 12.0);
///     canvas.text(&format!("Page {}", page + 1), Point::new(48, 48), &font, Color::BLACK);
/// });
/// ```
#[derive(Clone)]
pub struct PrintDocument {
    state: Rc<DocumentState>,
}

impl std::fmt::Debug for PrintDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrintDocument")
            .field("name", &self.state.name.borrow())
            .field("page_size", &self.state.page_size.get())
            .field("page_count", &self.state.page_count.get())
            .finish()
    }
}

impl PrintDocument {
    /// Creates `page_count` A4 pages.
    pub fn new(name: &str, page_count: usize, draw: impl Fn(usize, &Canvas) + 'static) -> PrintDocument {
        PrintDocument {
            state: Rc::new(DocumentState {
                name: RefCell::new(name.to_string()),
                page_size: Cell::new(A4),
                page_count: Cell::new(page_count),
                draw: Rc::new(draw),
            }),
        }
    }

    /// Logical preview page size (default [`A4`]).
    pub fn with_page_size(self, size: Size) -> PrintDocument {
        self.set_page_size(size);
        self
    }

    /// Uses [`PageSetupDialog`](crate::dialogs::PageSetupDialog) paper minus margins.
    pub fn with_page_setup(self, setup: &crate::dialogs::PageSetup) -> PrintDocument {
        let logical = |hundredths_mm: i32| hundredths_mm.max(0) * 96 / 2540;
        let m = setup.margins;
        self.set_page_size(Size::new(
            logical(setup.paper_width - m.left - m.right).max(1),
            logical(setup.paper_height - m.top - m.bottom).max(1),
        ));
        self
    }

    /// Document name in print queue.
    pub fn name(&self) -> String {
        self.state.name.borrow().clone()
    }

    /// Changes document name.
    pub fn set_name(&self, name: &str) {
        *self.state.name.borrow_mut() = name.to_string();
    }

    /// Logical preview page size.
    pub fn page_size(&self) -> Size {
        self.state.page_size.get()
    }

    /// Changes logical page size.
    pub fn set_page_size(&self, size: Size) {
        self.state.page_size.set(Size::new(size.width.max(1), size.height.max(1)));
    }

    /// Page count.
    pub fn page_count(&self) -> usize {
        self.state.page_count.get()
    }

    /// Changes page count, e.g. after repagination.
    pub fn set_page_count(&self, count: usize) {
        self.state.page_count.set(count);
    }

    /// Draws page `index` on given canvas.
    pub fn draw_page(&self, index: usize, canvas: &Canvas) {
        // Clona a função antes de chamá-la: ela pode trocar o nome ou o tamanho do documento.
        let draw = self.state.draw.clone();
        draw(index, canvas);
    }

    /// Renders page `index` to white bitmap at `dpi` (96 = natural screen size).
    pub(crate) fn render_bitmap(&self, index: usize, dpi: u32) -> GuiResult<(Bitmap, Vec<u8>)> {
        let size = self.page_size();
        let (width, height) = (to_pixels(size.width, dpi).max(1), to_pixels(size.height, dpi).max(1));
        Bitmap::render_at(width, height, dpi, Some(Color::WHITE), |canvas| self.draw_page(index, canvas))
    }

    /// Page `index` image at `dpi` on white background for thumbnails/export/tests.
    pub fn render_page(&self, index: usize, dpi: u32) -> GuiResult<PlusImage> {
        if index >= self.page_count() {
            return Err(GuiError::InvalidArgument(format!("page {index} is outside the document")));
        }
        let (bitmap, pixels) = self.render_bitmap(index, dpi.max(1))?;
        let size = bitmap.size();
        PlusImage::from_pixels(size.width, size.height, &pixels)
    }
}

/// Printer context from [`PrintDialog`](crate::dialogs::PrintDialog), releasing DC on drop.
#[derive(Debug)]
pub struct PrinterContext {
    hdc: HDC,
    range: PrintRange,
    copies: u16,
}

impl PrinterContext {
    /// Takes ownership of printer DC.
    ///
    /// # Safety
    /// `hdc` must be a valid printer DC that no other owner will release.
    pub(crate) unsafe fn from_raw(hdc: HDC) -> PrinterContext {
        PrinterContext { hdc, range: PrintRange::All, copies: 1 }
    }

    pub(crate) fn with_choice(mut self, range: PrintRange, copies: u16) -> PrinterContext {
        self.range = range;
        self.copies = copies.max(1);
        self
    }

    /// Dialog-selected pages.
    pub fn range(&self) -> PrintRange {
        self.range
    }

    /// Requested copies. If driver handles copies (usual), dialog returns 1 and no application repetition is needed.
    pub fn copies(&self) -> u16 {
        self.copies
    }

    /// Prints `document` within [`range`](Self::range), repeating [`copies`](Self::copies) times. Returns pages
    /// submitted.
    pub fn print_document(&self, document: &PrintDocument) -> GuiResult<usize> {
        let pages = self.range.pages(document.page_count());
        let all: Vec<usize> = (0..self.copies.max(1)).flat_map(|_| pages.iter().copied()).collect();
        self.print_pages(&document.name(), &all, |index, canvas| document.draw_page(index, canvas))
    }

    /// Prints zero-based `pages` in given order via `draw_page`. Returns pages submitted.
    pub fn print_pages(
        &self,
        document_name: &str,
        pages: &[usize],
        mut draw_page: impl FnMut(usize, &Canvas),
    ) -> GuiResult<usize> {
        if pages.is_empty() {
            return Ok(0);
        }
        let mut next = 0usize;
        self.print(document_name, |_, canvas| {
            draw_page(pages[next], canvas);
            next += 1;
            next < pages.len()
        })
    }

    fn dpi(&self) -> u32 {
        // SAFETY: DC válido possuído por este objeto.
        (unsafe { GetDeviceCaps(self.hdc, LOGPIXELSX) }).max(1) as u32
    }

    fn pixel_size(&self) -> Size {
        // SAFETY: DC válido possuído por este objeto.
        unsafe { Size::new(GetDeviceCaps(self.hdc, HORZRES), GetDeviceCaps(self.hdc, VERTRES)) }
    }

    /// Printable area in logical 1/96-inch units.
    pub fn page_size(&self) -> Size {
        let (pixels, dpi) = (self.pixel_size(), self.dpi());
        Size::new(to_logical(pixels.width, dpi), to_logical(pixels.height, dpi))
    }

    /// Prints document; `draw_page` receives zero-based index/canvas, returning `true` while more pages remain.
    ///
    /// Returns printed page count.
    pub fn print(&self, document_name: &str, mut draw_page: impl FnMut(usize, &Canvas) -> bool) -> GuiResult<usize> {
        let name = to_wide(document_name);
        let info = DOCINFOW {
            cbSize: std::mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: name.as_ptr(),
            lpszOutput: std::ptr::null(),
            lpszDatatype: std::ptr::null(),
            fwType: 0,
        };
        // SAFETY: estrutura completa; o nome vive durante a chamada.
        if unsafe { StartDocW(self.hdc, &info) } <= 0 {
            return Err(GuiError::last_os_error("StartDocW"));
        }
        let mut pages = 0usize;
        loop {
            // SAFETY: sequência StartPage/EndPage em um documento aberto.
            if unsafe { StartPage(self.hdc) } <= 0 {
                return Err(GuiError::last_os_error("StartPage"));
            }
            let more = {
                let canvas = Canvas::for_hdc(self.hdc, self.dpi(), self.pixel_size());
                draw_page(pages, &canvas)
            };
            // SAFETY: fecha a página aberta acima.
            if unsafe { EndPage(self.hdc) } <= 0 {
                return Err(GuiError::last_os_error("EndPage"));
            }
            pages += 1;
            if !more {
                break;
            }
        }
        // SAFETY: fecha o documento aberto por StartDocW.
        if unsafe { EndDoc(self.hdc) } <= 0 {
            return Err(GuiError::last_os_error("EndDoc"));
        }
        Ok(pages)
    }
}

impl Drop for PrinterContext {
    fn drop(&mut self) {
        // SAFETY: DC de impressora possuído por este objeto.
        unsafe { DeleteDC(self.hdc) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::comdlg32::*;

    #[test]
    fn ranges_select_pages() {
        assert_eq!(PrintRange::All.pages(3), vec![0, 1, 2]);
        assert_eq!(PrintRange::Pages { first: 2, last: 3 }.pages(5), vec![1, 2]);
        assert_eq!(PrintRange::Pages { first: 4, last: 2 }.pages(5), vec![1, 2, 3], "extremos invertidos");
        assert_eq!(PrintRange::Pages { first: 3, last: 9 }.pages(4), vec![2, 3], "corta no fim do documento");
        assert!(PrintRange::Pages { first: 7, last: 9 }.pages(4).is_empty());
        assert!(PrintRange::Pages { first: 2, last: 2 }.contains(1));
        assert!(!PrintRange::Pages { first: 2, last: 2 }.contains(0));
        assert_eq!(PrintRange::Selection.pages(2), vec![0, 1]);
    }

    #[test]
    fn options_become_dialog_fields_and_back() {
        let (flags, from, to, min, max, copies) = PrintOptions::new().dialog_fields();
        assert!(flags & PD_NOPAGENUMS != 0 && flags & PD_NOSELECTION != 0);
        assert_eq!((from, to, min, max, copies), (1, 1, 1, 1, 1));
        let options = PrintOptions::new().page_count(12).range(PrintRange::Pages { first: 3, last: 20 }).copies(2);
        let (flags, from, to, min, max, copies) = options.dialog_fields();
        assert!(flags & PD_PAGENUMS != 0 && flags & PD_NOPAGENUMS == 0);
        assert_eq!((from, to, min, max, copies), (3, 12, 1, 12, 2));
        let (flags, ..) = PrintOptions::new().allow_selection(true).range(PrintRange::Selection).dialog_fields();
        assert!(flags & PD_SELECTION != 0 && flags & PD_NOSELECTION == 0);
        assert_eq!(PrintOptions::range_from_dialog(PD_PAGENUMS, 2, 5), PrintRange::Pages { first: 2, last: 5 });
        assert_eq!(PrintOptions::range_from_dialog(PD_SELECTION | PD_PAGENUMS, 2, 5), PrintRange::Selection);
        assert_eq!(PrintOptions::range_from_dialog(0, 2, 5), PrintRange::All);
    }
}
