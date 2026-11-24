//! Modern controls — frameless window with its own title bar, dockable panes,
//! detachable tabs, `ToggleSwitch`, `NumberBox`, `Expander`,
//! `BreadcrumbBar`, `Card` and `RatingControl`.
//!
//! Try it: drag the window by the bar, use the bar's buttons; drag the
//! pane splitters, click "Float" in a pane's header (closing the floating
//! window gives it back); drag a tab out; open the expander; type
//! `2*3+1` in the number box and press Enter; click the breadcrumb trail.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    let window = Window::builder().title("Modern controls").size(980, 640).min_size(780, 520).centered().build()?;
    let bar = TitleBar::attach(&window)?;
    let search = TextBox::builder().placeholder("Search").parent(&bar).build()?;
    bar.add_control(&search, 220);

    let host = DockHost::builder().parent(&window).build()?;
    let root = StackLayout::vertical();
    root.add_with(&bar, LayoutParams::new().height(bar.height()));
    root.add_with(&host, LayoutParams::new().weight(1.0));
    window.set_layout(root);

    // Central area: trail + detachable tabs.
    let center = host.center();
    let path = BreadcrumbBar::builder().items(&["Home", "Documents", "Projects", "ArqRust"]).parent(&center).build()?;
    let trail = path.clone();
    path.on_item_click(move |index| trail.truncate(index + 1));
    let tabs = TabControl::builder().parent(&center).build()?;
    let center_layout = StackLayout::vertical().spacing(8).padding(Thickness::all(12));
    center_layout.add_with(&path, LayoutParams::new().height(32));
    center_layout.add_with(&tabs, LayoutParams::new().weight(1.0));
    center.set_layout(center_layout);
    tabs.set_detachable(true);
    let general = tabs.add_tab("General")?;
    let details = tabs.add_tab("Details")?;
    tabs.add_tab("Drag me out")?;

    let wifi = ToggleSwitch::builder().text("Wi-Fi").on(true).bounds(16, 16, 240, 28).parent(&general).build()?;
    let quantity = NumberBox::builder().range(0.0, 99.0).value(3.0).bounds(16, 56, 140, 28).parent(&general).build()?;
    let rating = RatingControl::builder().value(4.0).allow_half(true).position(16, 100).parent(&general).build()?;
    let status = Label::builder().text("").bounds(16, 140, 460, 24).parent(&general).build()?;
    let general_layout = StackLayout::vertical().spacing(12).padding(Thickness::all(16));
    general_layout.add_with(&wifi, LayoutParams::new().height(28));
    general_layout.add_with(&quantity, LayoutParams::new().width(140).height(28).align_h(Align::Start));
    general_layout
        .add_with(&rating, LayoutParams::new().width(rating.client_size().width).height(28).align_h(Align::Start));
    general_layout.add_with(&status, LayoutParams::new().height(24));
    general.set_layout(general_layout);
    let (s1, s2, s3) = (status.clone(), status.clone(), status.clone());
    wifi.on_toggled(move |on| s1.set_text(&format!("Wi-Fi: {}", if on { "on" } else { "off" })));
    quantity.on_value_changed(move |v| s2.set_text(&format!("Quantity: {v}")));
    rating.on_value_changed(move |v| s3.set_text(&format!("Rating: {v}")));

    let expander = Expander::builder()
        .header("Advanced options")
        .content_height(90)
        .bounds(16, 16, 460, 0)
        .parent(&details)
        .build()?;
    CheckBox::builder().text("Log diagnostics").bounds(14, 14, 240, 24).parent(&expander.content()).build()?;
    CheckBox::builder().text("Send reports").bounds(14, 48, 240, 24).parent(&expander.content()).build()?;
    let expanded = expander.clone();
    details.on_resize(move |size| {
        let height = expanded.bounds().height;
        expanded.set_bounds(Rect::new(16, 16, (size.width - 32).max(0), height));
    });

    // Panes.
    let files = host.add_pane("Files", DockSide::Left)?;
    let tree = TreeView::builder().parent(&files.content()).build()?;
    let docs = tree.add_root("Documents")?;
    tree.add_child(docs, "Report.docx")?;
    tree.add_child(docs, "Budget.xlsx")?;
    let files_content = files.content();
    let tree_fill = tree.clone();
    files_content.on_resize(move |size| tree_fill.set_bounds(Rect::new(0, 0, size.width, size.height)));
    let size = files_content.client_size();
    tree.set_bounds(Rect::new(0, 0, size.width, size.height));

    let cards = host.add_pane("Summary", DockSide::Right)?;
    let storage = Card::builder()
        .title("Storage")
        .subtitle("72% used")
        .bounds(12, 12, 230, 110)
        .parent(&cards.content())
        .build()?;
    let area = storage.content_bounds();
    ProgressBar::builder().value(72).bounds(area.x, area.y, area.width, 10).parent(&storage).build()?;
    let backup = Card::builder()
        .title("Backup")
        .subtitle("Click to run it now")
        .clickable(true)
        .bounds(12, 134, 230, 80)
        .parent(&cards.content())
        .build()?;
    let s4 = status.clone();
    backup.on_click(move || s4.set_text("Backup started"));

    let output = host.add_pane("Output", DockSide::Bottom)?;
    let log = ListBox::builder().items(&["Ready."]).parent(&output.content()).build()?;
    let output_content = output.content();
    let log_fill = log.clone();
    output_content.on_resize(move |size| log_fill.set_bounds(Rect::new(0, 0, size.width, size.height)));
    let size = output_content.client_size();
    log.set_bounds(Rect::new(0, 0, size.width, size.height));

    window.show();
    app.run()
}
