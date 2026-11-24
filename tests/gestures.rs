//! Gestos: mensagens WM_POINTER* (e o mouse) enviadas a um painel e reconhecidas como
//! toque, pinça, rotação e arrasto com inércia. Não há tela de toque no ambiente de
//! teste; as mensagens são sintéticas. O Wine responde `PT_MOUSE` ao `GetPointerType` de
//! ponteiros desconhecidos, então os testes de ponteiro ligam `mouse_pan` (o caminho do
//! toque real, `PT_TOUCH`, só difere nessa checagem).

mod common;

use arqrustui::ffi::user32::ClientToScreen;
use arqrustui::ffi::POINT;
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const WM_POINTERUPDATE: u32 = 0x0245;
const WM_POINTERDOWN: u32 = 0x0246;
const WM_POINTERUP: u32 = 0x0247;

fn pointer(widget: &impl Widget, message: u32, id: u16, x: i32, y: i32) {
    let mut point = POINT { x, y };
    // SAFETY: converte coordenadas de um controle vivo desta thread.
    unsafe { ClientToScreen(widget.handle() as _, &mut point) };
    let lp = ((point.y & 0xFFFF) << 16 | (point.x & 0xFFFF)) as isize;
    send(widget, message, id as usize, lp);
}

fn recorder(panel: &Panel, options: GestureOptions) -> (Rc<RefCell<Vec<GestureEvent>>>, gestures::GestureSubscription) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let subscription = gestures::on_gesture_with(panel, options, move |g| log.borrow_mut().push(*g)).unwrap();
    (events, subscription)
}

#[test]
fn pinch_rotate_and_tap_from_pointer_messages() {
    with_app(|_| {
        let window = test_window();
        let panel = Panel::builder().bounds(0, 0, 400, 300).parent(&window).build().unwrap();
        let (events, subscription) = recorder(&panel, GestureOptions { mouse_pan: true, ..GestureOptions::default() });
        pointer(&panel, WM_POINTERDOWN, 1, 100, 150);
        pointer(&panel, WM_POINTERDOWN, 2, 200, 150);
        for step in 1..=5 {
            pointer(&panel, WM_POINTERUPDATE, 1, 100 - step * 10, 150);
            pointer(&panel, WM_POINTERUPDATE, 2, 200 + step * 10, 150);
        }
        let last = *events.borrow().last().unwrap();
        assert_eq!(last.kind, GestureKind::Manipulation);
        assert!((last.total_scale - 2.0).abs() < 0.01, "pinça: {last:?}");
        assert!(last.total_translation.0.abs() < 1e-9, "centro parado");
        // Gira os dois dedos 90° em volta do centro (150, 150).
        pointer(&panel, WM_POINTERUPDATE, 1, 150, 50);
        pointer(&panel, WM_POINTERUPDATE, 2, 150, 250);
        let last = *events.borrow().last().unwrap();
        assert!((last.total_rotation - 90.0).abs() < 0.5, "rotação: {last:?}");
        // Parados antes de soltar: a velocidade não conta e não há inércia.
        std::thread::sleep(Duration::from_millis(150));
        pointer(&panel, WM_POINTERUP, 1, 150, 50);
        pointer(&panel, WM_POINTERUP, 2, 150, 250);
        assert_eq!(events.borrow().last().unwrap().phase, GesturePhase::Completed, "soltou devagar: sem inércia");

        events.borrow_mut().clear();
        pointer(&panel, WM_POINTERDOWN, 3, 30, 30);
        pointer(&panel, WM_POINTERUP, 3, 31, 30);
        assert_eq!(events.borrow().iter().map(|e| e.kind).collect::<Vec<_>>(), vec![GestureKind::Tap]);

        subscription.stop();
        pointer(&panel, WM_POINTERDOWN, 4, 30, 30);
        pointer(&panel, WM_POINTERUP, 4, 31, 30);
        assert_eq!(events.borrow().len(), 1, "parou de entregar");
        window.destroy();
    });
}

#[test]
fn swipe_continues_with_inertia() {
    with_app(|app| {
        let window = test_window();
        let panel = Panel::builder().bounds(0, 0, 400, 300).parent(&window).build().unwrap();
        let (events, _subscription) = recorder(&panel, GestureOptions { mouse_pan: true, ..GestureOptions::default() });
        pointer(&panel, WM_POINTERDOWN, 7, 20, 100);
        for step in 1..=6 {
            std::thread::sleep(Duration::from_millis(8));
            pointer(&panel, WM_POINTERUPDATE, 7, 20 + step * 25, 100);
        }
        pointer(&panel, WM_POINTERUP, 7, 20 + 6 * 25, 100);
        assert!(events.borrow().iter().all(|e| e.phase != GesturePhase::Completed), "rápido: continua por inércia");
        let completed = || events.borrow().iter().any(|e| e.phase == GesturePhase::Completed);
        assert!(pump_until(app, Duration::from_secs(5), completed));
        let inertia: f64 =
            events.borrow().iter().filter(|e| e.phase == GesturePhase::Inertia).map(|e| e.translation.0).sum();
        assert!(inertia > 20.0, "andou por inércia: {inertia}");
        // Um novo toque durante a inércia a interrompe (Completed antes do novo gesto).
        window.destroy();
    });
}

#[test]
fn mouse_drag_can_pan() {
    with_app(|_| {
        let window = test_window();
        let panel = Panel::builder().bounds(0, 0, 400, 300).parent(&window).build().unwrap();
        let options = GestureOptions { mouse_pan: true, inertia: false, ..GestureOptions::default() };
        let (events, _subscription) = recorder(&panel, options);
        let lp = |x: i32, y: i32| (y << 16 | x) as isize;
        send(&panel, 0x0201, 1, lp(10, 10)); // WM_LBUTTONDOWN
        send(&panel, 0x0200, 1, lp(40, 30)); // WM_MOUSEMOVE
        send(&panel, 0x0200, 1, lp(60, 50));
        send(&panel, 0x0202, 0, lp(60, 50)); // WM_LBUTTONUP
        let events = events.borrow();
        assert_eq!(events.first().unwrap().phase, GesturePhase::Started);
        let last = events.last().unwrap();
        assert_eq!(last.phase, GesturePhase::Completed);
        assert_eq!(last.total_translation, (50.0, 40.0));
        window.destroy();
    });
}
