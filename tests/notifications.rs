//! InfoBar (faixa de aviso na janela) e Toast (aviso temporário no canto).

mod common;

use arqrustui::controls::toast::debug_toasts;
use arqrustui::ffi::user32::{PostMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP};
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

fn post(app: &Application, widget: &impl Widget, message: u32, wparam: usize, lparam: isize) {
    // SAFETY: mensagem sem ponteiros postada a uma janela viva da thread de teste.
    unsafe { PostMessageW(widget.handle() as _, message, wparam, lparam) };
    app.process_pending().unwrap();
}

#[test]
fn info_bar_opens_closes_and_frees_its_space() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let bar = InfoBar::builder()
            .severity(Severity::Warning)
            .title("Sem conexão.")
            .message("Tentando de novo.")
            .parent(&window)
            .build()
            .unwrap();
        let below = Label::builder().text("conteúdo").parent(&window).build().unwrap();
        let root = StackLayout::vertical();
        root.add_with(&bar, LayoutParams::new().height(INFO_BAR_HEIGHT));
        root.add_with(&below, LayoutParams::new().height(20));
        window.set_layout(root);
        assert_eq!(below.bounds().y, INFO_BAR_HEIGHT);
        assert!(bar.is_open());
        assert!(bar.text().starts_with("Aviso: Sem conexão."), "nome acessível: {}", bar.text());

        let closed = Counter::new();
        let hit = closed.clone();
        bar.on_closed(move || hit.hit());
        let retries = Counter::new();
        let hit = retries.clone();
        let action = bar.add_action("Tentar agora", move || hit.hit()).unwrap();
        let place = action.bounds();
        let width = bar.bounds().width;
        assert!(place.right() < width - 32 && place.y > 0, "ação antes do ×: {place:?}");

        // Clique no × (centro em largura - 20, 22).
        let at = (22 << 16) | (width as isize - 20);
        post(app, &bar, WM_LBUTTONDOWN, 1, at);
        post(app, &bar, WM_LBUTTONUP, 0, at);
        assert!(!bar.is_open());
        assert_eq!(closed.get(), 1);
        assert_eq!(below.bounds().y, 0, "o layout recolhe o espaço da faixa");
        bar.close();
        assert_eq!(closed.get(), 1, "fechar de novo não avisa outra vez");

        bar.open_with(Severity::Error, "Falha.", "Disco cheio.");
        assert!(bar.is_open());
        assert_eq!(below.bounds().y, INFO_BAR_HEIGHT);
        assert_eq!(
            (bar.severity(), bar.title(), bar.message()),
            (Severity::Error, "Falha.".into(), "Disco cheio.".into())
        );
        assert!(bar.text().starts_with("Erro: Falha."));
        bar.set_closable(false);
        assert_eq!(retries.get(), 0);
    });
}

#[test]
fn toasts_stack_expire_and_close() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let dismissed = Counter::new();
        let hit = dismissed.clone();
        let short = Toast::new("Rápido")
            .message("some sozinho")
            .duration(Duration::from_millis(200))
            .on_dismissed(move || hit.hit())
            .show(&window)
            .unwrap();
        let sticky = Toast::new("Fixo").severity(Severity::Error).persistent().show(&window).unwrap();
        assert_eq!(Toast::count(&window), 2);
        assert!(short.is_open() && sticky.is_open());

        assert!(pump_until(app, Duration::from_secs(3), || !short.is_open()), "o toast curto expira");
        assert_eq!(dismissed.get(), 1);
        std::thread::sleep(Duration::from_millis(300));
        app.process_pending().unwrap();
        assert!(sticky.is_open(), "o persistente continua");
        assert!(pump_until(app, Duration::from_secs(2), || debug_toasts(&window).iter().all(|(_, a)| *a == 255)));

        sticky.close();
        assert!(pump_until(app, Duration::from_secs(2), || !sticky.is_open()));
        assert_eq!(Toast::count(&window), 0);

        Toast::new("Um").show(&window).unwrap();
        Toast::new("Dois").show(&window).unwrap();
        Toast::dismiss_all(&window);
        assert_eq!(Toast::count(&window), 0);

        let last = Toast::new("Fica").persistent().show(&window).unwrap();
        window.destroy();
        app.process_pending().unwrap();
        assert!(!last.is_open(), "toasts somem com a janela");
    });
}

#[test]
fn system_notifications_show_or_report_unsupported() {
    with_app(|_| {
        let activated = Rc::new(RefCell::new(Vec::new()));
        let dismissed = Rc::new(RefCell::new(Vec::new()));
        let (a, d) = (activated.clone(), dismissed.clone());
        let notification = SystemNotification::new("Backup concluído")
            .body("1.248 arquivos copiados.")
            .button("Abrir pasta", "abrir")
            .on_activated(move |argument| a.borrow_mut().push(argument.to_string()))
            .on_dismissed(move |reason| d.borrow_mut().push(reason));
        assert!(SystemNotification::app_id().starts_with("ArqRustUI."));
        match notification.show() {
            Ok(handle) => {
                // Eventos chegam à thread da interface e só uma vez por notificação.
                assert!(arqrustui::system::notifications::debug_deliver_latest(Some("abrir"), None));
                assert_eq!(*activated.borrow(), ["abrir"]);
                handle.hide().unwrap();
            }
            Err(GuiError::Unsupported(what)) => {
                // Wine e Windows 7: sem Windows.UI.Notifications.
                eprintln!("notificações do sistema indisponíveis: {what}");
                assert!(!SystemNotification::is_supported(), "{what}");
                assert!(dismissed.borrow().is_empty());
            }
            Err(other) => panic!("erro inesperado: {other:?}"),
        }
        assert!(SystemNotification::set_app_id("com espaço", "x", None).is_err());
    });
}

#[test]
fn notification_activator_receives_clicks_through_com() {
    use arqrustui::ffi::types::GUID;
    with_app(|app| {
        const CLSID: &str = "{6B1D3C2E-7A44-4F0B-9E21-5C8A3D9F0E17}";
        let got = Rc::new(RefCell::new(Vec::new()));
        let seen = got.clone();
        let activator = NotificationActivator::register(CLSID, move |activation| {
            seen.borrow_mut().push((activation.argument.clone(), activation.input("resposta").map(str::to_string)));
        })
        .unwrap();
        assert_eq!(activator.clsid(), CLSID);
        assert_eq!(NotificationActivator::registered_activator().as_deref(), Some(CLSID));
        assert!(!SystemNotification::launched_by_notification());

        // O Windows chamaria assim: CoCreateInstance(CLSID, LOCAL_SERVER) + Activate.
        let clsid = GUID::new(0x6B1D3C2E, 0x7A44, 0x4F0B, [0x9E, 0x21, 0x5C, 0x8A, 0x3D, 0x9F, 0x0E, 0x17]);
        let iid = GUID::new(0x53E31837, 0x6600, 0x4A81, [0x93, 0x95, 0x75, 0xCF, 0xFE, 0x74, 0x6F, 0x94]);
        let mut raw: *mut std::ffi::c_void = std::ptr::null_mut();
        // SAFETY: CLSID registrado acima nesta thread; interface liberada no fim.
        let hr = unsafe { arqrustui::ffi::ole32::CoCreateInstance(&clsid, std::ptr::null_mut(), 0x4, &iid, &mut raw) };
        if hr >= 0 && !raw.is_null() {
            let key: Vec<u16> = "resposta\0".encode_utf16().collect();
            let value: Vec<u16> = "olá\0".encode_utf16().collect();
            let args: Vec<u16> = "responder\0".encode_utf16().collect();
            let data =
                [arqrustui::ffi::winrt::NOTIFICATION_USER_INPUT_DATA { Key: key.as_ptr(), Value: value.as_ptr() }];
            type Activate = unsafe extern "system" fn(
                *mut std::ffi::c_void,
                *const u16,
                *const u16,
                *const arqrustui::ffi::winrt::NOTIFICATION_USER_INPUT_DATA,
                u32,
            ) -> i32;
            // SAFETY: vtable de INotificationActivationCallback (Activate é o 4º método).
            unsafe {
                let vtable = *(raw as *const *const usize);
                let activate: Activate = std::mem::transmute(*vtable.add(3));
                assert_eq!(activate(raw, std::ptr::null(), args.as_ptr(), data.as_ptr(), 1), 0);
                let release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 =
                    std::mem::transmute(*vtable.add(2));
                release(raw);
            }
            assert!(pump_until(app, Duration::from_secs(2), || !got.borrow().is_empty()));
            assert_eq!(*got.borrow(), [("responder".to_string(), Some("olá".to_string()))]);
        } else {
            // Alguns ambientes não ativam servidores locais do próprio processo.
            eprintln!("CoCreateInstance(LOCAL_SERVER) indisponível: 0x{hr:08X}");
        }
        drop(activator);
        NotificationActivator::unregister_from_registry(CLSID).unwrap();
    });
}

#[test]
fn scheduling_and_history_report_unsupported_without_winrt() {
    with_app(|_| {
        let future = std::time::SystemTime::now() + Duration::from_secs(60);
        let toast = SystemNotification::new("Lembrete").tag("t1").group("g1").progress("P", None, "", "...");
        match toast.show_at(future) {
            Ok(scheduled) => scheduled.cancel().unwrap(),
            Err(GuiError::Unsupported(_)) => {}
            Err(other) => panic!("agendamento falhou: {other}"),
        }
        assert!(toast.show_at(std::time::SystemTime::now() - Duration::from_secs(1)).is_err());
        for result in [SystemNotification::remove("t1", Some("g1")), SystemNotification::remove_group("g1")] {
            assert!(matches!(result, Ok(()) | Err(GuiError::Unsupported(_))), "{result:?}");
        }
    });
}
