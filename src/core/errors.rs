//! Library error types.

use crate::ffi::kernel32::{FormatMessageW, GetLastError, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS};
use crate::ffi::HRESULT;
use std::fmt;

/// Standard library operation result.
pub type GuiResult<T> = Result<T, GuiError>;

/// Errors returned by the library.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GuiError {
    /// Win32 function failed; `code` is `GetLastError`.
    Win32 {
        /// Operation being performed.
        context: &'static str,
        /// Windows error code.
        code: u32,
        /// System message for the code.
        message: String,
    },
    /// COM call returned a failure `HRESULT`.
    HResult {
        /// Operation being performed.
        context: &'static str,
        /// `HRESULT` value.
        hresult: i32,
    },
    /// GUI API used on a thread without an active [`Application`](crate::core::Application).
    NotInitialized,
    /// An `Application` already exists on this thread.
    AlreadyInitialized,
    /// Control/window already destroyed.
    WidgetDestroyed,
    /// Control builder missing `.parent(...)`.
    MissingParent,
    /// Invalid argument; message explains why.
    InvalidArgument(String),
    /// Feature unavailable on this Windows version or environment.
    Unsupported(&'static str),
    /// UI thread no longer exists, e.g. when posting a task.
    ApplicationGone,
}

impl GuiError {
    /// Captures `GetLastError()` and its system message.
    pub fn last_os_error(context: &'static str) -> Self {
        // SAFETY: `GetLastError` não tem pré-condições.
        let code = unsafe { GetLastError() };
        Self::from_win32(context, code)
    }

    /// Creates an error from an explicit Win32 code.
    pub fn from_win32(context: &'static str, code: u32) -> Self {
        Self::Win32 { context, code, message: system_message(code) }
    }

    /// Converts an `HRESULT` to `Ok(())` or an error.
    pub fn check_hresult(context: &'static str, hresult: HRESULT) -> GuiResult<()> {
        if hresult >= 0 {
            Ok(())
        } else {
            Err(Self::HResult { context, hresult })
        }
    }
}

/// System text for a Win32 error code (empty if unknown).
pub fn system_message(code: u32) -> String {
    let mut buffer = [0u16; 512];
    // SAFETY: o buffer tem o tamanho informado; com IGNORE_INSERTS não há argumentos.
    let len = unsafe {
        FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            std::ptr::null(),
            code,
            0,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            std::ptr::null_mut(),
        )
    } as usize;
    String::from_utf16_lossy(&buffer[..len.min(buffer.len())]).trim_end().to_string()
}

impl fmt::Display for GuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuiError::Win32 { context, code, message } => {
                write!(f, "{context}: Win32 error {code} ({message})")
            }
            GuiError::HResult { context, hresult } => {
                write!(f, "{context}: HRESULT 0x{:08X}", *hresult as u32)
            }
            GuiError::NotInitialized => {
                write!(f, "no active Application on this thread; call Application::new() first")
            }
            GuiError::AlreadyInitialized => write!(f, "an Application already exists on this thread"),
            GuiError::WidgetDestroyed => write!(f, "the control or window has already been destroyed"),
            GuiError::MissingParent => write!(f, "the control needs a parent: call .parent(&window)"),
            GuiError::InvalidArgument(reason) => write!(f, "invalid argument: {reason}"),
            GuiError::Unsupported(what) => write!(f, "feature not available on this system: {what}"),
            GuiError::ApplicationGone => write!(f, "the UI thread has already ended"),
        }
    }
}

impl std::error::Error for GuiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win32_error_has_system_message() {
        let err = GuiError::from_win32("teste", 5);
        match &err {
            GuiError::Win32 { code, message, .. } => {
                assert_eq!(*code, 5);
                assert!(!message.is_empty());
            }
            other => panic!("variante inesperada: {other:?}"),
        }
        assert!(err.to_string().contains("teste"));
    }

    #[test]
    fn hresult_conversion() {
        assert!(GuiError::check_hresult("ok", 0).is_ok());
        assert!(GuiError::check_hresult("ok", 1).is_ok());
        let err = GuiError::check_hresult("falha", 0x8000_4005_u32 as i32).unwrap_err();
        assert_eq!(err.to_string(), "falha: HRESULT 0x80004005");
    }

    #[test]
    fn display_is_never_empty() {
        for err in [
            GuiError::NotInitialized,
            GuiError::AlreadyInitialized,
            GuiError::WidgetDestroyed,
            GuiError::MissingParent,
            GuiError::InvalidArgument("x".into()),
            GuiError::Unsupported("y"),
            GuiError::ApplicationGone,
        ] {
            assert!(!err.to_string().is_empty());
        }
    }
}
