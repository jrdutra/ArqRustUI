//! Gráficos: geometria, cores conferidas por pixels (WM_PRINTCLIENT), clique, realce e
//! transição animada dos dados.

mod common;

use arqrustui::ffi::gdi32::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetPixel, SelectObject,
};
use arqrustui::ffi::user32::{GetDC, ReleaseDC, SendMessageW};
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

fn color_at(widget: &impl Widget, at: Point) -> Color {
    let size = widget.client_size();
    // SAFETY: DCs e bitmap temporários desta thread, liberados ao final; a janela é viva.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, size.width, size.height);
        let previous = SelectObject(memory, bitmap as _);
        SendMessageW(widget.handle() as _, 0x0318, memory as usize, 4);
        let c = GetPixel(memory, at.x, at.y);
        SelectObject(memory, previous);
        DeleteObject(bitmap as _);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
        Color::rgb((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8)
    }
}

fn lparam(at: Point) -> isize {
    ((at.y << 16) | (at.x & 0xFFFF)) as isize
}

#[test]
fn bars_are_drawn_clicked_and_highlighted() {
    with_app(|_| {
        let window = test_window();
        let chart = Chart::builder()
            .kind(ChartKind::Bar)
            .title("Vendas")
            .categories(&["T1", "T2", "T3"])
            .series(ChartSeries::new("2025", &[10.0, 20.0, 15.0]))
            .series(ChartSeries::new("2026", &[12.0, 25.0, 18.0]).with_color(Color::rgb(30, 160, 60)))
            .animated(false)
            .bounds(0, 0, 420, 280)
            .parent(&window)
            .build()
            .unwrap();
        let first = chart.item_center(0, 1).unwrap();
        let second = chart.item_center(1, 1).unwrap();
        assert_eq!(chart.hit_test(first), Some((0, 1)));
        assert_eq!(chart.hit_test(second), Some((1, 1)));
        assert_eq!(chart.hit_test(Point::new(2, 2)), None);
        assert_eq!(color_at(&chart, first), CHART_PALETTE[0]);
        assert_eq!(color_at(&chart, second), Color::rgb(30, 160, 60), "cor própria da série");
        assert_eq!(chart.color_of(1), Color::rgb(30, 160, 60));

        let clicks = Rc::new(RefCell::new(Vec::new()));
        let log = clicks.clone();
        chart.on_point_click(move |s, i| log.borrow_mut().push((s, i)));
        send(&chart, 0x0201, 1, lparam(second)); // WM_LBUTTONDOWN
        send(&chart, 0x0202, 0, lparam(second)); // WM_LBUTTONUP
        assert_eq!(*clicks.borrow(), vec![(1, 1)]);

        send(&chart, 0x0200, 0, lparam(first)); // WM_MOUSEMOVE
        assert_eq!(chart.hot_item(), Some((0, 1)));
        let hot = color_at(&chart, first);
        assert!(hot.luminance() > CHART_PALETTE[0].luminance(), "realce clareia a barra: {hot:?}");

        let summary = chart.summary();
        assert!(summary.contains("2025: T1 10, T2 20, T3 15"), "{summary}");
        chart.set_value_format(|v| format!("R$ {v:.0}"));
        assert!(chart.summary().contains("T2 R$ 25"));
        window.destroy();
    });
}

#[test]
fn lines_pies_and_animated_changes() {
    with_app(|app| {
        motion::set_animations_enabled(Some(true));
        let window = test_window();
        let chart = Chart::builder()
            .categories(&["Seg", "Ter", "Qua", "Qui"])
            .series(ChartSeries::new("Acessos", &[5.0, 9.0, 4.0, 12.0]))
            .bounds(0, 0, 420, 280)
            .parent(&window)
            .build()
            .unwrap();
        assert!(chart.is_animating(), "entrada animada");
        assert!(pump_until(app, Duration::from_secs(3), || !chart.is_animating()));
        let point = chart.item_center(0, 3).unwrap();
        assert_eq!(color_at(&chart, point), CHART_PALETTE[0], "marcador do ponto");
        assert_eq!(chart.hit_test(Point::new(point.x + 2, point.y + 2)), Some((0, 3)));

        chart.set_values(0, &[5.0, 9.0, 4.0, 2.0]);
        assert!(chart.is_animating());
        assert!(pump_until(app, Duration::from_secs(3), || !chart.is_animating()));
        let lower = chart.item_center(0, 3).unwrap();
        assert!(lower.y > point.y, "o ponto desceu: {point:?} → {lower:?}");

        chart.set_animated(false);
        chart.set_kind(ChartKind::Pie);
        assert!(!chart.is_animating());
        for (slice, color) in CHART_PALETTE.iter().enumerate().take(4) {
            let center = chart.item_center(0, slice).unwrap();
            assert_eq!(chart.hit_test(center), Some((0, slice)));
            assert_eq!(color_at(&chart, center), *color, "fatia {slice}");
        }
        chart.set_kind(ChartKind::StackedBar);
        chart.add_series(ChartSeries::new("Novos", &[1.0, 2.0, 3.0, 4.0]));
        let bottom = chart.item_center(0, 2).unwrap();
        let top = chart.item_center(1, 2).unwrap();
        assert_eq!(top.x, bottom.x);
        assert!(top.y < bottom.y, "segunda série empilhada em cima");
        motion::set_animations_enabled(None);
        window.destroy();
    });
}

#[test]
fn chart_follows_the_dark_theme() {
    with_app(|app| {
        let window = test_window();
        let chart = Chart::builder()
            .kind(ChartKind::Area)
            .series(ChartSeries::new("a", &[1.0, 3.0, 2.0]))
            .animated(false)
            .bounds(0, 0, 300, 200)
            .parent(&window)
            .build()
            .unwrap();
        assert!(color_at(&chart, Point::new(3, 3)).luminance() > 0.7);
        Theme::dark().apply(&window);
        app.process_pending().unwrap();
        assert!(color_at(&chart, Point::new(3, 3)).luminance() < 0.3);
        window.destroy();
    });
}
