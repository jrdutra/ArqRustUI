//! MenuStrip e menus desenhados pela biblioteca: geometria da faixa, teclado,
//! mnemônicos, atalhos, itens de opção, transbordamento e menus de contexto.

mod common;

use arqrustui::ffi::user32::{
    PostMessageW, WM_CHAR, WM_KEYDOWN, WM_LBUTTONDOWN, WM_SYSCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use arqrustui::menus::debug_state;
use arqrustui::prelude::*;
use common::*;
use std::time::Duration;

const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;
const VK_RIGHT: usize = 0x27;
const VK_DOWN: usize = 0x28;
const VK_MENU: usize = 0x12;
const VK_F10: usize = 0x79;
const VK_F5: usize = 0x74;

/// Posta uma mensagem e processa a fila (o loop de mensagens trata o teclado).
fn post(app: &Application, widget: &impl Widget, message: u32, wparam: usize, lparam: isize) {
    // SAFETY: mensagem sem ponteiros postada a uma janela viva da thread de teste.
    unsafe { PostMessageW(widget.handle() as _, message, wparam, lparam) };
    app.process_pending().unwrap();
}

fn depth() -> usize {
    debug_state().map(|(depth, _)| depth).unwrap_or(0)
}

fn hot(level: usize) -> Option<usize> {
    debug_state().and_then(|(_, hot)| hot.get(level).copied().flatten())
}

/// Janela com Arquivo (Novo, separador, Salvar desabilitado, Recentes ▸) e Editar.
fn sample(window: &Window) -> (MenuStrip, Counter, Counter, MenuItem) {
    let news = Counter::new();
    let copies = Counter::new();
    let file = Menu::new().unwrap();
    let hit = news.clone();
    file.add_item("&Novo", move || hit.hit()).unwrap().set_icon(&icons::FILE).unwrap();
    file.add_separator().unwrap();
    let save = file.add_item("&Salvar", || {}).unwrap();
    save.set_enabled(false);
    let recent = Menu::new().unwrap();
    recent.add_item("um.txt", || {}).unwrap();
    file.add_submenu_with_icon("&Recentes", &icons::CLOCK, &recent).unwrap();
    let edit = Menu::new().unwrap();
    let hit = copies.clone();
    edit.add_item("&Copiar", move || hit.hit()).unwrap();
    let strip = MenuStrip::new().unwrap();
    strip.add_menu("&Arquivo", &file).unwrap();
    strip.add_menu_with_icon("&Editar", &icons::EDIT, &edit).unwrap();
    window.set_menu_strip(&strip).unwrap();
    (strip, news, copies, save)
}

#[test]
fn strip_lives_in_the_frame_and_keeps_the_client_size() {
    with_app(|_| {
        let window = test_window();
        let before = window.outer_bounds();
        assert_eq!(window.client_size(), Size::new(400, 300));
        let (strip, ..) = sample(&window);
        assert_eq!(window.client_size(), Size::new(400, 300), "a área cliente não muda");
        assert!(strip.height() >= 20, "altura da faixa: {}", strip.height());
        let after = window.outer_bounds();
        assert!(after.height > before.height, "a janela cresce para abrigar a faixa");
        assert_eq!(window.menu_strip().map(|s| s.len()), Some(2));
        assert_eq!(strip.menu_text(1).as_deref(), Some("&Editar"));
        assert_eq!(strip.overflow_count(), 0);

        // A mesma barra não pode estar em duas janelas.
        let other = test_window();
        assert!(other.set_menu_strip(&strip).is_err());

        window.remove_menu_strip();
        assert!(window.menu_strip().is_none());
        assert_eq!(window.client_size(), Size::new(400, 300));
        assert_eq!(window.outer_bounds().height, before.height);
        assert!(other.set_menu_strip(&strip).is_ok(), "liberada, pode ir para outra janela");
    });
}

#[test]
fn keyboard_navigates_and_activates_items() {
    with_app(|app| {
        let window = test_window();
        let (strip, news, _, _) = sample(&window);
        strip.open(0).unwrap();
        assert!(strip.is_open());
        assert_eq!((depth(), hot(0)), (1, Some(0)), "aberto pelo teclado: primeiro item selecionado");

        post(app, &window, WM_KEYDOWN, VK_DOWN, 0);
        assert_eq!(hot(0), Some(2), "o separador é pulado; o item desabilitado pode ser selecionado");
        post(app, &window, WM_KEYDOWN, VK_RETURN, 0);
        assert!(strip.is_open(), "item desabilitado não executa");
        post(app, &window, WM_KEYDOWN, VK_DOWN, 0);
        post(app, &window, WM_KEYDOWN, VK_RIGHT, 0);
        assert_eq!((depth(), hot(1)), (2, Some(0)), "Direita abre o submenu");
        post(app, &window, WM_KEYDOWN, VK_ESCAPE, 0);
        assert_eq!(depth(), 1, "Esc fecha um nível");
        post(app, &window, WM_KEYDOWN, VK_DOWN, 0);
        assert_eq!(hot(0), Some(0), "a seleção dá a volta");
        post(app, &window, WM_KEYDOWN, VK_RETURN, 0);
        assert_eq!(news.get(), 1);
        assert!(!strip.is_open(), "executar um item encerra o modo de menu");
        assert!(debug_state().is_none());
    });
}

#[test]
fn escape_returns_to_the_bar_and_arrows_switch_menus() {
    with_app(|app| {
        let window = test_window();
        let (strip, ..) = sample(&window);
        strip.open(0).unwrap();
        post(app, &window, WM_KEYDOWN, VK_RIGHT, 0);
        assert_eq!(depth(), 1, "Direita num item comum passa ao menu seguinte da barra");
        post(app, &window, WM_KEYDOWN, VK_ESCAPE, 0);
        assert_eq!(depth(), 0);
        assert!(strip.is_open(), "a barra continua selecionada");
        post(app, &window, WM_KEYDOWN, VK_DOWN, 0);
        assert_eq!(depth(), 1, "Baixo reabre a lista");
        post(app, &window, WM_KEYDOWN, VK_ESCAPE, 0);
        post(app, &window, WM_KEYDOWN, VK_ESCAPE, 0);
        assert!(!strip.is_open());
    });
}

#[test]
fn alt_f10_and_mnemonics() {
    with_app(|app| {
        let window = test_window();
        let (strip, news, copies, _) = sample(&window);

        // Alt pressionado e solto sozinho: modo de menu, sem lista aberta.
        post(app, &window, WM_SYSKEYDOWN, VK_MENU, 0);
        post(app, &window, WM_SYSKEYUP, VK_MENU, 0);
        assert!(strip.is_open());
        assert_eq!(depth(), 0);
        // Uma letra no modo de menu abre o menu correspondente.
        post(app, &window, WM_CHAR, 'e' as usize, 0);
        assert_eq!(depth(), 1);
        post(app, &window, WM_CHAR, 'c' as usize, 0);
        assert_eq!(copies.get(), 1, "letra sublinhada executa o item");
        assert!(!strip.is_open());

        // Alt seguido de outra tecla não entra no modo de menu.
        post(app, &window, WM_SYSKEYDOWN, VK_MENU, 0);
        post(app, &window, WM_SYSKEYDOWN, VK_F5, 0);
        post(app, &window, WM_SYSKEYUP, VK_MENU, 0);
        assert!(!strip.is_open());

        // F10 entra; Alt sai.
        post(app, &window, WM_SYSKEYDOWN, VK_F10, 0);
        assert!(strip.is_open());
        post(app, &window, WM_SYSKEYDOWN, VK_MENU, 0);
        post(app, &window, WM_SYSKEYUP, VK_MENU, 0);
        assert!(!strip.is_open(), "a liberação do Alt que encerrou o menu não o reabre");

        // Alt+A abre Arquivo diretamente; N executa Novo.
        post(app, &window, WM_SYSCHAR, 'a' as usize, 0);
        assert_eq!((depth(), hot(0)), (1, Some(0)));
        post(app, &window, WM_CHAR, 'N' as usize, 0);
        assert_eq!(news.get(), 1);
        // Mnemônico sem menu correspondente não abre nada.
        post(app, &window, WM_SYSCHAR, 'z' as usize, 0);
        assert!(!strip.is_open());
    });
}

#[test]
fn item_shortcuts_respect_the_enabled_state() {
    with_app(|app| {
        let window = test_window();
        let refreshes = Counter::new();
        let opened = Counter::new();
        let menu = Menu::new().unwrap();
        let hit = refreshes.clone();
        let item = menu.add_item_with_shortcut("&Atualizar", Shortcut::key(Key::F5), move || hit.hit()).unwrap();
        assert_eq!(item.text(), "&Atualizar\tF5");
        let probe = opened.clone();
        menu.on_open(move |_| probe.hit());
        let strip = MenuStrip::new().unwrap();
        strip.add_menu("&Exibir", &menu).unwrap();
        window.set_menu_strip(&strip).unwrap();

        post(app, &window, WM_KEYDOWN, VK_F5, 0);
        assert_eq!((refreshes.get(), opened.get()), (1, 1), "on_open roda antes do atalho");
        item.set_enabled(false);
        post(app, &window, WM_KEYDOWN, VK_F5, 0);
        assert_eq!(refreshes.get(), 1, "item desabilitado não executa pelo atalho");
        item.set_enabled(true);
        strip.set_menu_enabled(0, false).unwrap();
        assert!(!strip.is_menu_enabled(0));
        post(app, &window, WM_KEYDOWN, VK_F5, 0);
        assert_eq!(refreshes.get(), 1, "menu da barra desabilitado também bloqueia o atalho");
        strip.open(0).unwrap();
        assert_eq!(depth(), 0, "menu desabilitado não abre lista");
        strip.close();
    });
}

#[test]
fn radio_items_form_groups() {
    with_app(|_| {
        let menu = Menu::new().unwrap();
        let chosen = Counter::new();
        let hit = chosen.clone();
        let small = menu.add_radio_item("Pequeno", true, move || hit.hit()).unwrap();
        let medium = menu.add_radio_item("Médio", false, || {}).unwrap();
        let large = menu.add_radio_item("Grande", false, || {}).unwrap();
        menu.add_separator().unwrap();
        let light = menu.add_radio_item("Claro", true, || {}).unwrap();
        let dark = menu.add_radio_item("Escuro", false, || {}).unwrap();
        assert!(small.is_checked() && !medium.is_checked() && !large.is_checked());
        large.trigger();
        assert!(!small.is_checked() && !medium.is_checked() && large.is_checked());
        assert!(light.is_checked() && !dark.is_checked(), "o separador separa os grupos");
        small.trigger();
        assert_eq!(chosen.get(), 1);
        assert!(small.is_checked() && !large.is_checked());
    });
}

#[test]
fn hidden_menus_go_to_the_overflow_button() {
    with_app(|_| {
        let window = Window::builder().title("Estreita").size(150, 120).position(40, 40).build().unwrap();
        let strip = MenuStrip::new().unwrap();
        for name in ["&Arquivo", "&Editar", "E&xibir", "&Ferramentas", "Aj&uda"] {
            let menu = Menu::new().unwrap();
            menu.add_item("Item", || {}).unwrap();
            strip.add_menu(name, &menu).unwrap();
        }
        window.set_menu_strip(&strip).unwrap();
        let hidden = strip.overflow_count();
        assert!(hidden > 0 && hidden < 5, "menus no »: {hidden}");
        strip.open(4).unwrap();
        assert_eq!(depth(), 2, "o » e, dentro dele, o menu pedido");
        strip.close();
        window.set_size(900, 120);
        assert_eq!(strip.overflow_count(), 0, "com espaço, todos voltam à faixa");
    });
}

#[test]
fn drawn_context_menu_opens_and_closes() {
    with_app(|app| {
        let window = test_window();
        let button = Button::builder().text("Alvo").bounds(10, 10, 100, 28).parent(&window).build().unwrap();
        let cleared = Counter::new();
        let context = ContextMenu::new().unwrap();
        assert!(!context.is_drawn());
        context.set_drawn(true);
        let hit = cleared.clone();
        context.add_item("&Limpar", move || hit.hit()).unwrap();
        context.show(&button, Point::new(5, 5));
        assert_eq!((depth(), hot(0)), (1, None), "aberto pelo mouse: nada selecionado");
        post(app, &window, WM_KEYDOWN, VK_DOWN, 0);
        post(app, &window, WM_KEYDOWN, VK_RETURN, 0);
        assert_eq!(cleared.get(), 1);
        assert!(debug_state().is_none());

        context.show(&button, Point::new(5, 5));
        context.close();
        assert!(debug_state().is_none());

        // Clique fora da lista fecha o menu sem chegar ao controle.
        let clicks = Counter::new();
        let probe = clicks.clone();
        button.on_click(move || probe.hit());
        context.show(&button, Point::new(60, 5));
        post(app, &button, WM_LBUTTONDOWN, 1, (8 << 16) | 8);
        assert!(debug_state().is_none());
    });
}

#[test]
fn menu_mode_ends_with_the_window() {
    with_app(|app| {
        let window = test_window();
        let (strip, ..) = sample(&window);
        strip.open(0).unwrap();
        assert!(strip.is_open());
        window.destroy();
        assert!(pump_until(app, Duration::from_secs(2), || debug_state().is_none()));
        assert!(!strip.is_open());
    });
}

#[test]
fn highlight_events_follow_the_keyboard_and_end_with_none() {
    with_app(|app| {
        let window = test_window();
        let file = Menu::new().unwrap();
        let first = file.add_item("&Novo", || {}).unwrap();
        file.add_separator().unwrap();
        let second = file.add_item("&Abrir", || {}).unwrap();
        let recent = Menu::new().unwrap();
        let inner = recent.add_item("um.txt", || {}).unwrap();
        file.add_submenu("&Recentes", &recent).unwrap();
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        file.on_highlight_changed({
            let seen = seen.clone();
            move |item| seen.borrow_mut().push(item.map(|i| i.text().replace('&', "")))
        });
        let strip = MenuStrip::new().unwrap();
        strip.add_menu("&Arquivo", &file).unwrap();
        window.set_menu_strip(&strip).unwrap();
        strip.open(0).unwrap();
        app.process_pending().unwrap();
        post(app, &window, WM_KEYDOWN, VK_DOWN, 0);
        post(app, &window, WM_KEYDOWN, VK_DOWN, 0);
        post(app, &window, WM_KEYDOWN, VK_RIGHT, 0);
        post(app, &window, WM_KEYDOWN, VK_ESCAPE, 0);
        post(app, &window, WM_KEYDOWN, VK_ESCAPE, 0);
        post(app, &window, WM_KEYDOWN, VK_ESCAPE, 0);
        app.process_pending().unwrap();
        let seen = seen.borrow().clone();
        assert_eq!(seen.first(), Some(&Some("Novo".to_string())), "primeiro item destacado ao abrir: {seen:?}");
        assert!(seen.contains(&Some("Abrir".to_string())), "{seen:?}");
        assert!(seen.contains(&Some(inner.text())), "itens do submenu sobem para o menu pai: {seen:?}");
        assert!(seen.contains(&None), "submenu e fechamento avisam None: {seen:?}");
        assert_eq!(seen.last(), Some(&None), "{seen:?}");
        assert!(first != second && first == first.clone());
    });
}

#[test]
fn native_menu_bar_shortcuts_include_items_added_later() {
    with_app(|app| {
        let window = test_window();
        let file = Menu::new().unwrap();
        file.add_item("&Novo", || {}).unwrap();
        let bar = MenuBar::new().unwrap();
        bar.add_menu("&Arquivo", &file).unwrap();
        window.set_menu(&bar).unwrap();
        let hits = Counter::new();
        let hit = hits.clone();
        let later = file.add_item_with_shortcut("Atualizar", Shortcut::key(Key::F5), move || hit.hit()).unwrap();
        post(app, &window, WM_KEYDOWN, VK_F5, 0);
        assert_eq!(hits.get(), 1, "atalho de item acrescentado depois de set_menu");
        later.set_enabled(false);
        post(app, &window, WM_KEYDOWN, VK_F5, 0);
        assert_eq!(hits.get(), 1, "item desabilitado não responde ao atalho");
        // WM_MENUSELECT do menu nativo também chega a on_highlight_changed.
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        file.on_highlight_changed({
            let seen = seen.clone();
            move |item| seen.borrow_mut().push(item.map(|i| i.text()))
        });
        send(&window, 0x011F /* WM_MENUSELECT */, later.id() as usize, file.handle());
        send(&window, 0x011F, 0x10 << 16, file.handle()); // MF_POPUP: submenu, sem item
        send(&window, 0x011F, 0xFFFF << 16, 0); // menu fechado
        assert_eq!(*seen.borrow(), [Some("Atualizar\tF5".to_string()), None, None]);
    });
}
