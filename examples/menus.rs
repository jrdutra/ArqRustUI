//! Example 7 — Menus: bar, submenus, checkable items, icons, dynamic menu,
//! context menu and integration with the system menu.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Menus").size(520, 300).centered().build()?;
    let log = ListBox::builder().bounds(10, 10, 500, 280).parent(&window).build()?;
    let record = {
        let log = log.clone();
        move |text: &str| {
            let index = log.add_item(text);
            log.set_selected_index(Some(index));
        }
    };

    let file = Menu::new()?;
    let r = record.clone();
    let _new_item = file.add_item_with_shortcut("&New", Shortcut::ctrl(Key(b'N' as u16)), move || r("File > New"))?;
    #[cfg(feature = "icons")]
    _new_item.set_icon(&icons::FILE)?;
    let r = record.clone();
    let save = file.add_item_with_shortcut("&Save", Shortcut::ctrl(Key(b'S' as u16)), move || r("File > Save"))?;
    save.set_enabled(false);

    let recent = Menu::new()?;
    file.add_submenu("&Recent", &recent)?;
    file.add_separator()?;
    let target = window.clone();
    file.add_item("E&xit\tAlt+F4", move || target.close())?;

    let view = Menu::new()?;
    let (r, save_item) = (record.clone(), save.clone());
    view.add_check_item("Allow &saving", false, move |on| {
        save_item.set_enabled(on);
        r(if on { "Save enabled" } else { "Save disabled" });
    })?;
    let (r, target) = (record.clone(), window.clone());
    view.add_check_item("Always on &top", false, move |on| {
        target.set_topmost(on);
        r(if on { "Window always on top" } else { "Normal window" });
    })?;

    // Dynamic menu: rebuilt for each new "document".
    let counter = Rc::new(Cell::new(0u32));
    let dynamic = Menu::new()?;
    let (r, recent_menu, count) = (record.clone(), recent.clone(), counter.clone());
    dynamic.add_item("Add item to Recent", move || {
        count.set(count.get() + 1);
        let name = format!("document{}.txt", count.get());
        let (r2, label) = (r.clone(), name.clone());
        if recent_menu.add_item(&name, move || r2(&format!("Recent > {label}"))).is_ok() {
            r(&format!("\"{name}\" added to the Recent submenu"));
        }
    })?;
    let (r, recent_menu) = (record.clone(), recent.clone());
    dynamic.add_item("Clear Recent", move || {
        recent_menu.clear();
        r("Recent submenu emptied");
    })?;

    let bar = MenuBar::new()?;
    bar.add_menu("&File", &file)?;
    bar.add_menu("&View", &view)?;
    bar.add_menu("&Dynamic", &dynamic)?;
    window.set_menu(&bar)?;

    let context = ContextMenu::new()?;
    let (r, list) = (record.clone(), log.clone());
    context.add_item("Clear log", move || {
        list.clear();
        r("Log cleared from the context menu");
    })?;
    let r = record.clone();
    context.add_item("Copy selection", {
        let list = log.clone();
        move || {
            if let Some(text) = list.selected_index().and_then(|i| list.item_text(i)) {
                if Clipboard::set_text(&text).is_ok() {
                    r("Selection copied to the clipboard");
                }
            }
        }
    })?;
    log.set_context_menu(&context);

    let system = window.system_menu()?;
    system.add_separator()?;
    let owner = window.clone();
    system.add_item("About this example...", move || {
        MessageBox::show(
            &owner,
            "About",
            "Item added to the window's system menu.",
            MessageButtons::Ok,
            MessageIcon::Information,
        );
    })?;

    record("Right-click the list for the context menu.");
    window.show();
    app.run()
}
