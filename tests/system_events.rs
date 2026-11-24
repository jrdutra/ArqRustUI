//! Entradas que vêm do Windows ou do usuário e que antes não eram simuladas: clique e
//! Enter num link, troca de tema do sistema, arquivos soltos na janela, exibição do
//! tooltip, menu de contexto (nativo e desenhado) e, na mudança de DPI, a recarga do
//! ícone da janela e o retângulo sugerido pelo sistema. Cada teste manda a mensagem exatamente como o Windows mandaria.

mod common;

use arqrustui::accessibility::uia_debug::menu_popups;
use arqrustui::ffi::types::{BOOL, HWND, LPARAM, POINT, RECT};
use arqrustui::ffi::user32::{ClientToScreen, GetClassNameW, IsWindowVisible, PostMessageW, SendMessageW};
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

const WM_SETTINGCHANGE: u32 = 0x001A;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_DROPFILES: u32 = 0x0233;
const WM_CONTEXTMENU: u32 = 0x007B;
const WM_GETICON: u32 = 0x007F;
const WM_SETICON: u32 = 0x0080;
const WM_DPICHANGED: u32 = 0x02E0;
const VK_RETURN: usize = 0x0D;
const VK_DOWN: usize = 0x28;
const GMEM_MOVEABLE: u32 = 0x0002;
const GMEM_ZEROINIT: u32 = 0x0040;
const ICON_SMALL: usize = 0;

#[link(name = "user32")]
extern "system" {
    fn EnumThreadWindows(thread: u32, callback: unsafe extern "system" fn(HWND, LPARAM) -> BOOL, data: LPARAM) -> BOOL;
    fn SetCursorPos(x: i32, y: i32) -> BOOL;
    fn EndMenu() -> BOOL;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> u32;
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut std::ffi::c_void;
    fn GlobalLock(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalUnlock(memory: *mut std::ffi::c_void) -> BOOL;
}

unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
    // SAFETY: `data` é o `Vec` local de quem chamou a enumeração, vivo durante ela.
    unsafe { (*(data as *mut Vec<HWND>)).push(hwnd) };
    1
}

/// Janelas visíveis desta thread com a classe pedida.
fn thread_windows(class: &str) -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::new();
    // SAFETY: a enumeração é síncrona e só usa o `Vec` local.
    unsafe { EnumThreadWindows(GetCurrentThreadId(), collect, &mut windows as *mut Vec<HWND> as LPARAM) };
    windows
        .into_iter()
        .filter(|&w| {
            let mut buffer = [0u16; 64];
            // SAFETY: buffer local com o tamanho informado; consulta simples.
            let length = unsafe { GetClassNameW(w, buffer.as_mut_ptr(), buffer.len() as i32) };
            // SAFETY: consulta simples.
            String::from_utf16_lossy(&buffer[..length.max(0) as usize]) == class && unsafe { IsWindowVisible(w) } != 0
        })
        .collect()
}

/// Os testes deste arquivo dependem da ativação, do foco e do cursor, que são da
/// sessão inteira: um menu aberto fecha se outro teste ativar uma janela no meio.
/// Por isso rodam um de cada vez (o `cargo test` roda os testes de um arquivo em
/// paralelo).
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lparam(x: i32, y: i32) -> isize {
    ((y as u32 as isize) << 16) | (x as u16 as isize)
}

#[test]
fn hyperlink_reports_clicks_and_enter() {
    let _serial = serial();
    with_app(|app| {
        let window = test_window();
        window.show();
        let link = Hyperlink::builder()
            .text("<a href=\"https://example.com/docs\">Documentação</a>")
            .bounds(10, 10, 200, 24)
            .parent(&window)
            .build()
            .unwrap();
        let targets = Rc::new(RefCell::new(Vec::new()));
        let log = targets.clone();
        link.on_link_click(move |target| log.borrow_mut().push(target.to_string()));
        app.process_pending().unwrap();

        // Clique do mouse sobre o texto do link (o SysLink manda NM_CLICK ao pai).
        let hwnd = link.handle() as HWND;
        // SAFETY: mensagens de mouse para o controle vivo desta thread.
        unsafe {
            SendMessageW(hwnd, WM_LBUTTONDOWN, 1, lparam(8, 8));
            SendMessageW(hwnd, WM_LBUTTONUP, 0, lparam(8, 8));
        }
        assert!(pump_until(app, Duration::from_secs(2), || !targets.borrow().is_empty()), "NM_CLICK");
        assert_eq!(targets.borrow()[0], "https://example.com/docs");

        // Enter com o link focado (NM_RETURN).
        link.focus();
        // SAFETY: teclas para o controle vivo desta thread.
        unsafe {
            SendMessageW(hwnd, WM_KEYDOWN, VK_RETURN, 0);
            SendMessageW(hwnd, WM_KEYUP, VK_RETURN, 0);
        }
        assert!(pump_until(app, Duration::from_secs(2), || targets.borrow().len() == 2), "NM_RETURN");
        assert_eq!(targets.borrow()[1], "https://example.com/docs");
        window.destroy();
    });
}

#[test]
fn system_theme_change_is_reported() {
    let _serial = serial();
    with_app(|_| {
        let window = test_window();
        let changes = Counter::new();
        let hit = changes.clone();
        window.on_theme_changed(move || hit.hit());
        let send = |area: &str| {
            let wide: Vec<u16> = area.encode_utf16().chain([0]).collect();
            // SAFETY: a string termina em NUL e vive durante o envio síncrono.
            unsafe { SendMessageW(window.handle() as HWND, WM_SETTINGCHANGE, 0, wide.as_ptr() as isize) };
        };
        // O Windows avisa a troca de claro/escuro com "ImmersiveColorSet".
        send("ImmersiveColorSet");
        assert_eq!(changes.get(), 1);
        send("WindowsThemeElement");
        assert_eq!(changes.get(), 2);
        // Outras áreas de configuração não são troca de tema.
        send("intl");
        send("Environment");
        // SAFETY: LPARAM nulo é permitido em WM_SETTINGCHANGE.
        unsafe { SendMessageW(window.handle() as HWND, WM_SETTINGCHANGE, 0, 0) };
        assert_eq!(changes.get(), 2);
        window.destroy();
    });
}

/// Monta um HDROP como o Explorer: `DROPFILES` seguido dos caminhos em UTF-16, cada um
/// terminado em NUL, e um NUL final.
fn hdrop(paths: &[&str]) -> *mut std::ffi::c_void {
    let mut text: Vec<u16> = Vec::new();
    for path in paths {
        text.extend(path.encode_utf16());
        text.push(0);
    }
    text.push(0);
    let header = 20usize; // DROPFILES: pFiles (u32), pt (POINT), fNC (BOOL), fWide (BOOL)
                          // SAFETY: memória global nova, preenchida dentro do tamanho pedido; o destinatário
                          // de WM_DROPFILES a libera com DragFinish.
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, header + text.len() * 2);
        assert!(!memory.is_null());
        let base = GlobalLock(memory) as *mut u8;
        std::ptr::write_unaligned(base as *mut u32, header as u32);
        std::ptr::write_unaligned(base.add(16) as *mut u32, 1); // fWide
        std::ptr::copy_nonoverlapping(text.as_ptr() as *const u8, base.add(header), text.len() * 2);
        GlobalUnlock(memory);
        memory
    }
}

#[test]
fn files_dropped_on_the_window_arrive_as_paths() {
    let _serial = serial();
    with_app(|_| {
        let window = Window::builder().title("Soltar arquivos").size(400, 300).accept_files(true).build().unwrap();
        let received = Rc::new(RefCell::new(Vec::<PathBuf>::new()));
        let log = received.clone();
        window.on_drop_files(move |files| log.borrow_mut().extend_from_slice(files));
        let drop = hdrop(&["C:\\dados\\relatório.pdf", "C:\\dados\\fotos\\praia 1.jpg"]);
        // SAFETY: HDROP válido montado acima; a janela o libera ao tratar a mensagem.
        unsafe { SendMessageW(window.handle() as HWND, WM_DROPFILES, drop as usize, 0) };
        assert_eq!(
            *received.borrow(),
            vec![PathBuf::from("C:\\dados\\relatório.pdf"), PathBuf::from("C:\\dados\\fotos\\praia 1.jpg")]
        );
        window.destroy();
    });
}

#[test]
fn tooltip_appears_when_the_pointer_rests_on_the_control() {
    let _serial = serial();
    with_app(|app| {
        let window = Window::builder().title("Dicas").size(400, 300).position(100, 100).build().unwrap();
        window.show();
        let button = Button::builder().text("Salvar").bounds(40, 40, 120, 32).parent(&window).build().unwrap();
        button.set_tooltip("Salva o documento (Ctrl+S)").unwrap();
        app.process_pending().unwrap();
        assert!(thread_windows("tooltips_class32").is_empty(), "invisível antes de o ponteiro chegar");

        // O ponteiro pousa sobre o botão: o tooltip (TTF_SUBCLASS) vê o WM_MOUSEMOVE e,
        // depois do atraso inicial, aparece sozinho.
        let mut point = POINT { x: 20, y: 12 };
        // SAFETY: ponteiro local; janelas vivas desta thread.
        unsafe {
            ClientToScreen(button.handle() as HWND, &mut point);
            SetCursorPos(point.x, point.y);
            PostMessageW(button.handle() as HWND, WM_MOUSEMOVE, 0, lparam(20, 12));
        }
        assert!(
            pump_until(app, Duration::from_secs(5), || !thread_windows("tooltips_class32").is_empty()),
            "o tooltip apareceu"
        );
        let tip = thread_windows("tooltips_class32")[0];
        let mut buffer = [0u16; 128];
        // SAFETY: buffer local com o tamanho informado.
        let length = unsafe { arqrustui::ffi::user32::GetWindowTextW(tip, buffer.as_mut_ptr(), buffer.len() as i32) };
        let text = String::from_utf16_lossy(&buffer[..length.max(0) as usize]);
        assert!(text.is_empty() || text == "Salva o documento (Ctrl+S)", "texto: {text:?}");
        window.destroy();
    });
}

#[test]
fn context_menus_open_on_wm_contextmenu() {
    let _serial = serial();
    with_app(|app| {
        let window = Window::builder().title("Contexto").size(400, 300).position(120, 120).build().unwrap();
        window.show();
        let area = Panel::builder().bounds(0, 0, 400, 300).parent(&window).build().unwrap();
        app.process_pending().unwrap();
        let chosen = Rc::new(RefCell::new(Vec::new()));

        // Menu do Windows: TrackPopupMenu roda um laço modal; um temporizador escolhe o
        // primeiro item pelo teclado, como o usuário faria.
        let menu = ContextMenu::new().unwrap();
        for name in ["Copiar", "Colar"] {
            let log = chosen.clone();
            menu.add_item(name, move || log.borrow_mut().push(name)).unwrap();
        }
        area.set_context_menu(&menu);
        let seen = Rc::new(RefCell::new(false));
        let (flag, owner) = (seen.clone(), area.handle());
        let _timer = Timer::interval(Duration::from_millis(50), move || {
            if *flag.borrow() || thread_windows("#32768").is_empty() {
                return;
            }
            *flag.borrow_mut() = true;
            // SAFETY: teclas para a janela dona do menu, no laço modal desta thread.
            unsafe {
                PostMessageW(owner as HWND, WM_KEYDOWN, VK_DOWN, 0);
                PostMessageW(owner as HWND, WM_KEYDOWN, VK_RETURN, 0);
            }
        })
        .unwrap();
        let mut point = POINT { x: 50, y: 50 };
        // SAFETY: ponteiro local; janela viva.
        unsafe { ClientToScreen(area.handle() as HWND, &mut point) };
        // SAFETY: WM_CONTEXTMENU com a posição de tela, como o Windows manda.
        unsafe {
            SendMessageW(area.handle() as HWND, WM_CONTEXTMENU, area.handle() as usize, lparam(point.x, point.y))
        };
        if chosen.borrow().is_empty() {
            // SAFETY: fecha um menu que tenha ficado aberto nesta thread.
            unsafe { EndMenu() };
        }
        assert!(*seen.borrow(), "o menu do Windows apareceu");
        assert_eq!(*chosen.borrow(), vec!["Copiar"], "o primeiro item foi escolhido pelo teclado");

        // Menu desenhado pela biblioteca: abre sem bloquear, numa janela própria.
        let drawn = ContextMenu::new().unwrap();
        drawn.set_drawn(true);
        let log = chosen.clone();
        drawn.add_item("Recortar", move || log.borrow_mut().push("Recortar")).unwrap();
        area.set_context_menu(&drawn);
        // Pela tecla de menu (posição -1, -1): abre junto ao canto do controle.
        // SAFETY: WM_CONTEXTMENU do teclado, como o Windows manda.
        unsafe { SendMessageW(area.handle() as HWND, WM_CONTEXTMENU, area.handle() as usize, -1) };
        assert!(pump_until(app, Duration::from_secs(2), || !menu_popups().is_empty()), "o menu desenhado abriu");
        drawn.close();
        assert!(pump_until(app, Duration::from_secs(2), || menu_popups().is_empty()));
        window.destroy();
    });
}

#[test]
fn dpi_change_reapplies_the_window_icon_and_the_suggested_bounds() {
    let _serial = serial();
    with_app(|_| {
        let window =
            Window::builder().title("Ícone").size(400, 300).position(50, 50).icon(&icons::FOLDER).build().unwrap();
        let hwnd = window.handle() as HWND;
        // SAFETY: consulta simples à janela viva.
        let small = || unsafe { SendMessageW(hwnd, WM_GETICON, ICON_SMALL, 0) };
        assert_ne!(small(), 0, "a janela tem ícone");
        // Tira o ícone: a mudança de DPI tem de recolocá-lo (no tamanho do DPI atual).
        // SAFETY: WM_SETICON com ícone nulo remove o ícone pequeno.
        unsafe { SendMessageW(hwnd, WM_SETICON, ICON_SMALL, 0) };
        assert_eq!(small(), 0);
        let reported = Rc::new(RefCell::new(Vec::new()));
        let log = reported.clone();
        window.on_dpi_changed(move |dpi| log.borrow_mut().push(dpi));

        // O sistema não muda o DPI real numa mensagem simulada: reenvia o atual, com um
        // retângulo sugerido novo, como faz tests/window_maximize.rs.
        let dpi = window.dpi();
        let suggested = RECT { left: 70, top: 80, right: 70 + 520, bottom: 80 + 390 };
        // SAFETY: o RECT vive durante o envio síncrono.
        unsafe { SendMessageW(hwnd, WM_DPICHANGED, (dpi | (dpi << 16)) as usize, &suggested as *const RECT as isize) };
        assert_ne!(small(), 0, "o ícone foi recarregado");
        assert_eq!(*reported.borrow(), vec![dpi], "on_dpi_changed");
        let mut rect = RECT::default();
        // SAFETY: ponteiro local; janela viva.
        unsafe { arqrustui::ffi::user32::GetWindowRect(hwnd, &mut rect) };
        assert_eq!(
            (rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top),
            (70, 80, 520, 390),
            "retângulo sugerido"
        );
        window.destroy();
    });
}
