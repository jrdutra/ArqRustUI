//! [`DataGrid`](super::DataGrid) cell editing: text/list editors, check boxes and validation.

use super::*;

impl DataGrid {
    // ------------------------------------------------------------------
    // Edição
    // ------------------------------------------------------------------

    /// Sets a text validation function called before accepting cell edits.
    pub fn set_validator(&self, validator: impl Fn(usize, usize, &str) -> bool + 'static) {
        self.model.borrow_mut().validator = Some(Rc::new(validator));
    }

    pub(crate) fn set_edit_hook(&self, hook: impl Fn(usize, usize) -> EditAction + 'static) {
        self.model.borrow_mut().edit_hook = Some(Rc::new(hook));
    }

    /// Cell rectangle in pixels relative to the grid (`None` if not displayed, such as in a collapsed group).
    fn cell_rect(&self, row: usize, column: usize) -> Option<RECT> {
        let item = self.model.borrow().row_to_item(row)?;
        Some(self.subitem_rect(item, column))
    }

    /// Commits a user value: validates, stores in memory or forwards to the virtual source, and notifies
    /// `on_cell_edited`. Returns `false` on rejection.
    fn apply_edit(&self, row: usize, column: usize, text: &str) -> bool {
        let validator = self.model.borrow().validator.clone();
        if let Some(validator) = validator {
            if !validator(row, column, text) {
                // SAFETY: aviso sonoro padrão do sistema.
                unsafe { MessageBeep(0) };
                return false;
            }
        }
        let previous = self.cell_value(row, column);
        if previous == text {
            return true;
        }
        let virtual_editor = {
            let model = self.model.borrow();
            model.is_virtual().then(|| model.virtual_editor.clone()).flatten()
        };
        match virtual_editor {
            Some(editor) => {
                if !editor(row, column, text) {
                    // SAFETY: aviso sonoro padrão do sistema.
                    unsafe { MessageBeep(0) };
                    return false;
                }
                self.redraw_row(row);
            }
            None => self.set_cell(row, column, text),
        }
        let listeners = self.model.borrow().edited.clone();
        for listener in listeners {
            listener(row, column, text);
        }
        true
    }

    fn can_edit(&self, row: usize, column: usize) -> bool {
        let model = self.model.borrow();
        model.editable
            && row < model.row_count()
            && column < model.columns.len().max(1)
            && !model.read_only_columns.contains(&column)
            && (!model.is_virtual() || model.virtual_editor.is_some())
    }

    /// Toggles a `CheckBox` column cell as a user click would. Returns `false` if not editable.
    pub fn toggle_check(&self, row: usize, column: usize) -> bool {
        if self.column_kind(column) != ColumnKind::CheckBox || !self.can_edit(row, column) {
            return false;
        }
        let next = if parse_check(&self.cell_value(row, column)) { "false" } else { "true" };
        self.apply_edit(row, column, next)
    }

    /// Opens a text/drop-down editor over the cell according to column type. Returns `false` if not editable.
    pub fn begin_edit(&self, row: usize, column: usize) -> bool {
        self.commit_edit();
        if !self.can_edit(row, column) || column >= self.column_count() {
            return false;
        }
        let hook = self.model.borrow().edit_hook.clone();
        let action = hook.map(|h| h(row, column)).unwrap_or(EditAction::Default);
        let choice = match action {
            EditAction::Deny => return false,
            EditAction::Handled => return true,
            EditAction::Choice(options, selected) => Some((options, selected)),
            EditAction::Default => match self.column_kind(column) {
                ColumnKind::CheckBox => return self.toggle_check(row, column),
                ColumnKind::Choice(options) => {
                    let current = self.cell_value(row, column);
                    let selected = options.iter().position(|o| *o == current).unwrap_or(0);
                    Some((options, selected))
                }
                ColumnKind::Text => None,
            },
        };
        let Some(item) = self.model.borrow().row_to_item(row) else { return false };
        native::send(self.core.hwnd(), LVM_ENSUREVISIBLE, item, 0);
        let created = match &choice {
            Some((options, selected)) => self.create_choice_editor(options, *selected),
            None => self.create_text_editor(row, column),
        };
        let Some(editor) = created else { return false };
        self.model.borrow_mut().editor = Some(Editor { core: editor.clone(), row, column, choice: choice.is_some() });
        self.place_editor();
        // SAFETY: foco para o editor recém-criado, vivo nesta thread.
        unsafe { SetFocus(editor.hwnd()) };
        if choice.is_some() {
            native::send(editor.hwnd(), CB_SHOWDROPDOWN, 1, 0);
        } else {
            native::send(editor.hwnd(), EM_SETSEL, 0, -1);
        }
        true
    }

    fn create_text_editor(&self, row: usize, column: usize) -> Option<Rc<WidgetCore>> {
        let editor = WidgetCore::create_child(
            "CellEditor",
            &self.core,
            "EDIT",
            &self.cell_value(row, column),
            WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL,
            0,
            Rect::default(),
            None,
        )
        .ok()?;
        editor.set_font(self.core.font());
        let grid = self.clone();
        editor.set_raw_hook(move |msg| {
            const DLGC_WANTALLKEYS: isize = 0x4;
            const DLGC_HASSETSEL: isize = 0x8;
            match msg.message {
                // Sem isto a navegação de diálogo consumiria Enter e Esc.
                WM_GETDLGCODE => Some(DLGC_WANTALLKEYS | DLGC_WANTCHARS | DLGC_WANTARROWS | DLGC_HASSETSEL),
                WM_KEYDOWN if msg.wparam == 0x0D => {
                    grid.commit_edit();
                    Some(0)
                }
                WM_KEYDOWN if msg.wparam == 0x1B => {
                    grid.cancel_edit();
                    Some(0)
                }
                // Evita o bipe padrão do EDIT de uma linha para Enter e Esc.
                WM_CHAR if msg.wparam == 0x0D || msg.wparam == 0x1B => Some(0),
                WM_KILLFOCUS => {
                    let grid = grid.clone();
                    // Adiado: destruir a janela durante a própria perda de foco é arriscado.
                    let _ = crate::core::defer(move || {
                        grid.commit_edit();
                    });
                    None
                }
                _ => None,
            }
        });
        Some(editor)
    }

    fn create_choice_editor(&self, options: &[String], selected: usize) -> Option<Rc<WidgetCore>> {
        let editor = WidgetCore::create_child(
            "CellChoiceEditor",
            &self.core,
            "COMBOBOX",
            "",
            WS_VISIBLE | WS_VSCROLL | CBS_DROPDOWNLIST,
            0,
            Rect::default(),
            None,
        )
        .ok()?;
        editor.set_font(self.core.font());
        for option in options {
            native::send_text(editor.hwnd(), CB_ADDSTRING, 0, option);
        }
        native::send(editor.hwnd(), CB_SETCURSEL, selected.min(options.len().saturating_sub(1)), 0);
        let grid = self.clone();
        editor.set_reflect(move |_, reflected| {
            let Reflected::Command { code, .. } = reflected else { return None };
            match *code {
                CBN_SELENDOK => {
                    let grid = grid.clone();
                    let _ = crate::core::defer(move || {
                        grid.commit_edit();
                    });
                    Some(0)
                }
                CBN_SELENDCANCEL => {
                    let grid = grid.clone();
                    let _ = crate::core::defer(move || grid.cancel_edit());
                    Some(0)
                }
                _ => None,
            }
        });
        let grid = self.clone();
        editor.set_raw_hook(move |msg| match msg.message {
            WM_KEYDOWN if msg.wparam == 0x1B => {
                grid.cancel_edit();
                Some(0)
            }
            WM_KEYDOWN if msg.wparam == 0x0D => {
                grid.commit_edit();
                Some(0)
            }
            WM_KILLFOCUS => {
                let grid = grid.clone();
                let _ = crate::core::defer(move || {
                    grid.commit_edit();
                });
                None
            }
            _ => None,
        });
        Some(editor)
    }

    /// Repositions the editor after scrolling/column resizing; hides it while the cell is outside the visible area.
    pub(super) fn place_editor(&self) {
        let (core, row, column, choice) = {
            let model = self.model.borrow();
            let Some(editor) = model.editor.as_ref() else { return };
            (editor.core.clone(), editor.row, editor.column, editor.choice)
        };
        if !core.is_alive() {
            return;
        }
        let hwnd = core.hwnd();
        let Some(rect) = self.cell_rect(row, column) else {
            // SAFETY: esconde o editor vivo.
            unsafe { ShowWindow(hwnd, SW_HIDE) };
            return;
        };
        let grid = self.core.hwnd();
        let client = native::client_rect(grid);
        let header = native::send(grid, LVM_GETHEADER, 0, 0) as HWND;
        let header_bottom = if header.is_null() {
            0
        } else {
            let r = native::window_rect(header);
            r.bottom - r.top
        };
        let visible =
            rect.bottom > header_bottom && rect.top < client.bottom && rect.right > 0 && rect.left < client.right;
        let height = rect.bottom - rect.top;
        // A lista suspensa precisa da altura da lista na janela do COMBOBOX.
        let window_height = if choice { height + to_pixels(160, self.core.dpi()) } else { height };
        // SAFETY: posiciona, em pixels exatos, o editor vivo sobre a célula.
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOP,
                rect.left,
                rect.top,
                rect.right - rect.left,
                window_height,
                SWP_NOACTIVATE | if visible { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW },
            );
        }
        if choice {
            let field = native::send(hwnd, CB_GETITEMHEIGHT, usize::MAX, 0) as i32;
            if field > 0 && field != height - 6 && height > 10 {
                native::send(hwnd, CB_SETITEMHEIGHT, usize::MAX, (height - 6) as isize);
            }
        }
    }

    /// Whether a cell is being edited.
    pub fn is_editing(&self) -> bool {
        self.model.borrow().editor.is_some()
    }

    /// Row and column being edited.
    pub fn editing_cell(&self) -> Option<(usize, usize)> {
        self.model.borrow().editor.as_ref().map(|e| (e.row, e.column))
    }

    /// Commits ongoing editing, if any. Returns `false` if validation rejects the text (retains the previous value).
    pub fn commit_edit(&self) -> bool {
        let Some(editor) = self.model.borrow_mut().editor.take() else { return true };
        let text = if editor.choice {
            let index = native::send(editor.core.hwnd(), CB_GETCURSEL, 0, 0);
            if index < 0 {
                editor.core.destroy();
                return true;
            }
            native::read_text(editor.core.hwnd(), CB_GETLBTEXTLEN, CB_GETLBTEXT, index as usize).unwrap_or_default()
        } else {
            native::window_text(editor.core.hwnd())
        };
        let had_focus = {
            // SAFETY: consultas simples de janelas desta thread (IsChild abaixo também).
            let focus = unsafe { GetFocus() };
            focus == editor.core.hwnd() || unsafe { IsChild(editor.core.hwnd(), focus) } != 0
        };
        editor.core.destroy();
        if had_focus {
            // SAFETY: devolve o foco à grade viva.
            unsafe { SetFocus(self.core.hwnd()) };
        }
        self.apply_edit(editor.row, editor.column, &text)
    }

    /// Discards ongoing editing.
    pub fn cancel_edit(&self) {
        let editor = self.model.borrow_mut().editor.take();
        if let Some(editor) = editor {
            editor.core.destroy();
            // SAFETY: devolve o foco à grade viva.
            unsafe { SetFocus(self.core.hwnd()) };
        }
    }

    /// The user edited a cell; receives row, column and new text.
    pub fn on_cell_edited(&self, f: impl Fn(usize, usize, &str) + 'static) {
        self.model.borrow_mut().edited.push(Rc::new(f));
    }
}
