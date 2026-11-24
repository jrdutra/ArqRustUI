//! MenuStrip — menu bar drawn by the library: icons, shortcuts, submenus,
//! checkable and radio items, a dynamic menu, light/dark theme, overflow (`»`)
//! and a context menu with the same drawing.
//!
//! Try it: Alt (or F10) and the arrows; Alt+F, Alt+E...; Ctrl+N, Ctrl+O, Ctrl+S;
//! make the window narrower to see the `»` button; right-click the log.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    let window = Window::builder().title("MenuStrip").size(640, 380).min_size(220, 160).centered().build()?;
    let log = ListBox::builder().parent(&window).build()?;
    let root = StackLayout::vertical().padding(Thickness::all(MARGIN));
    root.add_with(&log, LayoutParams::new().weight(1.0));
    window.set_layout(root);
    let record = {
        let log = log.clone();
        move |text: &str| {
            let index = log.add_item(text);
            log.set_selected_index(Some(index));
        }
    };

    // File: icons, shortcuts, a submenu with an icon and a disabled item.
    let file = Menu::new()?;
    let r = record.clone();
    file.add_item_with_shortcut("&New", Shortcut::ctrl(Key(b'N' as u16)), move || r("File > New"))?
        .set_icon(&icons::FILE)?;
    let r = record.clone();
    file.add_item_with_shortcut("&Open...", Shortcut::ctrl(Key(b'O' as u16)), move || r("File > Open"))?
        .set_icon(&icons::FOLDER_OPEN)?;
    let r = record.clone();
    let save = file.add_item_with_shortcut("&Save", Shortcut::ctrl(Key(b'S' as u16)), move || r("File > Save"))?;
    save.set_icon(&icons::SAVE)?;
    save.set_enabled(false);
    let recent = Menu::new()?;
    for name in ["report.txt", "notes.md", "data.json"] {
        let r = record.clone();
        recent.add_item(name, move || r(&format!("Recent > {name}")))?;
    }
    file.add_submenu_with_icon("&Recent", &icons::CLOCK, &recent)?;
    file.add_separator()?;
    let r = record.clone();
    file.add_item("&Print...\tCtrl+P", move || r("File > Print"))?.set_icon(&icons::PRINTER)?;
    file.add_separator()?;
    let target = window.clone();
    file.add_item("E&xit\tAlt+F4", move || target.close())?;

    // Edit: "Paste" is only enabled when there is text on the clipboard.
    let edit = Menu::new()?;
    let r = record.clone();
    edit.add_item_with_shortcut("Cu&t", Shortcut::ctrl(Key(b'X' as u16)), move || r("Edit > Cut"))?
        .set_icon(&icons::CUT)?;
    let r = record.clone();
    edit.add_item_with_shortcut("&Copy", Shortcut::ctrl(Key(b'C' as u16)), move || r("Edit > Copy"))?
        .set_icon(&icons::COPY)?;
    let r = record.clone();
    let paste = edit.add_item_with_shortcut("&Paste", Shortcut::ctrl(Key(b'V' as u16)), move || r("Edit > Paste"))?;
    paste.set_icon(&icons::CLIPBOARD)?;
    edit.add_separator()?;
    let r = record.clone();
    edit.add_item_with_shortcut("&Find...", Shortcut::ctrl(Key(b'F' as u16)), move || r("Edit > Find"))?
        .set_icon(&icons::SEARCH)?;
    edit.on_open(move |_| paste.set_enabled(Clipboard::text().ok().flatten().is_some_and(|t| !t.is_empty())));

    // View: checkable items and a radio group that changes the window theme.
    let view = Menu::new()?;
    let (r, s) = (record.clone(), save.clone());
    view.add_check_item("Allow &saving", false, move |on| {
        s.set_enabled(on);
        r(if on { "Save enabled" } else { "Save disabled" });
    })?;
    let (r, target) = (record.clone(), window.clone());
    view.add_check_item("Always on &top", false, move |on| {
        target.set_topmost(on);
        r(if on { "Window always on top" } else { "Normal window" });
    })?;
    view.add_separator()?;
    view.add_submenu_with_icon("T&heme", &icons::PALETTE, &theme_menu(&window, &record)?)?;
    let zoom = Menu::new()?;
    for percent in [50, 100, 150, 200] {
        let r = record.clone();
        zoom.add_radio_item(&format!("{percent}%"), percent == 100, move || r(&format!("Zoom {percent}%")))?;
    }
    view.add_submenu_with_icon("&Zoom", &icons::ZOOM_IN, &zoom)?;

    // Tools: menu rebuilt every time it opens.
    let tools = Menu::new()?;
    let opened = Rc::new(Cell::new(0u32));
    let r = record.clone();
    tools.on_open(move |menu| {
        opened.set(opened.get() + 1);
        menu.clear();
        let r2 = r.clone();
        let _ = menu.add_item(&format!("Opened {} time(s)", opened.get()), move || r2("Counter"));
        let _ = menu.add_separator();
        let r2 = r.clone();
        if let Ok(item) = menu.add_item("&Options...", move || r2("Tools > Options")) {
            let _ = item.set_icon(&icons::SETTINGS);
        }
    });

    let help = Menu::new()?;
    help.add_item("&About...", || MessageBox::info("About", "ArqRustUI MenuStrip example."))?.set_icon(&icons::INFO)?;

    let strip = MenuStrip::new()?;
    strip.add_menu("&File", &file)?;
    strip.add_menu("&Edit", &edit)?;
    strip.add_menu("&View", &view)?;
    strip.add_menu_with_icon("&Tools", &icons::TOOLS, &tools)?;
    strip.add_menu("&Help", &help)?;
    window.set_menu_strip(&strip)?;
    Theme::light().apply(&window);

    // Context menu with the same drawing as the bar.
    let context = ContextMenu::new()?;
    context.set_drawn(true);
    let list = log.clone();
    context.add_item("&Clear log", move || list.clear())?.set_icon(&icons::DELETE)?;
    context.add_separator()?;
    context.add_submenu_with_icon("&Theme", &icons::PALETTE, &theme_menu(&window, &record)?)?;
    log.set_context_menu(&context);

    record("Alt or F10 enter the menu; Alt+letter opens a menu; narrow the window to see »");
    window.show();
    app.run()
}

/// Radio submenu that changes the window theme (one per menu, since each submenu
/// belongs to a single parent).
fn theme_menu(window: &Window, record: &(impl Fn(&str) + Clone + 'static)) -> GuiResult<Menu> {
    let menu = Menu::new()?;
    let themes = [("&Light", Theme::light as fn() -> Theme), ("&Dark", Theme::dark), ("&System", Theme::system)];
    for (index, (label, make)) in themes.into_iter().enumerate() {
        let (r, target) = (record.clone(), window.clone());
        menu.add_radio_item(label, index == 0, move || {
            make().apply(&target);
            r(&format!("Theme: {}", label.replace('&', "")));
        })?;
    }
    Ok(menu)
}
