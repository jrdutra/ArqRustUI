//! CommandBar: botões, alternância, menus, botão dividido, busca, controles
//! hospedados, transbordamento (`»`), mouse e teclado.

mod common;

use arqrustui::ffi::user32::{PostMessageW, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP};
use arqrustui::menus::debug_state;
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::rc::Rc;

const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;
const VK_RIGHT: usize = 0x27;
const VK_DOWN: usize = 0x28;

fn post(app: &Application, widget: &impl Widget, message: u32, wparam: usize, lparam: isize) {
    // SAFETY: mensagem sem ponteiros postada a uma janela viva da thread de teste.
    unsafe { PostMessageW(widget.handle() as _, message, wparam, lparam) };
    app.process_pending().unwrap();
}

fn depth() -> usize {
    debug_state().map(|(depth, _)| depth).unwrap_or(0)
}

fn bar_in(window: &Window, width: i32) -> CommandBar {
    CommandBar::builder().bounds(0, 0, width, COMMAND_BAR_HEIGHT).parent(window).build().unwrap()
}

fn menu_with(counter: &Counter) -> Menu {
    let menu = Menu::new().unwrap();
    let hit = counter.clone();
    menu.add_item("&Primeiro", move || hit.hit()).unwrap();
    menu.add_item("&Segundo", || {}).unwrap();
    menu
}

#[test]
fn buttons_toggles_and_states() {
    with_app(|_| {
        let window = test_window();
        let bar = bar_in(&window, 400);
        assert_eq!(bar.height(), COMMAND_BAR_HEIGHT);
        let clicks = Counter::new();
        let hit = clicks.clone();
        let save =
            bar.add_button(CommandButton::new("&Salvar").icon(&icons::SAVE).on_click(move || hit.hit())).unwrap();
        let toggled = Rc::new(RefCell::new(Vec::new()));
        let log = toggled.clone();
        let bold = bar
            .add_button(CommandButton::new("Negrito").toggle(false).on_toggled(move |on| log.borrow_mut().push(on)))
            .unwrap();
        bar.add_separator().unwrap();
        let label = bar.add_label("Página 1").unwrap();
        assert_eq!(bar.len(), 4);

        save.trigger();
        assert_eq!(clicks.get(), 1);
        save.set_enabled(false);
        assert!(!save.is_enabled());
        save.trigger();
        assert_eq!(clicks.get(), 1, "botão desabilitado não executa");

        bold.trigger();
        bold.trigger();
        assert_eq!(*toggled.borrow(), vec![true, false]);
        bold.set_checked(true);
        assert!(bold.is_checked());
        assert_eq!(toggled.borrow().len(), 2, "set_checked não chama on_toggle");

        save.set_text("Salvar &tudo");
        assert_eq!(save.text(), "Salvar &tudo");
        label.set_text("Página 2");
        bold.set_visible(false);
        assert!(!bold.is_visible() && !bold.is_in_overflow());
        bar.clear();
        assert!(bar.is_empty());
    });
}

#[test]
fn mouse_clicks_buttons_and_opens_menus() {
    with_app(|app| {
        let window = test_window();
        let bar = bar_in(&window, 400);
        let clicks = Counter::new();
        let hit = clicks.clone();
        bar.add_button(CommandButton::new("Voltar").icon(&icons::BACK).icon_only().on_click(move || hit.hit()))
            .unwrap();
        let chosen = Counter::new();
        bar.add_button(CommandButton::new("Página").menu(&menu_with(&chosen))).unwrap();

        // Primeiro botão: margem de 6 e 32 de largura; centro em (22, 18).
        let at = (18 << 16) | 22;
        post(app, &bar, WM_LBUTTONDOWN, 1, at);
        post(app, &bar, WM_LBUTTONUP, 0, at);
        assert_eq!(clicks.get(), 1);
        // Soltar fora do botão cancela o clique.
        post(app, &bar, WM_LBUTTONDOWN, 1, at);
        post(app, &bar, WM_LBUTTONUP, 0, (18 << 16) | 300);
        assert_eq!(clicks.get(), 1);

        // Segundo botão: o clique abre o menu; Enter no primeiro item o executa.
        post(app, &bar, WM_LBUTTONDOWN, 1, (18 << 16) | 60);
        assert_eq!(depth(), 1, "o menu abre no clique");
        post(app, &bar, WM_KEYDOWN, VK_DOWN, 0);
        post(app, &bar, WM_KEYDOWN, VK_RETURN, 0);
        assert_eq!(chosen.get(), 1);
        assert!(debug_state().is_none());
    });
}

#[test]
fn dropdown_and_split_buttons() {
    with_app(|app| {
        let window = test_window();
        let bar = bar_in(&window, 500);
        let from_menu = Counter::new();
        let page = bar.add_button(CommandButton::new("Página").menu(&menu_with(&from_menu))).unwrap();
        page.open_menu();
        assert_eq!(depth(), 1);
        post(app, &bar, WM_KEYDOWN, VK_RETURN, 0);
        assert_eq!(from_menu.get(), 1, "aberto pelo teclado, o primeiro item já está selecionado");

        let main = Counter::new();
        let hit = main.clone();
        let other = Counter::new();
        let home =
            bar.add_button(CommandButton::new("Início").split(&menu_with(&other)).on_click(move || hit.hit())).unwrap();
        home.trigger();
        assert_eq!((main.get(), depth()), (1, 0), "a parte principal executa a ação, sem menu");
        home.open_menu();
        assert_eq!(depth(), 1, "a seta abre o menu");
        post(app, &bar, WM_KEYDOWN, VK_ESCAPE, 0);
        assert!(debug_state().is_none());
        assert_eq!(other.get(), 0);
    });
}

#[test]
fn search_box_submits_and_clears() {
    with_app(|app| {
        let window = test_window();
        let bar = bar_in(&window, 500);
        bar.add_spacer().unwrap();
        let searched = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = searched.clone();
        let search = bar.add_search("Buscar", 200, move |text| log.borrow_mut().push(text.to_string())).unwrap();
        let bounds = search.bounds();
        assert!(bounds.right() <= 500 && bounds.x >= 280, "a busca fica à direita: {bounds:?}");
        assert!(bounds.height < COMMAND_BAR_HEIGHT);

        search.set_text("relatório");
        post(app, &search, WM_KEYDOWN, VK_RETURN, 0);
        assert_eq!(*searched.borrow(), vec!["relatório".to_string()]);
        post(app, &search, WM_KEYDOWN, VK_ESCAPE, 0);
        assert_eq!(search.text(), "", "Esc limpa o campo");
    });
}

#[test]
fn hosted_widgets_and_overflow() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let bar = bar_in(&window, 600);
        for name in ["Um", "Dois", "Três", "Quatro"] {
            bar.add_button(CommandButton::new(name).icon(&icons::FILE).on_click(|| {})).unwrap();
        }
        bar.add_separator().unwrap();
        let combo = ComboBox::builder().parent(&bar).build().unwrap();
        let hosted = bar.add_widget(&combo, 120).unwrap();
        let outsider = Button::builder().text("Fora").parent(&window).build().unwrap();
        assert!(bar.add_widget(&outsider, 80).is_err(), "só controles criados dentro da barra");
        assert_eq!(bar.overflow_count(), 0);
        assert!(combo.is_visible());
        let placed = combo.bounds();
        assert!(placed.x > 200 && placed.bottom() <= COMMAND_BAR_HEIGHT);

        bar.set_size(150, COMMAND_BAR_HEIGHT);
        assert!(bar.overflow_count() >= 3, "itens no »: {}", bar.overflow_count());
        assert!(hosted.is_in_overflow());
        assert!(!combo.is_visible(), "controle que não cabe é escondido");
        bar.open_overflow();
        assert_eq!(depth(), 1);
        post(app, &bar, WM_KEYDOWN, VK_ESCAPE, 0);
        assert!(debug_state().is_none());
        bar.set_size(600, COMMAND_BAR_HEIGHT);
        assert_eq!(bar.overflow_count(), 0);
        assert!(combo.is_visible());
    });
}

#[test]
fn keyboard_moves_between_buttons() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let bar = bar_in(&window, 400);
        let (first, second) = (Counter::new(), Counter::new());
        let (a, b) = (first.clone(), second.clone());
        bar.add_button(CommandButton::new("A").on_click(move || a.hit())).unwrap();
        let disabled = bar.add_button(CommandButton::new("X").on_click(|| {})).unwrap();
        disabled.set_enabled(false);
        bar.add_button(CommandButton::new("B").on_click(move || b.hit())).unwrap();
        bar.focus();
        app.process_pending().unwrap();
        post(app, &bar, WM_KEYDOWN, VK_RETURN, 0);
        assert_eq!(first.get(), 1, "o foco começa no primeiro botão");
        post(app, &bar, WM_KEYDOWN, VK_RIGHT, 0);
        post(app, &bar, WM_KEYDOWN, VK_RETURN, 0);
        assert_eq!(second.get(), 1, "o botão desabilitado é pulado");
    });
}

fn texts(bar: &CommandBar) -> Vec<String> {
    bar.items().iter().filter(|i| i.is_visible()).map(|i| i.text()).collect()
}

#[test]
fn buttons_reorder_by_dragging_and_layouts_round_trip() {
    with_app(|app| {
        let window = test_window();
        let bar = bar_in(&window, 400);
        let clicks = Counter::new();
        let hit = clicks.clone();
        bar.add_button(CommandButton::new("A").icon(&icons::BACK).icon_only().on_click(move || hit.hit())).unwrap();
        bar.add_button(CommandButton::new("B").icon(&icons::HOME).icon_only()).unwrap();
        bar.add_button(CommandButton::new("C").icon(&icons::SAVE).icon_only()).unwrap();
        let changes = Counter::new();
        let seen = changes.clone();
        bar.on_reordered(move || seen.hit());
        bar.set_reorderable(true);
        // Botões de 32 a partir de x=6 (gap 2): A 6..38, B 40..72, C 74..106.
        post(app, &bar, WM_LBUTTONDOWN, 1, (18 << 16) | 22);
        post(app, &bar, 0x0200 /* WM_MOUSEMOVE */, 1, (18 << 16) | 60);
        post(app, &bar, 0x0200, 1, (18 << 16) | 80);
        post(app, &bar, WM_LBUTTONUP, 0, (18 << 16) | 80);
        assert_eq!(texts(&bar), ["B", "A", "C"], "A solto antes do meio de C");
        assert_eq!(clicks.get(), 0, "arrastar não clica");
        assert_eq!(changes.get(), 1);
        // Clique simples continua funcionando.
        post(app, &bar, WM_LBUTTONDOWN, 1, (18 << 16) | 56);
        post(app, &bar, WM_LBUTTONUP, 0, (18 << 16) | 56);
        assert_eq!(clicks.get(), 1);

        let saved = bar.layout().to_string();
        assert_eq!(saved, "2,1,3");
        let items = bar.items();
        bar.move_item(&items[2], 0);
        items[1].set_visible(false);
        assert_eq!(texts(&bar), ["C", "B"]);
        assert_eq!(bar.layout().to_string(), "3,2,-1");
        bar.set_layout(&saved.parse().unwrap());
        assert_eq!(texts(&bar), ["B", "A", "C"]);
        bar.set_layout(&"1,|,2,-3".parse().unwrap());
        assert_eq!(bar.layout().to_string(), "1,|,2,-3", "separador acrescentado pelo usuário");
        assert_eq!(bar.len(), 4);
        bar.set_default_layout();
        bar.set_layout(&"3,2,1".parse().unwrap());
        assert_eq!(bar.len(), 3, "o separador acrescentado sai quando o layout não o cita");
        bar.reset_layout();
        assert_eq!(bar.layout().to_string(), "1,|,2,-3");
    });
}

#[test]
fn hosted_controls_in_the_overflow_open_in_a_flyout() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let bar = bar_in(&window, 150);
        bar.add_button(CommandButton::new("Salvar").icon(&icons::SAVE)).unwrap();
        let search = bar.add_search("Buscar", 200, |_| {}).unwrap();
        assert_eq!(bar.overflow_count(), 1);
        let item = bar.items().into_iter().last().unwrap();
        assert!(item.is_in_overflow());
        assert_eq!(item.text(), "Buscar");
        bar.open_overflow();
        assert_eq!(depth(), 1, "o » lista o campo de busca");
        post(app, &bar, VK_ESCAPE_DOWN.0, VK_ESCAPE_DOWN.1, 0);
        bar.open_flyout(&item);
        app.process_pending().unwrap();
        assert!(bar.is_flyout_open());
        assert!(search.is_visible(), "o campo aparece na janela flutuante");
        assert_ne!(
            // SAFETY: consulta simples de uma janela viva do teste.
            unsafe { arqrustui::ffi::user32::GetParent(search.handle() as _) } as isize,
            bar.handle(),
            "o campo foi para a janela flutuante"
        );
        bar.close_flyout();
        assert!(!bar.is_flyout_open());
        // SAFETY: idem.
        let parent = unsafe { arqrustui::ffi::user32::GetParent(search.handle() as _) } as isize;
        assert_eq!(parent, bar.handle(), "o campo voltou para a barra");
        assert!(!search.is_visible(), "e continua no » (escondido)");
    });
}

const VK_ESCAPE_DOWN: (u32, usize) = (WM_KEYDOWN, VK_ESCAPE);

#[test]
fn customize_dialog_opens_and_cancels() {
    with_app(|_| {
        let window = test_window();
        window.show();
        let bar = bar_in(&window, 400);
        bar.add_button(CommandButton::new("&Novo")).unwrap();
        bar.add_separator().unwrap();
        bar.add_label("Texto").unwrap();
        // Fecha a caixa (WM_CLOSE = Cancelar) assim que o laço modal começar.
        let title: Vec<u16> = "Personalizar barra de comandos".encode_utf16().chain([0]).collect();
        let closer = Timer::interval(std::time::Duration::from_millis(50), move || {
            // SAFETY: título UTF-16 terminado em zero; WM_CLOSE postado à caixa da thread de teste.
            unsafe {
                let dialog = arqrustui::ffi::user32::FindWindowW(std::ptr::null(), title.as_ptr());
                if !dialog.is_null() {
                    arqrustui::ffi::user32::PostMessageW(dialog, 0x0010 /* WM_CLOSE */, 0, 0);
                }
            }
        })
        .unwrap();
        assert!(!bar.customize().unwrap(), "cancelado");
        drop(closer);
        assert_eq!(bar.layout().to_string(), "1,2,3", "cancelar não muda nada");
    });
}
