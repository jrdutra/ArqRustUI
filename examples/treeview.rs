//! Example 10 — Hierarchical TreeView with a resizable details panel.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("TreeView").size(600, 380).centered().build()?;
    let split = SplitContainer::builder().split_position(220).min_sizes(120, 160).parent(&window).build()?;
    let root_layout = DockLayout::new().padding(Thickness::all(8));
    root_layout.add(&split, Dock::Fill);
    window.set_layout(root_layout);

    let tree = TreeView::builder().parent(split.first()).build()?;
    let fill = DockLayout::new();
    fill.add(&tree, Dock::Fill);
    split.first().set_layout(fill);

    let details = TextBox::builder().multiline(true).read_only(true).parent(split.second()).build()?;
    let fill = DockLayout::new();
    fill.add(&details, Dock::Fill);
    split.second().set_layout(fill);

    let project = tree.add_root("arqrustui")?;
    for (folder, files) in [
        ("src", vec!["lib.rs", "core", "controls", "layout", "graphics"]),
        ("examples", vec!["hello_world.rs", "calculator.rs", "treeview.rs"]),
        ("docs", vec!["ARCHITECTURE.md", "FEATURE_MATRIX.md"]),
    ] {
        let node = tree.add_child(project, folder)?;
        for file in files {
            tree.add_child(node, file)?;
        }
    }
    tree.add_child(project, "Cargo.toml")?;
    tree.expand(project);

    let (source, output) = (tree.clone(), details.clone());
    tree.on_selection_changed(move |item| {
        let Some(item) = item else { return };
        let path = source.item_path(item).join("\\");
        let children = source.children(item);
        let kind = if children.is_empty() { "File" } else { "Folder" };
        output.set_text(&format!("Path: {path}\r\nType: {kind}\r\nChildren: {}", children.len()));
    });
    tree.select(Some(project));
    window.show();
    app.run()
}
