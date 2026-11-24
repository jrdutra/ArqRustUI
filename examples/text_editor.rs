//! Example 6 — Text editor: menus, shortcuts, open/save, find and status bar.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

struct Document {
    path: Option<PathBuf>,
}

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Untitled — Editor").size(720, 480).centered().accept_files(true).build()?;
    let editor = TextBox::builder().multiline(true).word_wrap(false).parent(&window).build()?;
    editor.set_font(Font::new("Consolas", 11.0));
    let status = StatusBar::new(&window)?;
    status.set_parts(&[220, -1]);

    let layout = DockLayout::new().padding(Thickness::new(0, 0, 0, status.height()));
    layout.add(&editor, Dock::Fill);
    window.set_layout(layout);

    let document = Rc::new(RefCell::new(Document { path: None }));
    let refresh = {
        let (window, editor, status, document) = (window.clone(), editor.clone(), status.clone(), document.clone());
        move || {
            let name = document
                .borrow()
                .path
                .as_ref()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "Untitled".into());
            let mark = if editor.is_modified() { "*" } else { "" };
            window.set_title(&format!("{mark}{name} — Editor"));
            status.set_part_text(
                0,
                &format!("{} character(s), {} line(s)", editor.text().chars().count(), editor.line_count()),
            );
            status.set_part_text(1, if editor.is_modified() { "Modified" } else { "Saved" });
        }
    };
    editor.on_text_changed(refresh.clone());

    let load = {
        let (window, editor, document, refresh) = (window.clone(), editor.clone(), document.clone(), refresh.clone());
        move |path: PathBuf| match std::fs::read_to_string(&path) {
            Ok(text) => {
                editor.set_text(&text.replace("\r\n", "\n").replace('\n', "\r\n"));
                editor.set_modified(false);
                document.borrow_mut().path = Some(path);
                refresh();
            }
            Err(error) => {
                MessageBox::show(
                    &window,
                    "Open",
                    &format!("Could not open the file:\n{error}"),
                    MessageButtons::Ok,
                    MessageIcon::Error,
                );
            }
        }
    };
    let save = {
        let (window, editor, document, refresh) = (window.clone(), editor.clone(), document.clone(), refresh.clone());
        move |ask: bool| -> bool {
            let current = document.borrow().path.clone();
            let path = match (current, ask) {
                (Some(path), false) => path,
                _ => match SaveFileDialog::new()
                    .filter("Text", "*.txt")
                    .filter("All", "*.*")
                    .default_extension("txt")
                    .show(&window)
                {
                    Ok(Some(path)) => path,
                    Ok(None) => return false,
                    Err(error) => {
                        MessageBox::show(&window, "Save", &error.to_string(), MessageButtons::Ok, MessageIcon::Error);
                        return false;
                    }
                },
            };
            match std::fs::write(&path, editor.text()) {
                Ok(()) => {
                    editor.set_modified(false);
                    document.borrow_mut().path = Some(path);
                    refresh();
                    true
                }
                Err(error) => {
                    MessageBox::show(
                        &window,
                        "Save",
                        &format!("Save failed:\n{error}"),
                        MessageButtons::Ok,
                        MessageIcon::Error,
                    );
                    false
                }
            }
        }
    };

    let file = Menu::new()?;
    {
        let (editor, document, refresh) = (editor.clone(), document.clone(), refresh.clone());
        file.add_item_with_shortcut("&New", Shortcut::ctrl(Key(b'N' as u16)), move || {
            editor.set_text("");
            editor.set_modified(false);
            document.borrow_mut().path = None;
            refresh();
        })?;
    }
    {
        let (window, load) = (window.clone(), load.clone());
        file.add_item_with_shortcut("&Open...", Shortcut::ctrl(Key(b'O' as u16)), move || {
            if let Ok(Some(path)) =
                OpenFileDialog::new().filter("Text", "*.txt;*.md;*.rs").filter("All", "*.*").show(&window)
            {
                load(path);
            }
        })?;
    }
    {
        let save = save.clone();
        file.add_item_with_shortcut("&Save", Shortcut::ctrl(Key(b'S' as u16)), move || {
            save(false);
        })?;
    }
    {
        let save = save.clone();
        file.add_item_with_shortcut("Save &as...", Shortcut::ctrl_shift(Key(b'S' as u16)), move || {
            save(true);
        })?;
    }
    file.add_separator()?;
    let target = window.clone();
    file.add_item("E&xit", move || target.close())?;

    let edit = Menu::new()?;
    let e = editor.clone();
    edit.add_item_with_shortcut("&Undo", Shortcut::ctrl(Key(b'Z' as u16)), move || e.undo())?;
    edit.add_separator()?;
    let e = editor.clone();
    edit.add_item("Cu&t\tCtrl+X", move || e.cut())?;
    let e = editor.clone();
    edit.add_item("&Copy\tCtrl+C", move || e.copy())?;
    let e = editor.clone();
    edit.add_item("&Paste\tCtrl+V", move || e.paste())?;
    let e = editor.clone();
    edit.add_item_with_shortcut("Select &all", Shortcut::ctrl(Key(b'A' as u16)), move || e.select_all())?;
    edit.add_separator()?;
    {
        let (window, editor) = (window.clone(), editor.clone());
        let open_dialog: Rc<RefCell<Option<FindDialog>>> = Rc::new(RefCell::new(None));
        edit.add_item_with_shortcut("&Find...", Shortcut::ctrl(Key(b'F' as u16)), move || {
            if open_dialog.borrow().as_ref().map(|d| d.is_open()).unwrap_or(false) {
                return;
            }
            let (owner, editor) = (window.clone(), editor.clone());
            let dialog = FindDialog::find(&window, "", move |event| {
                if event.action != FindAction::FindNext {
                    return;
                }
                let text: Vec<u16> = editor.text().encode_utf16().collect();
                let needle: Vec<u16> = event.text.encode_utf16().collect();
                let from = editor.selection().1.min(text.len());
                let found = text[from..].windows(needle.len().max(1)).position(|w| w == needle.as_slice());
                match found {
                    Some(offset) => editor.set_selection(from + offset, from + offset + needle.len()),
                    None => MessageBox::info("Find", "No more occurrences."),
                }
                let _ = &owner;
            });
            *open_dialog.borrow_mut() = dialog.ok();
        })?;
    }

    let bar = MenuBar::new()?;
    bar.add_menu("&File", &file)?;
    bar.add_menu("&Edit", &edit)?;
    window.set_menu(&bar)?;

    let dropped = load.clone();
    window.on_drop_files(move |files| {
        if let Some(path) = files.first() {
            dropped(path.clone());
        }
    });
    let (probe, owner, save_on_close) = (editor.clone(), window.clone(), save.clone());
    window.on_close_request(move || {
        if !probe.is_modified() {
            return true;
        }
        match MessageBox::show(
            &owner,
            "Editor",
            "Do you want to save your changes?",
            MessageButtons::YesNoCancel,
            MessageIcon::Question,
        ) {
            MessageResult::Yes => save_on_close(false),
            MessageResult::No => true,
            _ => false,
        }
    });

    refresh();
    window.show();
    editor.focus();
    app.run()
}
