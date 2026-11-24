//! Animations/transitions: [`Easing`] curves, value animations ([`animate`]) driven by a single UI-thread timer, and
//! ready-made control/window transitions ([`animate_bounds`], [`fade_window`], [`show_animated`], [`hide_animated`]).
//!
//! When Windows "Show animations" is off, or [`set_animations_enabled`]`(Some(false))`, animations jump to their final
//! value on the first frame.

use super::application as app;
use super::errors::{GuiError, GuiResult};
use super::timer::Timer;
use super::widget::Widget;
use super::window::Window;
use crate::utilities::{Color, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Frame interval (~60 Hz).
const FRAME: Duration = Duration::from_millis(15);

/// Easing curve transforming linear progress (0 → 1) to displayed progress.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum Easing {
    /// Constant speed.
    Linear,
    /// Starts slowly (cubic).
    EaseIn,
    /// Ends slowly (cubic), the default, suitable for entering elements.
    #[default]
    EaseOut,
    /// Starts and ends slowly (cubic).
    EaseInOut,
    /// Overshoots and returns, like a damped spring.
    Overshoot,
    /// Cubic Bézier `(x1, y1, x2, y2)`, like CSS `cubic-bezier()`.
    CubicBezier(f64, f64, f64, f64),
}

impl Easing {
    /// Displayed progress for linear `t`, clamped to 0..=1.
    pub fn apply(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Easing::Linear => t,
            Easing::EaseIn => t * t * t,
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::Overshoot => {
                let (c1, c3) = (1.70158, 2.70158);
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Easing::CubicBezier(x1, y1, x2, y2) => cubic_bezier(x1, y1, x2, y2, t),
        }
    }
}

/// Solves `x(s) = t` using Newton/bisection and returns `y(s)`.
fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, t: f64) -> f64 {
    let curve = |a: f64, b: f64, s: f64| 3.0 * a * s * (1.0 - s).powi(2) + 3.0 * b * s * s * (1.0 - s) + s * s * s;
    let derivative =
        |a: f64, b: f64, s: f64| 3.0 * a * (1.0 - s).powi(2) + 6.0 * (b - a) * s * (1.0 - s) + 3.0 * (1.0 - b) * s * s;
    let (x1, x2) = (x1.clamp(0.0, 1.0), x2.clamp(0.0, 1.0));
    let mut s = t;
    for _ in 0..8 {
        let error = curve(x1, x2, s) - t;
        let slope = derivative(x1, x2, s);
        if error.abs() < 1e-7 {
            return curve(y1, y2, s);
        }
        if slope.abs() < 1e-6 {
            break;
        }
        s = (s - error / slope).clamp(0.0, 1.0);
    }
    let (mut low, mut high) = (0.0, 1.0);
    s = t;
    for _ in 0..40 {
        let x = curve(x1, x2, s);
        if (x - t).abs() < 1e-7 {
            break;
        }
        if x < t {
            low = s;
        } else {
            high = s;
        }
        s = (low + high) / 2.0;
    }
    curve(y1, y2, s)
}

/// Linear interpolation between `a` and `b`.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Intermediate rectangle, rounded to integers.
pub fn lerp_rect(a: Rect, b: Rect, t: f64) -> Rect {
    let at = |x: i32, y: i32| lerp(x as f64, y as f64, t).round() as i32;
    Rect::new(at(a.x, b.x), at(a.y, b.y), at(a.width, b.width), at(a.height, b.height))
}

/// Intermediate color (channels and alpha).
pub fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    let at = |x: u8, y: u8| lerp(x as f64, y as f64, t).round().clamp(0.0, 255.0) as u8;
    Color { r: at(a.r, b.r), g: at(a.g, b.g), b: at(a.b, b.b), a: at(a.a, b.a) }
}

type FrameFn = Rc<dyn Fn(f64)>;
type DoneFn = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

struct Running {
    id: u64,
    started: Instant,
    duration: Duration,
    easing: Easing,
    frame: FrameFn,
    done: DoneFn,
    finished: Rc<Cell<bool>>,
}

thread_local! {
    static RUNNING: RefCell<Vec<Rc<Running>>> = const { RefCell::new(Vec::new()) };
    static DRIVER: RefCell<Option<Timer>> = const { RefCell::new(None) };
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    static ENABLED: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Whether animations are enabled: [`set_animations_enabled`] override or Windows "Show animations"
/// (`SPI_GETCLIENTAREAANIMATION`).
pub fn animations_enabled() -> bool {
    if let Some(forced) = ENABLED.with(Cell::get) {
        return forced;
    }
    const SPI_GETCLIENTAREAANIMATION: u32 = 0x1042;
    let mut enabled: i32 = 1;
    // SAFETY: consulta de um BOOL do sistema para uma variável local.
    let ok = unsafe {
        crate::ffi::user32::SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut enabled as *mut i32 as *mut _, 0)
    };
    ok == 0 || enabled != 0
}

/// Forces thread animations on (`Some(true)`), off (`Some(false)`, jumps to end) or follows Windows (`None`).
pub fn set_animations_enabled(enabled: Option<bool>) {
    ENABLED.with(|e| e.set(enabled));
}

/// Running animation count on this thread.
pub fn running_animations() -> usize {
    RUNNING.with(|r| r.borrow().len())
}

/// Running animation. Dropping does **not** stop it; use [`cancel`](Self::cancel) or [`finish`](Self::finish).
#[derive(Clone)]
pub struct Transition {
    id: u64,
    finished: Rc<Cell<bool>>,
    done: DoneFn,
}

impl std::fmt::Debug for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transition").field("id", &self.id).field("running", &self.is_running()).finish()
    }
}

impl Transition {
    /// `true` while neither completed nor canceled.
    pub fn is_running(&self) -> bool {
        !self.finished.get()
    }

    /// Called once on normal completion or [`finish`](Self::finish), never on cancellation. Called immediately if
    /// already finished.
    pub fn on_finished(&self, f: impl FnOnce() + 'static) {
        if self.finished.get() {
            f();
        } else {
            *self.done.borrow_mut() = Some(Box::new(f));
        }
    }

    /// Former name of [`on_finished`](Self::on_finished), retained for existing code.
    #[deprecated(since = "0.1.0", note = "use `on_finished`: events follow the naming convention in docs/EVENTS.md")]
    pub fn on_done(&self, f: impl FnOnce() + 'static) {
        self.on_finished(f)
    }

    /// Stops in place without final frame or `on_done`.
    pub fn cancel(&self) {
        if !self.finished.replace(true) {
            self.done.borrow_mut().take();
            remove(self.id);
        }
    }

    /// Jumps to end, delivering final frame (1.0) and calling `on_done`.
    pub fn finish(&self) {
        let running = RUNNING.with(|r| r.borrow().iter().find(|a| a.id == self.id).cloned());
        if let Some(running) = running {
            complete(&running);
        }
    }
}

fn remove(id: u64) {
    let empty = RUNNING.with(|r| {
        let mut list = r.borrow_mut();
        list.retain(|a| a.id != id);
        list.is_empty()
    });
    if empty {
        // O temporizador é solto fora do empréstimo da lista.
        let timer = DRIVER.with(|d| d.borrow_mut().take());
        drop(timer);
    }
}

fn complete(running: &Rc<Running>) {
    if running.finished.replace(true) {
        return;
    }
    remove(running.id);
    (running.frame)(running.easing.apply(1.0));
    let done = running.done.borrow_mut().take();
    if let Some(done) = done {
        done();
    }
}

fn tick() {
    let list: Vec<Rc<Running>> = RUNNING.with(|r| r.borrow().clone());
    let enabled = animations_enabled();
    for running in list {
        if running.finished.get() {
            continue;
        }
        let total = running.duration.as_secs_f64();
        let progress = if !enabled || total <= 0.0 { 1.0 } else { running.started.elapsed().as_secs_f64() / total };
        if progress >= 1.0 {
            complete(&running);
        } else {
            (running.frame)(running.easing.apply(progress));
        }
    }
}

/// Animates 0 to 1 over `duration`; `on_frame` receives `easing`-adjusted progress each frame, always ending with the
/// curve's final value.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # use std::time::Duration;
/// # fn demo(bar: ProgressBar) -> GuiResult<()> {
/// let transition = motion::animate(Duration::from_millis(300), Easing::EaseOut, move |t| {
///     bar.set_value((t * 100.0) as i32);
/// })?;
/// transition.on_finished(|| println!("ready"));
/// # Ok(()) }
/// ```
pub fn animate(duration: Duration, easing: Easing, on_frame: impl Fn(f64) + 'static) -> GuiResult<Transition> {
    app::require()?;
    let id = NEXT_ID.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    });
    let finished = Rc::new(Cell::new(false));
    let done: DoneFn = Rc::new(RefCell::new(None));
    let running = Rc::new(Running {
        id,
        started: Instant::now(),
        duration,
        easing,
        frame: Rc::new(on_frame),
        done: done.clone(),
        finished: finished.clone(),
    });
    RUNNING.with(|r| r.borrow_mut().push(running));
    let needs_driver = DRIVER.with(|d| d.borrow().is_none());
    if needs_driver {
        let timer = Timer::interval(FRAME, tick)?;
        DRIVER.with(|d| *d.borrow_mut() = Some(timer));
    }
    Ok(Transition { id, finished, done })
}

/// Animates `from` to `to`, delivering intermediate values.
pub fn animate_value(
    from: f64,
    to: f64,
    duration: Duration,
    easing: Easing,
    on_value: impl Fn(f64) + 'static,
) -> GuiResult<Transition> {
    animate(duration, easing, move |t| on_value(lerp(from, to, t)))
}

/// Moves/resizes a control/window to logical `to`.
pub fn animate_bounds(widget: &impl Widget, to: Rect, duration: Duration, easing: Easing) -> GuiResult<Transition> {
    let core = widget.core().clone();
    core.require_alive()?;
    let from = widget.bounds();
    let target = Rc::downgrade(&core);
    animate(duration, easing, move |t| {
        if let Some(core) = target.upgrade() {
            if core.is_alive() {
                core.set_bounds(lerp_rect(from, to, t));
            }
        }
    })
}

/// Fades a top-level window between opacities (0.0 to 1.0).
pub fn fade_window(window: &Window, from: f32, to: f32, duration: Duration) -> GuiResult<Transition> {
    window.core().require_alive()?;
    let target = window.clone();
    window.set_opacity(from);
    animate(duration, Easing::Linear, move |t| {
        if target.core().is_alive() {
            target.set_opacity(lerp(from as f64, to as f64, t) as f32);
        }
    })
}

/// [`show_animated`]/[`hide_animated`] effect (`AnimateWindow`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShowEffect {
    /// Fade (top-level windows only).
    Fade,
    /// Slide top to bottom.
    SlideDown,
    /// Slide bottom to top.
    SlideUp,
    /// Slide left to right.
    SlideRight,
    /// Slide right to left.
    SlideLeft,
    /// Grow from center.
    Center,
}

fn animate_window(widget: &impl Widget, effect: ShowEffect, duration: Duration, show: bool) -> GuiResult<()> {
    const AW_HOR_POSITIVE: u32 = 0x1;
    const AW_HOR_NEGATIVE: u32 = 0x2;
    const AW_VER_POSITIVE: u32 = 0x4;
    const AW_VER_NEGATIVE: u32 = 0x8;
    const AW_CENTER: u32 = 0x10;
    const AW_HIDE: u32 = 0x1_0000;
    const AW_SLIDE: u32 = 0x4_0000;
    const AW_BLEND: u32 = 0x8_0000;
    let hwnd = widget.core().require_alive()?;
    let mut flags = match effect {
        ShowEffect::Fade => AW_BLEND,
        ShowEffect::SlideDown => AW_SLIDE | AW_VER_POSITIVE,
        ShowEffect::SlideUp => AW_SLIDE | AW_VER_NEGATIVE,
        ShowEffect::SlideRight => AW_SLIDE | AW_HOR_POSITIVE,
        ShowEffect::SlideLeft => AW_SLIDE | AW_HOR_NEGATIVE,
        ShowEffect::Center => AW_CENTER,
    };
    if !show {
        flags |= AW_HIDE;
    }
    let visible = is_visible(hwnd);
    if visible == show {
        return Ok(());
    }
    let millis = if animations_enabled() { duration.as_millis().min(5_000) as u32 } else { 0 };
    // SAFETY: janela viva desta thread; a chamada só retorna ao fim da animação.
    let ok = millis > 0 && unsafe { crate::ffi::user32::AnimateWindow(hwnd, millis, flags) } != 0;
    if !ok || is_visible(hwnd) != show {
        // Sem animação (desligada, efeito não suportado, ou o sistema recusou): só mostra/oculta.
        let command = if show { crate::ffi::user32::SW_SHOWNA } else { crate::ffi::user32::SW_HIDE };
        // SAFETY: janela viva desta thread.
        unsafe { crate::ffi::user32::ShowWindow(hwnd, command) };
    }
    if is_visible(hwnd) != show {
        return Err(GuiError::last_os_error("AnimateWindow"));
    }
    widget.core().invalidate();
    Ok(())
}

/// Shows a control/window with a system effect (`AnimateWindow`), blocking until completion. Library-drawn controls
/// respond to its `WM_PRINTCLIENT`.
pub fn show_animated(widget: &impl Widget, effect: ShowEffect, duration: Duration) -> GuiResult<()> {
    animate_window(widget, effect, duration, true)
}

/// Hides a control/window with a system effect (`AnimateWindow`).
pub fn hide_animated(widget: &impl Widget, effect: ShowEffect, duration: Duration) -> GuiResult<()> {
    animate_window(widget, effect, duration, false)
}

fn is_visible(hwnd: crate::ffi::HWND) -> bool {
    // SAFETY: consulta simples.
    unsafe { crate::ffi::user32::IsWindowVisible(hwnd) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_curves() {
        for easing in [Easing::Linear, Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut, Easing::Overshoot] {
            assert!(easing.apply(0.0).abs() < 1e-9, "{easing:?} começa em 0");
            assert!((easing.apply(1.0) - 1.0).abs() < 1e-9, "{easing:?} termina em 1");
        }
        assert!(Easing::EaseIn.apply(0.5) < 0.5 && Easing::EaseOut.apply(0.5) > 0.5);
        assert!((Easing::EaseInOut.apply(0.5) - 0.5).abs() < 1e-9);
        assert!((0..=100).any(|i| Easing::Overshoot.apply(i as f64 / 100.0) > 1.0), "passa do fim");
        assert_eq!(Easing::Linear.apply(2.0), 1.0, "limitado");
        let ease = Easing::CubicBezier(0.25, 0.1, 0.25, 1.0); // "ease" do CSS
        assert!((ease.apply(0.5) - 0.8024).abs() < 0.01, "{}", ease.apply(0.5));
        let linear = Easing::CubicBezier(0.0, 0.0, 1.0, 1.0);
        assert!((linear.apply(0.3) - 0.3).abs() < 1e-4);
    }

    #[test]
    fn interpolation() {
        assert_eq!(lerp(10.0, 20.0, 0.25), 12.5);
        assert_eq!(lerp_rect(Rect::new(0, 0, 10, 10), Rect::new(10, 20, 30, 40), 0.5), Rect::new(5, 10, 20, 25));
        assert_eq!(lerp_color(Color::rgb(0, 0, 0), Color::rgb(255, 100, 50), 0.5), Color::rgb(128, 50, 25));
    }
}
