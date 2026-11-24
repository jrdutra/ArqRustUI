//! Drag and drop: reorder lists and a tree, drag items out and receive
//! files, text and images from other programs.
//!
//! Try it:
//! * reorder the tasks, files and folders by dragging (Esc cancels);
//! * drag a task into Notepad, or into the drop area below;
//! * drag the colored square into WordPad/Paint (text, HTML and image);
//! * drop files from Explorer onto the drop area.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Drag and drop").size(820, 560).min_size(640, 440).centered().build()?;

    // Tasks: reorderable and draggable out as text.
    let tasks_label = Label::builder().text("Tasks").parent(&window).build()?;
    let tasks = ListBox::builder()
        .items(&["Review contract", "Send quote", "Call the customer", "Update spreadsheet", "Schedule meeting"])
        .parent(&window)
        .build()?;
    tasks.set_reorderable(true);
    let source = tasks.clone();
    tasks.on_drag_out(move |index| source.item_text(index).map(|text| ClipboardContent::new().text(&text)));

    // Files: reorderable rows with icons.
    let files_label = Label::builder().text("Files").parent(&window).build()?;
    let files = ListView::builder().parent(&window).build()?;
    files.add_column("Name", 150, ColumnAlign::Left);
    files.add_column("Size", 70, ColumnAlign::Right);
    for (name, size, icon) in [
        ("report.pdf", "1.2 MB", &icons::FILE),
        ("photos", "—", &icons::FOLDER),
        ("budget.xlsx", "84 KB", &icons::FILE),
        ("logo.png", "36 KB", &icons::PICTURE),
    ] {
        let row = files.add_row(&[name, size]);
        files.set_row_icon(row, icon)?;
    }
    files.set_reorderable(true);
    let source = files.clone();
    files.on_drag_out(move |row| source.cell(row, 0).map(|name| ClipboardContent::new().text(&name)));

    // Folders: only accept items dropped inside nodes without an extension (folders).
    let tree_label = Label::builder().text("Folders").parent(&window).build()?;
    let tree = TreeView::builder().parent(&window).build()?;
    let projects = tree.add_root("Projects")?;
    let site = tree.add_child(projects, "site")?;
    tree.add_child(site, "index.html")?;
    tree.add_child(site, "style.css")?;
    let app_folder = tree.add_child(projects, "app")?;
    tree.add_child(app_folder, "main.rs")?;
    let archive = tree.add_root("Archive")?;
    tree.add_child(archive, "2023.zip")?;
    for item in [projects, site, app_folder, archive] {
        tree.set_item_icon(item, &icons::FOLDER)?;
        tree.expand(item);
    }
    tree.set_drag_enabled(true);
    let checker = tree.clone();
    tree.on_item_drop_request(move |_, position| match position {
        TreeDropPosition::Inside(target) => !checker.item_text(target).contains('.'),
        _ => true,
    });

    // Drop area: shows what arrived.
    let zone = Panel::builder().parent(&window).build()?;
    let hovering = Rc::new(Cell::new(false));
    let received = Rc::new(RefCell::new(String::from("Drop files, text or images here")));
    {
        let (hovering, received, target) = (hovering.clone(), received.clone(), zone.clone());
        zone.on_paint(move |canvas| {
            let size = target.size();
            let accent = Color::rgb(0, 120, 212);
            let fill = if hovering.get() { Color::rgb(229, 241, 251) } else { Color::rgb(248, 248, 248) };
            let pen =
                Pen::new(if hovering.get() { accent } else { Color::rgb(190, 190, 190) }, 1).with_style(PenStyle::Dash);
            canvas.round_rect(Rect::new(1, 1, size.width - 2, size.height - 2), 8, Some(&pen), Some(fill));
            let font = Font::system();
            canvas.text_in_rect(
                &received.borrow(),
                Rect::new(12, 8, size.width - 24, size.height - 16),
                &font,
                Color::rgb(60, 60, 60),
                TextAlign::Center,
            );
        });
    }
    let handler = {
        let (enter, leave, dropped) = (hovering.clone(), hovering.clone(), (hovering, received, zone.clone()));
        let (zone_enter, zone_leave) = (zone.clone(), zone.clone());
        DropHandler::new()
            .on_over(move |data| {
                if !enter.replace(true) {
                    zone_enter.invalidate();
                }
                if data.has_files() || data.has_text() || data.has_image() {
                    data.suggested_effect()
                } else {
                    DropEffect::None
                }
            })
            .on_leave(move || {
                leave.set(false);
                zone_leave.invalidate();
            })
            .on_drop(move |data, _| {
                let (hovering, received, zone) = &dropped;
                hovering.set(false);
                let text = if data.has_files() {
                    let names: Vec<String> = data.files().iter().map(|f| f.display().to_string()).collect();
                    format!("{} file(s): {}", names.len(), names.join(", "))
                } else if let Ok(Some(image)) = data.image() {
                    format!("Image of {} × {} pixels", image.size().width, image.size().height)
                } else if let Some(text) = data.text() {
                    format!("Text: “{text}”")
                } else {
                    String::from("Unrecognized format")
                };
                *received.borrow_mut() = text;
                zone.invalidate();
            })
    };
    accept_drops(&zone, handler)?;

    // Source: dragging the square carries text, HTML and an image.
    let swatch = Panel::builder().parent(&window).build()?;
    let color = Color::rgb(0, 120, 212);
    let target = swatch.clone();
    swatch.on_paint(move |canvas| {
        let size = target.size();
        canvas.round_rect(Rect::new(0, 0, size.width, size.height), 10, None, Some(color));
        canvas.text_in_rect(
            "Drag me",
            Rect::new(0, size.height / 2 - 9, size.width, 18),
            &Font::system().bold(),
            Color::rgb(255, 255, 255),
            TextAlign::Center,
        );
    });
    let pressed = Rc::new(Cell::new(None::<Point>));
    {
        let pressed = pressed.clone();
        swatch.on_mouse_down(move |event| {
            if event.button == MouseButton::Left {
                pressed.set(Some(event.position));
            }
        });
    }
    {
        let pressed = pressed.clone();
        swatch.on_mouse_up(move |_| pressed.set(None));
    }
    let origin = swatch.clone();
    swatch.on_mouse_move(move |event| {
        let Some(start) = pressed.get() else { return };
        if (event.position.x - start.x).abs() < 4 && (event.position.y - start.y).abs() < 4 {
            return;
        }
        pressed.set(None);
        let (width, height) = (64, 64);
        let pixels: Vec<u8> = (0..width * height).flat_map(|_| [212, 120, 0, 255]).collect();
        let Ok(image) = PlusImage::from_pixels(width, height, &pixels) else { return };
        let Ok(content) = ClipboardContent::new()
            .text("ArqRustUI blue (#0078D4)")
            .html("<span style=\"color:#0078D4\"><b>ArqRustUI blue</b></span>")
            .image(&image)
        else {
            return;
        };
        let _ = start_drag_with_image(&origin, &content, &[DropEffect::Copy], &image, Point::new(32, 32));
    });

    let hint = Label::builder()
        .text("Drag to reorder • Esc cancels • drag out of the list to carry the text")
        .parent(&window)
        .build()?;

    let columns = StackLayout::horizontal().spacing(SPACING);
    let column = |label: &Label, control: &dyn Fn(&StackLayout)| {
        let column = StackLayout::vertical().spacing(4);
        column.add_with(label, LayoutParams::new().height(20));
        control(&column);
        columns.add_with(column, LayoutParams::new().weight(1.0));
    };
    column(&tasks_label, &|c| {
        c.add_with(&tasks, LayoutParams::new().weight(1.0));
    });
    column(&files_label, &|c| {
        c.add_with(&files, LayoutParams::new().weight(1.0));
    });
    column(&tree_label, &|c| {
        c.add_with(&tree, LayoutParams::new().weight(1.0));
    });
    let bottom = StackLayout::horizontal().spacing(SPACING);
    bottom.add_with(&zone, LayoutParams::new().weight(1.0));
    bottom.add_with(&swatch, LayoutParams::new().width(120));
    let root = StackLayout::vertical().spacing(SPACING).padding(Thickness::all(MARGIN));
    root.add_with(columns, LayoutParams::new().weight(1.0));
    root.add_with(bottom, LayoutParams::new().height(120));
    root.add_with(&hint, LayoutParams::new().height(20));
    window.set_layout(root);
    window.show();
    app.run()
}
