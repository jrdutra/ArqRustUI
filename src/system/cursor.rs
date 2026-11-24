//! System/custom mouse cursors.

use crate::core::{GuiError, GuiResult};
use crate::ffi::user32::*;
use crate::ffi::{HCURSOR, PCWSTR, POINT};
use crate::utilities::{to_wide, Point};
use std::path::Path;
use std::rc::Rc;

/// Standard system cursors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SystemCursor {
    /// Arrow.
    Arrow,
    /// Text insertion I-beam.
    IBeam,
    /// Busy.
    Wait,
    /// Crosshair.
    Cross,
    /// Hand/link.
    Hand,
    /// Horizontal resize.
    SizeHorizontal,
    /// Vertical resize.
    SizeVertical,
    /// Diagonal resize ↘.
    SizeDiagonalDown,
    /// Diagonal resize ↗.
    SizeDiagonalUp,
    /// Move all directions.
    SizeAll,
    /// Forbidden.
    No,
    /// Arrow with background-work indicator.
    AppStarting,
    /// Arrow with question mark.
    Help,
}

#[derive(Debug)]
struct CursorInner {
    handle: HCURSOR,
    owned: bool,
}

impl Drop for CursorInner {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: cursores próprios vêm de LoadCursorFromFileW e pertencem a este objeto.
            unsafe { DestroyCursor(self.handle) };
        }
    }
}

/// Mouse cursor, cheaply cloned; file cursors released with last clone.
#[derive(Clone, Debug)]
pub struct Cursor(Rc<CursorInner>);

impl Cursor {
    /// Gets system cursor.
    pub fn system(cursor: SystemCursor) -> GuiResult<Cursor> {
        let id = match cursor {
            SystemCursor::Arrow => IDC_ARROW,
            SystemCursor::IBeam => IDC_IBEAM,
            SystemCursor::Wait => IDC_WAIT,
            SystemCursor::Cross => IDC_CROSS,
            SystemCursor::Hand => IDC_HAND,
            SystemCursor::SizeHorizontal => IDC_SIZEWE,
            SystemCursor::SizeVertical => IDC_SIZENS,
            SystemCursor::SizeDiagonalDown => IDC_SIZENWSE,
            SystemCursor::SizeDiagonalUp => IDC_SIZENESW,
            SystemCursor::SizeAll => IDC_SIZEALL,
            SystemCursor::No => IDC_NO,
            SystemCursor::AppStarting => IDC_APPSTARTING,
            SystemCursor::Help => IDC_HELP,
        };
        // SAFETY: com instância nula, o "nome" é o identificador inteiro de um cursor de sistema.
        let handle = unsafe { LoadCursorW(std::ptr::null_mut(), id as PCWSTR) };
        if handle.is_null() {
            return Err(GuiError::last_os_error("LoadCursorW"));
        }
        Ok(Cursor(Rc::new(CursorInner { handle, owned: false })))
    }

    /// Loads `.cur`/`.ani` cursor.
    pub fn from_file(path: impl AsRef<Path>) -> GuiResult<Cursor> {
        let wide = to_wide(&path.as_ref().to_string_lossy());
        // SAFETY: string válida terminada em NUL.
        let handle = unsafe { LoadCursorFromFileW(wide.as_ptr()) };
        if handle.is_null() {
            return Err(GuiError::last_os_error("LoadCursorFromFileW"));
        }
        Ok(Cursor(Rc::new(CursorInner { handle, owned: true })))
    }

    pub(crate) fn raw(&self) -> HCURSOR {
        self.0.handle
    }

    /// Current cursor screen pixel position.
    pub fn position() -> Point {
        let mut point = POINT::default();
        // SAFETY: ponteiro de saída válido.
        unsafe { GetCursorPos(&mut point) };
        Point::new(point.x, point.y)
    }
}
