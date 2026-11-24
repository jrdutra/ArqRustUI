//! Example 20 — General demo: brings the implemented components together in a
//! single application with a menu, toolbar, tabs and status bar.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::time::Duration;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    // Without `.icon(...)`, the window gets the library logo. An explicit icon
    // in the builder or `window.set_icon(...)` replaces this default.
    let window = Window::builder().title("ArqRustUI — demo").size(820, 560).min_size(640, 440).centered().build()?;

    let status = StatusBar::new(&window)?;
    status.set_parts(&[300, 160, -1]);
    let say = {
        let status = status.clone();
        move |text: &str| status.set_part_text(0, text)
    };

    let toolbar = ToolBar::new(&window)?;
    let s = say.clone();
    #[cfg(feature = "icons")]
    let new_icon: Option<&dyn IconSource> = Some(&icons::FILE);
    #[cfg(not(feature = "icons"))]
    let new_icon: Option<&dyn IconSource> = None;
    toolbar.add_button("New", new_icon, move || s("Toolbar: New"))?;
    let s = say.clone();
    toolbar
        .add_button("Information", Some(&Icon::system(SystemIcon::Information)?), move || s("Toolbar: Information"))?;
    toolbar.add_separator()?;
    let s = say.clone();
    toolbar.add_toggle_button("Pin", None, move |on| s(if on { "Pinned" } else { "Unpinned" }))?;

    let file = Menu::new()?;
    let target = window.clone();
    file.add_item_with_shortcut("E&xit", Shortcut::ctrl(Key(b'Q' as u16)), move || target.close())?;
    let view = Menu::new()?;
    let target = window.clone();
    view.add_check_item("&Dark theme", false, move |dark| {
        if dark { Theme::dark() } else { Theme::light() }.apply(&target)
    })?;
    let help = Menu::new()?;
    let owner = window.clone();
    help.add_item("&About...", move || {
        // On systems without TaskDialog, the classic message box is used.
        let shown = TaskDialog::new()
            .title("About")
            .heading("ArqRustUI")
            .content("Native Windows GUI library in pure Rust, with no external dependencies.")
            .icon(TaskIcon::Information)
            .show(&owner);
        if shown.is_err() {
            MessageBox::show(
                &owner,
                "About",
                "ArqRustUI — native GUI in pure Rust.",
                MessageButtons::Ok,
                MessageIcon::Information,
            );
        }
    })?;
    let bar = MenuBar::new()?;
    bar.add_menu("&File", &file)?;
    bar.add_menu("&View", &view)?;
    bar.add_menu("&Help", &help)?;
    window.set_menu(&bar)?;

    let tabs = TabControl::builder().parent(&window).build()?;
    let root = DockLayout::new().padding(Thickness::new(8, toolbar.height() + 8, 8, status.height() + 8));
    root.add(&tabs, Dock::Fill);
    window.set_layout(root);

    // ---- Tab: basic controls ----
    let basic = tabs.add_tab("Basic")?;
    Label::builder()
        .text("&Text:")
        .single_line(true)
        .bounds(12, 10, 70, DEFAULT_CONTROL_HEIGHT)
        .parent(&basic)
        .build()?;
    let text = TextBox::builder()
        .bounds(86, 10, 200, DEFAULT_CONTROL_HEIGHT)
        .placeholder("Type something")
        .parent(&basic)
        .build()?;
    Label::builder()
        .text("&Password:")
        .single_line(true)
        .bounds(12, 42, 70, DEFAULT_CONTROL_HEIGHT)
        .parent(&basic)
        .build()?;
    PasswordBox::builder().bounds(86, 42, 200, DEFAULT_CONTROL_HEIGHT).parent(&basic).build()?;
    let combo = ComboBox::builder()
        .bounds(86, 74, 200, DEFAULT_CONTROL_HEIGHT)
        .items(&["Apple", "Banana", "Orange"])
        .selected(0)
        .parent(&basic)
        .build()?;
    Label::builder()
        .text("F&ruit:")
        .single_line(true)
        .bounds(12, 74, 70, DEFAULT_CONTROL_HEIGHT)
        .parent(&basic)
        .build()?;
    let check =
        CheckBox::builder().text("Check box").bounds(12, 110, 180, DEFAULT_CONTROL_HEIGHT).parent(&basic).build()?;
    let group = GroupBox::builder().text("Options").bounds(12, 140, 274, 76).parent(&basic).build()?;
    RadioButton::builder()
        .text("First")
        .bounds(12, 22, 120, 20)
        .group_start(true)
        .checked(true)
        .parent(&group)
        .build()?;
    RadioButton::builder().text("Second").bounds(12, 46, 120, 20).parent(&group).build()?;
    ToggleButton::builder().text("Toggle").bounds(150, 30, 100, 28).parent(&group).build()?;
    let list = ListBox::builder()
        .bounds(304, 10, 180, 120)
        .items(&["Item A", "Item B", "Item C", "Item D"])
        .parent(&basic)
        .build()?;
    let slider = Slider::builder().bounds(304, 140, 180, 30).ticks(10).value(40).parent(&basic).build()?;
    let progress = ProgressBar::builder().bounds(304, 178, 180, 16).value(40).parent(&basic).build()?;
    let spin = SpinButton::builder()
        .bounds(304, 204, 80, DEFAULT_CONTROL_HEIGHT)
        .range(0, 100)
        .value(40)
        .parent(&basic)
        .build()?;
    ScrollBar::builder().bounds(500, 10, 18, 200).vertical(true).parent(&basic).build()?;
    DatePicker::builder().bounds(530, 10, 140, DEFAULT_CONTROL_HEIGHT).parent(&basic).build()?;
    TimePicker::builder().bounds(530, 42, 110, DEFAULT_CONTROL_HEIGHT).parent(&basic).build()?;
    let link = Hyperlink::builder()
        .text("Visit the <a href=\"https://www.rust-lang.org\">Rust website</a>")
        .bounds(530, 78, 240, 20)
        .parent(&basic)
        .build()?;
    link.open_links_in_browser();
    IpAddressBox::builder().bounds(530, 108, 150, 24).parent(&basic).build()?;
    HotKeyBox::builder().bounds(530, 140, 150, DEFAULT_CONTROL_HEIGHT).parent(&basic).build()?;
    let run = Button::builder()
        .text("&Run task")
        .bounds(530, 176, 150, 30)
        .tooltip("Runs work in the background")
        .parent(&basic)
        .build()?;

    let s = say.clone();
    let source = text.clone();
    text.on_text_changed(move || s(&format!("Text: {}", source.text())));
    let s = say.clone();
    let source = combo.clone();
    combo.on_selection_changed(move |_| s(&format!("Fruit: {}", source.selected_text().unwrap_or_default())));
    let s = say.clone();
    check.on_value_changed(move |on| s(if on { "Box checked" } else { "Box unchecked" }));
    let s = say.clone();
    let source = list.clone();
    list.on_selection_changed(move |index| {
        s(&format!("List: {}", index.and_then(|i| source.item_text(i)).unwrap_or_default()))
    });
    let (bar_target, spin_target) = (progress.clone(), spin.clone());
    slider.on_value_changed(move |value| {
        bar_target.set_value(value);
        spin_target.set_value(value);
    });
    let (bar_target, slider_target) = (progress.clone(), slider.clone());
    spin.on_value_changed(move |value| {
        bar_target.set_value(value);
        slider_target.set_value(value);
    });
    let (bar_target, s, button) = (progress.clone(), say.clone(), run.clone());
    run.on_click(move || {
        let (bar, say, button) = (bar_target.clone(), s.clone(), button.clone());
        button.set_enabled(false);
        let started = spawn_local(async move {
            for step in 0..=20 {
                bar.set_value(step * 5);
                sleep(Duration::from_millis(40)).await;
            }
            let answer = run_blocking(|| (1..=100u32).sum::<u32>()).await;
            say(&format!("Task completed: sum = {answer}"));
            button.set_enabled(true);
        });
        if started.is_err() {
            s("Could not start the task");
        }
    });

    // ---- Tab: data ----
    let data = tabs.add_tab("Data")?;
    let split = SplitContainer::builder().split_position(220).parent(&data).build()?;
    let fill = DockLayout::new().padding(Thickness::all(8));
    fill.add(&split, Dock::Fill);
    data.set_layout(fill);
    let tree = TreeView::builder().parent(split.first()).build()?;
    let fill = DockLayout::new();
    fill.add(&tree, Dock::Fill);
    split.first().set_layout(fill);
    let root_node = tree.add_root("Brazil")?;
    for (region, states) in [("Southeast", ["SP", "RJ", "MG"]), ("South", ["PR", "SC", "RS"])] {
        let node = tree.add_child(root_node, region)?;
        for state in states {
            tree.add_child(node, state)?;
        }
    }
    tree.expand(root_node);
    let grid = DataGrid::builder().editable(true).parent(split.second()).build()?;
    let fill = DockLayout::new();
    fill.add(&grid, Dock::Fill);
    split.second().set_layout(fill);
    grid.add_column("Product", 180, ColumnAlign::Left);
    grid.add_column("Stock", 90, ColumnAlign::Right);
    grid.add_column("Price", 90, ColumnAlign::Right);
    for index in 1..=200 {
        grid.add_row(&[
            &format!("Product {index:03}"),
            &format!("{}", (index * 37) % 500),
            &format!("{}.90", 10 + index % 90),
        ]);
    }
    let s = say.clone();
    grid.on_cell_edited(move |row, column, value| s(&format!("Cell ({row}, {column}) = {value}")));
    let (s, source) = (say.clone(), tree.clone());
    tree.on_selection_changed(move |item| {
        if let Some(item) = item {
            s(&format!("Tree: {}", source.item_path(item).join(" › ")));
        }
    });

    // ---- Tab: properties and formatted text ----
    let rich_tab = tabs.add_tab("Text and properties")?;
    let rich = RichTextBox::builder().parent(&rich_tab).build()?;
    rich.set_text("Formatted text with RichEdit.\r\nSelect some text and use the buttons.");
    rich.set_selection(0, 14);
    rich.format_selection(&TextFormat::new().bold(true).color(Color::rgb(0, 95, 184)).size(14.0));
    rich.set_selection(0, 0);
    let bold = Button::builder().text("Bold").parent(&rich_tab).build()?;
    let red = Button::builder().text("Red").parent(&rich_tab).build()?;
    let target = rich.clone();
    bold.on_click(move || target.format_selection(&TextFormat::new().bold(!target.selection_style().0)));
    let target = rich.clone();
    red.on_click(move || target.format_selection(&TextFormat::new().color(Color::RED)));
    let properties = PropertyGrid::new(&rich_tab, Rect::new(0, 0, 280, 200))?;
    properties.add_category("Document");
    properties.add_property("Title", PropertyValue::Text("Untitled".into()));
    properties.add_property("Pages", PropertyValue::Integer(1));
    properties.add_property("Read-only", PropertyValue::Bool(false));
    properties.add_property(
        "Alignment",
        PropertyValue::Choice { options: vec!["Left".into(), "Center".into(), "Right".into()], selected: 0 },
    );
    let (s, editor) = (say.clone(), rich.clone());
    properties.on_property_changed(move |name, value| {
        match (name, value) {
            ("Read-only", PropertyValue::Bool(read_only)) => editor.set_read_only(*read_only),
            ("Alignment", PropertyValue::Choice { selected, .. }) => {
                editor.select_all();
                editor.set_alignment([TextAlign::Left, TextAlign::Center, TextAlign::Right][*selected % 3]);
                editor.set_selection(0, 0);
            }
            _ => {}
        }
        s(&format!("Property {name} = {value:?}"));
    });
    let buttons = StackLayout::horizontal().spacing(8);
    buttons.add_with(&bold, LayoutParams::new().width(90));
    buttons.add_with(&red, LayoutParams::new().width(90));
    let left = StackLayout::vertical().spacing(8);
    left.add_with(buttons, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
    left.add_with(&rich, LayoutParams::new().weight(1.0));
    let columns = StackLayout::horizontal().spacing(8).padding(Thickness::all(8));
    columns.add_with(left, LayoutParams::new().weight(1.0));
    columns.add_with(&properties, LayoutParams::new().width(290));
    rich_tab.set_layout(columns);

    // ---- Tab: drawing ----
    let drawing = tabs.add_tab("Drawing")?;
    let canvas_panel = Panel::builder().background(Color::WHITE).border(true).parent(&drawing).build()?;
    let fill = DockLayout::new().padding(Thickness::all(8));
    fill.add(&canvas_panel, Dock::Fill);
    drawing.set_layout(fill);
    canvas_panel.on_paint(|canvas| {
        let size = canvas.size();
        canvas.gradient_rect(
            Rect::new(0, 0, size.width, size.height),
            Color::rgb(224, 242, 254),
            Color::WHITE,
            GradientDirection::Vertical,
        );
        let bars = [40, 85, 60, 120, 95, 150, 110];
        let width = ((size.width - 80) / bars.len() as i32).max(10);
        for (index, value) in bars.iter().enumerate() {
            let x = 40 + index as i32 * width;
            let height = value * (size.height - 80).max(1) / 160;
            let color = Color::rgb(37, 99, 235).mix(Color::rgb(14, 165, 233), index as f32 / bars.len() as f32);
            canvas.round_rect(Rect::new(x, size.height - 40 - height, width - 8, height), 4, None, Some(color));
        }
        canvas.line(
            Point::new(30, size.height - 40),
            Point::new(size.width - 30, size.height - 40),
            &Pen::new(Color::GRAY, 1),
        );
        canvas.text(
            "Chart drawn with GDI (double buffering)",
            Point::new(40, 12),
            &Font::system().bold(),
            Color::rgb(30, 41, 59),
        );
    });

    let (s, names) = (say.clone(), tabs.clone());
    tabs.on_selection_changed(move |index| s(&format!("Tab: {}", names.tab_title(index))));
    status.set_part_text(0, "Ready");
    status.set_part_text(1, &format!("DPI {}", window.dpi()));
    status.set_part_text(2, &Locale::user_default().format_long_date(&DateTime::now()));
    window.show();
    app.run()
}
