//! Rich text editor (RichEdit 4.1+, `Msftedit.dll`).

use super::common::{builder_common, simple_event, widget_type, ControlOptions};
use super::textbox::edit_methods;
use crate::core::native;
use crate::core::widget::Reflected;
use crate::core::{Event, GuiError, GuiResult};
use crate::ffi::comctl32::*;
use crate::ffi::dynamic::Library;
use crate::ffi::user32::*;
use crate::ffi::HWND;
use crate::graphics::TextAlign;
use crate::utilities::strings::copy_to_buffer;
use crate::utilities::Color;
use std::sync::OnceLock;

/// Character formatting for a [`RichTextBox`] selection.
///
/// `None` fields remain unchanged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextFormat {
    /// Bold.
    pub bold: Option<bool>,
    /// Italic.
    pub italic: Option<bool>,
    /// Underline.
    pub underline: Option<bool>,
    /// Strikethrough.
    pub strikeout: Option<bool>,
    /// Text color.
    pub color: Option<Color>,
    /// Text background/highlight color.
    pub highlight: Option<Color>,
    /// Size in points.
    pub size: Option<f32>,
    /// Font family.
    pub family: Option<String>,
}

impl TextFormat {
    /// Empty formatting (changes nothing).
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables/disables bold.
    pub fn bold(mut self, on: bool) -> Self {
        self.bold = Some(on);
        self
    }

    /// Enables/disables italic.
    pub fn italic(mut self, on: bool) -> Self {
        self.italic = Some(on);
        self
    }

    /// Enables/disables underline.
    pub fn underline(mut self, on: bool) -> Self {
        self.underline = Some(on);
        self
    }

    /// Sets text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets text background color.
    pub fn highlight(mut self, color: Color) -> Self {
        self.highlight = Some(color);
        self
    }

    /// Sets size in points.
    pub fn size(mut self, points: f32) -> Self {
        self.size = Some(points);
        self
    }

    /// Sets font family.
    pub fn family(mut self, family: &str) -> Self {
        self.family = Some(family.to_string());
        self
    }

    fn to_native(&self) -> CHARFORMAT2W {
        // SAFETY: CHARFORMAT2W contém apenas inteiros; zero é um valor válido.
        let mut cf: CHARFORMAT2W = unsafe { std::mem::zeroed() };
        cf.cbSize = std::mem::size_of::<CHARFORMAT2W>() as u32;
        let mut mask = 0;
        let mut effects = 0;
        for (flag, bit) in [
            (self.bold, CFM_BOLD),
            (self.italic, CFM_ITALIC),
            (self.underline, CFM_UNDERLINE),
            (self.strikeout, CFM_STRIKEOUT),
        ] {
            if let Some(on) = flag {
                mask |= bit;
                if on {
                    effects |= bit;
                }
            }
        }
        if let Some(color) = self.color {
            mask |= CFM_COLOR;
            cf.crTextColor = color.to_colorref();
        }
        if let Some(color) = self.highlight {
            mask |= CFM_BACKCOLOR;
            cf.crBackColor = color.to_colorref();
        }
        if let Some(size) = self.size {
            mask |= CFM_SIZE;
            // yHeight é medido em twips (1/20 de ponto).
            cf.yHeight = (size * 20.0).round() as i32;
        }
        if let Some(family) = &self.family {
            mask |= CFM_FACE;
            let mut face = [0u16; 32];
            copy_to_buffer(family, &mut face);
            cf.szFaceName = face;
        }
        cf.dwMask = mask;
        cf.dwEffects = effects;
        cf
    }
}

/// Sets default RichEdit text color (used by themes).
pub(crate) fn set_default_text_color(hwnd: HWND, color: Color) {
    let format = TextFormat::new().color(color).to_native();
    native::send_ref(hwnd, EM_SETCHARFORMAT, SCF_ALL, &format);
}

fn ensure_library() -> GuiResult<()> {
    static LOADED: OnceLock<bool> = OnceLock::new();
    if *LOADED.get_or_init(|| Library::load("Msftedit.dll").is_some()) {
        Ok(())
    } else {
        Err(GuiError::Unsupported("Msftedit.dll (RichEdit) is not available"))
    }
}

struct StreamCursor {
    data: Vec<u8>,
    position: usize,
}

unsafe extern "system" fn stream_out(cookie: usize, buffer: *mut u8, count: i32, written: *mut i32) -> u32 {
    crate::core::ffi_guard::guard(1, move || {
        // SAFETY: `cookie` é o ponteiro para o StreamCursor local de `rtf()`, válido durante
        // EM_STREAMOUT; `buffer` tem `count` bytes válidos, conforme o contrato do callback.
        let cursor = &mut *(cookie as *mut StreamCursor);
        cursor.data.extend_from_slice(std::slice::from_raw_parts(buffer, count.max(0) as usize));
        *written = count;
        0
    })
}

unsafe extern "system" fn stream_in(cookie: usize, buffer: *mut u8, count: i32, read: *mut i32) -> u32 {
    crate::core::ffi_guard::guard(1, move || {
        // SAFETY: `cookie` é o ponteiro para o StreamCursor local de `set_rtf()`, válido
        // durante EM_STREAMIN; `buffer` comporta `count` bytes.
        let cursor = &mut *(cookie as *mut StreamCursor);
        let remaining = &cursor.data[cursor.position..];
        let amount = remaining.len().min(count.max(0) as usize);
        std::ptr::copy_nonoverlapping(remaining.as_ptr(), buffer, amount);
        cursor.position += amount;
        *read = amount as i32;
        0
    })
}

widget_type!(
    /// Text editor with formatting (bold, colors, fonts) and RTF support.
    RichTextBox
);

/// Builder for [`RichTextBox`].
#[derive(Clone, Debug)]
pub struct RichTextBoxBuilder {
    opts: ControlOptions,
    read_only: bool,
    word_wrap: bool,
}

impl RichTextBoxBuilder {
    builder_common!();

    /// Prevents user editing.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Word wrap (default: `true`).
    pub fn word_wrap(mut self, word_wrap: bool) -> Self {
        self.word_wrap = word_wrap;
        self
    }

    /// Creates the editor.
    pub fn build(self) -> GuiResult<RichTextBox> {
        ensure_library()?;
        let mut style = ES_MULTILINE | ES_WANTRETURN | ES_AUTOVSCROLL | ES_NOHIDESEL | WS_VSCROLL;
        if !self.word_wrap {
            style |= ES_AUTOHSCROLL | WS_HSCROLL;
        }
        if self.read_only {
            style |= ES_READONLY;
        }
        let core = self.opts.create("RichTextBox", "RICHEDIT50W", style, WS_EX_CLIENTEDGE, true, None)?;
        let hwnd = core.hwnd();
        native::send(hwnd, EM_SETEVENTMASK, 0, ENM_CHANGE | ENM_SELCHANGE);
        native::send(hwnd, EM_EXLIMITTEXT, 0, i32::MAX as isize);
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Command { code, .. } if *code == EN_CHANGE => {
                core.emit(&Event::TextChanged);
                Some(0)
            }
            Reflected::Notify { code, .. } if *code == EN_SELCHANGE => {
                core.emit(&Event::SelectionChanged);
                Some(0)
            }
            _ => None,
        });
        Ok(RichTextBox { core })
    }
}

/// [`RichTextBox`] paragraph list style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ListStyle {
    /// Normal paragraph.
    #[default]
    None,
    /// Bullets (•).
    Bullet,
    /// 1. 2. 3. … starting at the given number.
    Numbered(u16),
    /// a. b. c.
    LowerLetter,
    /// A. B. C.
    UpperLetter,
    /// i. ii. iii.
    LowerRoman,
    /// I. II. III.
    UpperRoman,
}

/// RTF table (`\trowd … \row`) with cells and column widths in twips.
pub(crate) fn table_rtf(rows: &[Vec<String>], widths_twips: &[i32], header: bool) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0).max(widths_twips.len()).max(1);
    let mut rtf = String::from("{\\rtf1\\ansi\\deff0");
    for (index, row) in rows.iter().enumerate() {
        rtf.push_str("\\trowd\\trgaph108\\trleft0");
        let mut right = 0;
        for column in 0..columns {
            right += widths_twips.get(column).copied().unwrap_or(1800).max(200);
            rtf.push_str(&format!(
                "\\clbrdrt\\brdrs\\brdrw10\\clbrdrl\\brdrs\\brdrw10\\clbrdrb\\brdrs\\brdrw10\\clbrdrr\\brdrs\\brdrw10\\cellx{right}"
            ));
        }
        for column in 0..columns {
            let text = row.get(column).map(String::as_str).unwrap_or("");
            let bold = header && index == 0;
            rtf.push_str("\\pard\\intbl ");
            if bold {
                rtf.push_str("{\\b ");
            }
            rtf.push_str(&crate::controls::styled_text::escape_rtf(text));
            if bold {
                rtf.push('}');
            }
            rtf.push_str("\\cell");
        }
        rtf.push_str("\\row");
    }
    rtf.push_str("\\pard}");
    rtf
}

/// RTF image (`\pict\emfblip`) from EMF bytes with displayed size in twips.
pub(crate) fn picture_rtf(emf: &[u8], width_twips: i32, height_twips: i32, frame: (i32, i32)) -> String {
    let mut rtf = format!(
        "{{\\rtf1{{\\pict\\emfblip\\picw{}\\pich{}\\picwgoal{width_twips}\\pichgoal{height_twips} ",
        frame.0.max(1),
        frame.1.max(1)
    );
    rtf.reserve(emf.len() * 2 + emf.len() / 32 + 8);
    for (index, byte) in emf.iter().enumerate() {
        if index > 0 && index % 64 == 0 {
            rtf.push('\n');
        }
        rtf.push_str(&format!("{byte:02x}"));
    }
    rtf.push_str("}}");
    rtf
}

impl RichTextBox {
    /// Starts building (default size 320×200).
    pub fn builder() -> RichTextBoxBuilder {
        RichTextBoxBuilder { opts: ControlOptions::new(320, 200), read_only: false, word_wrap: true }
    }

    edit_methods!();

    simple_event!(
        /// Caret/selection position changed.
        on_selection_changed,
        SelectionChanged
    );

    /// Applies formatting to selection or insertion point.
    pub fn format_selection(&self, format: &TextFormat) {
        native::send_ref(self.core.hwnd(), EM_SETCHARFORMAT, SCF_SELECTION, &format.to_native());
    }

    /// Applies formatting to the entire document.
    pub fn format_all(&self, format: &TextFormat) {
        native::send_ref(self.core.hwnd(), EM_SETCHARFORMAT, SCF_ALL, &format.to_native());
    }

    /// Whether selection is bold, italic and underlined, respectively.
    pub fn selection_style(&self) -> (bool, bool, bool) {
        // SAFETY: CHARFORMAT2W contém apenas inteiros; zero é um valor válido.
        let mut cf: CHARFORMAT2W = unsafe { std::mem::zeroed() };
        cf.cbSize = std::mem::size_of::<CHARFORMAT2W>() as u32;
        native::send_mut(self.core.hwnd(), EM_GETCHARFORMAT, SCF_SELECTION, &mut cf);
        let effects = cf.dwEffects;
        (effects & CFM_BOLD != 0, effects & CFM_ITALIC != 0, effects & CFM_UNDERLINE != 0)
    }

    /// Sets selected paragraph alignment.
    pub fn set_alignment(&self, align: TextAlign) {
        // SAFETY: PARAFORMAT contém apenas inteiros; zero é um valor válido.
        let mut pf: PARAFORMAT = unsafe { std::mem::zeroed() };
        pf.cbSize = std::mem::size_of::<PARAFORMAT>() as u32;
        pf.dwMask = PFM_ALIGNMENT;
        pf.wAlignment = match align {
            TextAlign::Left => 1,
            TextAlign::Right => 2,
            TextAlign::Center => 3,
        };
        native::send_ref(self.core.hwnd(), EM_SETPARAFORMAT, 0, &pf);
    }

    fn para_format(&self) -> PARAFORMAT2 {
        // SAFETY: PARAFORMAT2 contém apenas inteiros; zero é um valor válido.
        let mut pf: PARAFORMAT2 = unsafe { std::mem::zeroed() };
        pf.base.cbSize = std::mem::size_of::<PARAFORMAT2>() as u32;
        pf
    }

    /// Converts selected paragraphs to a list or back to normal paragraphs.
    pub fn set_list_style(&self, style: ListStyle) {
        let mut pf = self.para_format();
        pf.base.dwMask =
            PFM_NUMBERING | PFM_NUMBERINGSTYLE | PFM_NUMBERINGSTART | PFM_NUMBERINGTAB | PFM_OFFSET | PFM_STARTINDENT;
        let (numbering, start) = match style {
            ListStyle::None => (0, 1),
            ListStyle::Bullet => (PFN_BULLET, 1),
            ListStyle::Numbered(start) => (PFN_ARABIC, start.max(1)),
            ListStyle::LowerLetter => (PFN_LCLETTER, 1),
            ListStyle::UpperLetter => (PFN_UCLETTER, 1),
            ListStyle::LowerRoman => (PFN_LCROMAN, 1),
            ListStyle::UpperRoman => (PFN_UCROMAN, 1),
        };
        pf.base.wNumbering = numbering;
        pf.wNumberingStart = start;
        pf.wNumberingStyle = if numbering > PFN_BULLET { PFNS_PERIOD } else { 0 };
        // Recuo de 0,25" para o número/marcador e o texto alinhado depois dele.
        let indent = if numbering == 0 { 0 } else { 360 };
        pf.wNumberingTab = indent as u16;
        pf.base.dxOffset = indent;
        pf.base.dxStartIndent = indent;
        native::send_ref(self.core.hwnd(), EM_SETPARAFORMAT, 0, &pf);
    }

    /// List style at the caret's paragraph.
    pub fn list_style(&self) -> ListStyle {
        let mut pf = self.para_format();
        native::send_mut(self.core.hwnd(), EM_GETPARAFORMAT, 0, &mut pf);
        let (numbering, start) = (pf.base.wNumbering, pf.wNumberingStart);
        match numbering {
            PFN_BULLET => ListStyle::Bullet,
            PFN_ARABIC => ListStyle::Numbered(start.max(1)),
            PFN_LCLETTER => ListStyle::LowerLetter,
            PFN_UCLETTER => ListStyle::UpperLetter,
            PFN_LCROMAN => ListStyle::LowerRoman,
            PFN_UCROMAN => ListStyle::UpperRoman,
            _ => ListStyle::None,
        }
    }

    /// Replaces selection or inserts a complete RTF fragment (`{\rtf1 …}`) at the caret, preserving the rest.
    pub fn insert_rtf(&self, rtf: &str) -> GuiResult<()> {
        self.core.require_alive()?;
        let mut cursor = StreamCursor { data: rtf.as_bytes().to_vec(), position: 0 };
        let mut stream = EDITSTREAM {
            dwCookie: &mut cursor as *mut StreamCursor as usize,
            dwError: 0,
            pfnCallback: Some(stream_in),
        };
        native::send_mut(self.core.hwnd(), EM_STREAMIN, SF_RTF | SFF_SELECTION, &mut stream);
        let error = stream.dwError;
        if error != 0 {
            return Err(GuiError::InvalidArgument(format!("invalid RTF (error {error})")));
        }
        Ok(())
    }

    /// Inserts a table: one row per `Vec`, logical column widths (120 for unspecified widths). `header` makes the first
    /// row bold.
    pub fn insert_table(&self, rows: &[Vec<String>], column_widths: &[i32], header: bool) -> GuiResult<()> {
        if rows.is_empty() {
            return Err(GuiError::InvalidArgument("the table needs at least one row".into()));
        }
        // 96 unidades lógicas = 1 polegada = 1440 twips.
        let twips: Vec<i32> = column_widths.iter().map(|w| w.max(&10) * 15).collect();
        let twips = if twips.is_empty() { vec![1800] } else { twips };
        self.insert_rtf(&table_rtf(rows, &twips, header))
    }

    /// Inserts an image at the given logical size (embedded in RTF as EMF and saved with the document).
    pub fn insert_image(&self, image: &crate::graphics::PlusImage, width: i32, height: i32) -> GuiResult<()> {
        let size = image.size();
        let pixels = image.to_pixels()?;
        // Compõe sobre branco: o GDI do metarquivo não leva o alfa.
        let opaque: Vec<u8> = pixels
            .chunks_exact(4)
            .flat_map(|p| {
                let a = p[3] as u32;
                let blend = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
                [blend(p[0]), blend(p[1]), blend(p[2]), 255]
            })
            .collect();
        let metafile = crate::system::Metafile::record(size.width, size.height, |canvas| {
            let header = crate::ffi::gdi32::BITMAPINFOHEADER {
                biSize: std::mem::size_of::<crate::ffi::gdi32::BITMAPINFOHEADER>() as u32,
                biWidth: size.width,
                biHeight: -size.height,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            };
            // SAFETY: DC do metarquivo vivo; o cabeçalho descreve exatamente `opaque`.
            unsafe {
                crate::ffi::gdi32::StretchDIBits(
                    canvas.hdc(),
                    0,
                    0,
                    size.width,
                    size.height,
                    0,
                    0,
                    size.width,
                    size.height,
                    opaque.as_ptr() as *const std::ffi::c_void,
                    &header,
                    crate::ffi::gdi32::DIB_RGB_COLORS,
                    crate::ffi::gdi32::SRCCOPY,
                );
            }
        })?;
        let frame = metafile.frame_hundredths_mm();
        self.insert_rtf(&picture_rtf(
            metafile.bytes(),
            width.max(1) * 15,
            height.max(1) * 15,
            (frame.width, frame.height),
        ))
    }

    /// Document RTF content.
    pub fn rtf(&self) -> String {
        let mut cursor = StreamCursor { data: Vec::new(), position: 0 };
        let mut stream = EDITSTREAM {
            dwCookie: &mut cursor as *mut StreamCursor as usize,
            dwError: 0,
            pfnCallback: Some(stream_out),
        };
        native::send_mut(self.core.hwnd(), EM_STREAMOUT, SF_RTF, &mut stream);
        String::from_utf8_lossy(&cursor.data).into_owned()
    }

    /// Replaces document with given RTF.
    pub fn set_rtf(&self, rtf: &str) -> GuiResult<()> {
        self.core.require_alive()?;
        let mut cursor = StreamCursor { data: rtf.as_bytes().to_vec(), position: 0 };
        let mut stream = EDITSTREAM {
            dwCookie: &mut cursor as *mut StreamCursor as usize,
            dwError: 0,
            pfnCallback: Some(stream_in),
        };
        native::send_mut(self.core.hwnd(), EM_STREAMIN, SF_RTF, &mut stream);
        let error = stream.dwError;
        if error != 0 {
            return Err(GuiError::InvalidArgument(format!("invalid RTF (error {error})")));
        }
        Ok(())
    }

    /// Finds `needle` starting at `from` in characters and selects the match. Returns its start position.
    pub fn find(&self, needle: &str, from: usize, match_case: bool) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        // O RichEdit conta cada quebra de parágrafo como um único caractere (CR).
        let text: Vec<u16> = crate::core::Widget::text(self).replace("\r\n", "\r").encode_utf16().collect();
        let needle: Vec<u16> = needle.encode_utf16().collect();
        let fold = |unit: u16| -> u16 {
            if match_case {
                unit
            } else {
                char::from_u32(unit as u32).map(|c| c.to_lowercase().next().unwrap_or(c) as u32 as u16).unwrap_or(unit)
            }
        };
        let haystack: Vec<u16> = text.iter().map(|&u| fold(u)).collect();
        let pattern: Vec<u16> = needle.iter().map(|&u| fold(u)).collect();
        if from >= haystack.len() || pattern.len() > haystack.len() {
            return None;
        }
        let found = haystack[from..].windows(pattern.len()).position(|w| w == pattern.as_slice())? + from;
        self.set_selection(found, found + pattern.len());
        Some(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_and_pictures_become_rtf() {
        let rtf = table_rtf(&[vec!["Nome".into(), "Preço".into()], vec!["Café".into()]], &[1500, 900], true);
        assert!(rtf.starts_with("{\\rtf1"));
        assert_eq!(rtf.matches("\\row").count(), 2);
        assert!(rtf.contains("\\cellx1500") && rtf.contains("\\cellx2400"));
        assert!(rtf.contains("{\\b Nome}\\cell"), "cabeçalho em negrito: {rtf}");
        assert!(rtf.contains("\\pard\\intbl \\cell\\row"), "célula que faltava fica vazia");
        let picture = picture_rtf(&[0x01, 0xAB, 0xFF], 1500, 750, (2646, 1323));
        assert!(picture.contains("\\pict\\emfblip\\picw2646\\pich1323\\picwgoal1500\\pichgoal750 01abff}"));
    }

    #[test]
    fn charformat_layout_and_masks() {
        assert_eq!(std::mem::size_of::<CHARFORMAT2W>(), 116);
        let cf = TextFormat::new()
            .bold(true)
            .italic(false)
            .color(Color::rgb(1, 2, 3))
            .size(12.0)
            .family("Arial")
            .to_native();
        let (mask, effects, height, color) = (cf.dwMask, cf.dwEffects, cf.yHeight, cf.crTextColor);
        assert_eq!(mask, CFM_BOLD | CFM_ITALIC | CFM_COLOR | CFM_SIZE | CFM_FACE);
        assert_eq!(effects, CFM_BOLD);
        assert_eq!(height, 240);
        assert_eq!(color, 0x0003_0201);
    }
}
