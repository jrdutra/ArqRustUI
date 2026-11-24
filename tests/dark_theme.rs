//! Tema escuro nos controles que a biblioteca pinta: abas, barra de status,
//! cabeçalhos de coluna, grupo e barra de ferramentas. A pintura é conferida pelos
//! pixels (WM_PRINTCLIENT), nos dois sentidos: escuro com tema escuro e o desenho
//! nativo (claro) de volta com o tema claro.

mod common;

use arqrustui::ffi::gdi32::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetPixel, SelectObject,
};
use arqrustui::ffi::user32::{GetDC, ReleaseDC, SendMessageW};
use arqrustui::prelude::*;
use common::*;

const WM_PRINTCLIENT: u32 = 0x0318;
const PRF_CLIENT: isize = 0x4;
const LVM_GETHEADER: u32 = 0x101F;

/// Luminância média de alguns pontos da área cliente desenhada com WM_PRINTCLIENT.
fn luminance_of(hwnd: isize, width: i32, height: i32, points: &[(i32, i32)]) -> f64 {
    // SAFETY: DCs e bitmap temporários desta thread, liberados ao final; a janela é viva.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        let previous = SelectObject(memory, bitmap as _);
        SendMessageW(hwnd as _, WM_PRINTCLIENT, memory as usize, PRF_CLIENT);
        let mut total = 0.0;
        for &(x, y) in points {
            let c = GetPixel(memory, x, y);
            let (r, g, b) = ((c & 0xFF) as f64, ((c >> 8) & 0xFF) as f64, ((c >> 16) & 0xFF) as f64);
            total += (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0;
        }
        SelectObject(memory, previous);
        DeleteObject(bitmap as _);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
        total / points.len() as f64
    }
}

#[test]
fn painted_controls_follow_the_theme() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let tabs = TabControl::builder().bounds(10, 10, 300, 180).parent(&window).build().unwrap();
        tabs.add_tab("Geral").unwrap();
        tabs.add_tab("Avançado").unwrap();
        let status = StatusBar::new(&window).unwrap();
        status.set_parts(&[120, -1]);
        status.set_part_text(0, "Pronto");
        let list = ListView::builder().bounds(10, 200, 300, 80).parent(&window).build().unwrap();
        list.add_column("Nome", 150, ColumnAlign::Left);
        let group = GroupBox::builder().text("Grupo").bounds(320, 10, 70, 80).parent(&window).build().unwrap();
        // SAFETY: mensagem sem ponteiros a uma ListView viva.
        let header = unsafe { SendMessageW(list.handle() as _, LVM_GETHEADER, 0, 0) };

        let probe =
            |hwnd: isize, w: i32, h: i32| luminance_of(hwnd, w, h, &[(w - 6, h / 2), (w / 2, h - 4), (4, h - 4)]);
        let status_size = status.client_size();
        let check = |dark: bool| {
            app.process_pending().unwrap();
            let threshold = |value: f64, what: &str| {
                if dark {
                    assert!(value < 0.35, "{what} deveria estar escuro: {value:.2}");
                } else {
                    assert!(value > 0.6, "{what} deveria estar claro: {value:.2}");
                }
            };
            threshold(probe(status.handle(), status_size.width, status_size.height), "barra de status");
            threshold(probe(tabs.handle(), 300, 180), "abas");
            threshold(probe(header, 300, 20), "cabeçalho da lista");
            threshold(probe(group.handle(), 70, 80), "grupo");
        };
        check(false);
        Theme::dark().apply(&window);
        check(true);
        Theme::light().apply(&window);
        check(false);
    });
}

#[test]
fn toolbar_and_native_colored_controls_accept_both_themes() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let toolbar = ToolBar::new(&window).unwrap();
        toolbar.add_button("Novo", Some(&icons::FILE), || {}).unwrap();
        toolbar.add_toggle_button("Negrito", None, |_| {}).unwrap();
        MonthCalendar::builder().bounds(10, 40, 240, 170).parent(&window).build().unwrap();
        DatePicker::builder().bounds(260, 40, 120, 28).parent(&window).build().unwrap();
        ProgressBar::builder().value(40).bounds(10, 220, 200, 16).parent(&window).build().unwrap();
        ScrollBar::builder().bounds(10, 240, 200, 16).parent(&window).build().unwrap();
        let text = TextBox::builder().bounds(10, 260, 200, 28).parent(&window).build().unwrap();
        for theme in [Theme::dark(), Theme::light(), Theme::dark()] {
            theme.apply(&window);
            window.invalidate();
            app.process_pending().unwrap();
        }
        // A barra de ferramentas pinta o fundo no tom do tema escuro.
        let size = toolbar.client_size();
        let value = luminance_of(toolbar.handle(), size.width, size.height, &[(size.width - 4, 4)]);
        assert!(value < 0.35, "barra de ferramentas no tom escuro: {value:.2}");
        assert!(text.is_alive());
    });
}

#[test]
fn date_pickers_sliders_and_the_native_menu_bar_follow_the_dark_theme() {
    with_app(|app| {
        let window = test_window();
        let file = Menu::new().unwrap();
        let hits = Counter::new();
        let hit = hits.clone();
        file.add_item_with_shortcut("&Novo", Shortcut::ctrl(Key(b'N' as u16)), move || hit.hit()).unwrap();
        let bar = MenuBar::new().unwrap();
        bar.add_menu("&Arquivo", &file).unwrap();
        window.set_menu(&bar).unwrap();
        window.show();
        let date = DatePicker::builder().bounds(10, 10, 160, 28).parent(&window).build().unwrap();
        let time = TimePicker::builder().bounds(10, 50, 160, 28).parent(&window).build().unwrap();
        let slider = Slider::builder().range(0, 10).value(3).bounds(10, 90, 200, 32).parent(&window).build().unwrap();
        let client = window.client_size();
        // Média de uma grade de pontos (o texto claro ocupa pouca área).
        let probe = |widget: &dyn Widget| {
            let size = widget.client_size();
            let points: Vec<(i32, i32)> =
                (1..6).flat_map(|i| (1..4).map(move |j| (size.width * i / 6, size.height * j / 4))).collect();
            luminance_of(widget.handle(), size.width, size.height, &points)
        };
        app.process_pending().unwrap();
        // (O DatePicker do Wine não responde a WM_PRINTCLIENT no desenho nativo; o claro
        // é conferido no Slider.)
        assert!(probe(&slider) > 0.6, "claro por padrão: {:.2}", probe(&slider));
        assert!(!window.is_menu_bar_drawn());

        Theme::dark().apply(&window);
        app.process_pending().unwrap();
        for (name, widget) in [("DatePicker", &date as &dyn Widget), ("TimePicker", &time), ("Slider", &slider)] {
            let value = probe(widget);
            assert!(value < 0.35, "{name} deveria estar escuro: {value:.2}");
        }
        assert!(window.is_menu_bar_drawn(), "barra nativa exibida como MenuStrip no escuro");
        assert!(window.menu_strip().is_some());
        assert_eq!(window.client_size(), client, "a troca não muda a área cliente");
        let strip = window.menu_strip().unwrap();
        strip.open(0).unwrap();
        assert!(strip.is_open(), "os mesmos menus abrem desenhados");
        strip.close();

        Theme::light().apply(&window);
        app.process_pending().unwrap();
        assert!(!window.is_menu_bar_drawn() && window.menu_strip().is_none(), "volta à barra nativa");
        assert_eq!(window.client_size(), client);
        assert!(probe(&slider) > 0.6, "Slider volta ao desenho nativo");

        window.set_menu_bar_follows_theme(false);
        Theme::dark().apply(&window);
        assert!(!window.is_menu_bar_drawn(), "a aplicação pode manter a barra nativa");
        let _ = hits;
    });
}
