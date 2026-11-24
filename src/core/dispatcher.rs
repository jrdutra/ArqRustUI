//! UI-thread scheduling and `async` integration.

use super::application::{self as app, WM_DISPATCH};
use super::errors::{GuiError, GuiResult};
use crate::ffi::user32::PostMessageW;
use crate::ffi::HWND;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

type SendTask = Box<dyn FnOnce() + Send>;

/// Queue shared between UI and other threads.
pub(crate) struct DispatchShared {
    hwnd: AtomicIsize,
    tasks: Mutex<VecDeque<SendTask>>,
}

impl DispatchShared {
    pub(crate) fn new(hwnd: HWND) -> Arc<Self> {
        Arc::new(Self { hwnd: AtomicIsize::new(hwnd as isize), tasks: Mutex::new(VecDeque::new()) })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, VecDeque<SendTask>> {
        self.tasks.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn pop(&self) -> Option<SendTask> {
        self.lock().pop_front()
    }

    /// Marks the application stopped and discards pending tasks.
    pub(crate) fn shutdown(&self) {
        self.hwnd.store(0, Ordering::SeqCst);
        let pending = std::mem::take(&mut *self.lock());
        drop(pending);
    }
}

/// Dispatcher executing closures on the UI thread.
///
/// `Send + Sync + Clone`: transferable to any thread. The correct way to access windows/controls from workers.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(app: &Application, label: Label) {
/// let ui = app.dispatcher();
/// std::thread::spawn(move || {
///     let total: u64 = (1..=1_000_000u64).sum();
///     // `label` is not Send: use only inside the closure sent to the UI.
///     let _ = ui.post(move || println!("ready: {total}"));
/// });
/// # }
/// ```
#[derive(Clone)]
pub struct UiDispatcher {
    shared: Arc<DispatchShared>,
}

impl UiDispatcher {
    pub(crate) fn new(shared: Arc<DispatchShared>) -> Self {
        Self { shared }
    }

    /// Queues `task` for the UI thread.
    ///
    /// Returns [`GuiError::ApplicationGone`] if `Application` has shut down.
    pub fn post(&self, task: impl FnOnce() + Send + 'static) -> GuiResult<()> {
        let hwnd = self.shared.hwnd.load(Ordering::SeqCst);
        if hwnd == 0 {
            return Err(GuiError::ApplicationGone);
        }
        self.shared.lock().push_back(Box::new(task));
        // SAFETY: PostMessageW é seguro entre threads; se a janela já não existir a
        // chamada apenas falha.
        if unsafe { PostMessageW(hwnd as HWND, WM_DISPATCH, 0, 0) } == 0 {
            return Err(GuiError::ApplicationGone);
        }
        Ok(())
    }
}

struct TaskWaker {
    id: u64,
    dispatcher: UiDispatcher,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        let id = self.id;
        // Se a aplicação já terminou, a tarefa foi descartada junto com ela.
        let _ = self.dispatcher.post(move || poll_task(id));
    }
}

fn poll_task(id: u64) {
    let Some(state) = app::state() else { return };
    let Some(mut future) = state.tasks.borrow_mut().remove(&id) else { return };
    let waker = Waker::from(Arc::new(TaskWaker { id, dispatcher: UiDispatcher::new(state.shared.clone()) }));
    let mut context = Context::from_waker(&waker);
    if future.as_mut().poll(&mut context).is_pending() {
        state.tasks.borrow_mut().insert(id, future);
    }
}

/// Runs a `Future` on the UI thread, cooperating with the event loop.
///
/// The future need not be `Send` and may capture controls. Polled within the event loop without blocking while waiting.
/// Combine with [`run_blocking`] and [`sleep`].
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(label: Label) -> GuiResult<()> {
/// spawn_local(async move {
///     label.set_text("Calculating...");
///     let total = run_blocking(|| (1..=10_000_000u64).sum::<u64>()).await;
///     label.set_text(&format!("Total: {total}"));
/// })
/// # }
/// ```
pub fn spawn_local(future: impl Future<Output = ()> + 'static) -> GuiResult<()> {
    let state = app::require()?;
    let id = state.next_task.get();
    state.next_task.set(id + 1);
    state.tasks.borrow_mut().insert(id, Box::pin(future));
    app::defer(move || poll_task(id))
}

struct Shared<T> {
    value: Option<T>,
    waker: Option<Waker>,
}

/// Future completed by another thread or timer.
pub struct Completion<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

impl<T> Future for Completion<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut shared = self.shared.lock().unwrap_or_else(|p| p.into_inner());
        match shared.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                shared.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

fn complete<T>(shared: &Arc<Mutex<Shared<T>>>, value: T) {
    let waker = {
        let mut guard = shared.lock().unwrap_or_else(|p| p.into_inner());
        guard.value = Some(value);
        guard.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

/// Runs `work` on a worker thread and returns a result future.
///
/// Recommended for lengthy operations without freezing UI. If `work` panics, the future never completes (panic stays on
/// the worker thread).
pub fn run_blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Completion<T> {
    let shared = Arc::new(Mutex::new(Shared { value: None, waker: None }));
    let remote = shared.clone();
    std::thread::spawn(move || {
        let value = work();
        complete(&remote, value);
    });
    Completion { shared }
}

/// Future completing after `duration` without blocking UI.
pub fn sleep(duration: Duration) -> Completion<()> {
    let shared = Arc::new(Mutex::new(Shared { value: None, waker: None }));
    let remote = shared.clone();
    if super::timer::Timer::once(duration, move || complete(&remote, ())).is_err() {
        // Sem Application ativa não há como agendar: completa imediatamente.
        complete(&shared, ());
    }
    Completion { shared }
}

/// Runs `work` on a worker and delivers its result to `done` on the UI thread.
///
/// Callback version of [`run_blocking`]. `done` need not be `Send` and may capture controls.
pub fn spawn_background<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) + 'static,
) -> GuiResult<()> {
    spawn_local(async move {
        let value = run_blocking(work).await;
        done(value);
    })
}
