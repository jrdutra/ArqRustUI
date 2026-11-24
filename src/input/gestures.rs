//! Touch gestures: tap, double tap, hold, simultaneous pan/pinch/rotation with release inertia.
//!
//! Pure [`GestureRecognizer`] consumes contacts; [`on_gesture`] connects it to `WM_POINTER*` (Windows 8+), `WM_GESTURE`
//! fallback (Windows 7) and optional mouse pan/Ctrl+wheel pinch like touchpads.

use crate::core::widget::{Widget, WidgetCore};
use crate::core::{Event, EventKind, GuiResult, HandlerId, Timer};
use crate::input::{MouseButton, PointerKind, PointerPhase};
use crate::utilities::to_logical;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

/// Logical drag threshold distance.
const SLOP: f64 = 8.0;
/// Maximum tap duration and double-tap interval in seconds.
const TAP_TIME: f64 = 0.4;
/// Maximum double-tap separation.
const DOUBLE_TAP_DISTANCE: f64 = 24.0;
/// Hold duration in seconds.
const HOLD_TIME: f64 = 0.7;
/// Minimum speed in units/s to start/continue inertia.
const INERTIA_START: f64 = 250.0;
const INERTIA_STOP: f64 = 20.0;
/// Exponential inertia deceleration, fraction of speed lost per second.
const FRICTION: f64 = 4.0;

/// Gesture type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GestureKind {
    /// Quick stationary tap.
    Tap,
    /// Two quick taps at one point, delivered after second `Tap`.
    DoubleTap,
    /// Stationary held contact (touch right click).
    Hold,
    /// Pan/pinch/rotation may occur in one event.
    Manipulation,
}

/// Manipulation phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GesturePhase {
    /// Started, including initial displacement.
    Started,
    /// Fingers moved.
    Changed,
    /// Post-release inertia.
    Inertia,
    /// Completed, final manipulation event.
    Completed,
}

/// Gesture event, distances in logical client units.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GestureEvent {
    /// Type.
    pub kind: GestureKind,
    /// Phase (instant gestures use `Completed`).
    pub phase: GesturePhase,
    /// Contact center.
    pub center: (f64, f64),
    /// Displacement since preceding event.
    pub translation: (f64, f64),
    /// Scale since preceding event (1.0 = no pinch).
    pub scale: f64,
    /// Rotation since preceding event, clockwise-positive degrees.
    pub rotation: f64,
    /// Cumulative displacement since `Started`.
    pub total_translation: (f64, f64),
    /// Cumulative scale since `Started`.
    pub total_scale: f64,
    /// Cumulative rotation since `Started`, degrees.
    pub total_rotation: f64,
    /// Displacement velocity in units/s.
    pub velocity: (f64, f64),
    /// Current contacts.
    pub contacts: usize,
}

impl GestureEvent {
    fn instant(kind: GestureKind, at: (f64, f64)) -> GestureEvent {
        GestureEvent {
            kind,
            phase: GesturePhase::Completed,
            center: at,
            translation: (0.0, 0.0),
            scale: 1.0,
            rotation: 0.0,
            total_translation: (0.0, 0.0),
            total_scale: 1.0,
            total_rotation: 0.0,
            velocity: (0.0, 0.0),
            contacts: 0,
        }
    }
}

/// Recognizer output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureOptions {
    /// Continues with release inertia (default yes).
    pub inertia: bool,
    /// Calculates two-finger rotation (default yes).
    pub rotation: bool,
    /// Calculates two-finger pinch (default yes).
    pub scaling: bool,
    /// Left mouse dragging also generates manipulation (default no).
    pub mouse_pan: bool,
    /// Ctrl+wheel, including touchpad pinch delivered this way, generates pinch (default yes).
    pub ctrl_wheel_zoom: bool,
}

impl Default for GestureOptions {
    fn default() -> Self {
        GestureOptions { inertia: true, rotation: true, scaling: true, mouse_pan: false, ctrl_wheel_zoom: true }
    }
}

#[derive(Clone, Copy, Debug)]
struct Contact {
    position: (f64, f64),
    start: (f64, f64),
    since: f64,
}

/// Pure gesture recognizer: feed [`down`](Self::down), [`moved`](Self::moved), [`up`](Self::up) with seconds and call
/// [`step`](Self::step) periodically for inertia/hold.
#[derive(Clone, Debug)]
pub struct GestureRecognizer {
    options: GestureOptions,
    contacts: BTreeMap<u32, Contact>,
    active: bool,
    held: bool,
    /// Previous reference centroid, distance and angle.
    last_centroid: (f64, f64),
    last_spread: Option<(f64, f64)>,
    totals: ((f64, f64), f64, f64),
    velocity: (f64, f64),
    last_time: f64,
    /// Last time the center actually moved.
    last_motion: f64,
    last_tap: Option<(f64, (f64, f64))>,
    inertia: Option<f64>,
    max_contacts: usize,
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn normalize(mut degrees: f64) -> f64 {
    while degrees > 180.0 {
        degrees -= 360.0;
    }
    while degrees < -180.0 {
        degrees += 360.0;
    }
    degrees
}

impl GestureRecognizer {
    /// Recognizer with given options.
    pub fn new(options: GestureOptions) -> GestureRecognizer {
        GestureRecognizer {
            options,
            contacts: BTreeMap::new(),
            active: false,
            held: false,
            last_centroid: (0.0, 0.0),
            last_spread: None,
            totals: ((0.0, 0.0), 1.0, 0.0),
            velocity: (0.0, 0.0),
            last_time: 0.0,
            last_motion: 0.0,
            last_tap: None,
            inertia: None,
            max_contacts: 0,
        }
    }

    /// Current options.
    pub fn options(&self) -> GestureOptions {
        self.options
    }

    /// `true` during finger/inertia manipulation.
    pub fn is_active(&self) -> bool {
        self.active || self.inertia.is_some()
    }

    /// `true` during inertia.
    pub fn in_inertia(&self) -> bool {
        self.inertia.is_some()
    }

    fn centroid(&self) -> (f64, f64) {
        let n = self.contacts.len().max(1) as f64;
        let (x, y) = self.contacts.values().fold((0.0, 0.0), |(x, y), c| (x + c.position.0, y + c.position.1));
        (x / n, y / n)
    }

    /// Distance/angle in degrees between first two contacts.
    fn spread(&self) -> Option<(f64, f64)> {
        let mut points = self.contacts.values().map(|c| c.position);
        let (a, b) = (points.next()?, points.next()?);
        Some((distance(a, b), (b.1 - a.1).atan2(b.0 - a.0).to_degrees()))
    }

    /// Resets references after contact changes to avoid jumping.
    fn rebase(&mut self) {
        self.last_centroid = self.centroid();
        self.last_spread = self.spread();
    }

    fn event(&self, phase: GesturePhase, translation: (f64, f64), scale: f64, rotation: f64) -> GestureEvent {
        GestureEvent {
            kind: GestureKind::Manipulation,
            phase,
            center: if self.contacts.is_empty() { self.last_centroid } else { self.centroid() },
            translation,
            scale,
            rotation,
            total_translation: self.totals.0,
            total_scale: self.totals.1,
            total_rotation: self.totals.2,
            velocity: self.velocity,
            contacts: self.contacts.len(),
        }
    }

    /// Ends ongoing inertia before a new contact.
    fn stop_inertia(&mut self, out: &mut Vec<GestureEvent>) {
        if self.inertia.take().is_some() {
            out.push(self.event(GesturePhase::Completed, (0.0, 0.0), 1.0, 0.0));
        }
    }

    /// Contact began.
    pub fn down(&mut self, id: u32, at: (f64, f64), time: f64) -> Vec<GestureEvent> {
        let mut out = Vec::new();
        self.stop_inertia(&mut out);
        if self.contacts.is_empty() {
            self.held = false;
            self.max_contacts = 0;
            self.velocity = (0.0, 0.0);
        }
        self.contacts.insert(id, Contact { position: at, start: at, since: time });
        self.max_contacts = self.max_contacts.max(self.contacts.len());
        self.last_time = time;
        self.rebase();
        out
    }

    /// Contact moved.
    pub fn moved(&mut self, id: u32, at: (f64, f64), time: f64) -> Vec<GestureEvent> {
        let mut out = Vec::new();
        let Some(contact) = self.contacts.get_mut(&id) else { return out };
        contact.position = at;
        if !self.active {
            let far = self.contacts.values().any(|c| distance(c.position, c.start) > SLOP);
            if !far || self.held {
                return out;
            }
            self.active = true;
            self.totals = ((0.0, 0.0), 1.0, 0.0);
        }
        let centroid = self.centroid();
        let translation = (centroid.0 - self.last_centroid.0, centroid.1 - self.last_centroid.1);
        let (mut scale, mut rotation) = (1.0, 0.0);
        if let (Some((d0, a0)), Some((d1, a1))) = (self.last_spread, self.spread()) {
            if self.options.scaling && d0 > 1.0 {
                scale = d1 / d0;
            }
            if self.options.rotation {
                rotation = normalize(a1 - a0);
            }
        }
        let dt = (time - self.last_time).max(1e-3);
        let instant = (translation.0 / dt, translation.1 / dt);
        // Média exponencial: suaviza a velocidade usada pela inércia.
        self.velocity = (self.velocity.0 * 0.4 + instant.0 * 0.6, self.velocity.1 * 0.4 + instant.1 * 0.6);
        self.last_time = time;
        if translation.0.abs() + translation.1.abs() > 0.5 {
            self.last_motion = time;
        }
        self.totals.0 = (self.totals.0 .0 + translation.0, self.totals.0 .1 + translation.1);
        self.totals.1 *= scale;
        self.totals.2 += rotation;
        let phase =
            if self.totals.0 == translation && (self.totals.1 - scale).abs() < 1e-12 && self.totals.2 == rotation {
                GesturePhase::Started
            } else {
                GesturePhase::Changed
            };
        self.last_centroid = centroid;
        self.last_spread = self.spread();
        out.push(self.event(phase, translation, scale, rotation));
        out
    }

    /// Contact ended.
    pub fn up(&mut self, id: u32, at: (f64, f64), time: f64) -> Vec<GestureEvent> {
        let mut out = self.moved(id, at, time);
        let Some(contact) = self.contacts.remove(&id) else { return out };
        if !self.contacts.is_empty() {
            self.rebase();
            return out;
        }
        if self.active {
            self.active = false;
            // Sem movimento recente, a velocidade não vale (o dedo parou antes de soltar).
            if time - self.last_motion > 0.1 {
                self.velocity = (0.0, 0.0);
            }
            let speed = distance(self.velocity, (0.0, 0.0));
            if self.options.inertia && speed >= INERTIA_START {
                self.inertia = Some(time);
            } else {
                out.push(self.event(GesturePhase::Completed, (0.0, 0.0), 1.0, 0.0));
            }
        } else if !self.held && self.max_contacts == 1 && time - contact.since <= TAP_TIME {
            out.push(GestureEvent::instant(GestureKind::Tap, at));
            match self.last_tap {
                Some((when, place)) if time - when <= TAP_TIME && distance(place, at) <= DOUBLE_TAP_DISTANCE => {
                    out.push(GestureEvent::instant(GestureKind::DoubleTap, at));
                    self.last_tap = None;
                }
                _ => self.last_tap = Some((time, at)),
            }
        }
        out
    }

    /// Advances time for hold/inertia events.
    pub fn step(&mut self, time: f64) -> Vec<GestureEvent> {
        let mut out = Vec::new();
        if !self.active && !self.held && self.contacts.len() == 1 {
            let contact = *self.contacts.values().next().expect("one contact");
            if time - contact.since >= HOLD_TIME && distance(contact.position, contact.start) <= SLOP {
                self.held = true;
                out.push(GestureEvent::instant(GestureKind::Hold, contact.position));
            }
        }
        if let Some(previous) = self.inertia {
            let dt = (time - previous).max(0.0);
            if dt <= 0.0 {
                return out;
            }
            let decay = (-FRICTION * dt).exp();
            self.velocity = (self.velocity.0 * decay, self.velocity.1 * decay);
            let translation = (self.velocity.0 * dt, self.velocity.1 * dt);
            self.totals.0 = (self.totals.0 .0 + translation.0, self.totals.0 .1 + translation.1);
            self.last_centroid = (self.last_centroid.0 + translation.0, self.last_centroid.1 + translation.1);
            self.inertia = Some(time);
            out.push(self.event(GesturePhase::Inertia, translation, 1.0, 0.0));
            if distance(self.velocity, (0.0, 0.0)) < INERTIA_STOP {
                self.inertia = None;
                self.velocity = (0.0, 0.0);
                out.push(self.event(GesturePhase::Completed, (0.0, 0.0), 1.0, 0.0));
            }
        }
        out
    }

    /// Whether [`step`](Self::step) is needed for pending hold/inertia.
    pub fn needs_steps(&self) -> bool {
        self.inertia.is_some() || (!self.active && !self.held && self.contacts.len() == 1)
    }

    /// Standalone pinch (Ctrl+wheel/`WM_GESTURE`): complete manipulation event.
    pub(crate) fn single(
        &self,
        phase: GesturePhase,
        center: (f64, f64),
        translation: (f64, f64),
        scale: f64,
        rotation: f64,
        totals: ((f64, f64), f64, f64),
    ) -> GestureEvent {
        GestureEvent {
            kind: GestureKind::Manipulation,
            phase,
            center,
            translation,
            scale,
            rotation,
            total_translation: totals.0,
            total_scale: totals.1,
            total_rotation: totals.2,
            velocity: (0.0, 0.0),
            contacts: 0,
        }
    }
}

/// Widget gesture subscription; [`stop`](Self::stop) ends delivery.
#[derive(Debug)]
pub struct GestureSubscription {
    core: Weak<WidgetCore>,
    handlers: Vec<HandlerId>,
    alive: Rc<Cell<bool>>,
}

impl GestureSubscription {
    /// Stops gesture delivery.
    pub fn stop(&self) {
        self.alive.set(false);
        if let Some(core) = self.core.upgrade() {
            for id in &self.handlers {
                core.remove_handler(*id);
            }
        }
    }
}

type GestureFn = Rc<dyn Fn(&GestureEvent)>;

struct Binding {
    recognizer: RefCell<GestureRecognizer>,
    callback: GestureFn,
    alive: Rc<Cell<bool>>,
    epoch: Instant,
    driver: RefCell<Option<Timer>>,
    native_seen: Cell<bool>,
    pointer_seen: Cell<bool>,
    mouse_pointer_seen: Cell<bool>,
    native_totals: Cell<((f64, f64), f64, f64)>,
    native_last: Cell<((f64, f64), f64, f64)>,
}

impl Binding {
    fn now(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    /// Delivers events outside recognizer borrow; runs inertia/hold clock only when needed.
    fn deliver(self: &Rc<Self>, events: Vec<GestureEvent>) {
        if self.alive.get() {
            for event in &events {
                (self.callback)(event);
            }
        }
        let needs = self.recognizer.borrow().needs_steps();
        let running = self.driver.borrow().is_some();
        if needs && !running && self.alive.get() {
            let weak = Rc::downgrade(self);
            let timer = Timer::interval(Duration::from_millis(15), move || {
                if let Some(binding) = weak.upgrade() {
                    let now = binding.now();
                    let events = binding.recognizer.borrow_mut().step(now);
                    binding.deliver(events);
                }
            });
            *self.driver.borrow_mut() = timer.ok();
        } else if !needs && running {
            let timer = self.driver.borrow_mut().take();
            drop(timer);
        }
    }
}

/// Attaches gesture recognition with default options.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(canvas: &Panel) -> GuiResult<()> {
/// let zoom = std::rc::Rc::new(std::cell::Cell::new(1.0));
/// let _gestures = gestures::on_gesture(canvas, move |g| {
///     if g.kind == GestureKind::Manipulation {
///         zoom.set(zoom.get() * g.scale);
///     }
/// })?;
/// # Ok(()) }
/// ```
pub fn on_gesture(widget: &impl Widget, f: impl Fn(&GestureEvent) + 'static) -> GuiResult<GestureSubscription> {
    on_gesture_with(widget, GestureOptions::default(), f)
}

/// Attaches gesture recognition with options.
pub fn on_gesture_with(
    widget: &impl Widget,
    options: GestureOptions,
    f: impl Fn(&GestureEvent) + 'static,
) -> GuiResult<GestureSubscription> {
    let core = widget.core().clone();
    core.require_alive()?;
    let alive = Rc::new(Cell::new(true));
    let binding = Rc::new(Binding {
        recognizer: RefCell::new(GestureRecognizer::new(options)),
        callback: Rc::new(f),
        alive: alive.clone(),
        epoch: Instant::now(),
        driver: RefCell::new(None),
        native_seen: Cell::new(false),
        pointer_seen: Cell::new(false),
        mouse_pointer_seen: Cell::new(false),
        native_totals: Cell::new(((0.0, 0.0), 1.0, 0.0)),
        native_last: Cell::new(((0.0, 0.0), 0.0, 0.0)),
    });
    let mut handlers = Vec::new();
    let b = binding.clone();
    handlers.push(core.on(EventKind::Pointer, move |event| {
        let Event::Pointer(p) = event else { return };
        // Mouse como ponteiro só existe com `EnableMouseInPointer`; segue `mouse_pan`.
        if (p.kind == PointerKind::Mouse && !options.mouse_pan) || !b.alive.get() || b.native_seen.get() {
            return;
        }
        if p.kind == PointerKind::Mouse {
            b.mouse_pointer_seen.set(true);
        } else {
            b.pointer_seen.set(true);
        }
        let at = (p.position.x as f64, p.position.y as f64);
        let now = b.now();
        let events = {
            let mut r = b.recognizer.borrow_mut();
            match p.phase {
                PointerPhase::Down => r.down(p.id, at, now),
                PointerPhase::Move => r.moved(p.id, at, now),
                PointerPhase::Up => r.up(p.id, at, now),
            }
        };
        b.deliver(events);
    }));
    if options.mouse_pan {
        const MOUSE: u32 = u32::MAX;
        for kind in [EventKind::MouseDown, EventKind::MouseMove, EventKind::MouseUp] {
            let b = binding.clone();
            handlers.push(core.on(kind, move |event| {
                // Com o mouse chegando como ponteiro, as mensagens clássicas duplicariam o contato.
                if !b.alive.get() || b.mouse_pointer_seen.get() {
                    return;
                }
                let now = b.now();
                let events = {
                    let mut r = b.recognizer.borrow_mut();
                    match event {
                        Event::MouseDown(m) if m.button == MouseButton::Left => {
                            r.down(MOUSE, (m.position.x as f64, m.position.y as f64), now)
                        }
                        Event::MouseMove(m) => r.moved(MOUSE, (m.position.x as f64, m.position.y as f64), now),
                        Event::MouseUp(m) if m.button == MouseButton::Left => {
                            r.up(MOUSE, (m.position.x as f64, m.position.y as f64), now)
                        }
                        _ => Vec::new(),
                    }
                };
                b.deliver(events);
            }));
        }
    }
    if options.ctrl_wheel_zoom {
        let b = binding.clone();
        handlers.push(core.on(EventKind::MouseWheel, move |event| {
            let Event::MouseWheel(m) = event else { return };
            if !m.modifiers.ctrl || m.wheel_delta == 0 || !b.alive.get() {
                return;
            }
            let scale = 1.1f64.powi(m.wheel_delta);
            let center = (m.position.x as f64, m.position.y as f64);
            let event = b.recognizer.borrow().single(
                GesturePhase::Completed,
                center,
                (0.0, 0.0),
                scale,
                0.0,
                ((0.0, 0.0), scale, 0.0),
            );
            (b.callback)(&event);
        }));
    }
    install_native(&core, &binding, options);
    Ok(GestureSubscription { core: Rc::downgrade(&core), handlers, alive })
}

/// `WM_GESTURE` Windows 7+ zoom/pan/rotation mapped to the same manipulation. Ignored after pointer messages arrive.
fn install_native(core: &Rc<WidgetCore>, binding: &Rc<Binding>, options: GestureOptions) {
    use crate::ffi::user32::*;
    let hwnd = core.hwnd();
    let configs = [
        GESTURECONFIG { dwID: GID_ZOOM, dwWant: 1, dwBlock: 0 },
        GESTURECONFIG {
            dwID: GID_ROTATE,
            dwWant: if options.rotation { 1 } else { 0 },
            dwBlock: if options.rotation { 0 } else { 1 },
        },
        GESTURECONFIG {
            dwID: GID_PAN,
            dwWant: GC_PAN_WITH_SINGLE_FINGER_VERTICALLY
                | GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY
                | if options.inertia { GC_PAN_WITH_INERTIA } else { 0 },
            dwBlock: GC_PAN_WITH_GUTTER,
        },
    ];
    if let Some(set) = crate::ffi::dynamic::optional().SetGestureConfig {
        // SAFETY: janela viva desta thread; vetor local com o tamanho informado.
        unsafe { set(hwnd, 0, configs.len() as u32, configs.as_ptr(), std::mem::size_of::<GESTURECONFIG>() as u32) };
    }
    let weak = Rc::downgrade(binding);
    let target = Rc::downgrade(core);
    core.add_message_filter(move |raw| {
        if raw.message != WM_GESTURE {
            return None;
        }
        let binding = weak.upgrade()?;
        let core = target.upgrade()?;
        if binding.pointer_seen.get() || !binding.alive.get() {
            return None;
        }
        let functions = crate::ffi::dynamic::optional();
        let (get, close) = (functions.GetGestureInfo?, functions.CloseGestureInfoHandle?);
        // SAFETY: GESTUREINFO aceita zeros; o handle do LPARAM é lido e fechado aqui.
        let info = unsafe {
            let mut info: GESTUREINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<GESTUREINFO>() as u32;
            if get(raw.lparam as _, &mut info) == 0 {
                return None;
            }
            close(raw.lparam as _);
            info
        };
        binding.native_seen.set(true);
        let dpi = core.dpi();
        let mut point = crate::ffi::POINT { x: info.ptsLocation.x as i32, y: info.ptsLocation.y as i32 };
        // SAFETY: converte coordenadas de tela para a janela viva desta thread.
        unsafe { ScreenToClient(core.hwnd(), &mut point) };
        let at = (to_logical(point.x, dpi) as f64, to_logical(point.y, dpi) as f64);
        let argument = info.ullArguments;
        let begin = info.dwFlags & GF_BEGIN != 0;
        let end = info.dwFlags & GF_END != 0;
        let (last_at, last_distance, last_angle) = binding.native_last.get();
        let mut totals = if begin { ((0.0, 0.0), 1.0, 0.0) } else { binding.native_totals.get() };
        let (translation, scale, rotation) = match info.dwID {
            GID_ZOOM => {
                let distance = (argument & 0xFFFF_FFFF) as f64;
                let scale = if begin || last_distance <= 0.0 { 1.0 } else { distance / last_distance };
                binding.native_last.set((at, distance, last_angle));
                ((0.0, 0.0), scale, 0.0)
            }
            GID_PAN => {
                let delta = if begin { (0.0, 0.0) } else { (at.0 - last_at.0, at.1 - last_at.1) };
                binding.native_last.set((at, last_distance, last_angle));
                (delta, 1.0, 0.0)
            }
            GID_ROTATE => {
                // GID_ROTATE_ANGLE_FROM_ARGUMENT: radianos em [-2π, 2π] (anti-horário).
                let radians =
                    ((argument & 0xFFFF) as f64 / 65535.0) * 4.0 * std::f64::consts::PI - 2.0 * std::f64::consts::PI;
                let angle = -radians.to_degrees();
                let delta = if begin { angle } else { normalize(angle - last_angle) };
                binding.native_last.set((at, last_distance, angle));
                ((0.0, 0.0), 1.0, delta)
            }
            _ => return None,
        };
        totals = ((totals.0 .0 + translation.0, totals.0 .1 + translation.1), totals.1 * scale, totals.2 + rotation);
        binding.native_totals.set(totals);
        let phase = if begin {
            GesturePhase::Started
        } else if end {
            GesturePhase::Completed
        } else if info.dwFlags & GF_INERTIA != 0 {
            GesturePhase::Inertia
        } else {
            GesturePhase::Changed
        };
        let event = binding.recognizer.borrow().single(phase, at, translation, scale, rotation, totals);
        (binding.callback)(&event);
        Some(0)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recognizer() -> GestureRecognizer {
        GestureRecognizer::new(GestureOptions::default())
    }

    #[test]
    fn taps_double_taps_and_holds() {
        let mut r = recognizer();
        assert!(r.down(1, (10.0, 10.0), 0.0).is_empty());
        let events = r.up(1, (11.0, 10.0), 0.1);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, GestureKind::Tap);
        r.down(1, (14.0, 12.0), 0.3);
        let events = r.up(1, (14.0, 12.0), 0.35);
        assert_eq!(events.iter().map(|e| e.kind).collect::<Vec<_>>(), vec![GestureKind::Tap, GestureKind::DoubleTap]);
        // Um toque longo demais não é toque; parado vira "manter".
        r.down(2, (50.0, 50.0), 2.0);
        assert!(r.needs_steps());
        assert!(r.step(2.5).is_empty());
        let hold = r.step(2.75);
        assert_eq!(hold.len(), 1);
        assert_eq!(hold[0].kind, GestureKind::Hold);
        assert!(r.step(3.0).is_empty(), "só uma vez");
        assert!(r.up(2, (50.0, 50.0), 3.1).is_empty(), "depois de manter, soltar não é toque");
    }

    #[test]
    fn pan_with_inertia() {
        let mut r = recognizer();
        r.down(1, (0.0, 0.0), 0.0);
        assert!(r.moved(1, (5.0, 0.0), 0.01).is_empty(), "dentro da folga");
        let started = r.moved(1, (20.0, 0.0), 0.02);
        assert_eq!(started[0].phase, GesturePhase::Started);
        assert_eq!(started[0].translation, (20.0, 0.0), "o primeiro evento leva o caminho todo");
        let changed = r.moved(1, (40.0, 0.0), 0.03);
        assert_eq!(changed[0].phase, GesturePhase::Changed);
        assert_eq!(changed[0].total_translation, (40.0, 0.0));
        let released = r.up(1, (60.0, 0.0), 0.04);
        assert!(released.iter().all(|e| e.phase != GesturePhase::Completed), "rápido: inércia");
        assert!(r.in_inertia());
        let mut time = 0.04;
        let mut traveled = 0.0;
        let mut completed = false;
        for _ in 0..400 {
            time += 0.016;
            for event in r.step(time) {
                match event.phase {
                    GesturePhase::Inertia => {
                        assert!(event.translation.0 > 0.0);
                        traveled += event.translation.0;
                    }
                    GesturePhase::Completed => completed = true,
                    other => panic!("{other:?}"),
                }
            }
            if completed {
                break;
            }
        }
        assert!(completed && traveled > 50.0, "desacelera e termina: {traveled}");
        assert!(!r.is_active());

        // Devagar (ou parado antes de soltar): termina sem inércia.
        r.down(1, (0.0, 0.0), 10.0);
        r.moved(1, (30.0, 0.0), 10.5);
        let end = r.up(1, (30.0, 0.0), 11.0);
        assert_eq!(end.last().unwrap().phase, GesturePhase::Completed);
        assert!(!r.in_inertia());
    }

    #[test]
    fn pinch_and_rotate() {
        let mut r = recognizer();
        r.down(1, (100.0, 100.0), 0.0);
        r.down(2, (200.0, 100.0), 0.0);
        // Os dois dedos se afastam (escala 2) — o centro fica onde estava.
        r.moved(1, (50.0, 100.0), 0.05);
        let events = r.moved(2, (250.0, 100.0), 0.1);
        let last = events.last().unwrap();
        assert!((last.total_scale - 2.0).abs() < 1e-9, "{last:?}");
        assert!((last.center.0 - 150.0).abs() < 1e-9);
        // Gira 90° no sentido horário em volta do centro.
        r.moved(1, (150.0, 0.0), 0.15);
        let events = r.moved(2, (150.0, 200.0), 0.2);
        let last = events.last().unwrap();
        assert!((last.total_rotation - 90.0).abs() < 1e-9, "{}", last.total_rotation);
        assert!((last.total_scale - 2.0).abs() < 1e-9);
        // Soltar um dedo não faz o centro "saltar".
        let lifted = r.up(1, (150.0, 0.0), 0.25);
        assert!(lifted.iter().all(|e| e.translation == (0.0, 0.0)));
        let after = r.moved(2, (160.0, 200.0), 0.3);
        assert_eq!(after[0].translation, (10.0, 0.0));
        assert_eq!(after[0].contacts, 1);
        let mut no_rotation = GestureRecognizer::new(GestureOptions { rotation: false, ..GestureOptions::default() });
        no_rotation.down(1, (0.0, 0.0), 0.0);
        no_rotation.down(2, (100.0, 0.0), 0.0);
        let events = no_rotation.moved(2, (0.0, 100.0), 0.1);
        assert_eq!(events[0].total_rotation, 0.0);
        assert!(normalize(350.0) == -10.0 && normalize(-190.0) == 170.0);
    }
}
