//! CommandBar — command bar drawn by the library: icon-only buttons,
//! text buttons, toggles, menu and split buttons, separators, flexible
//! space, a search field and the `»` button for whatever does not fit.
//!
//! Try it: hover over the buttons (tips); open "Page" and move to
//! "Tools" without clicking; use the arrow of "Home"; type in the search box and press
//! Enter (Esc clears); make the window narrower; Tab into the bar and use the arrows.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    let window = Window::builder().title("CommandBar").size(760, 420).min_size(240, 200).centered().build()?;
    let bar = CommandBar::builder().parent(&window).build()?;
    let log = ListBox::builder().parent(&window).build()?;
    let record = {
        let log = log.clone();
        move |text: &str| {
            let index = log.add_item(text);
            log.set_selected_index(Some(index));
        }
    };

    // Navigation: icons only, with tips.
    for (text, icon) in [("Back", &icons::BACK), ("Forward", &icons::FORWARD), ("Refresh", &icons::REFRESH)] {
        let r = record.clone();
        bar.add_button(CommandButton::new(text).icon(icon).icon_only().on_click(move || r(text)))?;
    }
    let r = record.clone();
    let stop =
        bar.add_button(CommandButton::new("Stop").icon(&icons::CLOSE).icon_only().on_click(move || r("Stop")))?;
    stop.set_enabled(false);
    bar.add_separator()?;

    // Split button: a click goes to the home page; the arrow opens the list.
    let homes = Menu::new()?;
    for page in ["Company portal", "Documentation", "Control panel"] {
        let r = record.clone();
        homes.add_item(page, move || r(&format!("Home: {page}")))?.set_icon(&icons::GLOBE)?;
    }
    homes.add_separator()?;
    let r = record.clone();
    homes.add_item("&Set home page...", move || r("Set home page"))?.set_icon(&icons::SETTINGS)?;
    let r = record.clone();
    bar.add_button(CommandButton::new("Home").icon(&icons::HOME).split(&homes).on_click(move || r("Home")))?;

    // Toggle.
    let (r, s) = (record.clone(), stop.clone());
    bar.add_button(
        CommandButton::new("Favorites")
            .icon(&icons::FAVORITE)
            .icon_only()
            .tooltip("Show favorites")
            .toggle(false)
            .on_toggled(move |on| {
                s.set_enabled(on);
                r(if on { "Favorites visible" } else { "Favorites hidden" });
            }),
    )?;
    bar.add_separator()?;

    // Menu buttons (the menu uses the same drawing as the MenuStrip).
    let page = Menu::new()?;
    let r = record.clone();
    page.add_item_with_shortcut("&Save as...", Shortcut::ctrl(Key(b'S' as u16)), move || r("Save as"))?
        .set_icon(&icons::SAVE)?;
    let r = record.clone();
    page.add_item("Send by &email...", move || r("Send by email"))?.set_icon(&icons::MAIL)?;
    page.add_separator()?;
    let zoom = Menu::new()?;
    for percent in [75, 100, 125, 150] {
        let r = record.clone();
        zoom.add_radio_item(&format!("{percent}%"), percent == 100, move || r(&format!("Zoom {percent}%")))?;
    }
    page.add_submenu_with_icon("&Zoom", &icons::ZOOM_IN, &zoom)?;
    let r = record.clone();
    page.add_check_item("&Reading mode", false, move |on| r(if on { "Reading on" } else { "Reading off" }))?;
    bar.add_button(CommandButton::new("Page").icon(&icons::FILE).menu(&page))?;

    let tools = Menu::new()?;
    let r = record.clone();
    tools.add_item("&Print...", move || r("Print"))?.set_icon(&icons::PRINTER)?;
    let r = record.clone();
    tools.add_item("&Downloads", move || r("Downloads"))?.set_icon(&icons::DOWNLOAD)?;
    tools.add_separator()?;
    let theme = Menu::new()?;
    let themes = [("&Light", Theme::light as fn() -> Theme), ("&Dark", Theme::dark), ("&System", Theme::system)];
    for (index, (label, make)) in themes.into_iter().enumerate() {
        let (r, target) = (record.clone(), window.clone());
        theme.add_radio_item(label, index == 0, move || {
            make().apply(&target);
            r(&format!("Theme: {}", label.replace('&', "")));
        })?;
    }
    tools.add_submenu_with_icon("&Theme", &icons::PALETTE, &theme)?;
    let r = record.clone();
    tools.add_item("&Options...", move || r("Options"))?.set_icon(&icons::SETTINGS)?;
    bar.add_button(CommandButton::new("Tools").icon(&icons::TOOLS).menu(&tools))?;

    // On the right: help and search.
    bar.add_spacer()?;
    let r = record.clone();
    bar.add_button(CommandButton::new("Help").icon(&icons::HELP).icon_only().on_click(move || r("Help")))?;
    let r = record.clone();
    let search = bar.add_search("Search", 220, move |text| r(&format!("Search: \"{text}\"")))?;
    let r = record.clone();
    let field = search.clone();
    search.on_text_changed(move || {
        if field.text().len() == 3 {
            r("(search also reports every typed letter)");
        }
    });

    let root = StackLayout::vertical();
    root.add_with(&bar, LayoutParams::new().height(bar.height()));
    let body = StackLayout::vertical().padding(Thickness::all(MARGIN));
    body.add_with(&log, LayoutParams::new().weight(1.0));
    root.add_with(body, LayoutParams::new().weight(1.0));
    window.set_layout(root);
    Theme::light().apply(&window);

    record("Hover over the buttons, open the menus, search and narrow the window to see »");
    window.show();
    app.run()
}
