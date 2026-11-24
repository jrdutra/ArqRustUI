//! Charts, SVG and animations — `Chart` (line, area, bars, stacked bars and pie)
//! with an animated transition when the data changes, SVG icons drawn by the library and a
//! panel that slides with `motion::animate_bounds`.
//!
//! Try it: change the chart type, click "New data", hover over the
//! bars/points/slices and click them; "Panel" opens and closes the side panel.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Charts, SVG and animations").size(860, 520).centered().build()?;
    let kinds = [
        ("Lines", ChartKind::Line),
        ("Area", ChartKind::Area),
        ("Bars", ChartKind::Bar),
        ("Stacked bars", ChartKind::StackedBar),
        ("Pie", ChartKind::Pie),
    ];
    let kind = ComboBox::builder()
        .items(&kinds.iter().map(|k| k.0).collect::<Vec<_>>())
        .selected(2)
        .bounds(12, 12, 200, 28)
        .parent(&window)
        .build()?;
    let shuffle = Button::builder().text("New data").bounds(224, 12, 120, 28).parent(&window).build()?;
    let toggle = Button::builder().text("Panel").bounds(356, 12, 100, 28).parent(&window).build()?;
    let status =
        Label::builder().text("Click an item in the chart.").bounds(470, 16, 370, 24).parent(&window).build()?;
    let chart = Chart::builder()
        .kind(ChartKind::Bar)
        .title("Sales per quarter")
        .categories(&["Q1", "Q2", "Q3", "Q4"])
        .series(ChartSeries::new("2025", &[12.0, 18.0, 9.0, 21.0]))
        .series(ChartSeries::new("2026", &[15.0, 22.0, 14.0, 25.0]))
        .bounds(12, 52, 600, 450)
        .parent(&window)
        .build()?;

    // Side panel with SVG icons (sharp at any size).
    let panel = Panel::builder().bounds(860, 52, 220, 450).parent(&window).build()?;
    for (row, icon) in [&icons::HOME, &icons::CLOCK, &icons::SHIELD].into_iter().enumerate() {
        let picture = PictureBox::builder().bounds(20, 20 + row as i32 * 140, 120, 120).parent(&panel).build()?;
        if let Some(Ok(svg)) = icon.svg_image() {
            picture.set_svg(svg);
        } else {
            picture.set_icon(icon);
        }
    }

    let target = chart.clone();
    kind.on_selection_changed(move |index| {
        if let Some((_, kind)) = index.and_then(|i| kinds.get(i)) {
            target.set_kind(*kind);
        }
    });
    let seed = Rc::new(Cell::new(7u32));
    let target = chart.clone();
    shuffle.on_click(move || {
        let next = || {
            seed.set(seed.get().wrapping_mul(1_103_515_245).wrapping_add(12_345));
            ((seed.get() >> 16) % 30 + 3) as f64
        };
        let first: Vec<f64> = (0..4).map(|_| next()).collect();
        let second: Vec<f64> = (0..4).map(|_| next()).collect();
        target.set_values(0, &first);
        target.set_values(1, &second);
    });
    let (label, source) = (status.clone(), chart.clone());
    chart.on_point_click(move |series, index| {
        let data = source.series();
        let value = data.get(series).and_then(|s| s.values.get(index)).copied().unwrap_or(0.0);
        let name = data.get(series).map(|s| s.name.clone()).unwrap_or_default();
        let category = source.categories().get(index).cloned().unwrap_or_default();
        label.set_text(&format!("{name}, {category}: {value}"));
    });
    let open = Rc::new(Cell::new(false));
    let (slide, area) = (panel.clone(), chart.clone());
    toggle.on_click(move || {
        let opening = !open.replace(!open.get());
        let (panel_to, chart_to) = if opening {
            (Rect::new(628, 52, 220, 450), Rect::new(12, 52, 600, 450))
        } else {
            (Rect::new(860, 52, 220, 450), Rect::new(12, 52, 836, 450))
        };
        let _ = motion::animate_bounds(&slide, panel_to, Duration::from_millis(280), Easing::EaseOut);
        let _ = motion::animate_bounds(&area, chart_to, Duration::from_millis(280), Easing::EaseOut);
    });
    chart.set_bounds(Rect::new(12, 52, 836, 450));

    window.show();
    app.run()
}
