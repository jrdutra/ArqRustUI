//! Standalone composition: scrolling, highlighted text and a footer fitted to the content.
#![windows_subsystem = "windows"]
use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Control composition").size(620, 400).build()?;
    let status = StatusBar::new(&window)?;
    status.set_part_text(0, "Ready");
    let scroll = ScrollViewport::new(&window)?;
    let content = scroll.content();
    let rows = StackLayout::vertical().spacing(4);
    for index in 1..=24 {
        let row = Label::builder().text(&format!("Item {index}")).parent(&content).build()?;
        rows.add_with(&row, LayoutParams::new().height(24));
    }
    content.set_layout(rows.clone());
    let (scrolling, rows_c) = (scroll.clone(), rows.clone());
    scroll.viewport().on_resize(move |_| {
        scrolling.layout_content(|_| rows_c.preferred_size().height);
    });
    let text = RichTextBox::builder().read_only(true).parent(&window).build()?;
    let mut document = StyledText::new("Segoe UI", 11.0);
    document.push_mapped("Hello Unicode: café 😀", |index, _| {
        TextFormat::new().highlight(if index < 5 { Color::rgb(210, 240, 255) } else { Color::WHITE })
    });
    text.set_rtf(&document.to_rtf())?;
    let toggle = Button::builder().text("Show/hide highlight").parent(&window).build()?;
    let body = StackLayout::vertical().spacing(8).padding(Thickness::new(12, 12, 12, 8));
    body.add_with(&scroll.viewport(), LayoutParams::new().height(180).weight(1.0));
    body.add_with(&text, LayoutParams::new().height(70));
    body.add_with(&toggle, LayoutParams::new().height(32));
    let layout = StatusBarLayout::new(body, &status).minimum_width(380);
    window.set_layout(layout.clone());
    layout.fit_height(&window);
    scroll.layout_content(|_| rows.preferred_size().height);
    let owner = window.clone();
    toggle.on_click(move || {
        text.set_visible(!text.is_visible());
        status.set_part_text(0, if text.is_visible() { "Highlight visible" } else { "Highlight hidden" });
        layout.fit_height(&owner);
    });
    window.show();
    app.run()
}
