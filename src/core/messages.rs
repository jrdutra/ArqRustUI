//! Advanced raw Win32 message access.

/// Raw Win32 message delivered to [`Widget::on_raw_message`](crate::core::Widget::on_raw_message).
///
/// Advanced API: most applications should use high-level events. Fields match `HWND`, `UINT`, `WPARAM` and `LPARAM`
/// semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RawMessage {
    /// Window handle (numeric `HWND`).
    pub hwnd: isize,
    /// Message code (`WM_*`).
    pub message: u32,
    /// `WPARAM` parameter.
    pub wparam: usize,
    /// `LPARAM` parameter.
    pub lparam: isize,
}
