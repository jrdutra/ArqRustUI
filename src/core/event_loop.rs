//! Message loop: `GetMessageW` → shortcuts → keyboard navigation → dispatch.

use super::application::{self as app, AppState};
use super::errors::{GuiError, GuiResult};
use super::native;
use super::widget;
use crate::ffi::user32::*;
use crate::input::{Key, Modifiers};
use std::rc::Rc;

/// Handles a dequeued message.
fn process_message(state: &Rc<AppState>, msg: &MSG) {
    // Menus desenhados pela biblioteca (MenuStrip e menus de contexto) recebem o
    // teclado e o mouse antes de todos enquanto estão abertos.
    if crate::menus::popup::pre_translate(msg) {
        return;
    }
    // Diálogos não modais (Localizar/Substituir) precisam ver as teclas primeiro.
    let modeless: Vec<_> = state.modeless.borrow().clone();
    for dialog in modeless {
        // SAFETY: `msg` é uma mensagem válida recém-retirada da fila.
        if unsafe { IsDialogMessageW(dialog, msg) } != 0 {
            return;
        }
    }
    let root = native::root_of(msg.hwnd);
    let root_core = widget::lookup(root);
    if let Some(core) = &root_core {
        if msg.message == WM_KEYDOWN || msg.message == WM_SYSKEYDOWN {
            let key = Key(msg.wParam as u16);
            let modifiers = Modifiers::current();
            let action = core
                .shortcuts
                .borrow()
                .iter()
                .find(|(shortcut, _)| shortcut.key == key && shortcut.modifiers == modifiers)
                .map(|(_, action)| action.clone());
            if let Some(action) = action {
                action();
                return;
            }
            let bar = core.menu_bar.borrow().clone();
            let item = bar.and_then(|bar| bar.shortcut_item(crate::input::Shortcut { key, modifiers }));
            if let Some(id) = item {
                app::invoke_menu(id);
                return;
            }
        }
        // IsDialogMessageW implementa Tab, setas, mnemônicos, Enter e Esc.
        // SAFETY: `root` é uma janela desta thread e `msg` é válida.
        if unsafe { IsDialogMessageW(root, msg) } != 0 {
            return;
        }
    }
    // SAFETY: tradução e despacho padrão de uma mensagem válida.
    unsafe {
        TranslateMessage(msg);
        DispatchMessageW(msg);
    }
}

/// Runs until `done()` returns `true` or `WM_QUIT` arrives.
pub(crate) fn run_loop(state: &Rc<AppState>, done: impl Fn() -> bool) -> GuiResult<()> {
    struct Depth<'a>(&'a AppState);
    impl Drop for Depth<'_> {
        fn drop(&mut self) {
            self.0.loop_depth.set(self.0.loop_depth.get() - 1);
        }
    }
    state.loop_depth.set(state.loop_depth.get() + 1);
    let _depth = Depth(state);
    // SAFETY: MSG é uma estrutura simples para a qual zero é um valor válido.
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    loop {
        if done() {
            return Ok(());
        }
        // SAFETY: ponteiro válido para MSG; HWND nulo recebe mensagens de toda a thread.
        let result = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
        if result == 0 {
            if state.loop_depth.get() > 1 {
                // Um loop aninhado (janela modal) repassa o pedido de saída ao externo.
                // SAFETY: reposta WM_QUIT na fila da thread atual.
                unsafe { PostQuitMessage(msg.wParam as i32) };
            }
            return Ok(());
        }
        if result == -1 {
            return Err(GuiError::last_os_error("GetMessageW"));
        }
        process_message(state, &msg);
        app::rethrow_panic();
    }
}

/// Processes pending messages without blocking.
pub(crate) fn pump_pending(state: &Rc<AppState>) -> GuiResult<()> {
    // SAFETY: MSG é uma estrutura simples para a qual zero é um valor válido.
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    // SAFETY: ponteiro válido para MSG.
    while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
        if msg.message == WM_QUIT {
            // SAFETY: devolve o WM_QUIT para o loop principal tratar.
            unsafe { PostQuitMessage(msg.wParam as i32) };
            break;
        }
        process_message(state, &msg);
        app::rethrow_panic();
    }
    Ok(())
}
