//! Desenho de SVG próprio: pixels conferidos depois de desenhar com o GDI+.

mod common;

use arqrustui::prelude::*;
use common::*;

/// (r, g, b, a) do pixel.
fn pixel(image: &PlusImage, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let width = image.size().width;
    let pixels = image.to_pixels().unwrap();
    let i = ((y * width + x) * 4) as usize;
    (pixels[i + 2], pixels[i + 1], pixels[i], pixels[i + 3])
}

fn render(source: &str, width: i32, height: i32) -> PlusImage {
    SvgImage::parse(source).unwrap().render(width, height).unwrap()
}

#[test]
fn shapes_fill_rules_and_transparency() {
    with_app(|_| {
        let image = render(
            r##"<svg viewBox="0 0 100 50">
                <rect width="50" height="50" fill="#ff0000"/>
                <circle cx="75" cy="25" r="20" fill="rgb(0,0,255)"/>
                <path fill-rule="evenodd" fill="#0f0" d="M2 2h20v20H2z M7 7h10v10H7z"/>
            </svg>"##,
            200,
            100,
        );
        assert_eq!(image.size(), Size::new(200, 100));
        assert_eq!(pixel(&image, 60, 80), (255, 0, 0, 255), "retângulo, escalado 2×");
        assert_eq!(pixel(&image, 150, 50), (0, 0, 255, 255), "centro do círculo");
        assert_eq!(pixel(&image, 104, 4).3, 0, "fora das formas: transparente");
        assert_eq!(pixel(&image, 8, 8), (0, 255, 0, 255), "borda do quadrado vazado");
        assert_eq!(pixel(&image, 24, 24), (255, 0, 0, 255), "furo pela regra par-ímpar mostra o vermelho");
        // (O antialiasing das bordas não é conferido: o GDI+ do Wine não suaviza o
        // preenchimento de caminhos em imagens; no Windows a borda tem alfa intermediário.)
    });
}

#[test]
fn gradients_pad_and_radial() {
    with_app(|_| {
        let image = render(
            r##"<svg viewBox="0 0 100 40">
                <defs>
                  <linearGradient id="l" gradientUnits="userSpaceOnUse" x1="25" x2="75">
                    <stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/>
                  </linearGradient>
                  <radialGradient id="r"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></radialGradient>
                </defs>
                <rect width="100" height="20" fill="url(#l)"/>
                <rect y="20" width="40" height="20" fill="url(#r)"/>
            </svg>"##,
            100,
            40,
        );
        let gray = |x: i32| pixel(&image, x, 10).0 as i32;
        assert!(gray(5) <= 8, "antes do início: cor da primeira parada (pad) — {}", gray(5));
        assert!(gray(95) >= 247, "depois do fim: cor da última parada (pad) — {}", gray(95));
        assert!((gray(50) - 128).abs() <= 24, "meio do gradiente — {}", gray(50));
        assert!(gray(30) < gray(50) && gray(50) < gray(70), "cresce da esquerda para a direita");
        let center = pixel(&image, 20, 30);
        assert!(center.0 > 200 && center.2 < 60, "centro do radial vermelho: {center:?}");
        let corner = pixel(&image, 1, 21);
        assert!(corner.2 > 200 && corner.0 < 60, "canto (fora da elipse) com a última parada: {corner:?}");
    });
}

#[test]
fn strokes_dashes_and_current_color() {
    with_app(|_| {
        let svg = SvgImage::parse(
            r##"<svg viewBox="0 0 100 20">
                <line x1="0" y1="5" x2="100" y2="5" stroke="currentColor" stroke-width="4"/>
                <line x1="0" y1="15" x2="100" y2="15" stroke="#000" stroke-width="4" stroke-dasharray="10 10"/>
            </svg>"##,
        )
        .unwrap()
        .with_current_color(Color::rgb(0, 128, 0));
        let image = svg.render(100, 20).unwrap();
        assert_eq!(pixel(&image, 50, 5), (0, 128, 0, 255), "currentColor");
        assert_eq!(pixel(&image, 5, 15).3, 255, "traço");
        assert_eq!(pixel(&image, 15, 15).3, 0, "intervalo do tracejado");
        assert_eq!(pixel(&image, 25, 15).3, 255);
        assert!(SvgImage::parse("<svg><rect></svg>").is_err());
        assert!(svg.render(0, 10).is_err());
    });
}

#[test]
fn every_bundled_svg_icon_renders() {
    with_app(|_| {
        let mut rendered = 0;
        for icon in icons::all() {
            let Some(svg) = icon.svg_image() else { continue };
            let svg = svg.unwrap_or_else(|e| panic!("{}: {e}", icon.name()));
            let image = svg.render(48, 48).unwrap();
            let pixels = image.to_pixels().unwrap();
            let opaque = pixels.chunks_exact(4).filter(|p| p[3] > 200).count();
            assert!(opaque > 48 * 48 / 20, "{} quase vazio ({opaque} pixels)", icon.name());
            rendered += 1;
        }
        assert_eq!(rendered, 79);
    });
}

#[test]
fn picture_box_draws_svg_at_its_size() {
    with_app(|app| {
        let window = test_window();
        let picture =
            PictureBox::builder().bounds(0, 0, 64, 32).background(Color::WHITE).parent(&window).build().unwrap();
        picture.set_svg(
            SvgImage::parse(r##"<svg viewBox="0 0 2 1"><rect width="1" height="1" fill="#00f"/></svg>"##).unwrap(),
        );
        assert!(picture.has_content());
        app.process_pending().unwrap();
        let probe = |x: i32, y: i32| {
            use arqrustui::ffi::gdi32::*;
            use arqrustui::ffi::user32::*;
            // SAFETY: DCs e bitmap temporários desta thread, liberados ao final; a janela é viva.
            unsafe {
                let screen = GetDC(std::ptr::null_mut());
                let memory = CreateCompatibleDC(screen);
                let bitmap = CreateCompatibleBitmap(screen, 64, 32);
                let previous = SelectObject(memory, bitmap as _);
                SendMessageW(picture.handle() as _, 0x0318, memory as usize, 4);
                let c = GetPixel(memory, x, y);
                SelectObject(memory, previous);
                DeleteObject(bitmap as _);
                DeleteDC(memory);
                ReleaseDC(std::ptr::null_mut(), screen);
                ((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8)
            }
        };
        assert_eq!(probe(16, 16), (0, 0, 255), "metade esquerda azul");
        assert_eq!(probe(48, 16), (255, 255, 255), "metade direita vazia (fundo)");
        window.destroy();
    });
}
