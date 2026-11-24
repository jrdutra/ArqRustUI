//! Notices: `InfoBar` (a band inside the window) and `Toast` (a temporary notice in the corner).
//!
//! Try it: the buttons show notices of each severity; `×` closes the band and the
//! layout adjusts; toasts disappear on their own (the timer pauses while the cursor is over them),
//! stack up and follow the window; the theme button switches light/dark; "Windows
//! notification" sends a notice to the Action Center, with buttons.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    let window = Window::builder().title("Notices").size(640, 420).min_size(420, 300).centered().build()?;

    let notice = InfoBar::builder()
        .severity(Severity::Warning)
        .title("No connection.")
        .message("Changes will be sent when the network is back.")
        .parent(&window)
        .build()?;
    let target = notice.clone();
    notice.add_action("Try now", move || {
        target.open_with(Severity::Success, "Connected.", "All changes have been sent.");
    })?;

    let heading = Label::builder().text("Show a notice in the window:").parent(&window).build()?;
    let row1 = StackLayout::horizontal().spacing(SPACING);
    let samples = [
        (Severity::Info, "Information", "New version available.", "Restart to update."),
        (Severity::Success, "Success", "Backup completed.", "1,248 files copied."),
        (Severity::Warning, "Warning", "Low disk space.", "2 GB left on drive C:."),
        (Severity::Error, "Error", "Save failed.", "The file is read-only."),
    ];
    for (severity, label, title, message) in samples {
        let button = Button::builder().text(label).parent(&window).build()?;
        let target = notice.clone();
        button.on_click(move || target.open_with(severity, title, message));
        row1.add_with(&button, LayoutParams::new().width(110));
    }

    let heading2 = Label::builder().text("Show a toast in the corner:").parent(&window).build()?;
    let row2 = StackLayout::horizontal().spacing(SPACING);
    let counter = Rc::new(Cell::new(0));
    for (severity, label, title, message) in samples {
        let button = Button::builder().text(label).parent(&window).build()?;
        let (owner, count) = (window.clone(), counter.clone());
        button.on_click(move || {
            count.set(count.get() + 1);
            let mut toast = Toast::new(title).message(message).severity(severity);
            if severity == Severity::Error {
                toast = toast.persistent().action("Save as...", || {});
            } else if severity == Severity::Success {
                toast = toast.action("Open folder", || {}).duration(Duration::from_secs(8));
            }
            let _ = toast.show(&owner);
        });
        row2.add_with(&button, LayoutParams::new().width(110));
    }

    let theme = Button::builder().text("Toggle theme").parent(&window).build()?;
    let (target, dark) = (window.clone(), Rc::new(Cell::new(false)));
    theme.on_click(move || {
        dark.set(!dark.get());
        if dark.get() { Theme::dark() } else { Theme::light() }.apply(&target);
    });
    let system = Button::builder().text("Windows notification").parent(&window).build()?;
    let owner = window.clone();
    system.on_click(move || {
        let target = owner.clone();
        let result = SystemNotification::new("Report ready")
            .body("The March sales report has been generated.")
            .button("Open", "open")
            .button("Later", "later")
            .on_activated(move |argument| {
                let _ = Toast::new("Windows notification").message(&format!("Clicked: “{argument}”")).show(&target);
            })
            .show();
        if let Err(error) = result {
            let _ = Toast::new("Not available")
                .message(&format!("System notifications: {error}"))
                .severity(Severity::Warning)
                .show(&owner);
        }
    });
    let clear = Button::builder().text("Close toasts").parent(&window).build()?;
    let owner = window.clone();
    clear.on_click(move || Toast::dismiss_all(&owner));

    let root = StackLayout::vertical().spacing(SPACING).padding(Thickness::all(MARGIN));
    root.add_with(&notice, LayoutParams::new().height(INFO_BAR_HEIGHT));
    root.add_with(&heading, LayoutParams::new().height(20));
    root.add_with(row1, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
    root.add_with(&heading2, LayoutParams::new().height(20));
    root.add_with(row2, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
    let row3 = StackLayout::horizontal().spacing(SPACING);
    row3.add_with(&theme, LayoutParams::new().width(130));
    row3.add_with(&clear, LayoutParams::new().width(130));
    row3.add_with(&system, LayoutParams::new().width(180));
    root.add_with(row3, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
    window.set_layout(root);
    Theme::light().apply(&window);

    window.show();
    Toast::new("Welcome").message("Notices appear here in the corner of the window.").show(&window)?;
    app.run()
}
