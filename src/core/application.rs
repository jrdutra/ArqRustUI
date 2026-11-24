//! Per-UI-thread application state and [`Application`].

use super::dispatcher::{DispatchShared, UiDispatcher};
use super::errors::{GuiError, GuiResult};
use super::event_loop;
use super::handles::GdiObject;
use super::native;
use super::widget;
use super::window;
use crate::ffi::comctl32;
use crate::ffi::dynamic::optional;
use crate::ffi::gdi32::CreateSolidBrush;
use crate::ffi::kernel32::*;
use crate::ffi::ole32::{OleInitialize, OleUninitialize};
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::utilities::{to_wide, Color};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

pub(crate) const WM_DISPATCH: UINT = WM_APP + 1;
pub(crate) const WM_TRAY: UINT = WM_APP + 2;
const DISPATCH_CLASS: &str = "ArqRustUI.Dispatch";

pub(crate) struct TimerEntry {
    pub callback: Rc<dyn Fn()>,
    pub once: bool,
}

type MessageHandler = Rc<dyn Fn(WPARAM, LPARAM)>;

pub(crate) type LocalTask = Pin<Box<dyn Future<Output = ()>>>;

/// Shared UI thread state.
pub(crate) struct AppState {
    pub dispatch_hwnd: HWND,
    pub shared: Arc<DispatchShared>,
    pub local_queue: RefCell<VecDeque<Box<dyn FnOnce()>>>,
    pub top_levels: Cell<usize>,
    pub quit_on_last: Cell<bool>,
    pub loop_depth: Cell<u32>,
    next_control_id: Cell<u16>,
    next_menu_id: Cell<u32>,
    pub menu_handlers: RefCell<HashMap<u32, Rc<dyn Fn()>>>,
    pub timers: RefCell<HashMap<usize, TimerEntry>>,
    pub next_timer: Cell<usize>,
    brushes: RefCell<HashMap<u32, GdiObject>>,
    panic: RefCell<Option<Box<dyn Any + Send>>>,
    pub modeless: RefCell<Vec<HWND>>,
    pub tasks: RefCell<HashMap<u64, LocalTask>>,
    pub next_task: Cell<u64>,
    pub message_handlers: RefCell<HashMap<(UINT, usize), MessageHandler>>,
    activation_cookie: Cell<usize>,
}

thread_local! {
    static APP: RefCell<Option<Rc<AppState>>> = const { RefCell::new(None) };
}

/// Current thread application state, if any.
pub(crate) fn state() -> Option<Rc<AppState>> {
    APP.with(|a| a.borrow().clone())
}

/// Requires an active `Application` on this thread.
pub(crate) fn require() -> GuiResult<Rc<AppState>> {
    state().ok_or(GuiError::NotInitialized)
}

pub(crate) fn next_control_id() -> u16 {
    state()
        .map(|app| {
            let id = app.next_control_id.get();
            app.next_control_id.set(if id >= 0xFFF0 { 100 } else { id + 1 });
            id
        })
        .unwrap_or(100)
}

/// Reserves a menu command identifier and attaches its handler.
pub(crate) fn register_menu_handler(handler: Rc<dyn Fn()>) -> GuiResult<u32> {
    let app = require()?;
    let mut handlers = app.menu_handlers.borrow_mut();
    // Identificadores de menu precisam caber em 16 bits e ficar abaixo de 0xF000
    // (faixa reservada aos comandos de sistema).
    for _ in 0..0xE000 {
        let id = app.next_menu_id.get();
        app.next_menu_id.set(if id >= 0xEFFF { 0x1000 } else { id + 1 });
        if let std::collections::hash_map::Entry::Vacant(slot) = handlers.entry(id) {
            slot.insert(handler);
            return Ok(id);
        }
    }
    Err(GuiError::InvalidArgument("menu identifiers exhausted".into()))
}

pub(crate) fn unregister_menu_handler(id: u32) {
    if let Some(app) = state() {
        let removed = app.menu_handlers.borrow_mut().remove(&id);
        drop(removed);
    }
}

/// Executes menu command `id`'s handler, if any.
pub(crate) fn invoke_menu(id: u32) -> bool {
    let handler = state().and_then(|app| app.menu_handlers.borrow().get(&id).cloned());
    match handler {
        Some(handler) => {
            handler();
            true
        }
        None => false,
    }
}

/// Cached solid brush for the color, lasting until application shutdown.
pub(crate) fn brush_for(color: Color) -> HBRUSH {
    let Some(app) = state() else {
        return std::ptr::null_mut();
    };
    let key = color.to_colorref();
    let mut brushes = app.brushes.borrow_mut();
    if let Some(brush) = brushes.get(&key) {
        return brush.raw();
    }
    // Limita o cache: aplicações que animam cores não acumulam pincéis indefinidamente.
    // Os pincéis antigos não estão mais em uso, pois cada um só é utilizado durante a
    // mensagem de pintura que o solicitou.
    if brushes.len() >= 512 {
        brushes.clear();
    }
    // SAFETY: cria um pincel GDI cuja posse passa ao cache.
    match unsafe { GdiObject::from_raw(CreateSolidBrush(key)) } {
        Some(brush) => {
            let raw = brush.raw();
            brushes.insert(key, brush);
            raw
        }
        None => std::ptr::null_mut(),
    }
}

pub(crate) fn store_panic(payload: Box<dyn Any + Send>) {
    if let Some(app) = state() {
        let mut slot = app.panic.borrow_mut();
        if slot.is_none() {
            *slot = Some(payload);
        }
        // SAFETY: posta uma mensagem nula apenas para acordar o loop de eventos.
        unsafe { PostMessageW(app.dispatch_hwnd, WM_NULL, 0, 0) };
    }
}

/// Resumes a handler panic outside system callbacks.
pub(crate) fn rethrow_panic() {
    let payload = state().and_then(|app| app.panic.borrow_mut().take());
    if let Some(payload) = payload {
        std::panic::resume_unwind(payload);
    }
}

pub(crate) fn top_level_created() {
    if let Some(app) = state() {
        app.top_levels.set(app.top_levels.get() + 1);
    }
}

pub(crate) fn top_level_closed() {
    if let Some(app) = state() {
        let remaining = app.top_levels.get().saturating_sub(1);
        app.top_levels.set(remaining);
        // SAFETY: mensagens postadas para a própria thread; acordam loops aninhados.
        unsafe {
            PostMessageW(app.dispatch_hwnd, WM_NULL, 0, 0);
            if remaining == 0 && app.quit_on_last.get() && app.loop_depth.get() > 0 {
                PostQuitMessage(0);
            }
        }
    }
}

/// Registers a message/`WPARAM` handler on the internal window.
pub(crate) fn register_message(msg: UINT, key: usize, handler: Rc<dyn Fn(WPARAM, LPARAM)>) {
    if let Some(app) = state() {
        app.message_handlers.borrow_mut().insert((msg, key), handler);
    }
}

pub(crate) fn unregister_message(msg: UINT, key: usize) {
    if let Some(app) = state() {
        let removed = app.message_handlers.borrow_mut().remove(&(msg, key));
        drop(removed);
    }
}

/// Schedules a closure for the next UI event loop iteration.
///
/// UI thread only; other threads must use [`UiDispatcher::post`].
pub fn defer(f: impl FnOnce() + 'static) -> GuiResult<()> {
    let app = require()?;
    app.local_queue.borrow_mut().push_back(Box::new(f));
    // SAFETY: posta para a janela interna desta thread.
    if unsafe { PostMessageW(app.dispatch_hwnd, WM_DISPATCH, 0, 0) } == 0 {
        return Err(GuiError::last_os_error("PostMessageW"));
    }
    Ok(())
}

fn run_dispatch_queues(app: &Rc<AppState>) {
    loop {
        let task = app.shared.pop();
        match task {
            Some(task) => task(),
            None => break,
        }
    }
    loop {
        let task = app.local_queue.borrow_mut().pop_front();
        match task {
            Some(task) => task(),
            None => break,
        }
    }
}

unsafe extern "system" fn dispatch_proc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let Some(app) = state() else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| match msg {
        WM_DISPATCH => {
            run_dispatch_queues(&app);
            true
        }
        WM_TIMER => {
            let entry = app.timers.borrow().get(&wp).map(|t| (t.callback.clone(), t.once));
            if let Some((callback, once)) = entry {
                if once {
                    super::timer::kill(&app, wp);
                }
                callback();
            }
            true
        }
        _ => {
            let handler = {
                let handlers = app.message_handlers.borrow();
                handlers.get(&(msg, wp)).or_else(|| handlers.get(&(msg, usize::MAX))).cloned()
            };
            match handler {
                Some(handler) => {
                    handler(wp, lp);
                    true
                }
                None => false,
            }
        }
    }));
    match outcome {
        Ok(true) => 0,
        Ok(false) => DefWindowProcW(hwnd, msg, wp, lp),
        Err(payload) => {
            store_panic(payload);
            0
        }
    }
}

/// Once-per-process initialization: DPI and visual style context.
fn process_init() -> usize {
    static ACT_CTX: OnceLock<usize> = OnceLock::new();
    *ACT_CTX.get_or_init(|| {
        // SAFETY: chamadas de configuração do processo, sem ponteiros; falham de forma
        // inofensiva se o processo já tiver sido configurado por um manifesto.
        unsafe {
            const PER_MONITOR_AWARE_V2: isize = -4;
            let done =
                optional().SetProcessDpiAwarenessContext.map(|set| set(PER_MONITOR_AWARE_V2) != 0).unwrap_or(false);
            if !done {
                SetProcessDPIAware();
            }
        }
        // O manifesto de Common Controls v6 embutido em shell32.dll (recurso 124) é
        // usado para ativar os estilos visuais sem exigir manifesto na aplicação.
        let mut directory = [0u16; 260];
        // SAFETY: buffer do tamanho informado.
        let len = unsafe { GetSystemDirectoryW(directory.as_mut_ptr(), directory.len() as u32) };
        if len == 0 || len as usize >= directory.len() {
            return 0;
        }
        let source = to_wide("shell32.dll");
        let context = ACTCTXW {
            cbSize: std::mem::size_of::<ACTCTXW>() as u32,
            dwFlags: ACTCTX_FLAG_ASSEMBLY_DIRECTORY_VALID | ACTCTX_FLAG_RESOURCE_NAME_VALID,
            lpSource: source.as_ptr(),
            wProcessorArchitecture: 0,
            wLangId: 0,
            lpAssemblyDirectory: directory.as_ptr(),
            lpResourceName: 124 as PCWSTR,
            lpApplicationName: std::ptr::null(),
            hModule: std::ptr::null_mut(),
        };
        // SAFETY: estrutura completa e strings válidas durante a chamada.
        let handle = unsafe { CreateActCtxW(&context) };
        if handle != INVALID_HANDLE_VALUE {
            if provides_common_controls_v6(handle) {
                return handle as usize;
            }
            // SAFETY: contexto criado acima e não usado em mais nenhum lugar.
            unsafe { ReleaseActCtx(handle) };
        }
        // Sem o recurso, ou com um recurso que não traz a v6 (Wine, sistemas reduzidos):
        // o mesmo pedido, em um manifesto gravado na pasta temporária.
        manifest_file_context().filter(|&handle| provides_common_controls_v6(handle as HANDLE)).unwrap_or(0)
    })
}

/// `true` if controls created in the active context use Common Controls v6. Verified with a hidden button responding to
/// v6-only `BCM_GETIDEALSIZE`.
fn provides_common_controls_v6(context: HANDLE) -> bool {
    let mut cookie = 0usize;
    // SAFETY: ativação temporária desfeita no fim; a janela de prova é destruída antes
    // de sair; o SIZE é local e válido durante o envio síncrono.
    unsafe {
        if ActivateActCtx(context, &mut cookie) == 0 {
            return false;
        }
        let class = to_wide("BUTTON");
        let probe = CreateWindowExW(
            0,
            class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            0,
            0,
            10,
            10,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            native::instance(),
            std::ptr::null_mut(),
        );
        let mut ideal = SIZE { cx: 0, cy: 0 };
        let v6 = !probe.is_null() && SendMessageW(probe, BCM_GETIDEALSIZE, 0, &mut ideal as *mut SIZE as LPARAM) != 0;
        if !probe.is_null() {
            DestroyWindow(probe);
        }
        DeactivateActCtx(0, cookie);
        v6
    }
}

/// Minimal Common Controls v6 manifest.
const COMMON_CONTROLS_V6_MANIFEST: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">"#,
    r#"<dependency><dependentAssembly><assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" "#,
    r#"version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>"#,
    r#"</dependentAssembly></dependency></assembly>"#
);

/// Creates an activation context from a temporary manifest file.
fn manifest_file_context() -> Option<usize> {
    let path = std::env::temp_dir().join("arqrustui-common-controls-v6.manifest");
    if std::fs::read_to_string(&path).ok().as_deref() != Some(COMMON_CONTROLS_V6_MANIFEST) {
        std::fs::write(&path, COMMON_CONTROLS_V6_MANIFEST).ok()?;
    }
    let source = to_wide(path.to_str()?);
    let context = ACTCTXW {
        cbSize: std::mem::size_of::<ACTCTXW>() as u32,
        dwFlags: 0,
        lpSource: source.as_ptr(),
        wProcessorArchitecture: 0,
        wLangId: 0,
        lpAssemblyDirectory: std::ptr::null(),
        lpResourceName: std::ptr::null(),
        lpApplicationName: std::ptr::null(),
        hModule: std::ptr::null_mut(),
    };
    // SAFETY: estrutura completa e caminho válido durante a chamada.
    let handle = unsafe { CreateActCtxW(&context) };
    (handle != INVALID_HANDLE_VALUE).then_some(handle as usize)
}

fn register_dispatch_class() -> GuiResult<()> {
    let name = to_wide(DISPATCH_CLASS);
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(dispatch_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: native::instance(),
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: name.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };
    // SAFETY: estrutura completa; o nome vive durante a chamada.
    if unsafe { RegisterClassExW(&class) } == 0 {
        // SAFETY: leitura do código de erro da chamada anterior.
        let code = unsafe { GetLastError() };
        if code != ERROR_CLASS_ALREADY_EXISTS {
            return Err(GuiError::from_win32("RegisterClassExW", code));
        }
    }
    Ok(())
}

/// Library entry point: initializes the UI thread and runs the event loop.
///
/// Create **one** `Application` per UI thread before any window. It is neither `Send` nor `Sync`: windows/controls
/// belong to their creating thread. Use [`Application::dispatcher`] from other threads.
///
/// On drop, destroys remaining windows and releases thread resources.
///
/// ```no_run
/// use arqrustui::prelude::*;
///
/// fn main() -> GuiResult<()> {
///     let app = Application::new()?;
///     let window = Window::builder().title("Example").size(400, 300).centered().build()?;
///     window.show();
///     app.run()
/// }
/// ```
pub struct Application {
    state: Rc<AppState>,
}

impl Application {
    /// Initializes the library on the current thread.
    ///
    /// Sets per-monitor DPI, visual styles (Common Controls v6), OLE/COM and the internal dispatch window. Returns
    /// [`GuiError::AlreadyInitialized`] if this thread already has an `Application`.
    pub fn new() -> GuiResult<Application> {
        if state().is_some() {
            return Err(GuiError::AlreadyInitialized);
        }
        let context = process_init();
        let mut cookie = 0usize;
        if context != 0 {
            // SAFETY: `context` é um handle de contexto de ativação criado em process_init.
            unsafe { ActivateActCtx(context as HANDLE, &mut cookie) };
        }
        if let Some(api) = comctl32::functions() {
            let init = comctl32::INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<comctl32::INITCOMMONCONTROLSEX>() as u32,
                dwICC: comctl32::ICC_ALL_CLASSES,
            };
            // SAFETY: estrutura local corretamente dimensionada.
            unsafe { (api.InitCommonControlsEx)(&init) };
        }
        // SAFETY: inicializa OLE (apartamento STA) nesta thread; pareado em Drop.
        let hr = unsafe { OleInitialize(std::ptr::null_mut()) };
        GuiError::check_hresult("OleInitialize", hr)?;
        let setup = || -> GuiResult<HWND> {
            register_dispatch_class()?;
            window::register_classes()?;
            let class = to_wide(DISPATCH_CLASS);
            // SAFETY: cria uma janela somente-mensagem com a classe recém-registrada.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    std::ptr::null(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    std::ptr::null_mut(),
                    native::instance(),
                    std::ptr::null_mut(),
                )
            };
            if hwnd.is_null() {
                Err(GuiError::last_os_error("CreateWindowExW(despacho)"))
            } else {
                Ok(hwnd)
            }
        };
        let dispatch_hwnd = match setup() {
            Ok(hwnd) => hwnd,
            Err(err) => {
                // SAFETY: desfaz a inicialização parcial feita acima.
                unsafe {
                    OleUninitialize();
                    if cookie != 0 {
                        DeactivateActCtx(0, cookie);
                    }
                }
                return Err(err);
            }
        };
        let state = Rc::new(AppState {
            dispatch_hwnd,
            shared: DispatchShared::new(dispatch_hwnd),
            local_queue: RefCell::new(VecDeque::new()),
            top_levels: Cell::new(0),
            quit_on_last: Cell::new(true),
            loop_depth: Cell::new(0),
            next_control_id: Cell::new(100),
            next_menu_id: Cell::new(0x1000),
            menu_handlers: RefCell::new(HashMap::new()),
            timers: RefCell::new(HashMap::new()),
            next_timer: Cell::new(1),
            brushes: RefCell::new(HashMap::new()),
            panic: RefCell::new(None),
            modeless: RefCell::new(Vec::new()),
            tasks: RefCell::new(HashMap::new()),
            next_task: Cell::new(1),
            message_handlers: RefCell::new(HashMap::new()),
            activation_cookie: Cell::new(cookie),
        });
        APP.with(|a| *a.borrow_mut() = Some(state.clone()));
        Ok(Application { state })
    }

    /// Runs the event loop until application shutdown.
    ///
    /// By default exits when the last top-level window closes (see
    /// [`set_quit_on_last_window_closed`](Self::set_quit_on_last_window_closed)) or [`Application::quit`] is called.
    /// Propagates panics from event handlers.
    pub fn run(&self) -> GuiResult<()> {
        if self.state.top_levels.get() == 0 && self.state.quit_on_last.get() {
            return Ok(());
        }
        event_loop::run_loop(&self.state, || false)
    }

    /// Processes pending messages without blocking and returns.
    ///
    /// Useful in tests and to keep UI responsive during lengthy UI-thread work.
    pub fn process_pending(&self) -> GuiResult<()> {
        event_loop::pump_pending(&self.state)
    }

    /// Requests termination of the current thread's event loop.
    pub fn quit() {
        // SAFETY: posta WM_QUIT para a fila da thread atual.
        unsafe { PostQuitMessage(0) };
    }

    /// Sets whether closing the last window quits (default `true`).
    ///
    /// Disable for applications that remain alive with only a tray icon.
    pub fn set_quit_on_last_window_closed(&self, quit: bool) {
        self.state.quit_on_last.set(quit);
    }

    /// Returns a `Send + Sync` dispatcher for UI-thread closures.
    pub fn dispatcher(&self) -> UiDispatcher {
        UiDispatcher::new(self.state.shared.clone())
    }

    /// Open top-level window count on this thread.
    pub fn window_count(&self) -> usize {
        self.state.top_levels.get()
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        self.state.quit_on_last.set(false);
        for core in widget::all_cores() {
            if core.is_top_level() {
                core.destroy();
            }
        }
        for core in widget::all_cores() {
            core.destroy();
        }
        self.state.shared.shutdown();
        let timers: Vec<usize> = self.state.timers.borrow().keys().copied().collect();
        for id in timers {
            super::timer::kill(&self.state, id);
        }
        // Esvazia a fila de mensagens (inclusive um WM_QUIT pendente), para que uma
        // nova Application nesta thread comece limpa.
        // SAFETY: MSG local preenchido por PeekMessageW; destruição da janela interna
        // desta thread.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {}
            DestroyWindow(self.state.dispatch_hwnd);
        }
        let tasks = std::mem::take(&mut *self.state.tasks.borrow_mut());
        let local = std::mem::take(&mut *self.state.local_queue.borrow_mut());
        let menus = std::mem::take(&mut *self.state.menu_handlers.borrow_mut());
        let messages = std::mem::take(&mut *self.state.message_handlers.borrow_mut());
        drop((tasks, local, menus, messages));
        self.state.brushes.borrow_mut().clear();
        APP.with(|a| *a.borrow_mut() = None);
        // SAFETY: pares de OleInitialize e ActivateActCtx feitos em `new`.
        unsafe {
            OleUninitialize();
            let cookie = self.state.activation_cookie.get();
            if cookie != 0 {
                DeactivateActCtx(0, cookie);
            }
        }
    }
}
