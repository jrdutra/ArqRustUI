//! Example 15 — Custom drawing with GDI and GDI+ and an interactive control of our own.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Custom drawing").size(680, 420).centered().build()?;

    // Left panel: shapes with GDI (lines, polygons, paths, gradient, transforms).
    let shapes = Panel::builder().border(true).background(Color::WHITE).parent(&window).build()?;
    shapes.on_paint(|canvas| {
        let size = canvas.size();
        canvas.gradient_rect(
            Rect::new(0, 0, size.width, 60),
            Color::rgb(0, 95, 184),
            Color::rgb(96, 205, 255),
            GradientDirection::Horizontal,
        );
        canvas.text("GDI + GDI+", Point::new(12, 16), &Font::new("Segoe UI", 16.0).bold(), Color::WHITE);
        let pen = Pen::new(Color::rgb(30, 30, 30), 2);
        canvas.rect(Rect::new(16, 80, 90, 60), Some(&pen), Some(Color::rgb(253, 224, 71)));
        canvas.round_rect(Rect::new(120, 80, 90, 60), 14, Some(&pen), Some(Color::rgb(134, 239, 172)));
        canvas.ellipse(Rect::new(224, 80, 70, 60), Some(&pen), Some(Color::rgb(252, 165, 165)));
        canvas.polygon(
            &[Point::new(60, 170), Point::new(100, 240), Point::new(20, 240)],
            Some(&pen),
            Some(Color::rgb(196, 181, 253)),
        );
        canvas.line(Point::new(120, 170), Point::new(290, 240), &Pen::new(Color::RED, 3).with_style(PenStyle::Solid));
        canvas.line(Point::new(120, 240), Point::new(290, 170), &Pen::new(Color::BLUE, 1).with_style(PenStyle::Dash));
        let wave = Path::new()
            .move_to(16, 300)
            .bezier_to(Point::new(70, 250), Point::new(110, 350), Point::new(160, 300))
            .bezier_to(Point::new(210, 250), Point::new(250, 350), Point::new(300, 300));
        canvas.path(&wave, Some(&Pen::new(Color::rgb(22, 163, 74), 3)), None);
        // Transform: rotated text.
        let state = canvas.save();
        canvas.translate(250.0, 260.0);
        canvas.rotate(-20.0);
        canvas.text("rotated", Point::new(0, 0), &Font::new("Segoe UI", 11.0).italic(), Color::GRAY);
        canvas.restore(state);
        // GDI+: antialiasing and transparency on the same canvas.
        let drawn = canvas.with_gdiplus(|g| {
            g.fill_ellipse(40.0, 320.0, 70.0, 70.0, Color::rgba(37, 99, 235, 150));
            g.fill_ellipse(80.0, 320.0, 70.0, 70.0, Color::rgba(220, 38, 38, 150));
            g.line(180.0, 330.0, 300.0, 385.0, Color::BLACK, 1.5);
        });
        if drawn.is_err() {
            canvas.text("GDI+ not available", Point::new(16, 330), &Font::system(), Color::RED);
        }
    });

    // Right panel: custom control (freehand drawing board).
    let board =
        Panel::builder().border(true).background(Color::rgb(250, 250, 245)).focusable(true).parent(&window).build()?;
    board.set_cursor(Some(Cursor::system(SystemCursor::Cross)?));
    accessibility::set_custom_role(&board, AccessibleRole::Custom, "Drawing board")?;
    let strokes: Rc<RefCell<Vec<Vec<Point>>>> = Rc::new(RefCell::new(Vec::new()));
    let model = strokes.clone();
    board.on_paint(move |canvas| {
        canvas.text("Drag to draw · Esc clears", Point::new(10, 8), &Font::system(), Color::GRAY);
        let pen = Pen::new(Color::rgb(17, 24, 39), 3);
        for stroke in model.borrow().iter() {
            canvas.polyline(stroke, &pen);
        }
    });
    let model = strokes.clone();
    board.on_mouse_down(move |event| {
        if event.button == MouseButton::Left {
            model.borrow_mut().push(vec![event.position]);
        }
    });
    let (model, target) = (strokes.clone(), board.clone());
    board.on_mouse_move(move |event| {
        // The pressed left button is tracked through the last open stroke.
        if arqrustui::input::Modifiers::current() == Modifiers::NONE && is_left_down() {
            if let Some(stroke) = model.borrow_mut().last_mut() {
                stroke.push(event.position);
            }
            target.invalidate();
        }
    });
    let (model, target) = (strokes.clone(), board.clone());
    board.on_key_down(move |event| {
        if event.key == Key::ESCAPE {
            model.borrow_mut().clear();
            target.invalidate();
        }
    });

    // Touch: pinch, rotation and drag (with inertia) over the shapes; the title shows the gesture.
    let view = Rc::new(Cell::new((1.0f64, 0.0f64, (0.0f64, 0.0f64))));
    let (state, title) = (view.clone(), window.clone());
    let _gestures = gestures::on_gesture(&shapes, move |g| {
        let (scale, angle, (x, y)) = match g.kind {
            GestureKind::Manipulation => {
                let (scale, angle, (x, y)) = state.get();
                (scale * g.scale, angle + g.rotation, (x + g.translation.0, y + g.translation.1))
            }
            GestureKind::DoubleTap => (1.0, 0.0, (0.0, 0.0)),
            _ => return,
        };
        state.set((scale, angle, (x, y)));
        title.set_title(&format!("Drawing — gesture: {:.0}% · {:.0}° · ({:.0}, {:.0})", scale * 100.0, angle, x, y));
    })?;

    let layout = SplitLayout::horizontal(&shapes, &board).ratio(0.5).gap(8);
    let root = DockLayout::new().padding(Thickness::all(8));
    root.add(layout, Dock::Fill);
    window.set_layout(root);
    window.show();
    app.run()
}

/// State of the left mouse button (advanced `ffi` API, used only in this example).
fn is_left_down() -> bool {
    const VK_LBUTTON: i32 = 0x01;
    // SAFETY: key state query, no preconditions.
    unsafe { arqrustui::ffi::user32::GetKeyState(VK_LBUTTON) < 0 }
}
