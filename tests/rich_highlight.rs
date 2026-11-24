use arqrustui::prelude::*;

#[test]
fn highlights_preserve_text_selection_modified_events_and_undo() {
    let _app = Application::new().unwrap();
    let window = Window::builder().title("Estilos por trecho").size(420, 240).build().unwrap();
    let view = RichTextBox::builder().parent(&window).build().unwrap();
    view.set_text("Olá 😀\r\nlinha");
    view.set_selection(0, 0);
    view.replace_selection("X");
    view.set_selection(2, 4);
    let before = view.text();
    let selected = view.selection();
    let modified = view.is_modified();
    let changed = std::rc::Rc::new(std::cell::Cell::new(0));
    let c = changed.clone();
    view.on_text_changed(move || c.set(c.get() + 1));
    view.apply_highlights(
        &TextFormat::new().color(Color::BLACK),
        &[(0..1, TextFormat::new().color(Color::rgb(20, 90, 180)))],
    )
    .unwrap();
    assert_eq!(view.text(), before);
    assert_eq!(view.selection(), selected);
    assert_eq!(view.is_modified(), modified);
    assert_eq!(changed.get(), 0);
    assert!(view.rtf().contains("\\red20\\green90\\blue180"));
    view.undo();
    assert!(!view.text().starts_with('X'), "o destaque não pode ocupar uma etapa de desfazer");
    assert!(view.apply_highlights(&TextFormat::new(), &[(500..501, TextFormat::new())]).is_err());
    window.destroy();
}
