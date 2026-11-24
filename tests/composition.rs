use arqrustui::prelude::*;

#[test]
fn status_layout_follows_visible_content() {
    let _app = Application::new().unwrap();
    let window = Window::builder().title("Composição").size(400, 500).build().unwrap();
    let bar = StatusBar::new(&window).unwrap();
    let result = Panel::builder().parent(&window).build().unwrap();
    let last = Button::builder().text("Último controle").parent(&window).build().unwrap();
    let content = StackLayout::vertical().spacing(8).padding(Thickness::new(12, 12, 12, 8));
    content.add_with(&result, LayoutParams::new().height(80));
    content.add_with(&last, LayoutParams::new().height(34));
    let layout = StatusBarLayout::new(content, &bar).minimum_width(300);
    window.set_layout(layout.clone());
    layout.fit_height(&window);
    let expanded = window.client_size().height;
    assert_eq!(last.bounds().bottom() + 8 + layout.footer_height(), expanded);
    result.hide();
    layout.fit_height(&window);
    assert_eq!(window.client_size().height, expanded - 88);
    assert_eq!(last.bounds().bottom() + 8 + layout.footer_height(), window.client_size().height);
    result.show();
    layout.fit_height(&window);
    assert_eq!(window.client_size().height, expanded);
    window.destroy();
    layout.fit_height(&window); // handles destruídos são seguros
}

#[test]
fn scroll_limits_resize_focus_and_filtering() {
    let _app = Application::new().unwrap();
    let window = Window::builder().title("Rolagem").size(400, 300).build().unwrap();
    let scroll = ScrollViewport::new(&window).unwrap();
    scroll.viewport().set_bounds(Rect::new(0, 0, 200, 100));
    let mut measured = Vec::new();
    let extent = scroll.layout_content(|width| {
        measured.push(width);
        300
    });
    assert_eq!(measured, vec![200, 183]);
    assert_eq!(extent, Size::new(183, 300));
    assert!(scroll.has_overflow());
    scroll.scroll_to(999);
    assert_eq!(scroll.offset(), 200);
    assert_eq!(scroll.content().bounds().y, -200);
    scroll.scroll_wheel(1);
    assert_eq!(scroll.offset(), 140);
    scroll.ensure_visible(Rect::new(0, 20, 100, 28), 8);
    assert_eq!(scroll.offset(), 12);
    scroll.set_content_height(50);
    assert!(!scroll.has_overflow());
    assert_eq!(scroll.offset(), 0);
    assert_eq!(scroll.content().bounds().width, 200);
    scroll.set_content_height(300);
    scroll.scroll_to(200);
    scroll.viewport().set_size(200, 250);
    assert_eq!(scroll.offset(), 50);
    window.destroy();
}

#[test]
fn scroll_mechanism_survives_handle_drop_until_native_destruction() {
    let _app = Application::new().unwrap();
    let window = Window::builder().title("Vida útil").size(400, 300).build().unwrap();
    let scroll = ScrollViewport::new(&window).unwrap();
    let viewport = scroll.viewport();
    let content = scroll.content();
    viewport.set_size(200, 100);
    scroll.set_content_height(300);
    drop(scroll);
    viewport.set_size(150, 100);
    assert_eq!(content.bounds().width, 133);
    window.destroy();
    assert!(!content.is_alive());
}
