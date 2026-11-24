//! `SetTimer` timers delivered on the UI thread.

use super::application::{self as app, AppState, TimerEntry};
use super::errors::{GuiError, GuiResult};
use crate::ffi::user32::{KillTimer, SetTimer};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

pub(crate) fn kill(state: &Rc<AppState>, id: usize) {
    // SAFETY: remove um temporizador criado por esta thread na janela interna.
    unsafe { KillTimer(state.dispatch_hwnd, id) };
    let removed = state.timers.borrow_mut().remove(&id);
    drop(removed);
}

fn start(period: Duration, once: bool, callback: Rc<dyn Fn()>) -> GuiResult<usize> {
    let state = app::require()?;
    let id = state.next_timer.get();
    state.next_timer.set(id + 1);
    let millis = period.as_millis().clamp(1, u32::MAX as u128) as u32;
    // SAFETY: cria um temporizador na janela interna desta thread, sem callback nativo.
    if unsafe { SetTimer(state.dispatch_hwnd, id, millis, std::ptr::null()) } == 0 {
        return Err(GuiError::last_os_error("SetTimer"));
    }
    state.timers.borrow_mut().insert(id, TimerEntry { callback, once });
    Ok(id)
}

/// Periodic timer stopped on drop.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # use std::time::Duration;
/// # fn demo(label: Label) -> GuiResult<()> {
/// let timer = Timer::interval(Duration::from_secs(1), move || label.set_text("tick"))?;
/// // Keep `timer` alive to receive ticks.
/// # drop(timer); Ok(()) }
/// ```
#[derive(Debug)]
pub struct Timer {
    id: usize,
    _not_send: std::marker::PhantomData<Rc<()>>,
}

impl Timer {
    /// Starts a timer calling `callback` every `period`.
    pub fn interval(period: Duration, callback: impl Fn() + 'static) -> GuiResult<Timer> {
        let id = start(period, false, Rc::new(callback))?;
        Ok(Timer { id, _not_send: std::marker::PhantomData })
    }

    /// Calls `callback` once after `delay`; no handle needs to be retained.
    pub fn once(delay: Duration, callback: impl FnOnce() + 'static) -> GuiResult<()> {
        let slot = Cell::new(Some(callback));
        start(
            delay,
            true,
            Rc::new(move || {
                if let Some(callback) = slot.take() {
                    callback();
                }
            }),
        )
        .map(|_| ())
    }

    /// Stops the timer, equivalent to dropping it.
    pub fn stop(self) {}
}

impl Drop for Timer {
    fn drop(&mut self) {
        if let Some(state) = app::state() {
            kill(&state, self.id);
        }
    }
}
