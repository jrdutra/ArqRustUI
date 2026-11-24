//! Web-style modals (`Modal`): a dimmed backdrop over the window, a centered
//! card with title, content and buttons, enter and exit animation.
//!
//! Try it: "Delete…" (confirmation with a primary button), "Rename…" (a form
//! with validation: the modal does not close with an empty name), "Alert" (`Modal::alert`),
//! click outside the card (it shakes or closes), `Esc`, `Enter` and Tab (focus stays trapped
//! in the card), and the dark theme.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    let window = Window::builder().title("Modals").size(720, 460).centered().build()?;
    let name = TextBox::builder().text("report.docx").parent(&window).build()?;
    let delete = Button::builder().text("Delete…").icon(&icons::DELETE).parent(&window).build()?;
    let rename = Button::builder().text("Rename…").parent(&window).build()?;
    let alert = Button::builder().text("Alert").parent(&window).build()?;
    let dark = CheckBox::builder().text("Dark theme").parent(&window).build()?;
    let status = Label::builder().text("No action yet.").parent(&window).build()?;
    let list = ListBox::builder()
        .items(&["report.docx", "budget.xlsx", "presentation.pptx", "notes.txt"])
        .parent(&window)
        .build()?;

    let row = StackLayout::horizontal().spacing(8);
    row.add_with(&delete, LayoutParams::new().width(120));
    row.add_with(&rename, LayoutParams::new().width(120));
    row.add_with(&alert, LayoutParams::new().width(90));
    row.add_with(&dark, LayoutParams::new().width(140));
    let root = StackLayout::vertical().spacing(10).padding(Thickness::all(16));
    root.add_with(&name, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
    root.add_with(row, LayoutParams::new().height(32));
    root.add_with(&list, LayoutParams::new().weight(1.0));
    root.add_with(&status, LayoutParams::new().height(24));
    window.set_layout(root);

    let (owner, field, label) = (window.clone(), name.clone(), status.clone());
    delete.on_click(move || {
        let modal = Modal::builder()
            .title("Delete file?")
            .message(&format!("\"{}\" will be deleted permanently. This action cannot be undone.", field.text()))
            .button("Cancel", DialogResult::Cancel)
            .primary_button("Delete", DialogResult::Ok)
            .close_on_backdrop(true)
            .build(&owner);
        match modal.and_then(|m| m.show()) {
            Ok(DialogResult::Ok) => label.set_text("File deleted."),
            Ok(_) => label.set_text("Deletion cancelled."),
            Err(e) => label.set_text(&format!("Error: {e}")),
        }
    });

    let (owner, field, label) = (window.clone(), name.clone(), status.clone());
    rename.on_click(move || {
        let built = Modal::builder()
            .title("Rename")
            .content_height(84)
            .button("Cancel", DialogResult::Cancel)
            .primary_button("Save", DialogResult::Ok)
            .build(&owner);
        let Ok(modal) = built else { return };
        let content = modal.content();
        let width = content.client_size().width;
        let _ = Label::builder().text("New name").bounds(0, 0, width, 20).parent(&content).build();
        let Ok(input) = TextBox::builder()
            .text(&field.text())
            .bounds(0, 24, width, DEFAULT_CONTROL_HEIGHT)
            .parent(&content)
            .build()
        else {
            return;
        };
        let Ok(hint) = Label::builder().text("").bounds(0, 58, width, 20).parent(&content).build() else { return };
        let (probe, warn) = (input.clone(), hint.clone());
        modal.on_close_request(move |result| {
            if result == DialogResult::Ok && probe.text().trim().is_empty() {
                warn.set_text("The name cannot be empty.");
                return false;
            }
            true
        });
        let (target, label) = (field.clone(), label.clone());
        let value = input.clone();
        let _ = modal.show_then(move |result| {
            if result == DialogResult::Ok {
                target.set_text(&value.text());
                label.set_text("File renamed.");
            }
        });
        input.select_all();
    });

    let owner = window.clone();
    alert.on_click(move || {
        let _ = Modal::alert(&owner, "Sync completed", "All files are up to date.");
    });

    let owner = window.clone();
    dark.on_value_changed(move |on| {
        if on {
            Theme::dark().apply(&owner);
        } else {
            Theme::light().apply(&owner);
        }
    });

    window.show();
    app.run()
}
