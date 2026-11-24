//! Classic common dialogs: color, font, printing and page setup.

use crate::core::application as app;
use crate::core::native;
use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult};
use crate::ffi::comdlg32::*;
use crate::ffi::kernel32::GlobalFree;
use crate::ffi::HWND;
use crate::graphics::Font;
use crate::system::{PrintOptions, PrinterContext};
use crate::utilities::{Color, Thickness};
use std::cell::RefCell;

/// Converts common dialog `FALSE` into cancellation or error.
fn cancelled_or_error<T>(context: &'static str) -> GuiResult<Option<T>> {
    // SAFETY: consulta o código de erro estendido do último diálogo comum desta thread.
    match unsafe { CommDlgExtendedError() } {
        0 => Ok(None),
        code => Err(GuiError::from_win32(context, code)),
    }
}

thread_local! {
    /// Custom color dialog colors retained between calls.
    static CUSTOM_COLORS: RefCell<[u32; 16]> = const { RefCell::new([0x00FF_FFFF; 16]) };
}

/// Color selection dialog.
#[derive(Clone, Copy, Debug)]
pub struct ColorDialog {
    initial: Color,
    full_open: bool,
}

impl Default for ColorDialog {
    fn default() -> Self {
        Self { initial: Color::BLACK, full_open: true }
    }
}

impl ColorDialog {
    /// Creates the dialog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initially selected color.
    pub fn initial(mut self, color: Color) -> Self {
        self.initial = color;
        self
    }

    /// Initially expands custom color selection (default `true`).
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.full_open = expanded;
        self
    }

    /// Shows dialog; `Ok(None)` on cancellation.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<Option<Color>> {
        app::require()?;
        CUSTOM_COLORS.with(|colors| {
            let mut colors = colors.borrow_mut();
            let mut config = CHOOSECOLORW {
                lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
                hwndOwner: owner.core().hwnd(),
                hInstance: std::ptr::null_mut(),
                rgbResult: self.initial.to_colorref(),
                lpCustColors: colors.as_mut_ptr(),
                Flags: CC_RGBINIT | CC_ANYCOLOR | if self.full_open { CC_FULLOPEN } else { 0 },
                lCustData: 0,
                lpfnHook: std::ptr::null(),
                lpTemplateName: std::ptr::null(),
            };
            // SAFETY: estrutura completa; `lpCustColors` aponta para 16 valores válidos.
            if unsafe { ChooseColorW(&mut config) } == 0 {
                return cancelled_or_error("ChooseColorW");
            }
            Ok(Some(Color::from_colorref(config.rgbResult)))
        })
    }
}

/// Font dialog result.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FontChoice {
    /// Selected font.
    pub font: Font,
    /// Selected color.
    pub color: Color,
}

/// Font selection dialog.
#[derive(Clone, Debug, Default)]
pub struct FontDialog {
    initial: Option<Font>,
    color: Option<Color>,
}

impl FontDialog {
    /// Creates the dialog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initially selected font.
    pub fn initial(mut self, font: &Font) -> Self {
        self.initial = Some(font.clone());
        self
    }

    /// Initial text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Shows dialog; `Ok(None)` on cancellation.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<Option<FontChoice>> {
        app::require()?;
        let hwnd: HWND = owner.core().hwnd();
        // O diálogo trabalha em pixels do DPI do sistema.
        let dpi = native::system_dpi();
        let mut logfont = self.initial.clone().unwrap_or_else(Font::system).to_logfont(dpi);
        let mut config = CHOOSEFONTW {
            lStructSize: std::mem::size_of::<CHOOSEFONTW>() as u32,
            hwndOwner: hwnd,
            hDC: std::ptr::null_mut(),
            lpLogFont: &mut logfont,
            iPointSize: 0,
            Flags: CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_EFFECTS,
            rgbColors: self.color.unwrap_or(Color::BLACK).to_colorref(),
            lCustData: 0,
            lpfnHook: std::ptr::null(),
            lpTemplateName: std::ptr::null(),
            hInstance: std::ptr::null_mut(),
            lpszStyle: std::ptr::null_mut(),
            nFontType: 0,
            missing_alignment: 0,
            nSizeMin: 0,
            nSizeMax: 0,
        };
        // SAFETY: estrutura completa; `lpLogFont` aponta para um LOGFONTW local válido.
        if unsafe { ChooseFontW(&mut config) } == 0 {
            return cancelled_or_error("ChooseFontW");
        }
        Ok(Some(FontChoice { font: Font::from_logfont(&logfont, dpi), color: Color::from_colorref(config.rgbColors) }))
    }
}

/// Print dialog: selects printer/options.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrintDialog;

impl PrintDialog {
    /// Creates the dialog.
    pub fn new() -> Self {
        PrintDialog
    }

    /// Shows dialog, returning selected printer context or `Ok(None)` on cancellation.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<Option<PrinterContext>> {
        self.show_with(owner, &PrintOptions::new())
    }

    /// Shows multipage document dialog with From/To range and optional selection. Chosen range is in
    /// [`PrinterContext::range`] and respected by [`PrinterContext::print_document`].
    ///
    /// ```no_run
    /// # use arqrustui::prelude::*;
    /// # fn demo(window: &Window, doc: &PrintDocument) -> GuiResult<()> {
    /// let options = PrintOptions::new().page_count(doc.page_count() as u32);
    /// if let Some(printer) = PrintDialog::new().show_with(window, &options)? {
    ///     printer.print_document(doc)?;
    /// }
    /// # Ok(()) }
    /// ```
    pub fn show_with(&self, owner: &impl Widget, options: &PrintOptions) -> GuiResult<Option<PrinterContext>> {
        app::require()?;
        // SAFETY: PRINTDLGW contém apenas inteiros e ponteiros; zero é um estado válido.
        let mut config: PRINTDLGW = unsafe { std::mem::zeroed() };
        config.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
        config.hwndOwner = owner.core().hwnd();
        let (flags, from, to, min, max, copies) = options.dialog_fields();
        config.Flags = flags;
        (config.nFromPage, config.nToPage, config.nMinPage, config.nMaxPage, config.nCopies) =
            (from, to, min, max, copies);
        // SAFETY: estrutura corretamente dimensionada; os blocos globais devolvidos
        // (DEVMODE/DEVNAMES) são liberados aqui e o DC passa ao PrinterContext.
        unsafe {
            let accepted = PrintDlgW(&mut config) != 0;
            let (mode, names, hdc) = (config.hDevMode, config.hDevNames, config.hDC);
            if !mode.is_null() {
                GlobalFree(mode);
            }
            if !names.is_null() {
                GlobalFree(names);
            }
            if !accepted {
                return cancelled_or_error("PrintDlgW");
            }
            if hdc.is_null() {
                return Err(GuiError::Unsupported("the selected printer did not provide a device context"));
            }
            let range = PrintOptions::range_from_dialog(config.Flags, config.nFromPage, config.nToPage);
            Ok(Some(PrinterContext::from_raw(hdc).with_choice(range, config.nCopies)))
        }
    }
}

/// User-selected page setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageSetup {
    /// Paper width in hundredths of a millimeter.
    pub paper_width: i32,
    /// Paper height in hundredths of a millimeter.
    pub paper_height: i32,
    /// Margins in hundredths of a millimeter.
    pub margins: Thickness,
}

/// Page setup dialog.
#[derive(Clone, Copy, Debug, Default)]
pub struct PageSetupDialog {
    margins: Option<Thickness>,
}

impl PageSetupDialog {
    /// Creates the dialog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initial margins in hundredths of a millimeter.
    pub fn margins(mut self, margins: Thickness) -> Self {
        self.margins = Some(margins);
        self
    }

    /// Shows dialog; `Ok(None)` on cancellation.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<Option<PageSetup>> {
        app::require()?;
        // SAFETY: PAGESETUPDLGW contém apenas inteiros e ponteiros; zero é um estado válido.
        let mut config: PAGESETUPDLGW = unsafe { std::mem::zeroed() };
        config.lStructSize = std::mem::size_of::<PAGESETUPDLGW>() as u32;
        config.hwndOwner = owner.core().hwnd();
        config.Flags = PSD_INHUNDREDTHSOFMILLIMETERS;
        if let Some(margins) = self.margins {
            config.Flags |= PSD_MARGINS;
            config.rtMargin =
                crate::ffi::RECT { left: margins.left, top: margins.top, right: margins.right, bottom: margins.bottom };
        }
        // SAFETY: estrutura corretamente dimensionada; os blocos globais devolvidos são liberados aqui.
        unsafe {
            let accepted = PageSetupDlgW(&mut config) != 0;
            if !config.hDevMode.is_null() {
                GlobalFree(config.hDevMode);
            }
            if !config.hDevNames.is_null() {
                GlobalFree(config.hDevNames);
            }
            if !accepted {
                return cancelled_or_error("PageSetupDlgW");
            }
        }
        let m = config.rtMargin;
        Ok(Some(PageSetup {
            paper_width: config.ptPaperSize.x,
            paper_height: config.ptPaperSize.y,
            margins: Thickness::new(m.left, m.top, m.right, m.bottom),
        }))
    }
}
