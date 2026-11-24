//! Diálogos do sistema: abrir, salvar, pasta, cor, fonte, impressão, configurar
//! página, localizar, `MessageBox` e `TaskDialog`.
//!
//! Esses diálogos são modais e bloqueiam a thread até o usuário responder. Um
//! `Timer` da biblioteca continua disparando dentro do laço modal do sistema: a
//! cada tique ele procura um diálogo (classe `#32770`) desta thread, registra o
//! título, os textos dos controles e se a janela dona estava bloqueada, e responde
//! com um comando (`IDOK`, `IDCANCEL`, `IDYES`...), como o usuário faria.

mod common;

use arqrustui::ffi::types::{BOOL, HWND, LPARAM};
use arqrustui::ffi::user32::{
    EnumChildWindows, GetClassNameW, GetWindowTextW, IsWindowEnabled, IsWindowVisible, PostMessageW, SendMessageW,
};
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

const WM_COMMAND: u32 = 0x0111;
const IDOK: usize = 1;
const IDCANCEL: usize = 2;
const IDYES: usize = 6;
const IDNO: usize = 7;
const WM_USER: u32 = 0x0400;
const TDM_CLICK_BUTTON: u32 = WM_USER + 102;
const TDM_CLICK_VERIFICATION: u32 = WM_USER + 113;

#[link(name = "user32")]
extern "system" {
    fn EnumThreadWindows(thread: u32, callback: unsafe extern "system" fn(HWND, LPARAM) -> BOOL, data: LPARAM) -> BOOL;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> u32;
}

/// O que o "usuário" viu no diálogo.
#[derive(Clone, Debug, Default)]
struct Seen {
    title: String,
    texts: Vec<String>,
    owner_blocked: bool,
}

fn window_text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 512];
    // SAFETY: buffer local com o tamanho informado; janela desta thread.
    let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 64];
    // SAFETY: buffer local com o tamanho informado.
    let length = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
    // SAFETY: `data` é o `Vec` local de quem chamou a enumeração, vivo durante ela.
    unsafe { (*(data as *mut Vec<HWND>)).push(hwnd) };
    1
}

/// Diálogos visíveis (`#32770`) desta thread.
fn thread_dialogs() -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::new();
    // SAFETY: a enumeração é síncrona e só usa o `Vec` local.
    unsafe { EnumThreadWindows(GetCurrentThreadId(), collect, &mut windows as *mut Vec<HWND> as LPARAM) };
    windows
        .into_iter()
        // SAFETY: consulta simples a janelas desta thread.
        .filter(|&w| class_name(w) == "#32770" && unsafe { IsWindowVisible(w) } != 0)
        .collect()
}

fn child_texts(dialog: HWND) -> Vec<String> {
    let mut children: Vec<HWND> = Vec::new();
    // SAFETY: a enumeração é síncrona e só usa o `Vec` local.
    unsafe { EnumChildWindows(dialog, collect, &mut children as *mut Vec<HWND> as LPARAM) };
    children.into_iter().map(window_text).filter(|t| !t.is_empty()).collect()
}

/// Responde ao primeiro diálogo que aparecer nesta thread: registra o que se via e
/// chama `respond(diálogo)`. Guarde o `Timer` até o `show` voltar.
fn answer(owner: &Window, respond: impl Fn(HWND) + 'static) -> (Timer, Rc<RefCell<Option<Seen>>>) {
    let seen = Rc::new(RefCell::new(None));
    let (slot, owner) = (seen.clone(), owner.handle());
    let timer = Timer::interval(Duration::from_millis(50), move || {
        if slot.borrow().is_some() {
            return;
        }
        if let Some(&dialog) = thread_dialogs().first() {
            *slot.borrow_mut() = Some(Seen {
                title: window_text(dialog),
                texts: child_texts(dialog),
                // SAFETY: consulta simples.
                owner_blocked: unsafe { IsWindowEnabled(owner as HWND) } == 0,
            });
            respond(dialog);
        }
    })
    .unwrap();
    (timer, seen)
}

fn command(id: usize) -> impl Fn(HWND) {
    // SAFETY: posta um comando ao diálogo vivo desta thread.
    move |dialog| unsafe {
        PostMessageW(dialog, WM_COMMAND, id, 0);
    }
}

fn owner() -> Window {
    let window = Window::builder().title("Dona dos diálogos").size(500, 300).position(80, 80).build().unwrap();
    window.show();
    window
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("arqrustui-dialogos-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn file_dialogs_open_save_and_pick_folders() {
    with_app(|_| {
        let window = owner();
        let dir = temp_dir("arquivos");
        let file = dir.join("relatorio.txt");
        std::fs::write(&file, "conteúdo").unwrap();

        // Abrir: título próprio, dona bloqueada, cancelar → None.
        let (_timer, seen) = answer(&window, command(IDCANCEL));
        let result = OpenFileDialog::new().title("Abrir relatório").folder(&dir).show(&window).unwrap();
        let seen = seen.borrow().clone().expect("o diálogo de abrir apareceu");
        assert_eq!(result, None);
        assert_eq!(seen.title, "Abrir relatório");
        assert!(seen.owner_blocked, "o diálogo é modal em relação à dona");
        assert!(window.is_enabled(), "a dona volta a ficar habilitada");

        // Abrir: arquivo existente + OK → o caminho.
        let (_timer, seen) = answer(&window, command(IDOK));
        let result = OpenFileDialog::new()
            .folder(&dir)
            .file_name("relatorio.txt")
            .filter("Texto", "*.txt")
            .show(&window)
            .unwrap();
        assert!(seen.borrow().is_some());
        assert_eq!(result.as_deref().and_then(|p| p.file_name()), file.file_name(), "{result:?}");

        // Salvar: nome sugerido + OK → caminho na pasta pedida.
        let (_timer, seen) = answer(&window, command(IDOK));
        let result = SaveFileDialog::new()
            .title("Salvar como")
            .folder(&dir)
            .file_name("novo.txt")
            .default_extension("txt")
            .show(&window)
            .unwrap();
        assert_eq!(seen.borrow().as_ref().map(|s| s.title.clone()).as_deref(), Some("Salvar como"));
        let saved = result.expect("salvar confirmado");
        assert_eq!(saved.file_name().and_then(|n| n.to_str()), Some("novo.txt"));
        assert_eq!(saved.parent().map(|p| p.canonicalize().unwrap()), Some(dir.canonicalize().unwrap()));

        // Pasta: cancelar → None.
        let (_timer, seen) = answer(&window, command(IDCANCEL));
        let result = FolderPicker::new().title("Escolha a pasta").folder(&dir).show(&window).unwrap();
        assert!(seen.borrow().is_some(), "o seletor de pasta apareceu");
        assert_eq!(result, None);

        std::fs::remove_dir_all(&dir).ok();
        window.destroy();
    });
}

#[test]
fn color_and_font_dialogs_return_the_choice_or_none() {
    with_app(|_| {
        let window = owner();
        let initial = Color::rgb(0x33, 0x66, 0x99);

        let (_timer, seen) = answer(&window, command(IDOK));
        let chosen = ColorDialog::new().initial(initial).show(&window).unwrap();
        assert!(seen.borrow().as_ref().expect("diálogo de cor").owner_blocked);
        assert_eq!(chosen, Some(initial), "OK devolve a cor inicial");

        let (_timer, _seen) = answer(&window, command(IDCANCEL));
        assert_eq!(ColorDialog::new().initial(initial).expanded(true).show(&window).unwrap(), None);

        let font = Font::new("Tahoma", 14.0);
        let (_timer, seen) = answer(&window, command(IDOK));
        let choice = FontDialog::new().initial(&font).color(initial).show(&window).unwrap();
        assert!(seen.borrow().is_some(), "diálogo de fonte");
        let choice = choice.expect("OK devolve a fonte");
        assert_eq!(choice.font.family(), "Tahoma");
        assert!((choice.font.size() - 14.0).abs() < 1.0, "tamanho: {}", choice.font.size());
        assert_eq!(choice.color, initial);

        let (_timer, _seen) = answer(&window, command(IDCANCEL));
        assert!(FontDialog::new().initial(&font).show(&window).unwrap().is_none());
        window.destroy();
    });
}

#[test]
fn print_and_page_setup_dialogs_cancel_cleanly() {
    with_app(|_| {
        let window = owner();
        // Sem impressora instalada (caso comum em máquinas de CI e no Wine) o sistema pode
        // recusar abrir o diálogo; com impressora, ele abre e é cancelado. Nos dois casos a
        // chamada volta sem travar e sem contexto de impressão.
        let (_timer, _seen) = answer(&window, command(IDCANCEL));
        match PrintDialog::new().show(&window) {
            Ok(context) => assert!(context.is_none()),
            Err(error) => assert!(matches!(error, GuiError::Win32 { .. } | GuiError::Unsupported(_)), "{error}"),
        }
        let (_timer, _seen) = answer(&window, command(IDCANCEL));
        match PageSetupDialog::new().margins(Thickness::all(20)).show(&window) {
            Ok(setup) => assert!(setup.is_none()),
            Err(error) => assert!(matches!(error, GuiError::Win32 { .. } | GuiError::Unsupported(_)), "{error}"),
        }
        assert!(window.is_enabled());
        window.destroy();
    });
}

#[test]
fn find_dialog_is_modeless_and_reports_searches() {
    with_app(|app| {
        let window = owner();
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        let dialog = FindDialog::find(&window, "agulha", move |event| log.borrow_mut().push(event.clone())).unwrap();
        assert!(dialog.is_open());
        assert!(pump_until(app, Duration::from_secs(3), || !thread_dialogs().is_empty()), "o diálogo apareceu");
        let hwnd = thread_dialogs()[0];
        assert!(child_texts(hwnd).iter().any(|t| t == "agulha"), "texto inicial: {:?}", child_texts(hwnd));
        assert!(window.is_enabled(), "localizar não é modal");

        // "Localizar próxima" (IDOK) chega como evento.
        // SAFETY: comando para o diálogo vivo desta thread.
        unsafe { SendMessageW(hwnd, WM_COMMAND, IDOK, 0) };
        assert!(pump_until(app, Duration::from_secs(2), || !events.borrow().is_empty()));
        let first = events.borrow()[0].clone();
        assert_eq!(first.action, FindAction::FindNext);
        assert_eq!(first.text, "agulha");

        dialog.close();
        assert!(pump_until(app, Duration::from_secs(2), || !dialog.is_open() && thread_dialogs().is_empty()));
        window.destroy();
    });
}

#[test]
fn message_box_and_task_dialog_answers() {
    with_app(|_| {
        let window = owner();

        let (_timer, seen) = answer(&window, command(IDYES));
        let result =
            MessageBox::show(&window, "Confirmar", "Apagar o arquivo?", MessageButtons::YesNo, MessageIcon::Question);
        assert_eq!(result, MessageResult::Yes);
        let seen = seen.borrow().clone().expect("caixa de mensagem");
        assert_eq!(seen.title, "Confirmar");
        assert!(seen.texts.iter().any(|t| t == "Apagar o arquivo?"), "{:?}", seen.texts);
        assert!(seen.owner_blocked);

        let (_timer, _seen) = answer(&window, command(IDNO));
        assert!(!MessageBox::confirm("Sair", "Deseja sair?"));
        let (_timer, _seen) = answer(&window, command(IDYES));
        assert!(MessageBox::confirm("Sair", "Deseja sair?"));

        // TaskDialog: botão personalizado e caixa de verificação.
        let (_timer, seen) = answer(&window, |dialog| {
            // SAFETY: mensagens do TaskDialog para o diálogo vivo desta thread.
            unsafe {
                SendMessageW(dialog, TDM_CLICK_VERIFICATION, 1, 0);
                PostMessageW(dialog, TDM_CLICK_BUTTON, 100, 0);
            }
        });
        let answer = TaskDialog::new()
            .title("Salvar alterações")
            .heading("Salvar relatório.txt?")
            .button(100, "&Salvar")
            .button(101, "&Não salvar")
            .verification("Não perguntar de novo", false)
            .show(&window);
        match answer {
            Ok(answer) => {
                assert_eq!(seen.borrow().as_ref().map(|s| s.title.clone()).as_deref(), Some("Salvar alterações"));
                assert_eq!(answer.button, TaskButton::Custom(100));
                assert!(answer.verification_checked);
            }
            // Sem Common Controls v6 não há TaskDialog.
            Err(error) => assert!(matches!(error, GuiError::Unsupported(_)), "{error}"),
        }
        window.destroy();
    });
}
