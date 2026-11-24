//! DPI-independent font descriptions.

use crate::core::handles::GdiObject;
use crate::core::native;
use crate::core::{GuiError, GuiResult};
use crate::ffi::gdi32::*;
use crate::ffi::user32::{SystemParametersInfoW, NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS};
use crate::utilities::strings::{copy_to_buffer, from_wide};
use crate::utilities::to_wide;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::{Rc, Weak};

/// Font family, point size and style.
///
/// Native `HFONT` is created on demand per window DPI and shared by controls with identical descriptions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Font {
    family: String,
    size_tenths: u32,
    bold: bool,
    italic: bool,
    underline: bool,
    strikeout: bool,
}

thread_local! {
    static CACHE: RefCell<HashMap<(Font, u32), Weak<GdiObject>>> = RefCell::new(HashMap::new());
    static SYSTEM_FONT: RefCell<Option<Font>> = const { RefCell::new(None) };
}

impl Font {
    /// Creates a font with the given family/point size.
    pub fn new(family: &str, size_pt: f32) -> Font {
        Font {
            family: family.to_string(),
            size_tenths: (size_pt.clamp(1.0, 1000.0) * 10.0).round() as u32,
            bold: false,
            italic: false,
            underline: false,
            strikeout: false,
        }
    }

    /// System UI font, usually Segoe UI 9 pt, multiplied by accessibility text scale.
    pub fn system() -> Font {
        if let Some(font) = SYSTEM_FONT.with(|f| f.borrow().clone()) {
            return font;
        }
        let font = query_system_font().unwrap_or_else(|| Font::new("Segoe UI", 9.0));
        let scale = crate::system::accessibility::text_scale_factor();
        let font = font.clone().with_size(font.size() * scale);
        SYSTEM_FONT.with(|f| *f.borrow_mut() = Some(font.clone()));
        font
    }

    /// Bold variant.
    pub fn bold(mut self) -> Font {
        self.bold = true;
        self
    }

    /// Italic variant.
    pub fn italic(mut self) -> Font {
        self.italic = true;
        self
    }

    /// Underlined variant.
    pub fn underline(mut self) -> Font {
        self.underline = true;
        self
    }

    /// Strikethrough variant.
    pub fn strikeout(mut self) -> Font {
        self.strikeout = true;
        self
    }

    /// Same font with another point size.
    pub fn with_size(mut self, size_pt: f32) -> Font {
        self.size_tenths = (size_pt.clamp(1.0, 1000.0) * 10.0).round() as u32;
        self
    }

    /// Same font with size multiplied by `factor` (text scaling).
    pub fn scaled(self, factor: f32) -> Font {
        let size = self.size() * factor;
        self.with_size(size)
    }

    /// Family name.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Size in points.
    pub fn size(&self) -> f32 {
        self.size_tenths as f32 / 10.0
    }

    /// Whether bold.
    pub fn is_bold(&self) -> bool {
        self.bold
    }

    /// Whether italic.
    pub fn is_italic(&self) -> bool {
        self.italic
    }

    /// Whether underlined.
    pub fn is_underline(&self) -> bool {
        self.underline
    }

    /// Whether struck through.
    pub fn is_strikeout(&self) -> bool {
        self.strikeout
    }

    /// Registers `.ttf`/`.otf` for this process only.
    ///
    /// After registration, use the family name in [`Font::new`]. Returns fonts added.
    pub fn load_file(path: impl AsRef<Path>) -> GuiResult<u32> {
        let wide = to_wide(&path.as_ref().to_string_lossy());
        // SAFETY: string válida terminada em NUL; o último parâmetro é reservado.
        let added = unsafe { AddFontResourceExW(wide.as_ptr(), FR_PRIVATE, std::ptr::null_mut()) };
        if added <= 0 {
            Err(GuiError::InvalidArgument(format!("could not load the font {}", path.as_ref().display())))
        } else {
            Ok(added as u32)
        }
    }

    pub(crate) fn from_logfont(lf: &LOGFONTW, dpi: u32) -> Font {
        let pixels = lf.lfHeight.unsigned_abs().max(1);
        Font {
            family: from_wide(&lf.lfFaceName),
            size_tenths: (pixels * 720 + dpi / 2) / dpi.max(1),
            bold: lf.lfWeight >= 600,
            italic: lf.lfItalic != 0,
            underline: lf.lfUnderline != 0,
            strikeout: lf.lfStrikeOut != 0,
        }
    }

    pub(crate) fn to_logfont(&self, dpi: u32) -> LOGFONTW {
        let mut lf = LOGFONTW {
            lfHeight: -(((self.size_tenths as i64 * dpi as i64 + 360) / 720) as i32),
            lfWidth: 0,
            lfEscapement: 0,
            lfOrientation: 0,
            lfWeight: if self.bold { FW_BOLD } else { FW_NORMAL },
            lfItalic: self.italic as u8,
            lfUnderline: self.underline as u8,
            lfStrikeOut: self.strikeout as u8,
            lfCharSet: DEFAULT_CHARSET,
            lfOutPrecision: 0,
            lfClipPrecision: 0,
            lfQuality: CLEARTYPE_QUALITY,
            lfPitchAndFamily: 0,
            lfFaceName: [0; LF_FACESIZE],
        };
        copy_to_buffer(&self.family, &mut lf.lfFaceName);
        lf
    }

    /// Native font handle at given DPI, shared via cache.
    pub(crate) fn handle_for_dpi(&self, dpi: u32) -> GuiResult<Rc<GdiObject>> {
        let key = (self.clone(), dpi);
        if let Some(existing) = CACHE.with(|c| c.borrow().get(&key).and_then(Weak::upgrade)) {
            return Ok(existing);
        }
        let lf = self.to_logfont(dpi);
        // SAFETY: LOGFONTW completo; a posse do HFONT passa ao wrapper.
        let handle = unsafe { GdiObject::from_raw(CreateFontIndirectW(&lf)) }
            .ok_or_else(|| GuiError::last_os_error("CreateFontIndirectW"))?;
        let handle = Rc::new(handle);
        CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            cache.retain(|_, weak| weak.strong_count() > 0);
            cache.insert(key, Rc::downgrade(&handle));
        });
        Ok(handle)
    }
}

fn query_system_font() -> Option<Font> {
    // SAFETY: NONCLIENTMETRICSW é composto apenas por inteiros; zero é válido.
    let mut metrics: NONCLIENTMETRICSW = unsafe { std::mem::zeroed() };
    metrics.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
    // SAFETY: buffer do tamanho informado em cbSize.
    let ok =
        unsafe { SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, metrics.cbSize, &mut metrics as *mut _ as *mut _, 0) };
    if ok == 0 {
        return None;
    }
    Some(Font::from_logfont(&metrics.lfMessageFont, native::system_dpi()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_style_flags() {
        let font = Font::new("Consolas", 11.5).bold().italic();
        assert_eq!(font.family(), "Consolas");
        assert!((font.size() - 11.5).abs() < 0.01);
        assert!(font.is_bold() && font.is_italic() && !font.is_underline());
        assert!((font.clone().scaled(2.0).size() - 23.0).abs() < 0.01);
    }

    #[test]
    fn logfont_round_trip_scales_with_dpi() {
        let font = Font::new("Segoe UI", 9.0).underline();
        let at96 = font.to_logfont(96);
        let at192 = font.to_logfont(192);
        assert_eq!(at96.lfHeight, -12);
        assert_eq!(at192.lfHeight, -24);
        assert_eq!(Font::from_logfont(&at96, 96), font);
        assert_eq!(Font::from_logfont(&at192, 192), font);
    }

    #[test]
    fn handles_are_cached_per_dpi() {
        let font = Font::new("Arial", 10.0);
        let a = font.handle_for_dpi(96).unwrap();
        let b = font.handle_for_dpi(96).unwrap();
        let c = font.handle_for_dpi(144).unwrap();
        assert!(Rc::ptr_eq(&a, &b));
        assert!(!Rc::ptr_eq(&a, &c));
        assert!(Font::load_file("Z:\\inexistente.ttf").is_err());
    }
}
