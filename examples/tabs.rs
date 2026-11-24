//! Example 8 — Application with tabs, each one with its own content and layout.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Tabs").size(520, 360).min_size(360, 260).centered().build()?;
    let tabs = TabControl::builder().parent(&window).build()?;
    let root = DockLayout::new().padding(Thickness::all(8));
    root.add(&tabs, Dock::Fill);
    window.set_layout(root);

    // Tab 1: preferences.
    let general = tabs.add_tab("General")?;
    let column = StackLayout::vertical().spacing(8).padding(Thickness::all(12));
    let start = CheckBox::builder().text("Start with Windows").checked(true).parent(&general).build()?;
    let updates = CheckBox::builder().text("Check for updates automatically").parent(&general).build()?;
    let language =
        ComboBox::builder().items(&["English", "Português", "Español"]).selected(0).parent(&general).build()?;
    column.add_with(&start, LayoutParams::new().height(22));
    column.add_with(&updates, LayoutParams::new().height(22));
    column.add_with(&language, LayoutParams::new().height(24).width(200).align_h(Align::Start));
    general.set_layout(column);

    // Tab 2: appearance.
    let appearance = tabs.add_tab("Appearance")?;
    Label::builder().text("Window opacity:").bounds(12, 14, 200, 20).parent(&appearance).build()?;
    let opacity =
        Slider::builder().range(40, 100).value(100).ticks(10).bounds(12, 38, 300, 32).parent(&appearance).build()?;
    let target = window.clone();
    opacity.on_value_changed(move |value| target.set_opacity(value as f32 / 100.0));
    let group = GroupBox::builder().text("Theme").bounds(12, 84, 300, 80).parent(&appearance).build()?;
    let light = RadioButton::builder()
        .text("Light")
        .bounds(14, 24, 120, 20)
        .group_start(true)
        .checked(true)
        .parent(&group)
        .build()?;
    let dark = RadioButton::builder().text("Dark").bounds(14, 48, 120, 20).parent(&group).build()?;
    let target = window.clone();
    light.on_value_changed(move |on| {
        if on {
            Theme::light().apply(&target)
        }
    });
    let target = window.clone();
    dark.on_value_changed(move |on| {
        if on {
            Theme::dark().apply(&target)
        }
    });

    // Tab 3: about.
    let about = tabs.add_tab("About")?;
    let text = format!(
        "ArqRustUI — native GUI in pure Rust.\r\nWindows {:?}\r\nLocale: {}",
        WindowsVersion::current(),
        Locale::user_default().display_name()
    );
    let info = TextBox::builder().multiline(true).read_only(true).text(&text).parent(&about).build()?;
    let fill = DockLayout::new().padding(Thickness::all(12));
    fill.add(&info, Dock::Fill);
    about.set_layout(fill);

    let title = window.clone();
    let names = tabs.clone();
    tabs.on_selection_changed(move |index| title.set_title(&format!("Tabs — {}", names.tab_title(index))));
    window.show();
    app.run()
}
