//! Example 16 — Direct2D + DirectWrite: hardware-accelerated vector animation.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Direct2D").size(640, 420).centered().build()?;
    let stage = Panel::builder().parent(&window).build()?;
    let layout = DockLayout::new();
    layout.add(&stage, Dock::Fill);
    window.set_layout(layout);

    let angle = Rc::new(Cell::new(0.0f32));
    let (rotation, probe) = (angle.clone(), stage.clone());
    let attached = Direct2DSurface::attach(&stage, move |d2d| {
        let size = probe.client_size();
        let (width, height) = (size.width, size.height);
        d2d.clear(Color::rgb(15, 23, 42));
        d2d.gradient_rect(
            Rect::new(0, 0, width, 70),
            (0.0, 0.0),
            (width as f32, 0.0),
            Color::rgb(79, 70, 229),
            Color::rgb(14, 165, 233),
        );
        d2d.text(
            "Direct2D + DirectWrite",
            Rect::new(16, 18, width - 32, 40),
            &Font::new("Segoe UI", 20.0).bold(),
            Color::WHITE,
        );

        // Spinning gear drawn with transforms.
        let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0 + 30.0);
        d2d.push_transform();
        d2d.translate(cx, cy);
        d2d.rotate(rotation.get());
        for step in 0..12 {
            d2d.push_transform();
            d2d.rotate(step as f32 * 30.0);
            let hue = Color::rgb(56, 189, 248).mix(Color::rgb(244, 114, 182), step as f32 / 11.0);
            d2d.rounded_rect(Rect::new(70, -8, 60, 16), 8.0, Some(hue), None);
            d2d.pop_transform();
        }
        d2d.ellipse(0.0, 0.0, 52.0, 52.0, Some(Color::rgb(30, 41, 59)), Some((Color::rgb(148, 163, 184), 3.0)));
        d2d.pop_transform();

        let star = Path::new()
            .move_to(60, 110)
            .line_to(74, 150)
            .line_to(116, 150)
            .line_to(82, 175)
            .line_to(95, 215)
            .line_to(60, 190)
            .line_to(25, 215)
            .line_to(38, 175)
            .line_to(4, 150)
            .line_to(46, 150)
            .close();
        d2d.path(&star, Some(Color::rgb(250, 204, 21)), Some((Color::WHITE, 1.5)));
        d2d.line(16.0, height as f32 - 24.0, width as f32 - 16.0, height as f32 - 24.0, Color::rgb(71, 85, 105), 1.0);
        d2d.text(
            &format!("angle {:.0}°", rotation.get() % 360.0),
            Rect::new(16, height - 22, 300, 20),
            &Font::new("Consolas", 10.0),
            Color::rgb(148, 163, 184),
        );
    });

    match attached {
        Ok(surface) => {
            // ~60 frames per second without blocking the interface.
            let timer = Timer::interval(Duration::from_millis(16), move || {
                angle.set(angle.get() + 1.5);
                surface.redraw();
            })?;
            window.show();
            let result = app.run();
            drop(timer);
            result
        }
        Err(error) => {
            MessageBox::error("Direct2D", &format!("Direct2D is not available on this system:\n{error}"));
            Ok(())
        }
    }
}
