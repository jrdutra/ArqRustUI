//! Message boxes.

use crate::core::widget::Widget;
use crate::ffi::user32::*;
use crate::ffi::HWND;
use crate::utilities::to_wide;

/// Chosen message box button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageResult {
    /// OK.
    Ok,
    /// Cancel or box closed.
    Cancel,
    /// Yes.
    Yes,
    /// No.
    No,
    /// Retry.
    Retry,
}

/// Message box button set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageButtons {
    /// OK.
    Ok,
    /// OK and Cancel.
    OkCancel,
    /// Yes and No.
    YesNo,
    /// Yes, No and Cancel.
    YesNoCancel,
    /// Retry and Cancel.
    RetryCancel,
}

/// Message box icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageIcon {
    /// No icon.
    None,
    /// Information.
    Information,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// Question.
    Question,
}

/// System modal message boxes.
///
/// Ownerless `info`, `warning`, `error`, `confirm` block all thread windows; use [`MessageBox::show`] to specify an
/// owner.
pub struct MessageBox;

impl MessageBox {
    fn run(owner: HWND, title: &str, text: &str, buttons: MessageButtons, icon: MessageIcon) -> MessageResult {
        let mut flags = match buttons {
            MessageButtons::Ok => MB_OK,
            MessageButtons::OkCancel => MB_OKCANCEL,
            MessageButtons::YesNo => MB_YESNO,
            MessageButtons::YesNoCancel => MB_YESNOCANCEL,
            MessageButtons::RetryCancel => MB_RETRYCANCEL,
        };
        flags |= match icon {
            MessageIcon::None => 0,
            MessageIcon::Information => MB_ICONINFORMATION,
            MessageIcon::Warning => MB_ICONWARNING,
            MessageIcon::Error => MB_ICONERROR,
            MessageIcon::Question => MB_ICONQUESTION,
        };
        if owner.is_null() {
            flags |= MB_TASKMODAL;
        }
        let (text, title) = (to_wide(text), to_wide(title));
        // SAFETY: strings terminadas em NUL válidas durante a chamada; dono nulo é permitido.
        match unsafe { MessageBoxW(owner, text.as_ptr(), title.as_ptr(), flags) } {
            1 => MessageResult::Ok,
            4 => MessageResult::Retry,
            6 => MessageResult::Yes,
            7 => MessageResult::No,
            _ => MessageResult::Cancel,
        }
    }

    /// Complete message box, modal to `owner`.
    pub fn show(
        owner: &impl Widget,
        title: &str,
        text: &str,
        buttons: MessageButtons,
        icon: MessageIcon,
    ) -> MessageResult {
        Self::run(owner.core().hwnd(), title, text, buttons, icon)
    }

    /// Information with OK.
    pub fn info(title: &str, text: &str) {
        Self::run(std::ptr::null_mut(), title, text, MessageButtons::Ok, MessageIcon::Information);
    }

    /// Warning with OK.
    pub fn warning(title: &str, text: &str) {
        Self::run(std::ptr::null_mut(), title, text, MessageButtons::Ok, MessageIcon::Warning);
    }

    /// Error with OK.
    pub fn error(title: &str, text: &str) {
        Self::run(std::ptr::null_mut(), title, text, MessageButtons::Ok, MessageIcon::Error);
    }

    /// Yes/No question; returns `true` for Yes.
    pub fn confirm(title: &str, text: &str) -> bool {
        Self::run(std::ptr::null_mut(), title, text, MessageButtons::YesNo, MessageIcon::Question) == MessageResult::Yes
    }
}
