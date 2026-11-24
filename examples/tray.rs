//! Example 14 — Application with a tray icon, a menu and notifications.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The application stays alive in the tray even with the window closed.
    app.set_quit_on_last_window_closed(false);

    let window = Window::builder().title("System tray").size(380, 160).centered().build()?;
    Label::builder()
        .text("Closing this window only hides it.\nUse the icon in the notification area to reopen or exit.")
        .bounds(16, 16, 348, 40)
        .parent(&window)
        .build()?;
    let notify = Button::builder().text("Send notification").bounds(16, 80, 160, 30).parent(&window).build()?;

    let tray = TrayIcon::new(&Icon::system(SystemIcon::Application)?, "ArqRustUI — tray example")?;
    let restore = {
        let window = window.clone();
        move || {
            window.show();
            window.restore();
            window.activate();
        }
    };
    tray.on_click(restore.clone());
    tray.on_notification_click(restore.clone());

    let menu = ContextMenu::new()?;
    menu.add_item("Open window", restore)?;
    let sender = tray.clone();
    menu.add_item("Notify", move || {
        // Failing to show the notification (notifications turned off, for example) is not critical here.
        let _ = sender.show_notification("ArqRustUI", "Notification sent from the tray menu.", NotificationKind::Info);
    })?;
    menu.add_separator()?;
    menu.add_item("Exit", Application::quit)?;
    tray.set_menu(&menu);

    let (sender, owner) = (tray.clone(), window.clone());
    notify.on_click(move || {
        if let Err(error) =
            sender.show_notification("Task completed", "Processing finished successfully.", NotificationKind::Info)
        {
            MessageBox::show(&owner, "Notification", &error.to_string(), MessageButtons::Ok, MessageIcon::Warning);
        }
    });

    // Closing hides instead of destroying.
    let target = window.clone();
    window.on_close_request(move || {
        target.hide();
        false
    });
    window.show();
    app.run()
}
