//! Modal no estilo da web: fundo em camadas sobre a janela dona, cartão centralizado,
//! bloqueio da dona, Esc/Enter/✕/clique no fundo, validação, foco e acompanhamento.
//! Visual conferido no Wine (que não compõe transparência por pixel: lá o fundo vira
//! uma foto escurecida da janela).

mod common;

use arqrustui::ffi::user32::{GetFocus, GetWindowLongPtrW, IsWindow, IsWindowEnabled, IsWindowVisible};
use arqrustui::prelude::*;
use common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const GWL_EXSTYLE: i32 = -20;

fn owner() -> Window {
    let window = Window::builder().title("Dona").size(640, 420).position(120, 90).build().unwrap();
    window.show();
    window
}

fn enabled(window: &Window) -> bool {
    // SAFETY: consulta simples.
    unsafe { IsWindowEnabled(window.handle() as _) != 0 }
}

fn alive(handle: isize) -> bool {
    // SAFETY: consulta simples.
    unsafe { IsWindow(handle as _) != 0 }
}

#[test]
fn opens_over_the_owner_blocks_it_and_closes_with_the_button() {
    with_app(|app| {
        motion::set_animations_enabled(Some(true));
        let window = owner();
        let field = TextBox::builder().bounds(10, 10, 200, 28).parent(&window).build().unwrap();
        field.focus();
        let modal = Modal::builder()
            .title("Excluir arquivo?")
            .message("O arquivo será excluído permanentemente.")
            .button("Cancelar", DialogResult::Cancel)
            .primary_button("Excluir", DialogResult::Ok)
            .build(&window)
            .unwrap();
        assert!(!modal.is_open());
        let results = Rc::new(RefCell::new(Vec::new()));
        let log = results.clone();
        modal.show_then(move |r| log.borrow_mut().push(r)).unwrap();
        assert!(modal.is_open());
        assert!(!enabled(&window), "a dona fica bloqueada");
        let (backdrop, card) = (modal.backdrop().handle(), modal.card().handle());
        // SAFETY: janelas vivas desta thread.
        unsafe {
            assert!(IsWindowVisible(backdrop as _) != 0 && IsWindowVisible(card as _) != 0);
            assert!(GetWindowLongPtrW(backdrop as _, GWL_EXSTYLE) as u32 & 0x80000 != 0, "fundo em camadas");
        }
        // O fundo cobre exatamente a área cliente da dona; o cartão fica centralizado nela.
        let (outer, area) = (native_rect(backdrop), window.client_size());
        assert_eq!((outer.right - outer.left, outer.bottom - outer.top), (area.width, area.height));
        app.process_pending().unwrap();
        assert!(pump_until(app, Duration::from_secs(2), || modal_is_settled(&modal)));
        let card_rect = modal.card_bounds();
        assert_eq!(card_rect.width, 440);
        assert!((card_rect.x + card_rect.width / 2 - area.width / 2).abs() <= 1, "centralizado: {card_rect:?}");
        assert!(card_rect.y >= 0 && card_rect.y + card_rect.height <= area.height);
        assert_eq!(modal.card().title(), "Excluir arquivo?");
        // Foco no botão principal (a mensagem não tem controles).
        let primary = modal.buttons()[1].clone();
        // SAFETY: consulta simples.
        assert_eq!(unsafe { GetFocus() } as isize, primary.handle());
        // Botões alinhados à direita, o principal por último.
        let (cancel, ok) = (modal.buttons()[0].bounds(), primary.bounds());
        assert!(cancel.x < ok.x && ok.x + ok.width == 440 - 24);
        primary.click();
        assert!(pump_until(app, Duration::from_secs(2), || !results.borrow().is_empty()));
        assert_eq!(*results.borrow(), vec![DialogResult::Ok]);
        assert_eq!(modal.result(), DialogResult::Ok);
        assert!(!modal.is_open() && enabled(&window));
        assert!(!alive(backdrop) && !alive(card), "janelas destruídas");
        // O foco volta ao campo (também quando a janela é reativada depois; outros testes
        // em paralelo podem ter tomado a ativação).
        // SAFETY: ativa a janela viva desta thread e consulta o foco.
        unsafe {
            arqrustui::ffi::user32::SetActiveWindow(window.handle() as _);
            assert_eq!(GetFocus() as isize, field.handle(), "o foco volta ao campo");
        }
        motion::set_animations_enabled(None);
        window.destroy();
    });
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

fn modal_is_settled(modal: &Modal) -> bool {
    modal.is_open() && motion::running_animations() == 0
}

#[test]
fn escape_enter_close_button_and_backdrop() {
    with_app(|app| {
        motion::set_animations_enabled(Some(false));
        let window = owner();
        let build = |backdrop_closes: bool| {
            Modal::builder()
                .title("Teste")
                .message("Mensagem")
                .button("Cancelar", DialogResult::Cancel)
                .primary_button("OK", DialogResult::Ok)
                .close_on_backdrop(backdrop_closes)
                .build(&window)
                .unwrap()
        };
        // Esc → Cancel (IDCANCEL vira WM_CLOSE no cartão).
        let modal = build(false);
        modal.show_then(|_| {}).unwrap();
        send(modal.card(), 0x0111, 2, 0); // WM_COMMAND IDCANCEL
        app.process_pending().unwrap();
        assert_eq!(modal.result(), DialogResult::Cancel);
        assert!(!modal.is_open() && enabled(&window));
        // Enter → botão principal (IDOK).
        let modal = build(false);
        modal.show_then(|_| {}).unwrap();
        send(modal.card(), 0x0111, 1, 0); // WM_COMMAND IDOK
        app.process_pending().unwrap();
        assert_eq!(modal.result(), DialogResult::Ok);
        // Clique no fundo: com close_on_backdrop desligado, continua aberto.
        let modal = build(false);
        modal.show_then(|_| {}).unwrap();
        send(modal.backdrop(), WM_LBUTTONDOWN, 1, (5 << 16 | 5) as isize);
        send(modal.backdrop(), WM_LBUTTONUP, 0, (5 << 16 | 5) as isize);
        assert!(modal.is_open(), "o cartão só balança");
        // ✕ fecha com Cancel.
        let close = modal.close_button().expect("botão fechar");
        send(&close, WM_LBUTTONUP, 0, (10 << 16 | 10) as isize);
        assert_eq!(modal.result(), DialogResult::Cancel);
        assert!(!modal.is_open());
        // Com close_on_backdrop ligado, o clique no fundo fecha.
        let modal = build(true);
        modal.show_then(|_| {}).unwrap();
        send(modal.backdrop(), WM_LBUTTONUP, 0, (5 << 16 | 5) as isize);
        assert!(!modal.is_open());
        assert_eq!(modal.result(), DialogResult::Cancel);
        assert!(enabled(&window));
        motion::set_animations_enabled(None);
        window.destroy();
    });
}

#[test]
fn validation_blocking_show_and_owner_tracking() {
    with_app(|app| {
        motion::set_animations_enabled(Some(false));
        let window = owner();
        let modal = Modal::builder()
            .title("Renomear")
            .content_height(60)
            .button("Cancelar", DialogResult::Cancel)
            .primary_button("Salvar", DialogResult::Ok)
            .build(&window)
            .unwrap();
        let content = modal.content();
        assert_eq!(content.client_size(), Size::new(440 - 48, 60));
        let input = TextBox::builder().bounds(0, 0, 300, 28).parent(&content).build().unwrap();
        let vetoes = Rc::new(Cell::new(0));
        let (probe, count) = (input.clone(), vetoes.clone());
        modal.on_close_request(move |result| {
            if result == DialogResult::Ok && probe.text().is_empty() {
                count.set(count.get() + 1);
                return false;
            }
            true
        });
        // `show` bloqueia; um temporizador faz o papel do usuário.
        let (step, target, field, owner_window) = (Rc::new(Cell::new(0)), modal.clone(), input.clone(), window.clone());
        let _timer = Timer::interval(Duration::from_millis(30), move || {
            match step.get() {
                0 => {
                    // SAFETY: consulta simples.
                    assert_eq!(unsafe { GetFocus() } as isize, field.handle(), "foco no primeiro controle do conteúdo");
                    target.buttons()[1].click(); // vazio: recusado
                }
                1 => {
                    assert!(target.is_open());
                    // A dona muda de lugar e de tamanho: fundo e cartão acompanham.
                    owner_window.set_bounds(Rect::new(300, 200, 700, 500));
                }
                2 => {
                    let area = owner_window.client_size();
                    let backdrop = native_rect(target.backdrop().handle());
                    assert_eq!(
                        (backdrop.right - backdrop.left, backdrop.bottom - backdrop.top),
                        (area.width, area.height)
                    );
                    let card = target.card_bounds();
                    assert!((card.x + card.width / 2 - area.width / 2).abs() <= 1, "recentralizado: {card:?}");
                    field.set_text("novo nome");
                    target.buttons()[1].click();
                }
                _ => {}
            }
            step.set(step.get() + 1);
        })
        .unwrap();
        let result = modal.show().unwrap();
        assert_eq!(result, DialogResult::Ok);
        assert_eq!(vetoes.get(), 1);
        assert!(enabled(&window));
        // Atalhos prontos e tema escuro.
        Theme::dark().apply(&window);
        let dark = Modal::builder()
            .title("Escuro")
            .message("texto")
            .primary_button("OK", DialogResult::Ok)
            .build(&window)
            .unwrap();
        dark.show_then(|_| {}).unwrap();
        let surface = color_at(dark.card(), 200, 60);
        assert!(surface.luminance() < 0.3, "cartão no tema escuro: {surface:?}");
        dark.close_with(DialogResult::Ok);
        let _ = app;
        window.destroy();
        motion::set_animations_enabled(None);
    });
}

#[test]
fn closing_the_owner_closes_the_modal() {
    with_app(|app| {
        motion::set_animations_enabled(Some(false));
        let window = owner();
        let modal =
            Modal::builder().title("X").message("y").primary_button("OK", DialogResult::Ok).build(&window).unwrap();
        let closed = Rc::new(Cell::new(false));
        let flag = closed.clone();
        modal.show_then(move |_| flag.set(true)).unwrap();
        window.destroy();
        app.process_pending().unwrap();
        assert!(closed.get() && !modal.is_open());
        motion::set_animations_enabled(None);
    });
}
