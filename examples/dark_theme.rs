//! Example 12 — Window with a dark theme, theme switching and following the system.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Dark theme").size(480, 330).centered().build()?;

    let column = StackLayout::vertical().spacing(10).padding(Thickness::all(16));
    let heading = Label::builder().text("Appearance").parent(&window).build()?;
    heading.set_font(Font::system().with_size(16.0).bold());
    let description = Label::builder()
        .text(&format!("Detected system theme: {:?}", SystemTheme::current()))
        .parent(&window)
        .build()?;
    let name = TextBox::builder().placeholder("Text field").parent(&window).build()?;
    let options =
        ComboBox::builder().items(&["Option A", "Option B", "Option C"]).selected(0).parent(&window).build()?;
    let check = CheckBox::builder().text("Check box").checked(true).parent(&window).build()?;
    let list = ListBox::builder().items(&["Item 1", "Item 2", "Item 3"]).parent(&window).build()?;
    let progress = ProgressBar::builder().value(65).parent(&window).build()?;
    let toggle = Button::builder().text("Toggle light/dark").parent(&window).build()?;
    let accent = Button::builder().text("Accent button").parent(&window).build()?;
    accent.set_style(Some(ButtonStyle::from_accent(Color::rgb(0, 120, 212), Color::WHITE)));

    column.add_with(&heading, LayoutParams::new().height(30));
    column.add_with(&description, LayoutParams::new().height(20));
    column.add_with(&name, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
    column.add_with(&options, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
    column.add_with(&check, LayoutParams::new().height(22));
    column.add_with(&list, LayoutParams::new().weight(1.0));
    column.add_with(&progress, LayoutParams::new().height(16));
    let buttons = StackLayout::horizontal().spacing(8);
    buttons.add_with(&toggle, LayoutParams::new().width(170));
    buttons.add_with(&accent, LayoutParams::new().width(150));
    column.add_with(buttons, LayoutParams::new().height(30));
    window.set_layout(column);

    let dark = Rc::new(Cell::new(true));
    Theme::dark().apply(&window);
    // DWM features vary by version; their absence is not an error for this example.
    let _ = window.set_corner_style(CornerStyle::Round);

    let (target, state) = (window.clone(), dark.clone());
    toggle.on_click(move || {
        state.set(!state.get());
        if state.get() { Theme::dark() } else { Theme::light() }.apply(&target);
    });
    // Follows theme changes made in Windows Settings.
    let (target, state, text) = (window.clone(), dark.clone(), description.clone());
    window.on_theme_changed(move || {
        let system = SystemTheme::current();
        text.set_text(&format!("Detected system theme: {system:?}"));
        state.set(system == SystemTheme::Dark);
        Theme::system().apply(&target);
    });

    window.show();
    app.run()
}
