//! Example 9 — ListView with several columns, sorting, multiple selection and icons.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("ListView").size(560, 360).centered().build()?;
    let list = ListView::builder().grid_lines(true).parent(&window).build()?;
    let status = StatusBar::new(&window)?;
    let layout = DockLayout::new().padding(Thickness::new(8, 8, 8, status.height() + 8));
    layout.add(&list, Dock::Fill);
    window.set_layout(layout);

    list.add_column("File", 220, ColumnAlign::Left);
    list.add_column("Size (KB)", 110, ColumnAlign::Right);
    list.add_column("Type", 120, ColumnAlign::Left);
    list.add_column("Modified", 90, ColumnAlign::Center);

    let images = ImageList::new(16)?;
    let document = images.add_icon(&Icon::system(SystemIcon::Application)?)?;
    let warning = images.add_icon(&Icon::system(SystemIcon::Warning)?)?;
    list.set_image_list(&images);

    let rows = [
        ("annual-report.pdf", "2048", "PDF document", "2026"),
        ("photo.png", "512", "PNG image", "2025"),
        ("spreadsheet.xlsx", "96", "Spreadsheet", "2026"),
        ("old-backup.zip", "104857", "ZIP archive", "2019"),
        ("notes.txt", "3", "Text", "2024"),
    ];
    for (name, size, kind, year) in rows {
        let row = list.add_row(&[name, size, kind, year]);
        list.set_row_image(row, if size.len() > 5 { warning } else { document });
    }

    // Clicking a header sorts; clicking again reverses.
    let last = Rc::new(Cell::new((usize::MAX, true)));
    let target = list.clone();
    list.on_column_click(move |column| {
        let ascending = if last.get().0 == column { !last.get().1 } else { true };
        last.set((column, ascending));
        target.sort_by_column(column, ascending);
    });

    let bar = status.clone();
    list.on_selection_changed(move |rows| bar.set_part_text(0, &format!("{} item(s) selected", rows.len())));
    let (target, owner) = (list.clone(), window.clone());
    list.on_row_activated(move |row| {
        let name = target.cell(row, 0).unwrap_or_default();
        MessageBox::show(
            &owner,
            "Item activated",
            &format!("You opened \"{name}\"."),
            MessageButtons::Ok,
            MessageIcon::Information,
        );
    });
    status.set_part_text(0, "Click a header to sort. Use Ctrl/Shift for multiple selection.");
    window.show();
    app.run()
}
