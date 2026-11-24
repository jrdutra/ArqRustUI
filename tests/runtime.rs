//! Loop de eventos, despachante entre threads, temporizadores, `async`, menus,
//! layouts aplicados a janelas reais, temas, desenho e integração com o sistema.

mod common;

use arqrustui::prelude::*;
use common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn dispatcher_runs_closures_on_ui_thread() {
    with_app(|app| {
        let ui_thread = std::thread::current().id();
        let ran_on_ui = Arc::new(AtomicBool::new(false));
        let dispatcher = app.dispatcher();
        let flag = ran_on_ui.clone();
        let worker = std::thread::spawn(move || {
            assert_ne!(std::thread::current().id(), ui_thread);
            dispatcher
                .post(move || flag.store(std::thread::current().id() == ui_thread, Ordering::SeqCst))
                .expect("post");
        });
        worker.join().unwrap();
        assert!(pump_until(app, Duration::from_secs(5), || ran_on_ui.load(Ordering::SeqCst)));
    });
}

#[test]
fn dispatcher_fails_cleanly_after_application_ends() {
    let dispatcher = {
        let app = Application::new().unwrap();
        app.dispatcher()
    };
    assert!(matches!(dispatcher.post(|| {}), Err(GuiError::ApplicationGone)));
    let worker = std::thread::spawn(move || dispatcher.post(|| {}).is_err());
    assert!(worker.join().unwrap());
}

#[test]
fn defer_and_timers() {
    with_app(|app| {
        let order = Rc::new(RefCell::new(Vec::new()));
        let log = order.clone();
        defer(move || log.borrow_mut().push("defer")).unwrap();
        assert!(order.borrow().is_empty(), "defer não executa de imediato");
        let ticks = Counter::new();
        let timer = Timer::interval(Duration::from_millis(10), {
            let ticks = ticks.clone();
            move || ticks.hit()
        })
        .unwrap();
        let once = Counter::new();
        Timer::once(Duration::from_millis(10), {
            let once = once.clone();
            move || once.hit()
        })
        .unwrap();
        assert!(pump_until(app, Duration::from_secs(5), || ticks.get() >= 3));
        assert_eq!(*order.borrow(), ["defer"]);
        drop(timer);
        let frozen = ticks.get();
        pump_until(app, Duration::from_millis(80), || false);
        assert_eq!(ticks.get(), frozen, "o temporizador deve parar ao ser solto");
        assert_eq!(once.get(), 1, "Timer::once dispara exatamente uma vez");
    });
}

#[test]
fn async_tasks_cooperate_with_event_loop() {
    with_app(|app| {
        let window = test_window();
        let label = Label::builder().text("início").parent(&window).build().unwrap();
        let finished = Rc::new(Cell::new(false));
        let (target, done) = (label.clone(), finished.clone());
        spawn_local(async move {
            target.set_text("calculando");
            let total = run_blocking(|| (1..=1000u64).sum::<u64>()).await;
            sleep(Duration::from_millis(10)).await;
            target.set_text(&format!("total {total}"));
            done.set(true);
        })
        .unwrap();
        assert!(pump_until(app, Duration::from_secs(5), || finished.get()));
        assert_eq!(label.text(), "total 500500");

        let received = Rc::new(Cell::new(0u32));
        let slot = received.clone();
        spawn_background(|| 6 * 7, move |value| slot.set(value)).unwrap();
        assert!(pump_until(app, Duration::from_secs(5), || received.get() == 42));
    });
}

#[test]
fn menus_items_state_and_shortcuts() {
    with_app(|_| {
        let window = test_window();
        let opened = Counter::new();
        let file = Menu::new().unwrap();
        let open = file
            .add_item_with_shortcut("&Abrir", Shortcut::ctrl_char('O').unwrap(), {
                let c = opened.clone();
                move || c.hit()
            })
            .unwrap();
        file.add_separator().unwrap();
        let toggles = Rc::new(RefCell::new(Vec::new()));
        let wrap = file
            .add_check_item("Quebra", false, {
                let toggles = toggles.clone();
                move |on| toggles.borrow_mut().push(on)
            })
            .unwrap();
        let recent = Menu::new().unwrap();
        recent.add_item("a.txt", || {}).unwrap();
        file.add_submenu("Recentes", &recent).unwrap();
        assert!(file.add_submenu("De novo", &recent).is_err(), "um menu só pode ter um pai");
        assert_eq!(file.len(), 4);

        let bar = MenuBar::new().unwrap();
        bar.add_menu("&Arquivo", &file).unwrap();
        let before = window.client_size();
        window.set_menu(&bar).unwrap();
        assert_eq!(window.client_size(), before, "a barra de menus não deve encolher a área cliente");

        open.trigger();
        assert_eq!(opened.get(), 1);
        wrap.trigger();
        wrap.trigger();
        assert_eq!(*toggles.borrow(), vec![true, false]);
        assert!(!wrap.is_checked());
        open.set_enabled(false);
        assert!(!open.is_enabled());
        open.set_text("Abrir arquivo");
        open.set_icon(&Icon::system(SystemIcon::Information).unwrap()).unwrap();

        // Menus dinâmicos: remover e reconstruir.
        recent.clear();
        assert!(recent.is_empty());
        let item = recent.add_item("b.txt", || {}).unwrap();
        recent.remove(&item);
        assert!(recent.is_empty());

        let context = ContextMenu::new().unwrap();
        context.add_item("Copiar", || {}).unwrap();
        window.set_context_menu(&context);
        let system = window.system_menu().unwrap();
        let about = Counter::new();
        system.add_separator().unwrap();
        let entry = system
            .add_item("Sobre...", {
                let c = about.clone();
                move || c.hit()
            })
            .unwrap();
        entry.trigger();
        assert_eq!(about.get(), 1);
    });
}

#[test]
fn layouts_position_real_widgets_and_follow_resize() {
    with_app(|app| {
        let window = Window::builder().size(400, 300).build().unwrap();
        let name = TextBox::builder().parent(&window).build().unwrap();
        let notes = TextBox::builder().multiline(true).parent(&window).build().unwrap();
        let ok = Button::builder().text("OK").parent(&window).build().unwrap();
        let cancel = Button::builder().text("Cancelar").parent(&window).build().unwrap();
        let hidden = Button::builder().text("oculto").visible(false).parent(&window).build().unwrap();

        let buttons = StackLayout::horizontal().spacing(8);
        buttons.add_with(Spacer, LayoutParams::new().weight(1.0));
        buttons.add_with(&ok, LayoutParams::new().width(90));
        buttons.add_with(&hidden, LayoutParams::new().width(500));
        buttons.add_with(&cancel, LayoutParams::new().width(90));
        let root = StackLayout::vertical().spacing(10).padding(Thickness::all(10));
        root.add_with(&name, LayoutParams::new().height(24));
        root.add_with(&notes, LayoutParams::new().weight(1.0));
        root.add_with(buttons, LayoutParams::new().height(28));
        window.set_layout(root);

        assert_eq!(name.bounds(), Rect::new(10, 10, 380, 24));
        assert_eq!(notes.bounds(), Rect::new(10, 44, 380, 208));
        assert_eq!(ok.bounds(), Rect::new(202, 262, 90, 28));
        assert_eq!(cancel.bounds(), Rect::new(300, 262, 90, 28), "controles ocultos não ocupam espaço");

        window.set_size(500, 400);
        app.process_pending().unwrap();
        assert_eq!(notes.bounds(), Rect::new(10, 44, 480, 308));
        assert_eq!(cancel.bounds(), Rect::new(400, 362, 90, 28));

        // Grade aninhada em um painel, com âncoras em outro.
        let panel = Panel::builder().bounds(0, 0, 200, 100).parent(&window).build().unwrap();
        let label = Label::builder().text("Nome:").size(60, 20).parent(&panel).build().unwrap();
        let field = TextBox::builder().parent(&panel).build().unwrap();
        let grid = GridLayout::new(&[Track::Auto, Track::Star(1.0)], &[Track::Fixed(24)]).gaps(6, 0);
        grid.add(&label, 0, 0);
        grid.add(&field, 0, 1);
        panel.set_layout(grid);
        assert_eq!(label.bounds(), Rect::new(0, 0, 60, 24));
        assert_eq!(field.bounds(), Rect::new(66, 0, 134, 24));
        panel.set_size(300, 100);
        assert_eq!(field.bounds().width, 234);

        let anchored = Panel::builder().bounds(0, 0, 200, 100).parent(&window).build().unwrap();
        let corner = Button::builder().bounds(110, 70, 80, 24).parent(&anchored).build().unwrap();
        let anchors = AnchorLayout::new(Size::new(200, 100));
        anchors.add(&corner, Anchors::BOTTOM_RIGHT);
        anchored.set_layout(anchors);
        anchored.set_size(260, 150);
        assert_eq!(corner.bounds(), Rect::new(170, 120, 80, 24));

        let docked = Panel::builder().bounds(0, 0, 300, 200).parent(&window).build().unwrap();
        let top = Panel::builder().parent(&docked).build().unwrap();
        let fill = Panel::builder().parent(&docked).build().unwrap();
        let dock = DockLayout::new();
        dock.add_with(&top, Dock::Top, LayoutParams::new().height(30));
        dock.add(&fill, Dock::Fill);
        docked.set_layout(dock);
        assert_eq!((top.bounds(), fill.bounds()), (Rect::new(0, 0, 300, 30), Rect::new(0, 30, 300, 170)));
    });
}

#[test]
fn themes_and_colors_apply_without_breaking_controls() {
    with_app(|app| {
        let window = test_window();
        let label = Label::builder().text("texto").parent(&window).build().unwrap();
        let list = ListView::builder().parent(&window).build().unwrap();
        let button = Button::builder().text("ok").parent(&window).build().unwrap();
        window.show();
        Theme::dark().apply(&window);
        // Controles criados depois herdam o tema.
        let late = TextBox::builder().parent(&window).build().unwrap();
        app.process_pending().unwrap();
        label.set_foreground(Color::rgb(255, 200, 0));
        label.set_background(Color::rgb(10, 10, 10));
        list.set_background(Color::rgb(20, 20, 20));
        Style::new().font(Font::new("Arial", 11.0)).foreground(Color::WHITE).apply_to(&late);
        assert_eq!(late.font().family(), "Arial");
        button.set_style(Some(ButtonStyle::from_accent(Color::BLUE, Color::WHITE)));
        Theme::light().with_font(Font::new("Tahoma", 10.0)).apply(&window);
        assert_eq!(label.font().family(), "Tahoma");
        app.process_pending().unwrap();
        // Efeitos do DWM dependem da versão: ou funcionam, ou devolvem erro explícito.
        for outcome in [
            window.set_dark_title_bar(true),
            window.set_corner_style(CornerStyle::Round),
            window.set_backdrop(Backdrop::Mica),
        ] {
            if let Err(error) = outcome {
                assert!(matches!(error, GuiError::Unsupported(_) | GuiError::HResult { .. }), "{error}");
            }
        }
    });
}

#[test]
fn custom_paint_runs_and_reaches_the_screen() {
    with_app(|app| {
        let window = Window::builder().size(120, 80).position(10, 10).borderless().topmost(true).build().unwrap();
        let painted = Counter::new();
        let probe = painted.clone();
        window.on_paint(move |canvas| {
            probe.hit();
            canvas.clear(Color::rgb(0, 128, 0));
            canvas.fill_rect(Rect::new(0, 0, 40, 40), Color::rgb(200, 0, 0));
            canvas.ellipse(Rect::new(50, 10, 30, 30), Some(&Pen::new(Color::BLACK, 2)), Some(Color::WHITE));
            canvas.text("Oi", Point::new(60, 50), &Font::system(), Color::WHITE);
            let size = canvas.size();
            assert_eq!((size.width, size.height), (120, 80));
        });
        window.show();
        assert!(pump_until(app, Duration::from_secs(5), || painted.get() >= 1));
        let before = painted.get();
        window.invalidate();
        assert!(pump_until(app, Duration::from_secs(5), || painted.get() > before));
    });
}

#[test]
fn direct2d_surface_attaches_or_reports_unsupported() {
    with_app(|app| {
        let window = test_window();
        let panel = Panel::builder().bounds(0, 0, 200, 150).parent(&window).build().unwrap();
        let frames = Counter::new();
        let probe = frames.clone();
        let attached = Direct2DSurface::attach(&panel, move |context| {
            probe.hit();
            context.clear(Color::WHITE);
            context.fill_rect(Rect::new(10, 10, 50, 50), Color::BLUE);
            context.ellipse(100.0, 60.0, 30.0, 20.0, Some(Color::RED), Some((Color::BLACK, 2.0)));
            context.line(0.0, 0.0, 200.0, 150.0, Color::GREEN, 1.5);
            context.rounded_rect(Rect::new(120, 90, 60, 40), 8.0, Some(Color::YELLOW), None);
            context.path(
                &Path::new().move_to(10, 100).line_to(60, 140).line_to(10, 140).close(),
                Some(Color::GRAY),
                None,
            );
            context.gradient_rect(Rect::new(0, 120, 200, 30), (0.0, 0.0), (200.0, 0.0), Color::BLUE, Color::RED);
            context.push_transform();
            context.translate(20.0, 20.0);
            context.rotate(15.0);
            context.text("Direct2D", Rect::new(0, 0, 150, 30), &Font::new("Segoe UI", 12.0), Color::BLACK);
            context.pop_transform();
        });
        match attached {
            Ok(surface) => {
                window.show();
                surface.redraw();
                assert!(pump_until(app, Duration::from_secs(5), || frames.get() >= 1), "nenhum quadro desenhado");
                panel.set_size(300, 200);
                app.process_pending().unwrap();
            }
            Err(error) => assert!(matches!(error, GuiError::HResult { .. } | GuiError::Unsupported(_)), "{error}"),
        }
    });
}

#[test]
fn clipboard_round_trip() {
    let _clipboard = clipboard_lock();
    with_app(|_| {
        let text = "ArqRustUI — área de transferência ✓ 漢字";
        match Clipboard::set_text(text) {
            Ok(()) => {
                assert!(Clipboard::has_text());
                assert_eq!(Clipboard::text().unwrap().as_deref(), Some(text));
                Clipboard::clear().unwrap();
                assert_eq!(Clipboard::text().unwrap(), None);
                assert!(Clipboard::files().unwrap().is_empty());
            }
            // Em sessões sem estação de janelas interativa o clipboard pode não existir.
            Err(error) => assert!(matches!(error, GuiError::Win32 { .. }), "{error}"),
        }
    });
}

#[test]
fn accessibility_and_input_helpers() {
    with_app(|_| {
        let window = test_window();
        let first = TextBox::builder().parent(&window).build().unwrap();
        let second = Button::builder().text("&Enviar").parent(&window).build().unwrap();
        let third = CheckBox::builder().text("Lembrar").parent(&window).build().unwrap();
        accessibility::set_tab_order(&[&second, &third, &first]);
        accessibility::show_focus_indicators(&window);
        // A anotação de nome depende do serviço de acessibilidade do sistema.
        if let Err(error) = first.set_accessible_name("Endereço de e-mail") {
            assert!(matches!(error, GuiError::HResult { .. } | GuiError::Unsupported(_)), "{error}");
        }
        let custom = Panel::builder().focusable(true).parent(&window).build().unwrap();
        match accessibility::set_custom_role(&custom, AccessibleRole::Button, "Reproduzir") {
            Ok(()) => {
                custom.set_accessible_name("Pausar").unwrap();
                custom.set_accessible_description("Alterna a reprodução").unwrap();
            }
            Err(error) => assert!(matches!(error, GuiError::Unsupported(_)), "{error}"),
        }
        Ime::set_enabled(&first, false);
        Ime::set_enabled(&first, true);
        assert!(!Ime::is_open(&second));
        Ime::set_composition_position(&first, Point::new(4, 4));
        first.set_cursor(Some(Cursor::system(SystemCursor::Hand).unwrap()));
        first.set_cursor(None);
        assert!(Cursor::from_file("Z:\\inexistente.cur").is_err());
        let _ = Cursor::position();
    });
}

#[test]
fn tray_icon_lifecycle() {
    with_app(|app| {
        app.set_quit_on_last_window_closed(false);
        match TrayIcon::new(&Icon::system(SystemIcon::Application).unwrap(), "ArqRustUI teste") {
            Ok(tray) => {
                tray.set_tooltip("novo texto").unwrap();
                let menu = ContextMenu::new().unwrap();
                menu.add_item("Sair", Application::quit).unwrap();
                tray.set_menu(&menu);
                tray.on_click(|| {});
                tray.remove();
                assert!(matches!(tray.set_tooltip("x"), Err(GuiError::WidgetDestroyed)));
            }
            // Sem Shell em execução (por exemplo, em servidores de build) não há bandeja.
            Err(error) => assert!(matches!(error, GuiError::Unsupported(_)), "{error}"),
        }
    });
}

#[test]
fn window_events_resize_move_and_shortcuts() {
    with_app(|app| {
        let window = test_window();
        let sizes = Rc::new(RefCell::new(Vec::new()));
        window.on_resize({
            let sizes = sizes.clone();
            move |size| sizes.borrow_mut().push(size)
        });
        let moves = Counter::new();
        window.on_move({
            let moves = moves.clone();
            move |_| moves.hit()
        });
        window.show();
        window.set_size(333, 222);
        window.set_position(120, 130);
        app.process_pending().unwrap();
        assert_eq!(sizes.borrow().last(), Some(&Size::new(333, 222)));
        assert!(moves.get() >= 1);
        let saved = Counter::new();
        window.add_shortcut(Shortcut::key(Key::F5), {
            let saved = saved.clone();
            move || saved.hit()
        });
        // Atalhos são resolvidos no loop de mensagens: a tecla é postada, não enviada.
        // SAFETY: posta uma mensagem de teclado para a janela viva da thread de teste.
        unsafe {
            arqrustui::ffi::user32::PostMessageW(
                window.handle() as _,
                arqrustui::ffi::user32::WM_KEYDOWN,
                Key::F5.0 as usize,
                0,
            )
        };
        assert!(pump_until(app, Duration::from_secs(5), || saved.get() == 1));
    });
}

#[test]
fn clipboard_holds_html_images_files_and_custom_formats_together() {
    let _clipboard = clipboard_lock();
    with_app(|_| {
        let pixels: Vec<u8> = (0..4 * 3).flat_map(|i| [i as u8, 100, 200, 255]).collect();
        let content = ClipboardContent::new()
            .text("texto simples")
            .html("<b>negrito</b> e <i>itálico</i>")
            .image_pixels(4, 3, &pixels)
            .unwrap()
            .files(&["C:\\temp\\a.txt", "C:\\temp\\b.png"])
            .custom("ArqRustUI.Teste", &[1, 2, 3, 250]);
        Clipboard::set(&content).unwrap();
        assert_eq!(Clipboard::text().unwrap().as_deref(), Some("texto simples"));
        assert_eq!(Clipboard::html().unwrap().as_deref(), Some("<b>negrito</b> e <i>itálico</i>"));
        assert!(Clipboard::has_html() && Clipboard::has_image() && Clipboard::has_format("ArqRustUI.Teste"));
        assert_eq!(Clipboard::data("ArqRustUI.Teste").unwrap(), Some(vec![1, 2, 3, 250]));
        assert_eq!(Clipboard::data("ArqRustUI.Inexistente").unwrap(), None);
        let files = Clipboard::files().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[1].to_string_lossy().ends_with("b.png"));
        let image = Clipboard::image().unwrap().expect("imagem");
        assert_eq!((image.size().width, image.size().height), (4, 3));
        assert_eq!(image.to_pixels().unwrap(), pixels, "pixels preservados ida e volta");
        assert!(Clipboard::format_names().unwrap().iter().any(|n| n == "ArqRustUI.Teste"));

        Clipboard::set_text("só texto").unwrap();
        assert!(!Clipboard::has_html() && !Clipboard::has_format("ArqRustUI.Teste"), "set substitui tudo");
        Clipboard::clear().unwrap();
        assert_eq!(Clipboard::text().unwrap(), None);
    });
}
