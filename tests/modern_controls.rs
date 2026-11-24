//! Controles modernos desenhados pela biblioteca: interruptor, avaliação, cartão,
//! caixa numérica, expansor e trilha de navegação.

mod common;

use arqrustui::accessibility::uia_debug::{number_property, perform, perform_extra};
use arqrustui::prelude::*;
use common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const WM_KEYDOWN: u32 = 0x0100;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_MOUSEMOVE: u32 = 0x0200;

fn click(widget: &impl Widget, x: i32, y: i32) {
    let lp = ((y << 16) | (x & 0xFFFF)) as isize;
    send(widget, WM_LBUTTONDOWN, 1, lp);
    send(widget, WM_LBUTTONUP, 0, lp);
}

fn color_at(widget: &impl Widget, x: i32, y: i32) -> Color {
    use arqrustui::ffi::gdi32::*;
    use arqrustui::ffi::user32::*;
    let size = widget.client_size();
    // SAFETY: DCs e bitmap temporários desta thread, liberados ao final; a janela é viva.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, size.width, size.height);
        let previous = SelectObject(memory, bitmap as _);
        SendMessageW(widget.handle() as _, 0x0318, memory as usize, 4);
        let c = GetPixel(memory, x, y);
        SelectObject(memory, previous);
        DeleteObject(bitmap as _);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
        Color::rgb((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8)
    }
}

#[test]
fn toggle_switch_click_keyboard_and_accessibility() {
    with_app(|app| {
        motion::set_animations_enabled(Some(false));
        let window = test_window();
        let switch = ToggleSwitch::builder().text("Wi-Fi").bounds(10, 10, 220, 28).parent(&window).build().unwrap();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let log = changes.clone();
        switch.on_toggled(move |on| log.borrow_mut().push(on));
        assert!(!switch.is_on());
        assert_eq!(switch.state_text(), "Desligado");
        click(&switch, 100, 14);
        assert!(switch.is_on());
        assert_eq!(switch.state_text(), "Ligado");
        send(&switch, WM_KEYDOWN, 0x20, 0);
        assert!(!switch.is_on());
        assert_eq!(*changes.borrow(), vec![true, false]);
        // SAFETY: janela viva desta thread.
        unsafe {
            assert_eq!(number_property(switch.handle(), "", 30086), Some(0.0), "ToggleState desligado");
            assert_eq!(perform(switch.handle(), "", "toggle"), 0);
            assert_eq!(number_property(switch.handle(), "", 30086), Some(1.0));
        }
        assert!(switch.is_on(), "Toggle pela automação");
        app.process_pending().unwrap();
        // Ligado: o trilho tem a cor de destaque; desligado, não.
        // SAFETY: consulta de cor do sistema.
        let accent = Color::from_colorref(unsafe { arqrustui::ffi::user32::GetSysColor(13) });
        let row = |s: &ToggleSwitch| (0..s.client_size().width).any(|x| color_at(s, x, 14) == accent);
        assert!(row(&switch), "trilho com a cor de destaque");
        switch.set_on(false);
        assert!(!row(&switch));
        motion::set_animations_enabled(None);
        window.destroy();
    });
}

#[test]
fn rating_mouse_keys_half_stars_and_range_value() {
    with_app(|_| {
        let window = test_window();
        let rating =
            RatingControl::builder().allow_half(true).value(2.0).bounds(10, 10, 0, 0).parent(&window).build().unwrap();
        assert_eq!(rating.client_size().width, 5 * 26, "largura das estrelas");
        let values = Rc::new(RefCell::new(Vec::new()));
        let log = values.clone();
        rating.on_value_changed(move |v| log.borrow_mut().push(v));
        let third = rating.star_bounds(2).unwrap();
        click(&rating, third.x + 3, third.y + 10);
        assert_eq!(rating.value(), 2.5, "metade esquerda da terceira estrela");
        click(&rating, third.x + third.width - 2, third.y + 10);
        assert_eq!(rating.value(), 3.0);
        click(&rating, third.x + third.width - 2, third.y + 10);
        assert_eq!(rating.value(), 0.0, "clicar na nota atual limpa");
        send(&rating, WM_KEYDOWN, 0x23, 0); // End
        assert_eq!(rating.value(), 5.0);
        send(&rating, WM_KEYDOWN, 0x25, 0); // ←
        assert_eq!(rating.value(), 4.5);
        send(&rating, WM_KEYDOWN, 0x32, 0); // 2
        assert_eq!(rating.value(), 2.0);
        assert_eq!(*values.borrow(), vec![2.5, 3.0, 0.0, 5.0, 4.5, 2.0]);
        // SAFETY: janela viva desta thread.
        unsafe {
            assert_eq!(number_property(rating.handle(), "", 30047), Some(2.0), "RangeValue.Value");
            assert_eq!(perform_extra(rating.handle(), "", "range:4"), 0);
        }
        assert_eq!(rating.value(), 4.0);
        // Estrelas preenchidas (dourado) e vazias.
        let filled = rating.star_bounds(0).unwrap();
        let empty = rating.star_bounds(4).unwrap();
        let gold = color_at(&rating, filled.x + filled.width / 2, filled.y + filled.height / 2 + 2);
        assert_eq!(gold, Color::rgb(0xF5, 0xB4, 0x00));
        let hollow = color_at(&rating, empty.x + empty.width / 2, empty.y + empty.height / 2 + 2);
        assert_ne!(hollow, gold);
        rating.set_read_only(true);
        click(&rating, 3, 10);
        assert_eq!(rating.value(), 4.0, "só leitura");
        window.destroy();
    });
}

#[test]
fn clickable_card_with_content() {
    with_app(|_| {
        let window = test_window();
        let card = Card::builder()
            .title("Armazenamento")
            .subtitle("72% usado")
            .clickable(true)
            .bounds(10, 10, 260, 140)
            .parent(&window)
            .build()
            .unwrap();
        let area = card.content_bounds();
        assert_eq!(area, Rect::new(16, 16 + 22 + 18 + 8, 228, 140 - 64 - 16));
        let bar = ProgressBar::builder().value(72).bounds(area.x, area.y, area.width, 8).parent(&card).build().unwrap();
        window.show();
        assert!(bar.is_visible());
        let clicks = Rc::new(Cell::new(0));
        let c = clicks.clone();
        card.on_click(move || c.set(c.get() + 1));
        click(&card, 200, 120);
        send(&card, WM_KEYDOWN, 0x0D, 0);
        assert_eq!(clicks.get(), 2);
        // SAFETY: janela viva desta thread.
        assert_eq!(unsafe { perform(card.handle(), "", "invoke") }, 0);
        assert_eq!(clicks.get(), 3);
        // Cantos arredondados: o canto mostra o fundo da janela, não a superfície.
        let corner = color_at(&card, 0, 0);
        let inside = color_at(&card, 130, 130);
        assert_ne!(corner, inside);
        send(&card, WM_MOUSEMOVE, 0, (100 << 16 | 100) as isize);
        assert_ne!(color_at(&card, 130, 130), inside, "realce sob o cursor");
        window.destroy();
    });
}

#[test]
fn number_box_steps_limits_and_expressions() {
    with_app(|_| {
        let window = test_window();
        let number = NumberBox::builder()
            .range(0.0, 10.0)
            .step(0.5)
            .decimals(1)
            .decimal_comma(true)
            .value(2.0)
            .bounds(10, 10, 140, 28)
            .parent(&window)
            .build()
            .unwrap();
        let field = number.text_box();
        assert_eq!(field.text(), "2,0");
        let values = Rc::new(RefCell::new(Vec::new()));
        let log = values.clone();
        number.on_value_changed(move |v| log.borrow_mut().push(v));
        send(&field, WM_KEYDOWN, 0x26, 0); // ↑
        assert_eq!(number.value(), 2.5);
        send(&field, WM_KEYDOWN, 0x21, 0); // Page Up (10 passos)
        assert_eq!(number.value(), 7.5);
        // Botão ▲ (metade de cima da coluna da direita) até o limite.
        let size = number.client_size();
        for _ in 0..10 {
            send(&number, WM_LBUTTONDOWN, 1, ((4 << 16) | (size.width - 5)) as isize);
        }
        assert_eq!(number.value(), 10.0, "limitado ao máximo");
        field.set_text("(1,25 + 2) * 2");
        send(&field, WM_KEYDOWN, 0x0D, 0); // Enter
        assert_eq!(number.value(), 6.5, "expressão avaliada e arredondada a 1 casa");
        assert_eq!(field.text(), "6,5");
        field.set_text("abc");
        assert!(!number.commit());
        assert_eq!(field.text(), "6,5", "inválido volta ao valor");
        number.set_range(0.0, 5.0);
        assert_eq!(number.value(), 5.0);
        assert_eq!(*values.borrow(), vec![2.5, 7.5, 8.0, 8.5, 9.0, 9.5, 10.0, 6.5, 5.0]);
        // SAFETY: janela viva desta thread.
        unsafe {
            assert_eq!(number_property(number.handle(), "", 30049), Some(0.0), "RangeValue.Minimum");
            assert_eq!(perform_extra(number.handle(), "", "range:3"), 0);
            assert_eq!(perform(number.handle(), "", "set:1+1"), 0, "Value.SetValue aceita expressão");
        }
        assert_eq!(number.value(), 2.0);
        window.destroy();
    });
}

#[test]
fn expander_opens_closes_and_resizes() {
    with_app(|app| {
        motion::set_animations_enabled(Some(false));
        let window = test_window();
        let expander = Expander::builder()
            .header("Avançado")
            .content_height(100)
            .bounds(10, 10, 300, 0)
            .parent(&window)
            .build()
            .unwrap();
        let check =
            CheckBox::builder().text("Opção").bounds(10, 10, 200, 24).parent(&expander.content()).build().unwrap();
        window.show();
        assert_eq!(expander.bounds().height, EXPANDER_HEADER);
        assert!(!expander.content().is_visible());
        let states = Rc::new(RefCell::new(Vec::new()));
        let log = states.clone();
        expander.on_expanded_changed(move |open| log.borrow_mut().push(open));
        click(&expander, 50, 20);
        assert!(
            pump_until(app, std::time::Duration::from_secs(2), || expander.bounds().height == EXPANDER_HEADER + 100)
        );
        assert!(expander.is_expanded() && expander.content().is_visible() && check.is_visible());
        send(&expander, WM_KEYDOWN, 0x25, 0); // ← fecha
        assert!(pump_until(app, std::time::Duration::from_secs(2), || !expander.content().is_visible()));
        assert_eq!(expander.bounds().height, EXPANDER_HEADER);
        // SAFETY: janela viva desta thread.
        unsafe { assert_eq!(perform(expander.handle(), "", "expand"), 0) };
        assert!(expander.is_expanded());
        assert_eq!(*states.borrow(), vec![true, false, true]);
        motion::set_animations_enabled(None);
        window.destroy();
    });
}

#[test]
fn breadcrumb_clicks_keyboard_and_overflow() {
    with_app(|_| {
        let window = test_window();
        let bar = BreadcrumbBar::builder()
            .items(&["Início", "Documentos", "Projetos", "ArqRust"])
            .bounds(0, 0, 500, 32)
            .parent(&window)
            .build()
            .unwrap();
        assert!(bar.hidden_items().is_empty());
        let clicked = Rc::new(RefCell::new(Vec::new()));
        let log = clicked.clone();
        let trail = bar.clone();
        bar.on_item_click(move |i| {
            log.borrow_mut().push(i);
            trail.truncate(i + 1);
        });
        let docs = bar.item_bounds(1).unwrap();
        click(&bar, docs.x + docs.width / 2, 16);
        assert_eq!(*clicked.borrow(), vec![1]);
        assert_eq!(bar.items(), ["Início", "Documentos"]);
        bar.push("Projetos");
        bar.push("Uma pasta com um nome bem comprido");
        bar.set_bounds(Rect::new(0, 0, 220, 32));
        assert!(!bar.hidden_items().is_empty(), "sem espaço, os primeiros vão para o …");
        assert!(bar.item_bounds(0).is_none() && bar.item_bounds(3).is_some());
        // Teclado: Home foca o …, → avança, End vai ao último, Enter aciona.
        // (Enter no … abriria o menu modal dos itens recolhidos.)
        send(&bar, WM_KEYDOWN, 0x24, 0);
        send(&bar, WM_KEYDOWN, 0x27, 0);
        send(&bar, WM_KEYDOWN, 0x23, 0);
        send(&bar, WM_KEYDOWN, 0x0D, 0);
        assert_eq!(clicked.borrow().last(), Some(&3));
        bar.truncate(3);
        // SAFETY: janela viva desta thread.
        unsafe {
            let tree = arqrustui::accessibility::uia_debug::walk(bar.handle());
            let names: Vec<&str> = tree.iter().map(|(_, e)| e.name.as_str()).collect();
            assert_eq!(names, ["Início", "Documentos", "Projetos"], "a trilha cortada cabe inteira de novo");
            assert_eq!(perform(bar.handle(), "Início", "invoke"), 0);
        }
        assert_eq!(bar.items(), ["Início"]);
        window.destroy();
    });
}

#[test]
fn custom_title_bar_replaces_the_system_frame() {
    with_app(|app| {
        use arqrustui::ffi::user32::{GetClientRect, GetWindowRect};
        let window = Window::builder().title("Sem moldura").size(500, 300).build().unwrap();
        let bar = TitleBar::attach(&window).unwrap();
        let search = TextBox::builder().parent(&bar).build().unwrap();
        bar.add_control(&search, 160);
        window.show();
        app.process_pending().unwrap();
        let (mut outer, mut inner) = (arqrustui::ffi::RECT::default(), arqrustui::ffi::RECT::default());
        // SAFETY: janela viva desta thread.
        unsafe {
            GetWindowRect(window.handle() as _, &mut outer);
            GetClientRect(window.handle() as _, &mut inner);
        }
        assert_eq!(inner.right - inner.left, outer.right - outer.left, "sem bordas do sistema: cliente = janela");
        assert_eq!(inner.bottom - inner.top, outer.bottom - outer.top);
        assert_eq!(bar.bounds(), Rect::new(0, 0, window.client_size().width, TITLE_BAR_HEIGHT));
        let screen = |x: i32, y: i32| (((outer.top + y) << 16) | ((outer.left + x) & 0xFFFF)) as isize;
        let hit = |x: i32, y: i32| send(&window, 0x0084, 0, screen(x, y));
        let width = outer.right - outer.left;
        assert_eq!(hit(250, 16), 2, "faixa do título arrasta (HTCAPTION)");
        assert_eq!(hit(width - 46 - 20, 16), 9, "botão maximizar (HTMAXBUTTON, layouts de encaixe)");
        assert_eq!(hit(250, 2), 12, "borda de cima redimensiona (HTTOP)");
        assert_eq!(hit(250, 150), 1, "conteúdo");
        let bar_hit = |x: i32, y: i32| send(&bar, 0x0084, 0, screen(x, y));
        assert_eq!(bar_hit(width - 20, 16), 1, "fechar é da barra");
        assert_eq!(bar_hit(250, 16), -1, "o resto da barra deixa passar para a janela");
        assert!(search.bounds().x > 0 && search.bounds().y >= 0);
        // SAFETY: janela viva desta thread.
        unsafe {
            let tree = arqrustui::accessibility::uia_debug::walk(bar.handle());
            let names: Vec<&str> = tree.iter().map(|(_, e)| e.name.as_str()).collect();
            assert_eq!(names, ["Minimizar", "Maximizar", "Fechar"]);
        }
        let closed = Rc::new(Cell::new(false));
        let c = closed.clone();
        window.on_close_request(move || {
            c.set(true);
            true
        });
        let close = bar.button_bounds(CaptionButton::Close);
        click(&bar, close.x + 10, close.y + 10);
        assert!(pump_until(app, std::time::Duration::from_secs(2), || closed.get()), "o botão fechar fecha a janela");
    });
}

#[test]
fn dock_host_panes_split_float_and_close() {
    with_app(|app| {
        use arqrustui::ffi::user32::GetParent;
        let window = test_window();
        let host = DockHost::builder().bounds(0, 0, 800, 500).parent(&window).build().unwrap();
        assert_eq!(host.center().bounds(), Rect::new(0, 0, 800, 500));
        let files = host.add_pane("Arquivos", DockSide::Left).unwrap();
        let output = host.add_pane("Saída", DockSide::Bottom).unwrap();
        let props = host.add_pane("Propriedades", DockSide::Left).unwrap();
        let list = ListBox::builder().bounds(0, 0, 100, 100).parent(&files.content()).build().unwrap();
        let left = host.zone_size(DockSide::Left);
        assert_eq!(files.bounds().width, left);
        assert_eq!(files.bounds().height + props.bounds().height + 5, 500, "dois painéis empilhados à esquerda");
        assert_eq!(output.bounds().x, left + 5, "embaixo começa depois da coluna esquerda");
        assert_eq!(host.center().bounds().x, left + 5);
        // Arrasta a divisória da esquerda 40 unidades para a direita.
        let splitter = host.splitter_bounds(DockSide::Left).unwrap();
        let at = |x: i32, y: i32| ((y << 16) | x) as isize;
        send(&host, WM_LBUTTONDOWN, 1, at(splitter.x + 2, 200));
        send(&host, WM_MOUSEMOVE, 1, at(splitter.x + 42, 200));
        send(&host, WM_LBUTTONUP, 0, at(splitter.x + 42, 200));
        assert_eq!(host.zone_size(DockSide::Left), left + 40);
        assert_eq!(files.bounds().width, left + 40);

        // Flutuar: o painel (com o conteúdo) vai para uma janela própria.
        output.float().unwrap();
        assert!(output.is_floating() && !output.is_docked());
        let floating = output.floating_window().unwrap();
        // SAFETY: janelas vivas desta thread.
        assert_eq!(unsafe { GetParent(output.handle() as _) } as isize, floating.handle());
        assert_eq!(host.center().bounds().height, 500, "sem painel embaixo, a central ocupa a altura toda");
        // Fechar a janela flutuante devolve o painel.
        floating.close();
        app.process_pending().unwrap();
        assert!(output.is_docked());
        // SAFETY: consulta simples a uma janela viva desta thread.
        assert_eq!(unsafe { GetParent(output.handle() as _) } as isize, host.handle());
        assert!(host.center().bounds().height < 500);
        // Pelo botão do cabeçalho (flutuar) e por acessibilidade (acoplar de volta).
        let width = files.client_size().width;
        let float_button = (width - 2 * 24 - 2 + 10, 12);
        click(&files, float_button.0, float_button.1);
        assert!(files.is_floating());
        assert!(list.handle() != 0);
        // SAFETY: janela viva desta thread.
        unsafe { assert_eq!(perform(files.handle(), "Acoplar", "invoke"), 0) };
        assert!(files.is_docked());
        // Fechar e reabrir; mudar de lado.
        props.close();
        assert!(props.is_closed() && !props.is_visible());
        assert_eq!(files.bounds().height, 500, "sozinho na coluna");
        props.show();
        props.dock_to(DockSide::Right);
        assert_eq!(props.bounds().x + props.bounds().width, 800);
        window.destroy();
    });
}

#[test]
fn tabs_detach_by_dragging_and_come_back() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let tabs = TabControl::builder().bounds(10, 10, 400, 260).parent(&window).build().unwrap();
        let first = tabs.add_tab("Primeira").unwrap();
        let second = tabs.add_tab("Segunda").unwrap();
        tabs.add_tab("Terceira").unwrap();
        let label = Label::builder().text("conteúdo").bounds(10, 10, 120, 20).parent(&second).build().unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let (d, a) = (events.clone(), events.clone());
        tabs.on_tab_detached(move |t| d.borrow_mut().push(format!("saiu {}", t.title)));
        tabs.on_tab_attached(move |t| a.borrow_mut().push(format!("voltou {}", t.title)));
        let tab = tabs.tab_bounds(1).unwrap();
        // Sem `set_detachable`, arrastar não destaca.
        let at = |x: i32, y: i32| ((y << 16) | (x & 0xFFFF)) as isize;
        send(&tabs, WM_LBUTTONDOWN, 1, at(tab.x + 5, tab.y + 5));
        send(&tabs, WM_MOUSEMOVE, 1, at(tab.x + 5, tab.y + 120));
        send(&tabs, WM_LBUTTONUP, 0, at(tab.x + 5, tab.y + 120));
        assert_eq!(tabs.tab_count(), 3);
        tabs.set_detachable(true);
        assert_eq!(tabs.tab_at(Point::new(tab.x + 5, tab.y + 5)), Some(1));
        send(&tabs, WM_LBUTTONDOWN, 1, at(tab.x + 5, tab.y + 5));
        send(&tabs, WM_MOUSEMOVE, 1, at(tab.x + 5, tab.y + 10));
        assert_eq!(tabs.tab_count(), 3, "pequenos movimentos não destacam");
        send(&tabs, WM_MOUSEMOVE, 1, at(tab.x + 5, tab.y + 140));
        assert_eq!(tabs.tab_count(), 2);
        assert_eq!(tabs.tab_title(1), "Terceira");
        let detached = tabs.detached_tabs();
        assert_eq!(detached.len(), 1);
        assert_eq!(detached[0].title, "Segunda");
        assert_eq!(detached[0].window.title(), "Segunda");
        assert!(label.is_visible(), "o conteúdo foi junto e continua visível");
        assert_eq!(second.bounds().width, detached[0].window.client_size().width);
        // Fechar a janela devolve a aba no fim.
        detached[0].window.close();
        app.process_pending().unwrap();
        assert_eq!(tabs.tab_count(), 3);
        assert_eq!(tabs.tab_title(2), "Segunda");
        assert_eq!(tabs.page(2).unwrap().handle(), second.handle());
        assert_eq!(tabs.selected_index(), Some(2));
        // Por código.
        let again = tabs.detach_tab(0).unwrap();
        assert_eq!(again.page.handle(), first.handle());
        assert_eq!(tabs.attach_tab(&again.window), Some(2));
        assert!(tabs.detached_tabs().is_empty());
        assert_eq!(*events.borrow(), ["saiu Segunda", "voltou Segunda", "saiu Primeira", "voltou Primeira"]);
        window.destroy();
    });
}
