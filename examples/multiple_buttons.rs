//! Example 2 — Window with several buttons, a default button (Enter) and a cancel button (Esc).
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Multiple buttons").size(420, 220).centered().build()?;

    let status = Label::builder().text("Click a button.").bounds(20, 20, 380, 24).parent(&window).build()?;
    let total = Rc::new(Cell::new(0u32));

    let row = FlowLayout::new().gaps(10, 10).padding(Thickness::new(20, 60, 20, 20));
    for name in ["One", "Two", "Three", "Four", "Five"] {
        let button = Button::builder().text(name).size(110, 32).parent(&window).build()?;
        let (status, total) = (status.clone(), total.clone());
        button.on_click(move || {
            total.set(total.get() + 1);
            status.set_text(&format!("Button \"{name}\" clicked — {} click(s) in total.", total.get()));
        });
        row.add(&button);
    }

    let reset = Button::builder().text("&Reset").size(110, 32).default(true).parent(&window).build()?;
    reset.set_tooltip("Enter also triggers this button")?;
    let (status_reset, total_reset) = (status.clone(), total.clone());
    reset.on_click(move || {
        total_reset.set(0);
        status_reset.set_text("Counter reset.");
    });
    row.add(&reset);

    let close = Button::builder().text("&Close").size(110, 32).cancel(true).parent(&window).build()?;
    let target = window.clone();
    close.on_click(move || target.close());
    row.add(&close);

    window.set_layout(row);
    window.show();
    app.run()
}
