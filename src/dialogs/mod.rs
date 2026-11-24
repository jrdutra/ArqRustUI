//! Dialogs: messages, files, colors, fonts, printing, Find/Replace, TaskDialog and custom dialogs.
//!
//! | Type | Modal | Native API |
//! |---|---|---|
//! | [`MessageBox`] | yes | `MessageBoxW` |
//! | [`OpenFileDialog`], [`SaveFileDialog`], [`FolderPicker`] | yes | `IFileDialog` (COM) |
//! | [`ColorDialog`], [`FontDialog`] | yes | `ChooseColorW`, `ChooseFontW` |
//! | [`PrintDialog`], [`PageSetupDialog`] | yes | `PrintDlgW`, `PageSetupDlgW` |
//! | [`FindDialog`] | no (asynchronous) | `FindTextW`, `ReplaceTextW` |
//! | [`TaskDialog`] | yes | `TaskDialogIndirect` |
//! | [`Dialog`] | modal/nonmodal | library window |

pub mod common;
pub mod custom;
pub mod file;
pub mod find;
pub mod message_box;
pub mod modal;
pub mod task;

pub use common::{ColorDialog, FontChoice, FontDialog, PageSetup, PageSetupDialog, PrintDialog};
pub use custom::{Dialog, DialogResult};
pub use file::{FolderPicker, OpenFileDialog, SaveFileDialog};
pub use find::{FindAction, FindDialog, FindEvent};
pub use message_box::{MessageBox, MessageButtons, MessageIcon, MessageResult};
pub use modal::{Modal, ModalBuilder};
pub use task::{TaskButton, TaskDialog, TaskDialogResult, TaskIcon};
