//! Example 11 — Native dialogs: open, save, folder, color, font, TaskDialog and printing.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    let window = Window::builder().title("Dialogs").size(560, 330).centered().build()?;
    let output = TextBox::builder().multiline(true).read_only(true).parent(&window).build()?;
    let buttons = FlowLayout::new().gaps(8, 8);
    let root = StackLayout::vertical().spacing(8).padding(Thickness::all(10));
    root.add_with(&buttons, LayoutParams::new().height(72));
    root.add_with(&output, LayoutParams::new().weight(1.0));

    let log = {
        let output = output.clone();
        move |line: String| output.append(&format!("{line}\r\n"))
    };
    let add = |caption: &str, action: Box<dyn Fn()>| -> GuiResult<()> {
        let button = Button::builder().text(caption).size(128, 30).parent(&window).build()?;
        button.on_click(action);
        buttons.add(&button);
        Ok(())
    };

    let (w, l) = (window.clone(), log.clone());
    add(
        "Open file...",
        Box::new(move || {
            match OpenFileDialog::new()
                .title("Choose a file")
                .filter("Text", "*.txt;*.md")
                .filter("All", "*.*")
                .show(&w)
            {
                Ok(Some(path)) => l(format!("Open: {}", path.display())),
                Ok(None) => l("Open: cancelled".into()),
                Err(error) => l(format!("Open: error — {error}")),
            }
        }),
    )?;
    let (w, l) = (window.clone(), log.clone());
    add(
        "Open several...",
        Box::new(move || match OpenFileDialog::new().show_multiple(&w) {
            Ok(paths) => l(format!("Several: {} file(s)", paths.len())),
            Err(error) => l(format!("Several: error — {error}")),
        }),
    )?;
    let (w, l) = (window.clone(), log.clone());
    add(
        "Save as...",
        Box::new(move || {
            match SaveFileDialog::new().file_name("document").default_extension("txt").filter("Text", "*.txt").show(&w)
            {
                Ok(Some(path)) => l(format!("Save: {}", path.display())),
                Ok(None) => l("Save: cancelled".into()),
                Err(error) => l(format!("Save: error — {error}")),
            }
        }),
    )?;
    let (w, l) = (window.clone(), log.clone());
    add(
        "Choose folder...",
        Box::new(move || match FolderPicker::new().title("Select a folder").show(&w) {
            Ok(Some(path)) => l(format!("Folder: {}", path.display())),
            Ok(None) => l("Folder: cancelled".into()),
            Err(error) => l(format!("Folder: error — {error}")),
        }),
    )?;
    let (w, l, target) = (window.clone(), log.clone(), output.clone());
    add(
        "Color...",
        Box::new(move || match ColorDialog::new().initial(Color::BLUE).show(&w) {
            Ok(Some(color)) => {
                target.set_foreground(color);
                l(format!("Color: #{:02X}{:02X}{:02X}", color.r, color.g, color.b));
            }
            Ok(None) => l("Color: cancelled".into()),
            Err(error) => l(format!("Color: error — {error}")),
        }),
    )?;
    let (w, l, target) = (window.clone(), log.clone(), output.clone());
    add(
        "Font...",
        Box::new(move || match FontDialog::new().initial(&target.font()).show(&w) {
            Ok(Some(choice)) => {
                l(format!("Font: {} {} pt", choice.font.family(), choice.font.size()));
                target.set_font(choice.font);
            }
            Ok(None) => l("Font: cancelled".into()),
            Err(error) => l(format!("Font: error — {error}")),
        }),
    )?;
    let (w, l) = (window.clone(), log.clone());
    add(
        "TaskDialog...",
        Box::new(move || {
            let result = TaskDialog::new()
                .title("Sync")
                .heading("How do you want to resolve the conflict?")
                .content("The file was changed on this computer and in the cloud.")
                .icon(TaskIcon::Warning)
                .button(100, "Keep the local version\nThe cloud version will be replaced.")
                .button(101, "Use the cloud version\nLocal changes will be discarded.")
                .command_links(true)
                .cancel_button(true)
                .verification("Remember my choice", false)
                .show(&w);
            match result {
                Ok(answer) => l(format!("TaskDialog: {:?}, remember = {}", answer.button, answer.verification_checked)),
                Err(error) => l(format!("TaskDialog: error — {error}")),
            }
        }),
    )?;
    let (w, l) = (window.clone(), log.clone());
    add(
        "Print...",
        Box::new(move || match PrintDialog::new().show(&w) {
            Ok(Some(printer)) => {
                let printed = printer.print("ArqRustUI example", |page, canvas| {
                    let size = canvas.size();
                    canvas.text(
                        &format!("Page {} printed with ArqRustUI", page + 1),
                        Point::new(48, 48),
                        &Font::new("Segoe UI", 16.0),
                        Color::BLACK,
                    );
                    canvas.rect(
                        Rect::new(40, 40, size.width - 80, size.height - 80),
                        Some(&Pen::new(Color::GRAY, 1)),
                        None,
                    );
                    false
                });
                l(format!("Print: {printed:?}"));
            }
            Ok(None) => l("Print: cancelled".into()),
            Err(error) => l(format!("Print: error — {error}")),
        }),
    )?;
    let (w, l) = (window.clone(), log.clone());
    add(
        "Print preview...",
        Box::new(move || {
            // Three pages; the print dialog opened by the preview offers "From/To".
            let doc = PrintDocument::new("Sample report", 3, |page, canvas| {
                let size = canvas.size();
                canvas.text(
                    &format!("Report — page {} of 3", page + 1),
                    Point::new(48, 48),
                    &Font::new("Segoe UI", 16.0),
                    Color::BLACK,
                );
                for line in 0..20 {
                    let y = 110 + line * 36;
                    if y + 20 > size.height - 48 {
                        break;
                    }
                    canvas
                        .fill_rect(Rect::new(48, y, (size.width - 96) * (20 - line) / 20, 20), Color::rgb(0, 120, 215));
                }
                canvas.rect(
                    Rect::new(40, 40, size.width - 80, size.height - 80),
                    Some(&Pen::new(Color::GRAY, 1)),
                    None,
                );
            });
            match PrintPreviewDialog::new(doc).show(&w) {
                Ok(Some(pages)) => l(format!("Preview: {pages} page(s) printed")),
                Ok(None) => l("Preview: closed without printing".into()),
                Err(error) => l(format!("Preview: error — {error}")),
            }
        }),
    )?;

    window.set_layout(root);
    window.show();
    app.run()
}
