//! Regressões de geometria ao maximizar e atualizar o conteúdo/escala.
use arqrustui::ffi::{user32::*, POINT, RECT};
use arqrustui::prelude::*;

fn fixture() -> (Window, StatusBar, Panel, StatusBarLayout) {
    let window = Window::builder().title("Geometria da janela").size(420, 320).build().unwrap();
    let status = StatusBar::new(&window).unwrap();
    status.set_part_text(0, "Rodapé");
    let panel = Panel::builder().parent(&window).build().unwrap();
    let body = StackLayout::vertical();
    body.add_with(&panel, LayoutParams::new().height(180).weight(1.0));
    let layout = StatusBarLayout::new(body, &status).minimum_width(300);
    window.set_layout(layout.clone());
    (window, status, panel, layout)
}

fn assert_footer(window: &Window, status: &StatusBar) {
    let mut client = RECT::default();
    let mut footer = RECT::default();
    let mut origin = POINT::default();
    // SAFETY: handles vivos desta thread; ponteiros graváveis para estruturas locais.
    unsafe {
        assert_ne!(GetClientRect(window.handle() as _, &mut client), 0);
        assert_ne!(GetWindowRect(status.handle() as _, &mut footer), 0);
        assert_ne!(ClientToScreen(window.handle() as _, &mut origin), 0);
    }
    assert!(footer.bottom > footer.top, "rodapé sem altura: {footer:?}");
    assert_eq!(footer.bottom, origin.y + client.bottom, "rodapé fora da base");
    assert_eq!(footer.left, origin.x);
    assert_eq!(footer.right, origin.x + client.right);
}

fn assert_fills_work_area(window: &Window) {
    let work = window.monitor().unwrap().work_area;
    let mut client = RECT::default();
    let mut origin = POINT::default();
    // SAFETY: janela viva e estruturas locais graváveis.
    unsafe {
        assert_ne!(GetClientRect(window.handle() as _, &mut client), 0);
        assert_ne!(ClientToScreen(window.handle() as _, &mut origin), 0);
    }
    assert_eq!(origin.x, work.x);
    assert_eq!(origin.x + client.right, work.right());
    assert_eq!(origin.y + client.bottom, work.bottom());
}

#[test]
fn fitting_content_preserves_maximized_geometry() {
    let app = Application::new().unwrap();
    let (window, status, panel, layout) = fixture();
    layout.fit_height(&window);
    for _ in 0..3 {
        window.maximize();
        app.process_pending().unwrap();
        let maximized = window.outer_bounds();
        let size = window.client_size();
        assert_fills_work_area(&window);
        assert_footer(&window, &status);
        panel.hide();
        layout.fit_height(&window);
        assert!(window.is_maximized());
        assert_eq!(window.outer_bounds(), maximized);
        assert_eq!(window.client_size(), size);
        assert_fills_work_area(&window);
        assert_footer(&window, &status);
        panel.show();
        layout.fit_height(&window);
        assert_eq!(window.outer_bounds(), maximized);
        window.restore();
        layout.fit_height(&window);
        assert_eq!(window.client_size().height, layout.preferred_size().height);
    }
    window.destroy();
}

#[test]
fn dpi_refresh_keeps_footer_at_bottom() {
    let app = Application::new().unwrap();
    let (window, status, _, _) = fixture();
    window.maximize();
    app.process_pending().unwrap();
    assert_footer(&window, &status);
    let bounds = window.outer_bounds();
    let suggested = RECT { left: bounds.x, top: bounds.y, right: bounds.right(), bottom: bounds.bottom() };
    let dpi = window.dpi() as usize;
    // Exercita a reaplicação de DPI sem alterar a configuração dos monitores.
    // SAFETY: RECT válido durante o envio síncrono à janela da thread atual.
    unsafe {
        SendMessageW(window.handle() as _, WM_DPICHANGED, dpi | (dpi << 16), &suggested as *const RECT as isize);
    }
    app.process_pending().unwrap();
    assert_footer(&window, &status);
    window.destroy();
}

#[test]
fn fitting_content_preserves_minimized_placement() {
    let app = Application::new().unwrap();
    let (window, status, panel, layout) = fixture();
    layout.fit_height(&window);
    window.show();
    let normal = window.outer_bounds();
    window.minimize();
    panel.hide();
    layout.fit_height(&window);
    assert!(window.is_minimized());
    window.restore();
    app.process_pending().unwrap();
    assert_eq!(window.outer_bounds(), normal);
    assert_footer(&window, &status);
    layout.fit_height(&window);
    assert_eq!(window.client_size().height, layout.preferred_size().height);
    window.destroy();
}

#[test]
fn fitting_content_preserves_fullscreen_geometry() {
    let app = Application::new().unwrap();
    let (window, status, panel, layout) = fixture();
    window.show();
    window.set_fullscreen(true);
    app.process_pending().unwrap();
    let fullscreen = window.outer_bounds();
    panel.hide();
    layout.fit_height(&window);
    assert!(window.is_fullscreen());
    assert_eq!(window.outer_bounds(), fullscreen);
    assert_footer(&window, &status);
    window.set_fullscreen(false);
    window.destroy();
}
