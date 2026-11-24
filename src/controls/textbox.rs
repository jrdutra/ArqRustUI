//! Text fields: [`TextBox`] and [`PasswordBox`].

use super::common::{builder_common, simple_event, widget_type, ControlOptions};
use crate::core::native;
use crate::core::widget::{Reflected, Widget, WidgetCore};
use crate::core::{Event, GuiResult};
use crate::ffi::user32::*;
use crate::graphics::TextAlign;
use std::rc::Rc;

widget_type!(
    /// Single/multiline text editing field.
    ///
    /// ```no_run
    /// # use arqrustui::prelude::*;
    /// # fn demo(window: &Window) -> GuiResult<()> {
    /// let name = TextBox::builder().placeholder("Your name").bounds(20, 20, 240, 24).parent(window).build()?;
    /// let echo = name.clone();
    /// name.on_text_changed(move || println!("text: {}", echo.text()));
    /// # Ok(()) }
    /// ```
    TextBox
);

widget_type!(
    /// Password field: entered characters are masked.
    PasswordBox
);

/// Reflects `EDIT` notifications.
pub(crate) fn reflect_edit(core: &Rc<WidgetCore>, reflected: &Reflected) -> Option<isize> {
    match reflected {
        Reflected::Command { code, .. } if *code == EN_CHANGE => {
            core.emit(&Event::TextChanged);
            Some(0)
        }
        _ => None,
    }
}

/// Builder for [`TextBox`].
#[derive(Clone, Debug)]
pub struct TextBoxBuilder {
    opts: ControlOptions,
    multiline: bool,
    read_only: bool,
    numeric: bool,
    word_wrap: bool,
    align: TextAlign,
    placeholder: Option<String>,
    max_length: Option<usize>,
}

impl TextBoxBuilder {
    builder_common!();

    /// Allows multiple lines with vertical scrolling.
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    /// Wraps long lines in multiline fields (default: `true`).
    pub fn word_wrap(mut self, word_wrap: bool) -> Self {
        self.word_wrap = word_wrap;
        self
    }

    /// Prevents user editing while allowing selection.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Accepts digits only.
    pub fn numeric(mut self, numeric: bool) -> Self {
        self.numeric = numeric;
        self
    }

    /// Horizontal text alignment.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Placeholder displayed when empty.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = Some(text.to_string());
        self
    }

    /// Maximum accepted character count.
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Creates the field.
    pub fn build(self) -> GuiResult<TextBox> {
        let mut style = match self.align {
            TextAlign::Left => ES_LEFT,
            TextAlign::Center => ES_CENTER,
            TextAlign::Right => ES_RIGHT,
        };
        if self.multiline {
            style |= ES_MULTILINE | ES_WANTRETURN | ES_AUTOVSCROLL | WS_VSCROLL;
            if !self.word_wrap {
                style |= ES_AUTOHSCROLL | WS_HSCROLL;
            }
        } else {
            style |= ES_AUTOHSCROLL;
        }
        if self.read_only {
            style |= ES_READONLY;
        }
        if self.numeric {
            style |= ES_NUMBER;
        }
        let core = self.opts.create("TextBox", "EDIT", style, WS_EX_CLIENTEDGE, true, None)?;
        core.set_reflect(reflect_edit);
        // Uma linha: o texto fica centralizado na altura da caixa, como nos botões.
        core.set_centered_text(!self.multiline);
        let textbox = TextBox { core };
        if let Some(placeholder) = &self.placeholder {
            textbox.set_placeholder(placeholder);
        }
        if let Some(max) = self.max_length {
            textbox.set_max_length(max);
        }
        Ok(textbox)
    }
}

/// Methods shared by `EDIT`-based controls.
macro_rules! edit_methods {
    () => {
        simple_event!(
            /// Text changed by typing, paste or `set_text`.
            on_text_changed,
            TextChanged
        );

        /// Sets the empty-field placeholder.
        pub fn set_placeholder(&self, text: &str) {
            native::send_text(self.core.hwnd(), EM_SETCUEBANNER, 1, text);
        }

        /// Limits accepted characters (0 removes limit).
        pub fn set_max_length(&self, max: usize) {
            native::send(self.core.hwnd(), EM_LIMITTEXT, max, 0);
        }

        /// Selects all text.
        pub fn select_all(&self) {
            native::send(self.core.hwnd(), EM_SETSEL, 0, -1);
        }

        /// Selects `[start, end)` in UTF-16 units.
        pub fn set_selection(&self, start: usize, end: usize) {
            native::send(self.core.hwnd(), EM_SETSEL, start, end as isize);
            native::send(self.core.hwnd(), EM_SCROLLCARET, 0, 0);
        }

        /// Selected `(start, end)` range in UTF-16 units.
        pub fn selection(&self) -> (usize, usize) {
            let (mut start, mut end) = (0u32, 0u32);
            // SAFETY: EM_GETSEL grava dois DWORD nos ponteiros recebidos, válidos durante a chamada.
            unsafe {
                SendMessageW(
                    self.core.hwnd(),
                    EM_GETSEL,
                    &mut start as *mut u32 as usize,
                    &mut end as *mut u32 as isize,
                )
            };
            (start as usize, end as usize)
        }

        /// Replaces selection or inserts at caret as an undoable edit.
        pub fn replace_selection(&self, text: &str) {
            native::send_text(self.core.hwnd(), EM_REPLACESEL, 1, text);
        }

        /// Appends text.
        pub fn append(&self, text: &str) {
            let end = native::send(self.core.hwnd(), WM_GETTEXTLENGTH, 0, 0) as usize;
            native::send(self.core.hwnd(), EM_SETSEL, end, end as isize);
            self.replace_selection(text);
        }

        /// Enables/disables read-only mode.
        pub fn set_read_only(&self, read_only: bool) {
            native::send(self.core.hwnd(), EM_SETREADONLY, read_only as usize, 0);
        }

        /// Whether the field is read-only.
        pub fn is_read_only(&self) -> bool {
            native::style(self.core.hwnd()) & ES_READONLY != 0
        }

        /// Undoes the last edit.
        pub fn undo(&self) {
            native::send(self.core.hwnd(), EM_UNDO, 0, 0);
        }

        /// Whether undo is available.
        pub fn can_undo(&self) -> bool {
            native::send(self.core.hwnd(), EM_CANUNDO, 0, 0) != 0
        }

        /// Cuts selection to clipboard.
        pub fn cut(&self) {
            native::send(self.core.hwnd(), WM_CUT, 0, 0);
        }

        /// Copies selection to clipboard.
        pub fn copy(&self) {
            native::send(self.core.hwnd(), WM_COPY, 0, 0);
        }

        /// Pastes clipboard content.
        pub fn paste(&self) {
            native::send(self.core.hwnd(), WM_PASTE, 0, 0);
        }

        /// Whether content changed since `set_modified(false)`.
        pub fn is_modified(&self) -> bool {
            native::send(self.core.hwnd(), EM_GETMODIFY, 0, 0) != 0
        }

        /// Sets the modified flag.
        pub fn set_modified(&self, modified: bool) {
            native::send(self.core.hwnd(), EM_SETMODIFY, modified as usize, 0);
        }
    };
}

pub(crate) use edit_methods;

impl TextBox {
    /// Starts building a text field (default size 160 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
    pub fn builder() -> TextBoxBuilder {
        TextBoxBuilder {
            opts: ControlOptions::new(160, super::DEFAULT_CONTROL_HEIGHT),
            multiline: false,
            read_only: false,
            numeric: false,
            word_wrap: true,
            align: TextAlign::Left,
            placeholder: None,
            max_length: None,
        }
    }

    edit_methods!();

    /// Line count (multiline fields).
    pub fn line_count(&self) -> usize {
        native::send(self.core.hwnd(), EM_GETLINECOUNT, 0, 0).max(0) as usize
    }
}

/// Builder for [`PasswordBox`].
#[derive(Clone, Debug)]
pub struct PasswordBoxBuilder {
    opts: ControlOptions,
    placeholder: Option<String>,
    max_length: Option<usize>,
}

impl PasswordBoxBuilder {
    builder_common!();

    /// Placeholder displayed when empty.
    pub fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = Some(text.to_string());
        self
    }

    /// Maximum accepted character count.
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Creates the password field.
    pub fn build(self) -> GuiResult<PasswordBox> {
        let core =
            self.opts.create("PasswordBox", "EDIT", ES_PASSWORD | ES_AUTOHSCROLL, WS_EX_CLIENTEDGE, true, None)?;
        core.set_reflect(reflect_edit);
        core.set_centered_text(true);
        // Senhas não devem passar por IMEs que armazenam histórico de digitação.
        let password = PasswordBox { core };
        crate::input::Ime::set_enabled(&password, false);
        if let Some(placeholder) = &self.placeholder {
            password.set_placeholder(placeholder);
        }
        if let Some(max) = self.max_length {
            password.set_max_length(max);
        }
        Ok(password)
    }
}

impl PasswordBox {
    /// Starts building a password field (default size 160 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT)).
    pub fn builder() -> PasswordBoxBuilder {
        PasswordBoxBuilder {
            opts: ControlOptions::new(160, super::DEFAULT_CONTROL_HEIGHT),
            placeholder: None,
            max_length: None,
        }
    }

    edit_methods!();

    /// Entered password (same as [`Widget::text`]).
    pub fn password(&self) -> String {
        self.text()
    }

    /// Shows (`true`) or masks (`false`) entered characters.
    pub fn set_revealed(&self, revealed: bool) {
        // U+25CF é o caractere de máscara padrão dos estilos visuais.
        let mask = if revealed { 0 } else { 0x25CF };
        native::send(self.core.hwnd(), EM_SETPASSWORDCHAR, mask, 0);
        self.core.invalidate();
    }
}
