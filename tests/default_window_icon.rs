//! Logo padrão, precedência de ícones explícitos e reaplicação ao mudar o DPI.

mod common;

use arqrustui::ffi::dynamic::optional;
use arqrustui::ffi::user32::{GetSystemMetrics, GetWindowRect, ICON_BIG, ICON_SMALL, SM_CXICON, SM_CXSMICON};
use arqrustui::ffi::RECT;
use arqrustui::prelude::*;
use common::*;

fn window_icon(window: &Window, kind: usize) -> isize {
    send(window, arqrustui::ffi::user32::WM_GETICON, kind, 0)
}

fn assert_default_logo(window: &Window) {
    for (kind, metric) in [(ICON_SMALL, SM_CXSMICON), (ICON_BIG, SM_CXICON)] {
        // SAFETY: consultas sem ponteiros; a função por DPI só é chamada quando disponível.
        let size = unsafe {
            optional().GetSystemMetricsForDpi.map_or_else(|| GetSystemMetrics(metric), |get| get(metric, window.dpi()))
        };
        let expected = icons::DEFAULT_WINDOW_ICON.icon(size).expect("logo válido");
        assert_ne!(window_icon(window, kind), 0);
        assert_eq!(window_icon(window, kind), expected.raw() as isize, "logo no DPI da janela");
    }
}

fn reapply_dpi(window: &Window) {
    let mut bounds = RECT::default();
    // SAFETY: RECT local e janela viva; a mensagem síncrona mantém o ponteiro válido.
    unsafe { GetWindowRect(window.handle() as _, &mut bounds) };
    let dpi = window.dpi();
    send(window, 0x02E0, (dpi | dpi << 16) as usize, &bounds as *const RECT as isize);
}

#[test]
fn windows_and_custom_dialogs_use_the_logo_without_an_explicit_icon() {
    with_app(|_| {
        let first = test_window();
        let second = Window::builder().owner(&first).build().unwrap();
        let dialog = Dialog::new(&first, "Diálogo", 200, 100).unwrap();
        assert_default_logo(&first);
        assert_default_logo(&second);
        assert_default_logo(dialog.window());
        reapply_dpi(&first);
        assert_default_logo(&first);
        first.destroy();
    });
}

#[test]
fn explicit_builder_and_runtime_icons_override_the_logo_and_survive_dpi_changes() {
    with_app(|_| {
        let warning = Icon::system(SystemIcon::Warning).unwrap();
        let window = Window::builder().icon(&warning).build().unwrap();
        for kind in [ICON_SMALL, ICON_BIG] {
            assert_eq!(window_icon(&window, kind), warning.raw() as isize);
        }
        reapply_dpi(&window);
        assert_eq!(window_icon(&window, ICON_SMALL), warning.raw() as isize);

        window.set_icon(&icons::DEFAULT_WINDOW_ICON);
        assert_default_logo(&window);
        let error = Icon::system(SystemIcon::Error).unwrap();
        window.set_icon(&error);
        reapply_dpi(&window);
        for kind in [ICON_SMALL, ICON_BIG] {
            assert_eq!(window_icon(&window, kind), error.raw() as isize);
        }
        let dialog = Dialog::from_builder(Window::builder().owner(&window).icon(&warning), Some(&window)).unwrap();
        assert_eq!(window_icon(dialog.window(), ICON_SMALL), warning.raw() as isize);
        window.destroy();
    });
}

#[test]
fn logo_is_available_at_all_supplied_sizes_without_the_optional_catalog() {
    with_app(|_| {
        let logo = &icons::DEFAULT_WINDOW_ICON;
        assert_eq!(logo.sizes(), vec![16, 24, 32, 48, 64, 96, 128, 256]);
        for size in logo.sizes() {
            assert_eq!(logo.icon(size as i32).unwrap().size(), Size::new(size as i32, size as i32));
        }
        assert_eq!(logo.png_size(), Size::new(256, 256));
        assert_eq!(logo.image().unwrap().size(), Size::new(256, 256));
    });
}
