//! Ciclo de vida: aplicação, janelas, controles, destruição e erros de uso.

mod common;

use arqrustui::prelude::*;
use common::*;
use std::time::Duration;

#[test]
fn application_is_single_per_thread_and_restartable() {
    {
        let app = Application::new().expect("primeira");
        assert!(matches!(Application::new(), Err(GuiError::AlreadyInitialized)));
        assert_eq!(app.window_count(), 0);
        // Sem janelas, run() retorna imediatamente em vez de bloquear.
        app.run().expect("run sem janelas");
    }
    // Depois de encerrada, a thread pode criar outra aplicação.
    let app = Application::new().expect("segunda");
    drop(app);
}

#[test]
fn gui_objects_require_an_application() {
    assert!(matches!(Window::builder().build(), Err(GuiError::NotInitialized)));
    assert!(matches!(Menu::new(), Err(GuiError::NotInitialized)));
    assert!(matches!(Timer::once(Duration::from_millis(1), || {}), Err(GuiError::NotInitialized)));
    assert!(matches!(defer(|| {}), Err(GuiError::NotInitialized)));
}

#[test]
fn control_builder_requires_parent() {
    with_app(|_| {
        assert!(matches!(Button::builder().text("x").build(), Err(GuiError::MissingParent)));
        assert!(matches!(TextBox::builder().build(), Err(GuiError::MissingParent)));
    });
}

#[test]
fn window_has_requested_client_size_and_title() {
    with_app(|app| {
        let window = Window::builder().title("Título ação 漢字 😀").size(320, 200).build().unwrap();
        assert_eq!(window.client_size(), Size::new(320, 200));
        assert_eq!(window.title(), "Título ação 漢字 😀");
        assert_eq!(app.window_count(), 1);
        assert!(!window.is_visible());
        window.show();
        assert!(window.is_visible());
        window.set_size(400, 250);
        assert_eq!(window.client_size(), Size::new(400, 250));
        let outer = window.outer_bounds();
        assert!(outer.width >= 400 && outer.height >= 250);
        assert!(window.dpi() >= 96);
        assert!(Window::builder().size(-1, 10).build().is_err());
    });
}

#[test]
fn destroy_is_idempotent_and_handles_become_inert() {
    with_app(|app| {
        let window = test_window();
        let button = Button::builder().text("OK").parent(&window).build().unwrap();
        let clicks = Counter::new();
        let destroyed = Counter::new();
        button.on_click({
            let clicks = clicks.clone();
            move || clicks.hit()
        });
        button.on_destroy({
            let destroyed = destroyed.clone();
            move || destroyed.hit()
        });
        button.click();
        assert_eq!(clicks.get(), 1);

        button.destroy();
        button.destroy();
        assert!(!button.is_alive());
        assert_eq!(destroyed.get(), 1);
        assert_eq!(button.handle(), 0);
        // Uso após destruição: nada acontece e nada quebra.
        button.click();
        button.set_text("ignorado");
        button.set_bounds(Rect::new(0, 0, 10, 10));
        assert_eq!(button.text(), "");
        assert_eq!(button.bounds(), Rect::default());
        assert!(button.set_tooltip("x").is_err());
        assert_eq!(clicks.get(), 1);
        // Registrar manipuladores em um controle morto não os retém.
        button.on_click(|| panic!("não deveria ser chamado"));
        assert!(window.is_alive());
        assert_eq!(app.window_count(), 1);
    });
}

#[test]
fn destroying_parent_destroys_children() {
    with_app(|app| {
        let window = test_window();
        let panel = Panel::builder().parent(&window).build().unwrap();
        let label = Label::builder().text("filho").parent(&panel).build().unwrap();
        let text = TextBox::builder().parent(&window).build().unwrap();
        let destroyed = Counter::new();
        label.on_destroy({
            let d = destroyed.clone();
            move || d.hit()
        });
        text.on_destroy({
            let d = destroyed.clone();
            move || d.hit()
        });
        window.destroy();
        assert!(!window.is_alive() && !panel.is_alive() && !label.is_alive() && !text.is_alive());
        assert_eq!(destroyed.get(), 2);
        assert_eq!(app.window_count(), 0);
    });
}

#[test]
fn application_drop_destroys_remaining_windows() {
    let window;
    let button;
    {
        let _app = Application::new().unwrap();
        window = test_window();
        button = Button::builder().parent(&window).build().unwrap();
        assert!(window.is_alive());
    }
    assert!(!window.is_alive());
    assert!(!button.is_alive());
}

#[test]
fn close_handler_can_veto() {
    with_app(|_| {
        let window = test_window();
        let allow = std::rc::Rc::new(std::cell::Cell::new(false));
        let asked = Counter::new();
        window.on_close_request({
            let (allow, asked) = (allow.clone(), asked.clone());
            move || {
                asked.hit();
                allow.get()
            }
        });
        window.close();
        assert!(window.is_alive());
        allow.set(true);
        window.close();
        assert!(!window.is_alive());
        assert_eq!(asked.get(), 2);
    });
}

#[test]
fn run_ends_when_last_window_closes() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let target = window.clone();
        Timer::once(Duration::from_millis(30), move || target.close()).unwrap();
        app.run().expect("run");
        assert!(!window.is_alive());
    });
}

#[test]
fn modal_window_blocks_owner_until_closed() {
    with_app(|_| {
        let owner = test_window();
        owner.show();
        let dialog = Dialog::new(&owner, "Modal", 200, 100).unwrap();
        let observed_disabled = std::rc::Rc::new(std::cell::Cell::new(false));
        let handle = dialog.clone();
        let (owner_probe, flag) = (owner.clone(), observed_disabled.clone());
        Timer::once(Duration::from_millis(30), move || {
            flag.set(!owner_probe.is_enabled());
            handle.close_with(DialogResult::Ok);
        })
        .unwrap();
        let result = dialog.show_modal().expect("modal");
        assert_eq!(result, DialogResult::Ok);
        assert!(observed_disabled.get(), "a janela dona deveria estar desabilitada durante o modal");
        assert!(owner.is_enabled());
        assert!(!dialog.window().is_alive());
    });
}

#[test]
fn modeless_dialog_reports_result_asynchronously() {
    with_app(|app| {
        let owner = test_window();
        let dialog = Dialog::new(&owner, "Não modal", 200, 100).unwrap();
        let result = std::rc::Rc::new(std::cell::Cell::new(None));
        let slot = result.clone();
        dialog.show_modeless(move |r| slot.set(Some(r)));
        assert!(result.get().is_none());
        assert!(owner.is_enabled());
        dialog.close_with(DialogResult::Custom(7));
        app.process_pending().unwrap();
        assert_eq!(result.get(), Some(DialogResult::Custom(7)));
    });
}

#[test]
fn panic_in_handler_propagates_out_of_event_loop() {
    let outcome = std::panic::catch_unwind(|| {
        with_app(|app| {
            let window = test_window();
            let button = Button::builder().parent(&window).build().unwrap();
            button.on_click(|| panic!("falha proposital"));
            let target = button.clone();
            defer(move || target.click()).unwrap();
            app.process_pending().unwrap();
        });
    });
    let payload = outcome.expect_err("o panic deveria ter sido propagado");
    let message = payload.downcast_ref::<&str>().copied().unwrap_or_default();
    assert_eq!(message, "falha proposital");
    // A thread continua utilizável depois do panic.
    with_app(|_| {
        assert!(test_window().is_alive());
    });
}

#[test]
fn window_state_changes() {
    with_app(|app| {
        let window = test_window();
        window.show();
        window.set_min_size(200, 150);
        window.set_size(50, 50);
        app.process_pending().unwrap();
        let size = window.client_size();
        assert!(size.width >= 200 && size.height >= 150, "tamanho mínimo ignorado: {size:?}");
        assert!(!window.is_fullscreen());
        window.set_fullscreen(true);
        assert!(window.is_fullscreen());
        let monitor = window.monitor().expect("monitor");
        assert_eq!(window.outer_bounds().size(), monitor.bounds.size());
        window.set_fullscreen(false);
        assert!(!window.is_fullscreen());
        window.set_opacity(0.5);
        window.set_topmost(true);
        window.set_topmost(false);
        window.center();
        window.set_icon(&Icon::system(SystemIcon::Information).unwrap());
        assert!(!Monitor::all().is_empty());
        assert!(Monitor::primary().is_some());
    });
}

#[test]
fn destroyed_widgets_release_their_state_even_with_reference_cycles() {
    with_app(|_| {
        let window = test_window();
        let button = Button::builder().parent(&window).build().unwrap();
        let text = TextBox::builder().parent(&window).build().unwrap();
        // Ciclos propositais: manipuladores que capturam o próprio controle e o vizinho.
        let (b, t) = (button.clone(), text.clone());
        button.on_click(move || t.set_text(&b.text()));
        let (b, t) = (button.clone(), text.clone());
        text.on_text_changed(move || b.set_enabled(!t.text().is_empty()));
        let weak_button = std::rc::Rc::downgrade(button.core());
        let weak_text = std::rc::Rc::downgrade(text.core());
        let weak_window = std::rc::Rc::downgrade(window.core());
        window.destroy();
        drop((button, text, window));
        assert!(weak_button.upgrade().is_none(), "o estado do botão deveria ter sido liberado");
        assert!(weak_text.upgrade().is_none(), "o estado do campo deveria ter sido liberado");
        assert!(weak_window.upgrade().is_none(), "o estado da janela deveria ter sido liberado");
    });
}

#[test]
fn repeated_create_destroy_does_not_leak_native_objects() {
    fn counts() -> (u32, u32) {
        // SAFETY: consultas de contadores do próprio processo.
        unsafe {
            let process = arqrustui::ffi::kernel32::GetCurrentProcess();
            (arqrustui::ffi::user32::GetGuiResources(process, 0), arqrustui::ffi::user32::GetGuiResources(process, 1))
        }
    }
    with_app(|app| {
        let cycle = |index: usize| {
            let window = test_window();
            let menu = Menu::new().unwrap();
            menu.add_item("item", || {}).unwrap();
            let bar = MenuBar::new().unwrap();
            bar.add_menu("Menu", &menu).unwrap();
            window.set_menu(&bar).unwrap();
            for row in 0..5 {
                let button = Button::builder().text("b").position(0, row * 30).parent(&window).build().unwrap();
                button.set_font(Font::new("Arial", 8.0 + (index % 7) as f32));
                button.set_tooltip("dica").unwrap();
                button.set_background(Color::rgb((index % 250) as u8, 10, 10));
                TextBox::builder().position(100, row * 30).parent(&window).build().unwrap();
            }
            let list = ListView::builder().parent(&window).build().unwrap();
            list.add_column("c", 50, ColumnAlign::Left);
            list.add_row(&["x"]);
            Theme::dark().apply(&window);
            window.destroy();
        };
        // Aquecimento: caches (fontes, pincéis, classes) atingem o regime permanente.
        for index in 0..20 {
            cycle(index);
        }
        app.process_pending().unwrap();
        let (gdi_before, user_before) = counts();
        for index in 0..200 {
            cycle(index);
        }
        app.process_pending().unwrap();
        let (gdi_after, user_after) = counts();
        // Tolerância: o cache de pincéis guarda no máximo uma entrada por cor usada.
        assert!(gdi_after <= gdi_before + 300, "vazamento de objetos GDI: {gdi_before} -> {gdi_after}");
        assert!(user_after <= user_before + 20, "vazamento de objetos USER: {user_before} -> {user_after}");
    });
}

#[test]
fn thread_safety_contract() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<UiDispatcher>();
    assert_send_sync::<GuiError>();
    assert_send_sync::<Font>();
    assert_send_sync::<Theme>();
}
