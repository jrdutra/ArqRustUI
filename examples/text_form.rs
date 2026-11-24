//! Example 3 — Form with text fields arranged in a grid.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Registration").size(460, 300).min_size(360, 260).centered().build()?;

    let grid = GridLayout::new(
        &[Track::Auto, Track::Star(1.0)],
        &[Track::Fixed(26), Track::Fixed(26), Track::Fixed(26), Track::Star(1.0), Track::Fixed(30)],
    )
    .gaps(8, 8)
    .padding(Thickness::all(14));

    let field = |row: usize, caption: &str, placeholder: &str| -> GuiResult<TextBox> {
        let label = Label::builder().text(caption).size(90, 20).parent(&window).build()?;
        let text = TextBox::builder().placeholder(placeholder).parent(&window).build()?;
        grid.add_with(&label, row, 0, LayoutParams::new().align_v(Align::Center).height(20));
        grid.add(&text, row, 1);
        Ok(text)
    };
    let name = field(0, "&Name:", "Full name")?;
    let email = field(1, "&Email:", "name@example.com")?;
    let phone = field(2, "&Phone:", "(000) 000-0000")?;

    let notes_label = Label::builder().text("N&otes:").size(90, 20).parent(&window).build()?;
    let notes = TextBox::builder().multiline(true).parent(&window).build()?;
    grid.add_with(&notes_label, 3, 0, LayoutParams::new().align_v(Align::Start).height(20));
    grid.add(&notes, 3, 1);

    let save = Button::builder().text("&Save").default(true).parent(&window).build()?;
    grid.add_with(&save, 4, 1, LayoutParams::new().width(110).align_h(Align::End));

    let owner = window.clone();
    save.on_click(move || {
        let summary = format!(
            "Name: {}\nEmail: {}\nPhone: {}\nNotes: {} character(s)",
            name.text(),
            email.text(),
            phone.text(),
            notes.text().chars().count()
        );
        MessageBox::show(&owner, "Data entered", &summary, MessageButtons::Ok, MessageIcon::Information);
    });

    window.set_layout(grid);
    window.show();
    app.run()
}
