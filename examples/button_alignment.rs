//! Horizontal alignment independent of the icon's relative position.
#![windows_subsystem = "windows"]
use arqrustui::prelude::*;
fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let w = Window::builder().title("Button alignment").size(440, 250).build()?;
    let layout = StackLayout::vertical().spacing(10).padding(Thickness::all(16));
    for (caption, alignment) in [("Left", TextAlign::Left), ("Center", TextAlign::Center), ("Right", TextAlign::Right)]
    {
        let b = Button::builder()
            .text(caption)
            .icon(&icons::FOLDER_OPEN)
            .content_alignment(alignment)
            .parent(&w)
            .build()?;
        layout.add_with(&b, LayoutParams::new().height(56));
    }
    w.set_layout(layout);
    w.show();
    app.run()
}
