//! Batch visual formatting for editors of any language/format.
use super::richtext::{RichTextBox, TextFormat};
use crate::core::{native, GuiError, GuiResult, Widget};
use crate::ffi::{comctl32::EM_SETEVENTMASK, ole32::ComPtr, tom::*, user32::WM_SETREDRAW, GUID, POINT};
use std::{ffi::c_void, ops::Range};

type Undo = unsafe extern "system" fn(*mut c_void, i32, *mut i32) -> i32;
impl RichTextBox {
    /// Reapplies visual styles without replacing text or adding undo actions. Ranges use RichEdit UTF-16 positions
    /// (CRLF counts as one break). Preserves selection, scrolling and modified flag; emits no edit events. Applies
    /// `base` to all text, then spans in order. Invalid ranges return errors before any modification.
    pub fn apply_highlights(&self, base: &TextFormat, spans: &[(Range<usize>, TextFormat)]) -> GuiResult<()> {
        self.core.require_alive()?;
        let length = self.text().replace("\r\n", "\r").encode_utf16().count();
        if spans.iter().any(|(r, _)| r.start > r.end || r.end > length || r.end > i32::MAX as usize) {
            return Err(GuiError::InvalidArgument("formatting range is outside the text".into()));
        }
        let hwnd = self.core.hwnd();
        let mut raw: *mut c_void = std::ptr::null_mut();
        native::send_mut(hwnd, EM_GETOLEINTERFACE, 0, &mut raw);
        // SAFETY: EM_GETOLEINTERFACE entrega uma referência COM contada, ou nulo.
        let ole = unsafe { ComPtr::from_raw(raw) }.ok_or(GuiError::Unsupported("RichEdit without OLE interface"))?;
        let mut doc = std::ptr::null_mut();
        // SAFETY: entrada 0 é QueryInterface, com a assinatura COM e IID documentadas.
        let hr = unsafe {
            let query: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32 = ole.method(0);
            query(ole.as_ptr(), &IID_ITEXTDOCUMENT, &mut doc)
        };
        if hr < 0 {
            return Err(GuiError::HResult { context: "ITextDocument::QueryInterface", hresult: hr });
        }
        // SAFETY: QueryInterface entrega uma referência contada de ITextDocument.
        let doc = unsafe { ComPtr::from_raw(doc) }.ok_or(GuiError::Unsupported("RichEdit without ITextDocument"))?;
        // SAFETY: TOM.h define Undo como entrada 22 da vtable de ITextDocument.
        let undo: Undo = unsafe { doc.method(22) };
        let mut count = 0;
        // SAFETY: interface válida nesta thread; Count usa a constante TOM de suspensão.
        let hr = unsafe { undo(doc.as_ptr(), TOM_SUSPEND, &mut count) };
        if hr < 0 {
            return Err(GuiError::HResult { context: "ITextDocument::Undo(suspend)", hresult: hr });
        }
        let mut selection = CharRange::default();
        let mut scroll = POINT::default();
        native::send_mut(hwnd, EM_EXGETSEL, 0, &mut selection);
        native::send_mut(hwnd, EM_GETSCROLLPOS, 0, &mut scroll);
        let modified = self.is_modified();
        let mask = native::send(hwnd, EM_SETEVENTMASK, 0, 0);
        let visible = self.is_visible();
        if visible {
            native::send(hwnd, WM_SETREDRAW, 0, 0);
        }
        self.format_all(base);
        for (range, format) in spans {
            let r = CharRange { start: range.start as i32, end: range.end as i32 };
            native::send_ref(hwnd, EM_EXSETSEL, 0, &r);
            self.format_selection(format);
        }
        native::send_ref(hwnd, EM_EXSETSEL, 0, &selection);
        native::send_ref(hwnd, EM_SETSCROLLPOS, 0, &scroll);
        self.set_modified(modified);
        native::send(hwnd, EM_SETEVENTMASK, 0, mask);
        if visible {
            native::send(hwnd, WM_SETREDRAW, 1, 0);
        }
        // SAFETY: restaura o histórico da mesma interface após a operação síncrona.
        let hr = unsafe { undo(doc.as_ptr(), TOM_RESUME, &mut count) };
        self.invalidate();
        if hr < 0 {
            return Err(GuiError::HResult { context: "ITextDocument::Undo(resume)", hresult: hr });
        }
        Ok(())
    }
}
