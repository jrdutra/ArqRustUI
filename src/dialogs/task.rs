//! TaskDialog: modern dialog with main instruction, custom buttons, command links and verification check box.

use crate::core::application as app;
use crate::core::native;
use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult};
use crate::ffi::comctl32::*;
use crate::ffi::{HWND, PCWSTR};
use crate::utilities::to_wide;

/// [`TaskDialog`] main icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskIcon {
    /// No icon.
    None,
    /// Information.
    Information,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// Shield for an operation requiring elevation.
    Shield,
}

/// Selected [`TaskDialog`] button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskButton {
    /// OK.
    Ok,
    /// Cancel or dialog closed.
    Cancel,
    /// Yes.
    Yes,
    /// No.
    No,
    /// Retry.
    Retry,
    /// Close.
    Close,
    /// Custom button identified by [`TaskDialog::button`] number.
    Custom(i32),
}

/// [`TaskDialog`] result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TaskDialogResult {
    /// Selected button.
    pub button: TaskButton,
    /// Final verification check box state, if present.
    pub verification_checked: bool,
}

/// TaskDialog builder.
///
/// Requires Common Controls v6, automatically activated by the library. [`show`](Self::show) returns
/// [`GuiError::Unsupported`] if unavailable.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let answer = TaskDialog::new()
///     .title("Save changes")
///     .heading("Save changes to report.txt?")
///     .content("If you do not save, changes will be lost.")
///     .icon(TaskIcon::Warning)
///     .button(100, "&Save")
///     .button(101, "&Do not save")
///     .cancel_button(true)
///     .show(window)?;
/// if answer.button == TaskButton::Custom(100) { /* salvar */ }
/// # Ok(()) }
/// ```
#[derive(Clone, Debug, Default)]
pub struct TaskDialog {
    title: String,
    heading: String,
    content: String,
    footer: String,
    expanded: String,
    verification: Option<(String, bool)>,
    icon: Option<TaskIcon>,
    custom_icon: Option<std::rc::Rc<dyn crate::core::IconSource>>,
    common: u32,
    custom: Vec<(i32, String)>,
    command_links: bool,
    default_button: Option<i32>,
}

impl TaskDialog {
    /// Creates an empty TaskDialog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Window title.
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Main instruction (emphasized text).
    pub fn heading(mut self, heading: &str) -> Self {
        self.heading = heading.to_string();
        self
    }

    /// Explanatory text.
    pub fn content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }

    /// Footer text.
    pub fn footer(mut self, footer: &str) -> Self {
        self.footer = footer.to_string();
        self
    }

    /// Details displayed when expanded.
    pub fn expanded_information(mut self, text: &str) -> Self {
        self.expanded = text.to_string();
        self
    }

    /// Adds a verification check box, e.g. "Do not ask again".
    pub fn verification(mut self, text: &str, checked: bool) -> Self {
        self.verification = Some((text.to_string(), checked));
        self
    }

    /// Main icon.
    pub fn icon(mut self, icon: TaskIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Custom main icon, e.g. `&icons::CLOUD_SYNC`, overriding [`icon`](Self::icon).
    pub fn custom_icon(mut self, icon: &impl crate::core::IconSource) -> Self {
        self.custom_icon = Some(icon.to_shared());
        self
    }

    /// Adds a custom button. Use identifiers starting at 100.
    pub fn button(mut self, id: i32, text: &str) -> Self {
        self.custom.push((id, text.to_string()));
        self
    }

    /// Displays custom buttons as large command links, with description after a text newline.
    pub fn command_links(mut self, enabled: bool) -> Self {
        self.command_links = enabled;
        self
    }

    fn common(mut self, flag: u32, enabled: bool) -> Self {
        if enabled {
            self.common |= flag;
        } else {
            self.common &= !flag;
        }
        self
    }

    /// Includes standard OK.
    pub fn ok_button(self, enabled: bool) -> Self {
        self.common(TDCBF_OK_BUTTON, enabled)
    }

    /// Includes standard Cancel, also enabling Esc.
    pub fn cancel_button(self, enabled: bool) -> Self {
        self.common(TDCBF_CANCEL_BUTTON, enabled)
    }

    /// Includes standard Yes/No.
    pub fn yes_no_buttons(self, enabled: bool) -> Self {
        self.common(TDCBF_YES_BUTTON | TDCBF_NO_BUTTON, enabled)
    }

    /// Includes standard Retry.
    pub fn retry_button(self, enabled: bool) -> Self {
        self.common(TDCBF_RETRY_BUTTON, enabled)
    }

    /// Includes standard Close.
    pub fn close_button(self, enabled: bool) -> Self {
        self.common(TDCBF_CLOSE_BUTTON, enabled)
    }

    /// Initially focused button identifier.
    pub fn default_button(mut self, id: i32) -> Self {
        self.default_button = Some(id);
        self
    }

    /// Shows modally relative to `owner`.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<TaskDialogResult> {
        self.run(owner.core().require_alive()?)
    }

    fn run(&self, owner: HWND) -> GuiResult<TaskDialogResult> {
        app::require()?;
        let show = comctl32_task_dialog()?;
        let optional = |text: &str| (!text.is_empty()).then(|| to_wide(text));
        let pointer = |wide: &Option<Vec<u16>>| wide.as_ref().map_or(std::ptr::null(), |w| w.as_ptr());
        let title = optional(&self.title);
        let heading = optional(&self.heading);
        let content = optional(&self.content);
        let footer = optional(&self.footer);
        let expanded = optional(&self.expanded);
        let verification = self.verification.as_ref().map(|(text, _)| to_wide(text));
        let button_texts: Vec<Vec<u16>> = self.custom.iter().map(|(_, text)| to_wide(text)).collect();
        let buttons: Vec<TASKDIALOG_BUTTON> = self
            .custom
            .iter()
            .zip(&button_texts)
            .map(|((id, _), text)| TASKDIALOG_BUTTON { nButtonID: *id, pszButtonText: text.as_ptr() })
            .collect();
        // Ícones padrão são identificados por inteiros negativos de 16 bits (MAKEINTRESOURCE).
        let icon: PCWSTR = match self.icon.unwrap_or(TaskIcon::None) {
            TaskIcon::None => std::ptr::null(),
            TaskIcon::Warning => 0xFFFF_usize as PCWSTR,
            TaskIcon::Error => 0xFFFE_usize as PCWSTR,
            TaskIcon::Information => 0xFFFD_usize as PCWSTR,
            TaskIcon::Shield => 0xFFFC_usize as PCWSTR,
        };
        let mut flags = TDF_POSITION_RELATIVE_TO_WINDOW | TDF_SIZE_TO_CONTENT;
        // O ícone próprio precisa viver até o diálogo fechar: fica nesta variável.
        let dpi = if owner.is_null() { native::system_dpi() } else { native::dpi_for_window(owner) };
        let custom_icon = match &self.custom_icon {
            Some(source) => Some(source.icon_pixels(native::system_metric(crate::ffi::user32::SM_CXICON, dpi))?),
            None => None,
        };
        let icon = match &custom_icon {
            Some(custom) => {
                flags |= TDF_USE_HICON_MAIN;
                // `hMainIcon` e `pszMainIcon` ocupam a mesma posição (união).
                custom.raw() as PCWSTR
            }
            None => icon,
        };
        if self.common & TDCBF_CANCEL_BUTTON != 0 {
            flags |= TDF_ALLOW_DIALOG_CANCELLATION;
        }
        if self.command_links && !buttons.is_empty() {
            flags |= TDF_USE_COMMAND_LINKS;
        }
        if matches!(self.verification, Some((_, true))) {
            flags |= TDF_VERIFICATION_FLAG_CHECKED;
        }
        let common = if self.common == 0 && buttons.is_empty() { TDCBF_OK_BUTTON } else { self.common };
        let config = TASKDIALOGCONFIG {
            cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
            hwndParent: owner,
            hInstance: native::instance(),
            dwFlags: flags,
            dwCommonButtons: common,
            pszWindowTitle: pointer(&title),
            pszMainIcon: icon,
            pszMainInstruction: pointer(&heading),
            pszContent: pointer(&content),
            cButtons: buttons.len() as u32,
            pButtons: if buttons.is_empty() { std::ptr::null() } else { buttons.as_ptr() },
            nDefaultButton: self.default_button.unwrap_or(0),
            cRadioButtons: 0,
            pRadioButtons: std::ptr::null(),
            nDefaultRadioButton: 0,
            pszVerificationText: pointer(&verification),
            pszExpandedInformation: pointer(&expanded),
            pszExpandedControlText: std::ptr::null(),
            pszCollapsedControlText: std::ptr::null(),
            pszFooterIcon: std::ptr::null(),
            pszFooter: pointer(&footer),
            pfCallback: std::ptr::null(),
            lpCallbackData: 0,
            cxWidth: 0,
        };
        let (mut button, mut radio, mut checked) = (0i32, 0i32, 0i32);
        // SAFETY: estrutura completa; todas as strings e o array de botões vivem até o
        // fim da chamada; os ponteiros de saída são locais.
        let hr = unsafe { show(&config, &mut button, &mut radio, &mut checked) };
        GuiError::check_hresult("TaskDialogIndirect", hr)?;
        let button = match button {
            1 => TaskButton::Ok,
            2 => TaskButton::Cancel,
            4 => TaskButton::Retry,
            6 => TaskButton::Yes,
            7 => TaskButton::No,
            8 => TaskButton::Close,
            other => TaskButton::Custom(other),
        };
        Ok(TaskDialogResult { button, verification_checked: checked != 0 })
    }
}

type ShowFn = unsafe extern "system" fn(*const TASKDIALOGCONFIG, *mut i32, *mut i32, *mut i32) -> i32;

fn comctl32_task_dialog() -> GuiResult<ShowFn> {
    functions()
        .and_then(|api| api.TaskDialogIndirect)
        .ok_or(GuiError::Unsupported("TaskDialog requires Common Controls version 6"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_layout_matches_packed_header() {
        // Com empacotamento de 1 byte: 4 + 8 + 8 + 4 + 4 + 8*4 + 4 + 8 + 4 + 4 + 8 + 4 + 8*6 + 8 + 8 + 4.
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(std::mem::size_of::<TASKDIALOGCONFIG>(), 160);
            assert_eq!(std::mem::size_of::<TASKDIALOG_BUTTON>(), 12);
        }
    }

    #[test]
    fn builder_accumulates_buttons() {
        let dialog = TaskDialog::new().yes_no_buttons(true).cancel_button(true).cancel_button(false).button(100, "A");
        assert_eq!(dialog.common, TDCBF_YES_BUTTON | TDCBF_NO_BUTTON);
        assert_eq!(dialog.custom, vec![(100, "A".to_string())]);
    }
}
