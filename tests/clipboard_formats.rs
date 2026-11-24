//! Área de transferência: PNG, `CF_DIBV5`, metarquivos EMF, número de sequência e
//! observação de mudanças (`AddClipboardFormatListener`).
//!
//! Um único teste: o clipboard é do sistema inteiro e testes paralelos se atrapalhariam.

mod common;

use arqrustui::prelude::*;
use arqrustui::system::Metafile;
use common::*;
use std::time::Duration;

#[test]
fn png_dibv5_metafiles_and_change_notifications() {
    let _clipboard = clipboard_lock();
    with_app(|app| {
        let changes = Counter::new();
        let seen = changes.clone();
        let watcher = ClipboardWatcher::new(move || seen.hit()).unwrap();
        let before = Clipboard::sequence_number();

        // Imagem: gravada como PNG, CF_DIBV5 e CF_DIB; lida de volta pelo PNG, com alfa.
        let pixels: Vec<u8> = (0..6 * 4).flat_map(|i| [i as u8 * 9, 40, 200, (i * 10) as u8]).collect();
        let image = PlusImage::from_pixels(6, 4, &pixels).unwrap();
        let png = image.to_png().unwrap();
        assert_eq!(&png[1..4], b"PNG", "assinatura PNG");
        assert_eq!(PlusImage::from_bytes(&png).unwrap().to_pixels().unwrap(), pixels);
        Clipboard::set_image(&image).unwrap();
        assert!(Clipboard::has_format("PNG"));
        assert_eq!(Clipboard::png().unwrap().as_deref(), Some(&png[..]));
        assert_eq!(Clipboard::image().unwrap().expect("imagem").to_pixels().unwrap(), pixels);
        assert_ne!(Clipboard::sequence_number(), before);
        assert!(
            pump_until(app, Duration::from_secs(2), || changes.get() >= 1),
            "o observador recebe WM_CLIPBOARDUPDATE"
        );

        // Metarquivo gravado com um Canvas: ida e volta e desenho em pixels.
        let metafile = Metafile::record(40, 20, |canvas| {
            canvas.fill_rect(Rect::new(0, 0, 40, 20), Color::rgb(255, 0, 0));
        })
        .unwrap();
        assert!(metafile.size().width >= 39 && metafile.size().height >= 19, "{:?}", metafile.size());
        assert!(metafile.frame_hundredths_mm().width > 1000);
        let drawn = metafile.render(40, 20, Color::WHITE).unwrap();
        let center = &drawn.to_pixels().unwrap()[(10 * 40 + 20) * 4..][..4];
        assert_eq!(center, [0, 0, 255, 255], "vermelho no centro");
        let count = changes.get();
        Clipboard::set_metafile(&metafile).unwrap();
        assert!(Clipboard::has_metafile() && Clipboard::has_image());
        let back = Clipboard::metafile().unwrap().expect("metarquivo");
        assert_eq!(back.size(), metafile.size());
        // Sem bitmap no clipboard, a imagem vem do metarquivo desenhado.
        let from_metafile = Clipboard::image().unwrap().expect("imagem do metarquivo");
        assert_eq!(from_metafile.size(), back.size());
        assert!(pump_until(app, Duration::from_secs(2), || changes.get() > count));

        drop(watcher);
        let count = changes.get();
        Clipboard::set_text("depois").unwrap();
        pump_until(app, Duration::from_millis(200), || false);
        assert_eq!(changes.get(), count, "observador solto não recebe avisos");
        Clipboard::clear().unwrap();
    });
}
