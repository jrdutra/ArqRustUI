//! Library-drawn themed IME candidate window.

use super::ImeCandidates;
use crate::core::application as app;
use crate::core::native;
use crate::core::widget::{lookup, Widget, WidgetCore};
use crate::core::{EventKind, GuiError, GuiResult};
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::graphics::{Bitmap, Canvas, Font, TextAlign};
use crate::i18n::{library_text, LibraryText};
use crate::menus::strip as menu_strip;
use crate::utilities::{to_logical, to_pixels, to_wide, Point, Rect, Size};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

const CLASS: &str = "ArqRustUI.ImeCandidates";
/// Logical padding, row height, number column and minimum width.
const PADDING: i32 = 4;
const ROW: i32 = 24;
const NUMBER: i32 = 26;
const MIN_WIDTH: i32 = 120;

type PickFn = Rc<dyn Fn(usize)>;

struct CandidateState {
    owner: Weak<WidgetCore>,
    hwnd: Cell<HWND>,
    list: RefCell<Option<ImeCandidates>>,
    hot: Cell<Option<usize>>,
    picks: RefCell<Vec<PickFn>>,
    dpi: Cell<u32>,
    width: Cell<i32>,
}

impl CandidateState {
    fn rows(&self) -> usize {
        self.list.borrow().as_ref().map_or(0, |l| l.page().len())
    }

    fn has_footer(&self) -> bool {
        self.list.borrow().as_ref().is_some_and(|l| l.page_position().1 > 1)
    }

    fn size(&self) -> Size {
        let footer = if self.has_footer() { ROW } else { 0 };
        Size::new(self.width.get(), 2 * PADDING + self.rows() as i32 * ROW + footer)
    }

    fn row_at(&self, y: i32) -> Option<usize> {
        let row = (y - PADDING).div_euclid(ROW);
        (y >= PADDING && (row as usize) < self.rows()).then_some(row as usize)
    }
}

thread_local! {
    static WINDOWS: RefCell<Vec<Rc<CandidateState>>> = const { RefCell::new(Vec::new()) };
    static CLASS_READY: Cell<bool> = const { Cell::new(false) };
}

fn state_of(hwnd: HWND) -> Option<Rc<CandidateState>> {
    WINDOWS.with(|w| w.borrow().iter().find(|s| s.hwnd.get() == hwnd).cloned())
}

/// Library candidate window: numbered current page, highlighted selection and light/dark/high-contrast theme. Never
/// takes focus; keyboard stays with IME.
///
/// Usually created by [`Ime::use_candidate_window`](super::Ime::use_candidate_window); also usable with
/// [`Ime::on_candidates_changed`](super::Ime::on_candidates_changed) or autocomplete suggestions.
#[derive(Clone)]
pub struct ImeCandidateWindow {
    state: Rc<CandidateState>,
}

impl std::fmt::Debug for ImeCandidateWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImeCandidateWindow").field("visible", &self.is_visible()).finish()
    }
}

impl ImeCandidateWindow {
    /// Creates a hidden window for the text-receiving widget.
    pub fn new(widget: &impl Widget) -> GuiResult<ImeCandidateWindow> {
        app::require()?;
        let core = widget.core();
        let owner = native::root_of(core.require_alive()?);
        if !register_class() {
            return Err(GuiError::last_os_error("RegisterClassExW"));
        }
        let class = to_wide(CLASS);
        let title = to_wide(&library_text(LibraryText::Candidates));
        // SAFETY: strings terminadas em NUL; dono é uma janela viva desta thread.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                owner,
                std::ptr::null_mut(),
                native::instance(),
                std::ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err(GuiError::last_os_error("CreateWindowExW"));
        }
        let state = Rc::new(CandidateState {
            owner: Rc::downgrade(core),
            hwnd: Cell::new(hwnd),
            list: RefCell::new(None),
            hot: Cell::new(None),
            picks: RefCell::new(Vec::new()),
            dpi: Cell::new(core.dpi()),
            width: Cell::new(MIN_WIDTH),
        });
        WINDOWS.with(|w| w.borrow_mut().push(state.clone()));
        let weak = Rc::downgrade(&state);
        core.on(EventKind::Destroyed, move |_| {
            if let Some(state) = weak.upgrade() {
                let hwnd = state.hwnd.get();
                if !hwnd.is_null() {
                    // SAFETY: popup desta thread; o WM_DESTROY o retira da lista.
                    unsafe { DestroyWindow(hwnd) };
                }
            }
        });
        Ok(ImeCandidateWindow { state })
    }

    /// Shows `candidates` below logical widget point `at`, or above if monitor space is insufficient.
    pub fn show(&self, candidates: &ImeCandidates, at: Point) {
        let state = &self.state;
        let hwnd = state.hwnd.get();
        let Some(owner) = state.owner.upgrade() else { return };
        if hwnd.is_null() || !owner.is_alive() {
            return;
        }
        let previous = state.list.borrow().as_ref().map(|l| (l.selection, l.items.get(l.selection).cloned()));
        *state.list.borrow_mut() = Some(candidates.clone());
        state.hot.set(None);
        let dpi = owner.dpi();
        state.dpi.set(dpi);
        state.width.set(measure_width(candidates));
        let size = state.size();
        let (width, height) = (to_pixels(size.width, dpi), to_pixels(size.height, dpi));
        let mut origin = POINT { x: to_pixels(at.x, dpi), y: to_pixels(at.y, dpi) };
        // SAFETY: converte coordenadas do widget vivo desta thread.
        unsafe { ClientToScreen(owner.hwnd(), &mut origin) };
        if let Some(monitor) = crate::system::Monitor::from_point(Point::new(origin.x, origin.y)) {
            let area = monitor.work_area;
            if origin.y + height > area.y + area.height {
                origin.y -= height + to_pixels(ROW, dpi);
            }
            origin.x = origin.x.min(area.x + area.width - width).max(area.x);
            origin.y = origin.y.max(area.y);
        }
        // SAFETY: reposiciona e mostra o popup sem ativá-lo.
        unsafe {
            SetWindowPos(hwnd, HWND_TOPMOST, origin.x, origin.y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            InvalidateRect(hwnd, std::ptr::null(), FALSE);
        }
        let selected = candidates.items.get(candidates.selection).cloned();
        if previous != Some((candidates.selection, selected.clone())) {
            if let (Some(text), Some(root)) = (selected, lookup(native::root_of(owner.hwnd()))) {
                let number = candidates.selection.saturating_sub(candidates.page_start) + 1;
                let _ =
                    crate::accessibility::announce(&crate::core::Window { core: root }, &format!("{number}. {text}"));
            }
        }
    }

    /// Hides window.
    pub fn hide(&self) {
        self.state.list.borrow_mut().take();
        let hwnd = self.state.hwnd.get();
        if !hwnd.is_null() {
            // SAFETY: popup desta thread.
            unsafe { ShowWindow(hwnd, SW_HIDE) };
        }
    }

    /// `true` while visible.
    pub fn is_visible(&self) -> bool {
        let hwnd = self.state.hwnd.get();
        // SAFETY: consulta simples.
        !hwnd.is_null() && unsafe { IsWindowVisible(hwnd) } != 0
    }

    /// Currently displayed candidates.
    pub fn candidates(&self) -> Option<ImeCandidates> {
        self.state.list.borrow().clone()
    }

    /// Called with clicked candidate index in [`ImeCandidates::items`].
    pub fn on_pick(&self, f: impl Fn(usize) + 'static) {
        self.state.picks.borrow_mut().push(Rc::new(f));
    }

    /// Logical client rectangle of current-page `row`.
    pub fn row_bounds(&self, row: usize) -> Option<Rect> {
        (row < self.state.rows())
            .then(|| Rect::new(PADDING, PADDING + row as i32 * ROW, self.state.width.get() - 2 * PADDING, ROW))
    }

    /// Current logical window size.
    pub fn size(&self) -> Size {
        self.state.size()
    }

    /// Window handle (numeric `HWND`).
    pub fn handle(&self) -> isize {
        self.state.hwnd.get() as isize
    }
}

/// Logical width fitting widest candidate and page indicator.
fn measure_width(list: &ImeCandidates) -> i32 {
    let font = Font::system();
    let mut widest = 0;
    let _ = Bitmap::render(1, 1, |canvas| {
        for item in list.page() {
            widest = widest.max(canvas.measure_text(item, &font).width);
        }
        let (page, pages) = list.page_position();
        if pages > 1 {
            widest = widest.max(canvas.measure_text(&format!("‹ {}/{} ›", page + 1, pages), &font).width - NUMBER);
        }
    });
    (2 * PADDING + NUMBER + widest + 16).max(MIN_WIDTH)
}

fn paint(state: &CandidateState, canvas: &Canvas) {
    let owner = state.owner.upgrade().map_or(std::ptr::null_mut(), |c| native::root_of(c.hwnd()));
    let style = menu_strip::style_for_window(owner);
    let size = canvas.size();
    canvas.clear(style.popup_background);
    canvas.rect(
        Rect::new(0, 0, size.width, size.height),
        Some(&crate::graphics::Pen::new(style.popup_border, 1)),
        None,
    );
    let list = state.list.borrow().clone();
    let Some(list) = list else { return };
    let font = Font::system();
    for (row, item) in list.page().iter().enumerate() {
        let index = list.page_start + row;
        let rect = Rect::new(PADDING, PADDING + row as i32 * ROW, size.width - 2 * PADDING, ROW);
        let selected = index == list.selection;
        let hot = state.hot.get() == Some(row);
        let (fill, text) = if selected || hot {
            (Some(style.highlight), style.highlight_foreground)
        } else {
            (None, style.popup_foreground)
        };
        if let Some(fill) = fill {
            canvas.round_rect(rect, style.corner_radius, None, Some(fill));
        }
        let number = if selected || hot { text } else { style.shortcut_foreground };
        canvas.text_in_rect(
            &format!("{}.", row + 1),
            Rect::new(rect.x, rect.y, NUMBER, rect.height),
            &font,
            number,
            TextAlign::Center,
        );
        canvas.text_line(item, Rect::new(rect.x + NUMBER, rect.y, rect.width - NUMBER - 4, rect.height), &font, text);
    }
    let (page, pages) = list.page_position();
    if pages > 1 {
        let y = PADDING + list.page().len() as i32 * ROW;
        canvas.line(
            Point::new(PADDING, y),
            Point::new(size.width - PADDING, y),
            &crate::graphics::Pen::new(style.separator, 1),
        );
        canvas.text_in_rect(
            &format!("‹ {}/{} ›", page + 1, pages),
            Rect::new(PADDING, y, size.width - 2 * PADDING, ROW),
            &font,
            style.shortcut_foreground,
            TextAlign::Center,
        );
    }
}

fn register_class() -> bool {
    if CLASS_READY.with(Cell::get) {
        return true;
    }
    let name = to_wide(CLASS);
    // SAFETY: cursor compartilhado do sistema (não precisa ser liberado).
    let cursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW as PCWSTR) };
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_DROPSHADOW,
        lpfnWndProc: Some(candidate_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: native::instance(),
        hIcon: std::ptr::null_mut(),
        hCursor: cursor,
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: name.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };
    // SAFETY: estrutura completa; o nome vive durante a chamada.
    let ok = unsafe { RegisterClassExW(&class) } != 0
        || unsafe { crate::ffi::kernel32::GetLastError() } == crate::ffi::kernel32::ERROR_CLASS_ALREADY_EXISTS;
    CLASS_READY.with(|c| c.set(ok));
    ok
}

/// Candidate window procedure.
///
/// # Safety
/// Called only by Windows on the window's owning thread.
unsafe extern "system" fn candidate_proc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if msg == WM_MOUSEACTIVATE {
            return Some(MA_NOACTIVATE);
        }
        let state = state_of(hwnd)?;
        let logical = |v: i32| to_logical(v, state.dpi.get());
        match msg {
            WM_ERASEBKGND => Some(1),
            WM_PAINT => {
                // SAFETY: PAINTSTRUCT aceita zeros; par BeginPaint/EndPaint desta janela.
                unsafe {
                    let mut ps: PAINTSTRUCT = std::mem::zeroed();
                    let hdc = BeginPaint(hwnd, &mut ps);
                    if !hdc.is_null() {
                        let client = native::client_rect(hwnd);
                        crate::graphics::gdi::paint_buffered(hdc, client, state.dpi.get(), None, |canvas| {
                            paint(&state, canvas)
                        });
                    }
                    EndPaint(hwnd, &ps);
                }
                Some(0)
            }
            WM_PRINTCLIENT if wp != 0 => {
                let client = native::client_rect(hwnd);
                crate::graphics::gdi::paint_buffered(wp as HDC, client, state.dpi.get(), None, |canvas| {
                    paint(&state, canvas)
                });
                Some(0)
            }
            WM_MOUSEMOVE => {
                let hot = state.row_at(logical(y_lparam(lp)));
                if state.hot.replace(hot) != hot {
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: estrutura local corretamente dimensionada; invalida a própria janela.
                    unsafe {
                        TrackMouseEvent(&mut track);
                        InvalidateRect(hwnd, std::ptr::null(), FALSE);
                    }
                }
                Some(0)
            }
            WM_MOUSELEAVE => {
                if state.hot.replace(None).is_some() {
                    // SAFETY: invalida a própria janela.
                    unsafe { InvalidateRect(hwnd, std::ptr::null(), FALSE) };
                }
                Some(0)
            }
            WM_LBUTTONUP => {
                let row = state.row_at(logical(y_lparam(lp)));
                let start = state.list.borrow().as_ref().map(|l| l.page_start);
                if let (Some(row), Some(start)) = (row, start) {
                    // Copia os callbacks antes de chamar (um deles pode esconder a janela).
                    let picks = state.picks.borrow().clone();
                    for pick in picks {
                        pick(start + row);
                    }
                }
                Some(0)
            }
            WM_DESTROY => {
                state.hwnd.set(std::ptr::null_mut());
                WINDOWS.with(|w| w.borrow_mut().retain(|s| !Rc::ptr_eq(s, &state)));
                None
            }
            _ => None,
        }
    }));
    match result {
        Ok(Some(value)) => value,
        Ok(None) => DefWindowProcW(hwnd, msg, wp, lp),
        Err(payload) => {
            app::store_panic(payload);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_and_hit_testing() {
        let state = CandidateState {
            owner: Weak::new(),
            hwnd: Cell::new(std::ptr::null_mut()),
            list: RefCell::new(Some(ImeCandidates {
                items: (0..12).map(|i| format!("c{i}")).collect(),
                selection: 10,
                page_start: 9,
                page_size: 9,
            })),
            hot: Cell::new(None),
            picks: RefCell::new(Vec::new()),
            dpi: Cell::new(96),
            width: Cell::new(MIN_WIDTH),
        };
        assert_eq!(state.rows(), 3, "última página: 3 candidatos");
        assert!(state.has_footer());
        assert_eq!(state.size(), Size::new(MIN_WIDTH, 2 * PADDING + 4 * ROW));
        assert_eq!(state.row_at(PADDING), Some(0));
        assert_eq!(state.row_at(PADDING + 2 * ROW + 5), Some(2));
        assert_eq!(state.row_at(PADDING + 3 * ROW + 5), None, "rodapé não é candidato");
        assert_eq!(state.row_at(1), None);
    }
}
