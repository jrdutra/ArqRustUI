use arqrustui::prelude::*;

#[test]
fn styled_unicode_roundtrip_in_native_control() {
    let _app = Application::new().unwrap();
    let window = Window::builder().title("Teste de texto").size(300, 150).build().unwrap();
    let view = RichTextBox::builder().parent(&window).build().unwrap();
    let mut text = StyledText::new("Consolas", 10.0);
    text.push("Olá {mundo} 😀", TextFormat::new().highlight(Color::rgb(210, 240, 255)));
    text.push(" texto normal", TextFormat::new());
    view.set_rtf(&text.to_rtf()).unwrap();
    assert_eq!(view.text(), "Olá {mundo} 😀 texto normal");
    window.destroy();
}
