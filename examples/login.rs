//! Example 4 — Login with user name and password in a modal dialog.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let main = Window::builder().title("Restricted area").size(420, 200).centered().build()?;
    let greeting = Label::builder().text("Waiting for login...").bounds(20, 20, 380, 24).parent(&main).build()?;
    main.show();

    let dialog = Dialog::new(&main, "Sign in", 320, 170)?;
    let parent = dialog.window();
    Label::builder().text("&User:").bounds(16, 20, 70, 20).parent(parent).build()?;
    let user = TextBox::builder().bounds(92, 16, 212, DEFAULT_CONTROL_HEIGHT).max_length(32).parent(parent).build()?;
    Label::builder().text("&Password:").bounds(16, 54, 70, 20).parent(parent).build()?;
    let password = PasswordBox::builder().bounds(92, 50, 212, DEFAULT_CONTROL_HEIGHT).parent(parent).build()?;
    let reveal = CheckBox::builder().text("&Show password").bounds(92, 82, 212, 20).parent(parent).build()?;
    let error = Label::builder().bounds(16, 108, 288, 18).parent(parent).build()?;
    error.set_foreground(Color::RED);
    let enter = Button::builder().text("Sign in").bounds(112, 132, 92, 28).default(true).parent(parent).build()?;
    let cancel = Button::builder().text("Cancel").bounds(212, 132, 92, 28).cancel(true).parent(parent).build()?;

    let field = password.clone();
    reveal.on_value_changed(move |show| field.set_revealed(show));

    let (handle, user_field, password_field) = (dialog.clone(), user.clone(), password.clone());
    enter.on_click(move || {
        // Fixed credentials, for demonstration only.
        if user_field.text() == "admin" && password_field.password() == "1234" {
            handle.close_with(DialogResult::Ok);
        } else {
            error.set_text("Invalid user name or password (use admin / 1234).");
            password_field.set_text("");
            password_field.focus();
        }
    });
    let handle = dialog.clone();
    cancel.on_click(move || handle.close_with(DialogResult::Cancel));
    user.focus();

    match dialog.show_modal()? {
        DialogResult::Ok => greeting.set_text(&format!("Welcome, {}!", user.text())),
        _ => {
            main.close();
            return Ok(());
        }
    }
    app.run()
}
