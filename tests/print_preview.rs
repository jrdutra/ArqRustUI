//! Documento de impressão, pré-visualização e diálogo de pré-visualização. A impressão
//! real (PrintDlgW + StartDoc) não é exercida: o ambiente de teste não tem impressora.

mod common;

use arqrustui::ffi::gdi32::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetPixel, SelectObject,
};
use arqrustui::ffi::user32::{GetDC, ReleaseDC, SendMessageW};
use arqrustui::prelude::*;
use common::*;
use std::cell::Cell;
use std::rc::Rc;

const WM_PRINTCLIENT: u32 = 0x0318;
const PRF_CLIENT: isize = 0x4;
const WM_KEYDOWN: u32 = 0x0100;

/// Pixels (r, g, b) de alguns pontos do controle desenhado com WM_PRINTCLIENT.
fn colors_of(widget: &impl Widget, points: &[(i32, i32)]) -> Vec<(u8, u8, u8)> {
    let size = widget.client_size();
    // SAFETY: DCs e bitmap temporários desta thread, liberados ao final; a janela é viva.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, size.width, size.height);
        let previous = SelectObject(memory, bitmap as _);
        SendMessageW(widget.handle() as _, WM_PRINTCLIENT, memory as usize, PRF_CLIENT);
        let colors = points
            .iter()
            .map(|&(x, y)| {
                let c = GetPixel(memory, x, y);
                ((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8)
            })
            .collect();
        SelectObject(memory, previous);
        DeleteObject(bitmap as _);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
        colors
    }
}

/// Página 200×100 com um quadrado preto de 50×50 no canto e o número da página em
/// tons de vermelho (página 1 = vermelho 100, página 2 = 200...).
fn document(drawn: Rc<Cell<usize>>) -> PrintDocument {
    PrintDocument::new("Teste", 3, move |page, canvas| {
        drawn.set(drawn.get() + 1);
        assert_eq!(canvas.size(), Size::new(200, 100), "o canvas tem o tamanho lógico da página");
        canvas.fill_rect(Rect::new(0, 0, 50, 50), Color::BLACK);
        canvas.fill_rect(Rect::new(150, 50, 50, 50), Color::rgb(((page + 1) * 80) as u8, 0, 0));
    })
    .with_page_size(Size::new(200, 100))
}

#[test]
fn documents_render_at_any_resolution() {
    with_app(|_| {
        let drawn = Rc::new(Cell::new(0));
        let doc = document(drawn.clone());
        assert_eq!((doc.name(), doc.page_count()), ("Teste".to_string(), 3));
        let full = doc.render_page(1, 96).unwrap();
        assert_eq!(full.size(), Size::new(200, 100));
        let pixels = full.to_pixels().unwrap();
        let at = |pixels: &[u8], width: i32, x: i32, y: i32| {
            let i = ((y * width + x) * 4) as usize;
            (pixels[i + 2], pixels[i + 1], pixels[i])
        };
        assert_eq!(at(&pixels, 200, 10, 10), (0, 0, 0));
        assert_eq!(at(&pixels, 200, 100, 20), (255, 255, 255), "fundo branco");
        assert_eq!(at(&pixels, 200, 170, 70), (160, 0, 0));
        let half = doc.render_page(1, 48).unwrap();
        assert_eq!(half.size(), Size::new(100, 50), "48 DPI = metade do tamanho");
        let pixels = half.to_pixels().unwrap();
        assert_eq!(at(&pixels, 100, 10, 10), (0, 0, 0));
        assert_eq!(at(&pixels, 100, 85, 35), (160, 0, 0));
        assert!(doc.render_page(3, 96).is_err());
        assert_eq!(drawn.get(), 2);
        let setup =
            PageSetup { paper_width: 21000, paper_height: 29700, margins: Thickness::new(2540, 2540, 2540, 2540) };
        let a4 = doc.clone().with_page_setup(&setup);
        assert_eq!(a4.page_size(), Size::new(601, 930), "A4 menos 1\" de cada lado");
        assert_eq!(PrintRange::Pages { first: 2, last: 3 }.pages(doc.page_count()), vec![1, 2]);
    });
}

#[test]
fn preview_shows_navigates_zooms_and_caches_pages() {
    with_app(|app| {
        let window = test_window();
        let drawn = Rc::new(Cell::new(0));
        let doc = document(drawn.clone());
        let preview = PrintPreview::builder().document(doc).bounds(0, 0, 432, 332).parent(&window).build().unwrap();
        // 400×300 disponíveis para 200×100: cabe em 2×.
        assert!((preview.effective_scale() - 2.0).abs() < 1e-9);
        assert_eq!(preview.page_bounds(), Rect::new(16, 66, 400, 200));
        let changes = Rc::new(Cell::new(Vec::new()));
        let seen = changes.clone();
        preview.on_page_changed(move |page| {
            let mut list = seen.take();
            list.push(page);
            seen.set(list);
        });

        let probe = |preview: &PrintPreview| colors_of(preview, &[(56, 106), (216, 116), (376, 236), (4, 4)]);
        let colors = probe(&preview);
        assert_eq!(colors[0], (0, 0, 0), "quadrado preto ampliado: {colors:?}");
        assert_eq!(colors[1], (255, 255, 255), "papel branco: {colors:?}");
        assert_eq!(colors[2], (80, 0, 0), "página 1: {colors:?}");
        assert_eq!(colors[3], (128, 128, 128), "fundo em volta da página: {colors:?}");
        let first = drawn.get();
        probe(&preview);
        assert_eq!(drawn.get(), first, "página reaproveitada do cache");

        preview.next_page();
        assert_eq!(probe(&preview)[2], (160, 0, 0));
        send(&preview, WM_KEYDOWN, 0x23, 0); // End
        assert_eq!(preview.page(), 2);
        assert_eq!(probe(&preview)[2], (240, 0, 0));
        preview.next_page();
        assert_eq!(preview.page(), 2, "não passa da última");
        send(&preview, WM_KEYDOWN, 0x21, 0); // Page Up
        send(&preview, WM_KEYDOWN, 0x24, 0); // Home
        assert_eq!(preview.page(), 0);
        assert_eq!(changes.take(), vec![1, 2, 1, 0]);

        preview.zoom_in();
        assert_eq!(preview.zoom(), PreviewZoom::Percent(300));
        assert_eq!(preview.page_bounds().width, 600, "300% de 200");
        assert_eq!(preview.page_bounds().x, 16, "maior que o controle: alinhada à margem");
        preview.set_zoom(PreviewZoom::Percent(400));
        send(&preview, WM_KEYDOWN, 0x28, 0); // ↓ rola
        assert_eq!(preview.page_bounds().y, 16 - 48);
        for _ in 0..5 {
            send(&preview, WM_KEYDOWN, 0x28, 0);
        }
        assert_eq!(preview.page_bounds().y, 16 - 100, "para no fim da página (400 + 32 − 332)");
        preview.set_zoom(PreviewZoom::FitWidth);
        assert!((preview.effective_scale() - 2.0).abs() < 1e-9);
        preview.zoom_out();
        assert_eq!(preview.zoom(), PreviewZoom::Percent(150));

        preview.set_document(Some(PrintDocument::new("Vazio", 0, |_, _| {})));
        assert_eq!((preview.page_count(), preview.page()), (0, 0));
        assert_eq!(probe(&preview)[1], (128, 128, 128), "sem páginas, só o fundo");
        app.process_pending().unwrap();
        window.destroy();
    });
}

#[test]
fn preview_dialog_tracks_pages_and_closes() {
    with_app(|app| {
        let owner = test_window();
        let doc = document(Rc::new(Cell::new(0)));
        let dialog = PrintPreviewDialog::new(doc).size(640, 480).create(&owner).unwrap();
        assert!(dialog.window().title().contains("Teste"));
        assert_eq!(dialog.page_text(), "Página 1 de 3");
        dialog.preview().next_page();
        assert_eq!(dialog.page_text(), "Página 2 de 3");
        dialog.show();
        app.process_pending().unwrap();
        let bounds = dialog.preview().bounds();
        assert!(bounds.width > 500 && bounds.height > 380, "a pré-visualização ocupa o diálogo: {bounds:?}");
        assert_eq!(dialog.printed_pages(), None);
        dialog.close();
        app.process_pending().unwrap();
        owner.destroy();
    });
}
