use arqrustui::prelude::*;

#[test]
fn lists_tables_images_and_rtf_fragments() {
    let _app = Application::new().unwrap();
    let window = Window::builder().title("Documento rico").size(480, 320).build().unwrap();
    let view = RichTextBox::builder().parent(&window).build().unwrap();

    // Listas: o estilo volta pelo EM_GETPARAFORMAT e aparece no RTF.
    view.set_text("um\rdois\rtrês");
    view.set_selection(0, 7);
    view.set_list_style(ListStyle::Bullet);
    view.set_selection(1, 1);
    assert_eq!(view.list_style(), ListStyle::Bullet);
    view.set_selection(0, 7);
    view.set_list_style(ListStyle::Numbered(3));
    view.set_selection(1, 1);
    assert_eq!(view.list_style(), ListStyle::Numbered(3));
    view.set_selection(9, 9);
    assert_eq!(view.list_style(), ListStyle::None, "o último parágrafo ficou fora da seleção");
    view.set_selection(0, 7);
    view.set_list_style(ListStyle::UpperRoman);
    view.set_selection(1, 1);
    assert_eq!(view.list_style(), ListStyle::UpperRoman);
    view.set_selection(0, 7);
    view.set_list_style(ListStyle::None);
    view.set_selection(1, 1);
    assert_eq!(view.list_style(), ListStyle::None);

    // Trecho RTF entra no cursor sem apagar o resto.
    view.set_text("início fim");
    view.set_selection(7, 7);
    view.insert_rtf("{\\rtf1 {\\b meio }}").unwrap();
    assert!(view.text().starts_with("início meio fim"), "texto: {:?}", view.text());

    // Tabela.
    view.set_text("");
    let rows = vec![vec!["Produto".to_string(), "Preço".to_string()], vec!["Café".to_string(), "R$ 12".to_string()]];
    view.insert_table(&rows, &[160, 80], true).unwrap();
    let text = view.text();
    for cell in ["Produto", "Preço", "Café", "R$ 12"] {
        assert!(text.contains(cell), "célula {cell} ausente em {text:?}");
    }
    let rtf = view.rtf();
    assert!(rtf.contains("\\trowd") && rtf.contains("\\cell"), "tabela não ficou no RTF: {rtf}");
    assert!(view.insert_table(&[], &[], false).is_err());

    // Imagem: vira \pict no documento.
    let mut pixels = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            let red = if (x + y) % 2 == 0 { 255 } else { 0 };
            pixels.extend_from_slice(&[0, 0, red, 255]);
        }
    }
    let image = PlusImage::from_pixels(8, 8, &pixels).unwrap();
    view.set_selection(usize::MAX, usize::MAX);
    view.insert_image(&image, 32, 32).unwrap();
    let rtf = view.rtf();
    assert!(rtf.contains("\\pict"), "imagem não ficou no RTF");
    window.destroy();
}
