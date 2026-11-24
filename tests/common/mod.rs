//! Utilitários compartilhados pelos testes de integração.
//!
//! Estes testes criam janelas reais: precisam de uma sessão gráfica do Windows
//! (ou do Wine com um servidor X) para rodar.
#![allow(dead_code)]

use arqrustui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Cria a `Application` da thread de teste, executa o corpo e a encerra.
pub fn with_app(test: impl FnOnce(&Application)) {
    let app = Application::new().expect("Application::new");
    test(&app);
}

/// Serializa os testes que usam a área de transferência.
///
/// A área de transferência é uma só para o sistema inteiro, e o `cargo test` roda os
/// testes de um mesmo arquivo em paralelo: sem esta trava, um teste pode trocar o
/// conteúdo no meio da verificação de outro. Segure o valor devolvido até o fim do
/// teste.
pub fn clipboard_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // Um teste que falhou com a trava na mão não deve derrubar os outros.
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Janela oculta padrão para hospedar controles.
pub fn test_window() -> Window {
    Window::builder().title("Teste").size(400, 300).position(50, 50).build().expect("janela")
}

/// Processa mensagens até `done()` ser verdadeiro ou o tempo limite acabar.
pub fn pump_until(app: &Application, timeout: Duration, done: impl Fn() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        app.process_pending().expect("process_pending");
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    done()
}

/// Contador compartilhável entre o teste e os manipuladores de evento.
#[derive(Clone, Default)]
pub struct Counter(Rc<Cell<u32>>);

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hit(&self) {
        self.0.set(self.0.get() + 1);
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

/// Envia uma mensagem Win32 bruta a um widget (para simular entrada do usuário).
pub fn send(widget: &impl Widget, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: mensagens sem ponteiros enviadas a uma janela viva da própria thread de teste.
    unsafe { arqrustui::ffi::user32::SendMessageW(widget.handle() as _, message, wparam, lparam) }
}

/// Primeira janela filha de `widget` com a classe Win32 indicada (`"Edit"`, `"ComboBox"`...).
pub fn child_of_class(widget: &impl Widget, class: &str) -> Option<isize> {
    use arqrustui::ffi::user32::{GetClassNameW, GetWindow};
    const GW_CHILD: u32 = 5;
    const GW_HWNDNEXT: u32 = 2;
    // SAFETY: percorre as filhas de uma janela viva da thread de teste; o buffer é local.
    unsafe {
        let mut child = GetWindow(widget.handle() as _, GW_CHILD);
        while !child.is_null() {
            let mut buffer = [0u16; 64];
            let len = GetClassNameW(child, buffer.as_mut_ptr(), buffer.len() as i32);
            if String::from_utf16_lossy(&buffer[..len.max(0) as usize]).eq_ignore_ascii_case(class) {
                return Some(child as isize);
            }
            child = GetWindow(child, GW_HWNDNEXT);
        }
    }
    None
}

/// Troca o texto de uma janela nativa qualquer (por exemplo, o editor de uma célula).
pub fn set_native_text(hwnd: isize, text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: WM_SETTEXT com uma string UTF-16 terminada em zero, válida durante o envio.
    unsafe { arqrustui::ffi::user32::SendMessageW(hwnd as _, 0x000C, 0, wide.as_ptr() as isize) };
}

/// Envia uma mensagem Win32 bruta a uma janela nativa qualquer.
pub fn send_raw(hwnd: isize, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: mensagens sem ponteiros enviadas a uma janela viva da própria thread de teste.
    unsafe { arqrustui::ffi::user32::SendMessageW(hwnd as _, message, wparam, lparam) }
}

/// Retângulo de tela de uma janela nativa.
pub fn native_rect(hwnd: isize) -> arqrustui::ffi::RECT {
    let mut rect = arqrustui::ffi::RECT::default();
    // SAFETY: RECT local; janela viva da thread de teste.
    unsafe { arqrustui::ffi::user32::GetWindowRect(hwnd as _, &mut rect) };
    rect
}
