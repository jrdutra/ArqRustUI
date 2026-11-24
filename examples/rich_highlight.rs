//! Visual formatting independent of any language or parser.
#![windows_subsystem = "windows"]
use arqrustui::prelude::*;
fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Styles by range").size(540, 300).build()?;
    let text = RichTextBox::builder().parent(&window).build()?;
    text.set_text("Title: editable sample text.");
    text.apply_highlights(
        &TextFormat::new().color(Color::BLACK),
        &[(0..5, TextFormat::new().color(Color::rgb(30, 80, 180)).bold(true))],
    )?;
    let layout = StackLayout::vertical().padding(Thickness::all(12));
    layout.add_with(&text, LayoutParams::new().weight(1.0));
    window.set_layout(layout);
    window.show();
    app.run()
}
