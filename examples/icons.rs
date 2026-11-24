//! Example 21 — Icon pack browser: categories in a tree, icons in
//! a grid, an enlarged preview and export in the three formats. Uses icons in the
//! window, toolbar, tree, grid, tabs, buttons, labels, combo box
//! and status bar.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

/// Icon that represents each category in the tree.
fn category_icon(category: IconCategory) -> &'static IconAsset {
    match category {
        IconCategory::Actions => &icons::ADD,
        IconCategory::Status => &icons::INFO,
        IconCategory::Files => &icons::FOLDER,
        IconCategory::FileTypes => &icons::PDF_FILE,
        IconCategory::Office => &icons::BRIEFCASE,
        IconCategory::Communication => &icons::EMAIL,
        IconCategory::People => &icons::USERS,
        IconCategory::Devices => &icons::COMPUTER,
        IconCategory::Hardware => &icons::CPU,
        IconCategory::Network => &icons::GLOBE,
        IconCategory::Data => &icons::DATABASE,
        IconCategory::Security => &icons::SHIELD_CHECK,
        IconCategory::Media => &icons::CAMERA,
        IconCategory::Design => &icons::PALETTE,
        IconCategory::Development => &icons::CODE_WINDOW,
        IconCategory::Technologies => &icons::GITHUB,
        IconCategory::Commerce => &icons::SHOPPING_CART,
        IconCategory::Navigation => &icons::COMPASS,
        IconCategory::Entertainment => &icons::TROPHY,
        IconCategory::Social => &icons::THREADS,
        IconCategory::Emoji => &icons::EMOJI_PARTY,
        IconCategory::Gestures => &icons::HAND_ROCK,
        // `IconCategory` is `#[non_exhaustive]`: future categories use a generic icon.
        _ => &icons::FOLDER,
    }
}

fn formats(icon: &IconAsset) -> String {
    let mut text = String::from("ICO, PNG");
    if icon.svg_bytes().is_some() {
        text.push_str(", SVG");
    }
    text
}

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    // The library's own texts (tips, accessible names, category names...) in English too.
    arqrustui::i18n::set_library_language("en");
    let window = Window::builder()
        .title("ArqRustUI — icon pack")
        .icon(&icons::PALETTE)
        .size(900, 600)
        .min_size(700, 460)
        .centered()
        .build()?;

    let status = StatusBar::new(&window)?;
    status.set_parts(&[360, -1]);
    status.set_part_icon(0, &icons::INFO)?;
    status.set_part_text(0, &format!("{} icons in {} categories", icons::all().len(), IconCategory::ALL.len()));

    // ---- State: selected category and icon ------------------------------------------
    let category = Rc::new(Cell::new(IconCategory::Actions));
    let selected = Rc::new(Cell::new(&icons::ADD));

    // ---- Category tree and icon grid -------------------------------------------------
    let split = SplitContainer::builder().split_position(220).min_sizes(160, 360).parent(&window).build()?;
    let tree = TreeView::builder().parent(split.first()).build()?;
    let all = tree.add_root("All icons")?;
    tree.set_item_icons(all, &icons::FOLDER, &icons::FOLDER_OPEN)?;
    let mut nodes = Vec::new();
    for cat in IconCategory::ALL {
        let count = icons::in_category(cat).count();
        let node = tree.add_child(all, &format!("{} ({count})", cat.label()))?;
        tree.set_item_icon(node, category_icon(cat))?;
        nodes.push((node, cat));
    }
    tree.expand(all);
    split.first().set_layout(DockLayout::new().padding(Thickness::all(6)).add(&tree, Dock::Fill).clone());

    let tabs = TabControl::builder().parent(split.second()).build()?;
    let list_page = tabs.add_tab("Icons")?;
    tabs.set_tab_icon(0, &icons::PHOTO_GALLERY)?;
    let about_page = tabs.add_tab("About the pack")?;
    tabs.set_tab_icon(1, &icons::HELP)?;

    let grid = DataGrid::builder().single_selection(true).parent(&list_page).build()?;
    grid.add_column("Name", 170, ColumnAlign::Left);
    grid.add_column("Constant", 170, ColumnAlign::Left);
    grid.add_column("Formats", 110, ColumnAlign::Left);
    grid.add_column("Aliases", 260, ColumnAlign::Left);
    grid.set_icon_size(24)?;

    // ---- Details panel -------------------------------------------------------------
    let picture = PictureBox::builder().icon(&icons::ADD).size(160, 160).parent(&list_page).build()?;
    let title = Label::builder().text("add").icon(&icons::ADD).icon_size(24).parent(&list_page).build()?;
    title.set_font(Font::new("Segoe UI", 12.0).bold());
    let details = Label::builder().text("").parent(&list_page).build()?;
    let size_label = Label::builder().text("Size:").parent(&list_page).build()?;
    let size_choice = ComboBox::builder().with_icons(true).parent(&list_page).build()?;
    for size in [16, 24, 32, 48, 64, 128, 256] {
        size_choice.add_item_with_icon(&format!("{size} × {size} px"), &icons::RULER)?;
    }
    size_choice.add_item_with_icon("Fit to space", &icons::ZOOM_IN)?;
    size_choice.set_selected_index(Some(7));
    let save_ico = Button::builder().text("Save .ico").icon(&icons::SAVE).parent(&list_page).build()?;
    let save_png = Button::builder().text("Save .png").icon(&icons::PICTURE).parent(&list_page).build()?;
    let save_svg = Button::builder().text("Save .svg").icon(&icons::EDIT).parent(&list_page).build()?;

    let buttons = StackLayout::horizontal().spacing(8);
    buttons.add_with(&save_ico, LayoutParams::new().width(120));
    buttons.add_with(&save_png, LayoutParams::new().width(120));
    buttons.add_with(&save_svg, LayoutParams::new().width(120));
    let size_row = StackLayout::horizontal().spacing(8);
    size_row.add_with(&size_label, LayoutParams::new().width(64).height(24));
    size_row.add_with(&size_choice, LayoutParams::new().width(180));
    let info = StackLayout::vertical().spacing(8);
    info.add_with(&title, LayoutParams::new().height(32));
    info.add_with(&details, LayoutParams::new().height(40));
    info.add_with(size_row, LayoutParams::new().height(DEFAULT_CONTROL_HEIGHT));
    info.add_with(buttons, LayoutParams::new().height(32));
    let bottom = StackLayout::horizontal().spacing(12);
    bottom.add_with(&picture, LayoutParams::new().width(160));
    bottom.add_with(info, LayoutParams::new().weight(1.0));
    let page = StackLayout::vertical().spacing(10).padding(Thickness::all(8));
    page.add_with(&grid, LayoutParams::new().weight(1.0));
    page.add_with(bottom, LayoutParams::new().height(160));
    list_page.set_layout(page);

    let about = Label::builder()
        .text(
            "392 retro-style icons, each one in .ico (16 to 256 px) and .png; 79 of them also in .svg. \
             For file lists, icons::for_path(path) gives the file type icon from the extension. \
             Use the arqrustui::icons constants (for example, icons::FOLDER) in any control that \
             shows icons, or icons::find(\"name\") to search by name or alias.",
        )
        .parent(&about_page)
        .build()?;
    let pack = PictureBox::builder().icon(&icons::PALETTE).icon_size(96).parent(&about_page).build()?;
    let about_layout = StackLayout::vertical().spacing(12).padding(Thickness::all(16));
    about_layout.add_with(&pack, LayoutParams::new().height(110));
    about_layout.add_with(&about, LayoutParams::new().weight(1.0));
    about_page.set_layout(about_layout);

    let toolbar = ToolBar::new(&window)?;
    let root = DockLayout::new().padding(Thickness::new(0, toolbar.height(), 0, status.height()));
    root.add(&split, Dock::Fill);
    split.second().set_layout(DockLayout::new().padding(Thickness::all(6)).add(&tabs, Dock::Fill).clone());
    window.set_layout(root);

    // ---- Behavior ------------------------------------------------------------------
    let show_icon = {
        let (picture, title, details, selected) = (picture.clone(), title.clone(), details.clone(), selected.clone());
        let save_svg = save_svg.clone();
        move |icon: &'static IconAsset| {
            selected.set(icon);
            picture.set_icon(icon);
            title.set_text(icon.name());
            let _ = title.set_icon(icon);
            let category = icon.category().map(IconCategory::label).unwrap_or_else(|| "—".to_string());
            details.set_text(&format!(
                "Category: {category}   ·   PNG {}×{} px\n.ico sizes: {:?}",
                icon.png_size().width,
                icon.png_size().height,
                icon.sizes()
            ));
            save_svg.set_enabled(icon.svg_bytes().is_some());
        }
    };
    let fill_grid = {
        let (grid, category, show_icon) = (grid.clone(), category.clone(), show_icon.clone());
        move |filter: Option<IconCategory>| {
            let items: Vec<&'static IconAsset> = match filter {
                Some(cat) => icons::in_category(cat).collect(),
                None => icons::all().to_vec(),
            };
            if let Some(cat) = filter {
                category.set(cat);
            }
            grid.set_rows(
                items
                    .iter()
                    .map(|icon| {
                        vec![icon.name().into(), icon.name().to_uppercase(), formats(icon), icon.aliases().join(", ")]
                    })
                    .collect(),
            );
            for (row, icon) in items.iter().enumerate() {
                let _ = grid.set_row_icon(row, *icon);
            }
            if let Some(first) = items.first() {
                grid.set_row_selected(0, true);
                show_icon(first);
            }
        }
    };
    fill_grid(Some(IconCategory::Actions));

    {
        let fill_grid = fill_grid.clone();
        let tree_handle = tree.clone();
        tree.on_selection_changed(move |item| {
            let Some(item) = item else { return };
            let chosen = nodes.iter().find(|(node, _)| *node == item).map(|(_, cat)| *cat);
            if item == all || chosen.is_some() {
                fill_grid(chosen);
            }
            let _ = &tree_handle;
        });
    }
    {
        let (grid_handle, show_icon) = (grid.clone(), show_icon.clone());
        grid.on_selection_changed(move |rows| {
            if let Some(icon) = rows.first().and_then(|row| grid_handle.cell(*row, 0)).and_then(|n| icons::find(&n)) {
                show_icon(icon);
            }
        });
    }
    {
        let (picture, choice) = (picture.clone(), size_choice.clone());
        size_choice.on_selection_changed(move |index| {
            let sizes = [16, 24, 32, 48, 64, 128, 256];
            picture.set_icon_size(index.and_then(|i| sizes.get(i)).map(|px| *px * 96 / picture.dpi() as i32));
            let _ = &choice;
        });
    }
    for (button, extension) in [(&save_ico, "ico"), (&save_png, "png"), (&save_svg, "svg")] {
        let (owner, selected, status) = (window.clone(), selected.clone(), status.clone());
        button.on_click(move || {
            let icon = selected.get();
            let dialog = SaveFileDialog::new()
                .file_name(&format!("{}.{extension}", icon.name()))
                .default_extension(extension)
                .filter(&extension.to_uppercase(), &format!("*.{extension}"));
            match dialog.show(&owner) {
                Ok(Some(path)) => match icon.save(&path) {
                    Ok(()) => status.set_part_text(1, &format!("Saved to {}", path.display())),
                    Err(error) => MessageBox::error("Could not save", &error.to_string()),
                },
                Ok(None) => {}
                Err(error) => MessageBox::error("Error", &error.to_string()),
            }
        });
    }

    let target = window.clone();
    toolbar.add_button("Dark theme", Some(&icons::LIGHTBULB), move || Theme::dark().apply(&target))?;
    let target = window.clone();
    toolbar.add_button("Light theme", Some(&icons::SETTINGS), move || Theme::light().apply(&target))?;
    toolbar.add_separator()?;
    let target = window.clone();
    toolbar.add_button("Exit", Some(&icons::POWER), move || target.close())?;
    window.relayout();

    window.show();
    app.run()
}
