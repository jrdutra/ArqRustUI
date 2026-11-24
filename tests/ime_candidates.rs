//! Lista de candidatos própria do IME. Sem um IME instalado no ambiente de teste, os
//! candidatos chegam por `Ime::simulate_candidates` e pelas notificações `WM_IME_NOTIFY`
//! enviadas diretamente; a troca de candidatos dentro de um IME real não é exercida.

mod common;

use arqrustui::ffi::gdi32::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetPixel, SelectObject,
};
use arqrustui::ffi::user32::{GetDC, ReleaseDC, SendMessageW};
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::rc::Rc;

const WM_PRINTCLIENT: u32 = 0x0318;
const PRF_CLIENT: isize = 0x4;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_IME_NOTIFY: u32 = 0x0282;
const WM_IME_SETCONTEXT: u32 = 0x0281;
const IMN_CLOSECANDIDATE: usize = 4;

fn luminance(hwnd: isize, size: Size, at: (i32, i32)) -> f64 {
    // SAFETY: DCs e bitmap temporários desta thread, liberados ao final; a janela é viva.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, size.width, size.height);
        let previous = SelectObject(memory, bitmap as _);
        SendMessageW(hwnd as _, WM_PRINTCLIENT, memory as usize, PRF_CLIENT);
        let c = GetPixel(memory, at.0, at.1);
        SelectObject(memory, previous);
        DeleteObject(bitmap as _);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
        let (r, g, b) = ((c & 0xFF) as f64, ((c >> 8) & 0xFF) as f64, ((c >> 16) & 0xFF) as f64);
        (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0
    }
}

fn candidates() -> ImeCandidates {
    ImeCandidates::new(["你", "尼", "泥", "拟", "逆", "腻", "妮", "倪", "匿", "霓", "溺"])
}

#[test]
fn library_candidate_window_follows_the_ime_and_picks() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let field = TextBox::builder().bounds(10, 10, 240, 26).parent(&window).build().unwrap();
        field.focus();
        assert!(!Ime::has_custom_candidates(&field));
        let list = Ime::use_candidate_window(&field).unwrap();
        assert!(Ime::has_custom_candidates(&field));
        let seen: Rc<RefCell<Vec<Option<usize>>>> = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        Ime::on_candidates_changed(&field, move |list| log.borrow_mut().push(list.map(|l| l.items.len())));
        let picked = Rc::new(RefCell::new(Vec::new()));
        let log = picked.clone();
        list.on_pick(move |index| log.borrow_mut().push(index));

        Ime::set_composition_position(&field, Point::new(30, 4));
        Ime::simulate_candidates(&field, Some(candidates()));
        app.process_pending().unwrap();
        assert!(list.is_visible());
        assert_eq!(Ime::candidates(&field).unwrap().items.len(), 11);
        assert_eq!(list.candidates().unwrap().page().len(), 9);
        let size = list.size();
        assert_eq!(size.height, 4 + 9 * 24 + 24 + 4, "9 linhas e o rodapé de páginas: {size:?}");
        let bounds = native_rect(list.handle());
        let field_rect = native_rect(field.handle());
        assert!(
            bounds.left >= field_rect.left + 30 && bounds.top >= field_rect.top + 20,
            "abaixo da posição de composição: {bounds:?} × {field_rect:?}"
        );

        // Clique na terceira linha da segunda página.
        Ime::simulate_candidates(&field, Some(candidates().with_page(9, 9).with_selection(9)));
        assert_eq!(list.size().height, 4 + 2 * 24 + 24 + 4);
        let row = list.row_bounds(1).unwrap();
        let lp = ((row.y + row.height / 2) << 16 | (row.x + 10)) as isize;
        send_raw(list.handle(), WM_LBUTTONUP, 0, lp);
        assert_eq!(*picked.borrow(), vec![10]);
        assert!(list.row_bounds(2).is_none());

        // O IME avisa que a lista fechou.
        send(&field, WM_IME_NOTIFY, IMN_CLOSECANDIDATE, 1);
        assert!(!list.is_visible());
        assert!(Ime::candidates(&field).is_none());
        assert_eq!(*seen.borrow(), vec![Some(11), Some(11), None]);
        // WM_IME_SETCONTEXT passa adiante (com as janelas do IME escondidas) sem falhar.
        send(&field, WM_IME_SETCONTEXT, 1, 0xC000_000Fu32 as i32 as isize);
        window.destroy();
    });
}

#[test]
fn candidate_window_uses_the_theme_colors() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let field = TextBox::builder().bounds(10, 10, 240, 26).parent(&window).build().unwrap();
        let list = ImeCandidateWindow::new(&field).unwrap();
        list.show(&candidates(), Point::new(0, 30));
        app.process_pending().unwrap();
        let light = luminance(list.handle(), list.size(), (list.size().width - 6, 60));
        Theme::dark().apply(&window);
        list.show(&candidates(), Point::new(0, 30));
        app.process_pending().unwrap();
        let dark = luminance(list.handle(), list.size(), (list.size().width - 6, 60));
        assert!(light > 0.8 && dark < 0.3, "claro {light:.2}, escuro {dark:.2}");
        list.hide();
        assert!(!list.is_visible() && list.candidates().is_none());
        // Destruir o widget fecha a janela de candidatos.
        let handle = list.handle();
        field.destroy();
        app.process_pending().unwrap();
        assert!(!list.is_visible());
        // SAFETY: consulta simples.
        assert_eq!(unsafe { arqrustui::ffi::user32::IsWindow(handle as _) }, 0);
        window.destroy();
    });
}
