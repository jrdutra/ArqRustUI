use arqrustui::prelude::*;
#[test]
fn content_alignment_is_configurable_without_changing_icon_position() {
    let _app = Application::new().unwrap();
    let w = Window::builder().title("Alinhamento").size(420, 180).build().unwrap();
    let b = Button::builder()
        .text("Abrir documento")
        .icon(&icons::FILE)
        .content_alignment(TextAlign::Right)
        .parent(&w)
        .build()
        .unwrap();
    assert_eq!(b.content_alignment(), TextAlign::Right);
    b.set_content_alignment(TextAlign::Left);
    assert_eq!(b.content_alignment(), TextAlign::Left);
    b.set_style(Some(ButtonStyle::system()));
    b.set_content_alignment(TextAlign::Right);
    assert_eq!(b.content_alignment(), TextAlign::Right);
    w.destroy();
}
