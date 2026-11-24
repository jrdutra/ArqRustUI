//! Animações e transições dirigidas pelo temporizador da thread de interface.

mod common;

use arqrustui::prelude::*;
use common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[test]
fn values_progress_and_finish_once() {
    with_app(|app| {
        motion::set_animations_enabled(Some(true));
        let frames = Rc::new(RefCell::new(Vec::new()));
        let log = frames.clone();
        let done = Rc::new(Cell::new(0));
        let d = done.clone();
        let transition = motion::animate_value(10.0, 20.0, Duration::from_millis(120), Easing::Linear, move |v| {
            log.borrow_mut().push(v)
        })
        .unwrap();
        transition.on_finished(move || d.set(d.get() + 1));
        assert!(transition.is_running());
        assert_eq!(motion::running_animations(), 1);
        assert!(pump_until(app, Duration::from_secs(3), || done.get() == 1));
        let frames = frames.borrow();
        assert!(frames.len() >= 3, "vários quadros: {frames:?}");
        assert!(frames.windows(2).all(|w| w[0] <= w[1]), "crescente: {frames:?}");
        assert_eq!(*frames.last().unwrap(), 20.0, "último quadro no valor final");
        assert!(!transition.is_running());
        assert_eq!(motion::running_animations(), 0, "o temporizador para sem animações");
        let late = Rc::new(Cell::new(false));
        let l = late.clone();
        transition.on_finished(move || l.set(true));
        assert!(late.get(), "on_done depois do fim é chamado na hora");
    });
}

#[test]
fn cancel_finish_and_disabled_animations() {
    with_app(|app| {
        motion::set_animations_enabled(Some(true));
        let last = Rc::new(Cell::new(-1.0));
        let l = last.clone();
        let done = Rc::new(Cell::new(false));
        let d = done.clone();
        let t = motion::animate(Duration::from_secs(10), Easing::EaseOut, move |v| l.set(v)).unwrap();
        t.on_finished(move || d.set(true));
        t.cancel();
        app.process_pending().unwrap();
        std::thread::sleep(Duration::from_millis(40));
        app.process_pending().unwrap();
        assert!(!t.is_running() && !done.get(), "cancelada: sem on_done");
        assert!(last.get() < 1.0);

        let l = last.clone();
        let t = motion::animate(Duration::from_secs(10), Easing::EaseOut, move |v| l.set(v)).unwrap();
        t.finish();
        assert_eq!(last.get(), 1.0, "finish entrega o quadro final");
        assert!(!t.is_running());

        // Animações desligadas: vai direto ao fim no primeiro quadro.
        motion::set_animations_enabled(Some(false));
        assert!(!motion::animations_enabled());
        let l = last.clone();
        last.set(0.0);
        let t = motion::animate(Duration::from_secs(10), Easing::Linear, move |v| l.set(v)).unwrap();
        assert!(pump_until(app, Duration::from_secs(2), || !t.is_running()));
        assert_eq!(last.get(), 1.0);
        motion::set_animations_enabled(None);
    });
}

#[test]
fn bounds_and_window_effects() {
    with_app(|app| {
        motion::set_animations_enabled(Some(true));
        let window = test_window();
        window.show();
        let button = Button::builder().text("Mover").bounds(10, 10, 80, 28).parent(&window).build().unwrap();
        let t =
            motion::animate_bounds(&button, Rect::new(110, 60, 120, 32), Duration::from_millis(100), Easing::EaseInOut)
                .unwrap();
        let seen_middle = Rc::new(Cell::new(false));
        let check = button.clone();
        let middle = seen_middle.clone();
        let _probe = Timer::interval(Duration::from_millis(5), move || {
            let x = check.bounds().x;
            if x > 10 && x < 110 {
                middle.set(true);
            }
        })
        .unwrap();
        assert!(pump_until(app, Duration::from_secs(3), || !t.is_running()));
        assert_eq!(button.bounds(), Rect::new(110, 60, 120, 32));
        assert!(seen_middle.get(), "passou por posições intermediárias");

        let panel = Panel::builder()
            .bounds(10, 120, 100, 60)
            .background(Color::rgb(0, 120, 215))
            .parent(&window)
            .build()
            .unwrap();
        motion::hide_animated(&panel, ShowEffect::SlideDown, Duration::from_millis(60)).unwrap();
        assert!(!panel.is_visible());
        motion::show_animated(&panel, ShowEffect::Center, Duration::from_millis(60)).unwrap();
        assert!(panel.is_visible());

        let fade = motion::fade_window(&window, 0.2, 1.0, Duration::from_millis(60)).unwrap();
        assert!(pump_until(app, Duration::from_secs(3), || !fade.is_running()));
        window.destroy();
        motion::set_animations_enabled(None);
    });
}
