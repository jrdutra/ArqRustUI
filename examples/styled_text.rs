//! Text by runs, lists, a table and an image in the RichTextBox, an IME candidate list
//! with the window's theme and native file dialogs.
use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    let window = Window::builder().title("Formatted text").size(620, 300).build()?;
    let view = RichTextBox::builder().parent(&window).build()?;
    let mut document = StyledText::new("Segoe UI", 11.0);
    document.push("Plain text. ", TextFormat::new());
    document.push("Unicode highlight: café 😀", TextFormat::new().highlight(Color::rgb(220, 240, 255)).bold(true));
    view.set_rtf(&document.to_rtf())?;
    // Numbered list, table and an image (an icon from the pack) inserted at the end.
    let end = view.text().encode_utf16().count();
    view.set_selection(end, end);
    view.replace_selection("\rFirst step\rSecond step\r");
    let added = "\rFirst step\rSecond step\r".encode_utf16().count();
    view.set_selection(end, end + added);
    view.format_selection(&TextFormat::new().bold(false).highlight(Color::WHITE));
    view.set_selection(end + 1, end + added - 1);
    view.set_list_style(ListStyle::Numbered(1));
    let end = view.text().encode_utf16().count();
    view.set_selection(end, end);
    let rows = vec![
        vec!["Item".to_string(), "Quantity".to_string()],
        vec!["Coffee".to_string(), "2".to_string()],
        vec!["Bread".to_string(), "6".to_string()],
    ];
    view.insert_table(&rows, &[160, 100], true)?;
    if let Ok(image) = icons::CHECK.image() {
        view.insert_image(&image, 32, 32)?;
    }
    // IME candidates (Chinese, Japanese...) drawn with the window's colors.
    let _candidates = Ime::use_candidate_window(&view)?;
    let open = Button::builder().text("Open…").icon(&icons::FOLDER_OPEN).parent(&window).build()?;
    let save = Button::builder().text("Save as…").icon(&icons::SAVE).parent(&window).build()?;
    let row = StackLayout::horizontal().spacing(8);
    row.add_with(&open, LayoutParams::new().width(110));
    row.add_with(&save, LayoutParams::new().width(140));
    let layout = StackLayout::vertical().spacing(8).padding(Thickness::all(12));
    layout.add_with(&view, LayoutParams::new().weight(1.0));
    layout.add_with(row, LayoutParams::new().height(32));
    window.set_layout(layout);
    let (owner, field) = (window.clone(), view.clone());
    open.on_click(move || {
        let result = (|| -> GuiResult<()> {
            if let Some(path) = OpenFileDialog::new().filter("UTF-8 text", "*.txt").filter("All", "*.*").show(&owner)? {
                let content = std::fs::read_to_string(path).map_err(|e| GuiError::InvalidArgument(e.to_string()))?;
                field.set_text(&content);
            }
            Ok(())
        })();
        if let Err(e) = result {
            MessageBox::error("Open", &e.to_string());
        }
    });
    let owner = window.clone();
    save.on_click(move || {
        let result = (|| -> GuiResult<()> {
            if let Some(path) =
                SaveFileDialog::new().default_extension("txt").filter("UTF-8 text", "*.txt").show(&owner)?
            {
                std::fs::write(path, view.text()).map_err(|e| GuiError::InvalidArgument(e.to_string()))?;
            }
            Ok(())
        })();
        if let Err(e) = result {
            MessageBox::error("Save", &e.to_string());
        }
    });
    window.show();
    app.run()
}
