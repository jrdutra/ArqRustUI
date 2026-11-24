//! Example 1 — Hello World: equivalent to the original `helloWorldGui.rs`, now with the
//! library's high-level API.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;

    let window = Window::builder().title("My first GUI in Rust").size(500, 300).position(200, 200).build()?;

    let button = Button::builder().text("Click here").bounds(150, 100, 180, 40).parent(&window).build()?;
    let owner = window.clone();
    button.on_click(move || {
        MessageBox::show(&owner, "Rust + Win32", "Hello! Button clicked!", MessageButtons::Ok, MessageIcon::None);
    });

    window.show();
    app.run()
}
