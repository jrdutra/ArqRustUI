//! Shared base for library-drawn modern controls (`ToggleSwitch`, `NumberBox`, `Expander`, `BreadcrumbBar`, `Card`,
//! `RatingControl`, `TitleBar`, `DockHost`): theme/system colors, focus and a simple accessible model.

use crate::accessibility::{AccessibleAction, AccessibleItem, AccessibleModel};
use crate::core::native;
use crate::core::widget::{lookup, WidgetCore};
use crate::ffi::user32::*;
use crate::utilities::Color;
use std::rc::{Rc, Weak};

/// Modern control colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Palette {
    pub background: Color,
    pub surface: Color,
    pub foreground: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub hover: Color,
    pub dark: bool,
}

/// Colors for `core`: custom, window theme or system; high contrast always uses system colors.
pub(crate) fn palette(core: &WidgetCore) -> Palette {
    let hwnd = core.hwnd();
    let theme = lookup(native::root_of(hwnd)).and_then(|root| root.theme.borrow().clone());
    let high_contrast = crate::system::accessibility::high_contrast();
    let system = |index: i32| {
        // SAFETY: consulta de cor do sistema.
        Color::from_colorref(unsafe { GetSysColor(index) })
    };
    let (background, foreground, surface) = match (&theme, high_contrast) {
        (Some(t), false) => (core.effective_background().unwrap_or(t.background), t.foreground, t.input_background),
        _ => (
            core.effective_background().unwrap_or_else(|| system(COLOR_BTNFACE)),
            system(COLOR_WINDOWTEXT),
            system(COLOR_WINDOW),
        ),
    };
    let foreground = core.foreground.get().unwrap_or(foreground);
    let accent = crate::input::drag::accent_for(hwnd);
    let on_accent = if accent.luminance() > 0.55 { Color::BLACK } else { Color::WHITE };
    Palette {
        background,
        surface,
        foreground,
        muted: foreground.mix(background, 0.45),
        border: foreground.mix(background, 0.72),
        accent,
        on_accent,
        hover: background.mix(foreground, 0.08),
        dark: background.luminance() < 0.5,
    }
}

/// `true` if `core` has keyboard focus.
pub(crate) fn focused(core: &WidgetCore) -> bool {
    let hwnd = core.hwnd();
    // SAFETY: consulta simples do foco desta thread.
    !hwnd.is_null() && unsafe { GetFocus() } == hwnd
}

/// `true` if `core` is enabled.
pub(crate) fn enabled(core: &WidgetCore) -> bool {
    core.is_alive() && native::is_enabled(core.hwnd())
}

type ItemsFn = Box<dyn Fn() -> (AccessibleItem, Vec<AccessibleItem>)>;
type ActFn = Box<dyn Fn(u32, AccessibleAction) -> bool>;

/// Accessible model from two functions (controls retain only weak references).
pub(crate) struct ClosureModel {
    pub items: ItemsFn,
    pub act: ActFn,
}

impl AccessibleModel for ClosureModel {
    fn root(&self) -> AccessibleItem {
        (self.items)().0
    }

    fn children(&self) -> Vec<AccessibleItem> {
        (self.items)().1
    }

    fn perform(&self, id: u32, action: AccessibleAction) -> bool {
        (self.act)(id, action)
    }
}

/// Publishes an accessible model for `core`; `items` and `act` receive state if still alive.
pub(crate) fn publish<S: 'static>(
    core: &Rc<WidgetCore>,
    state: &Rc<S>,
    items: impl Fn(&Rc<WidgetCore>, &Rc<S>) -> (AccessibleItem, Vec<AccessibleItem>) + 'static,
    act: impl Fn(&Rc<WidgetCore>, &Rc<S>, u32, AccessibleAction) -> bool + 'static,
) {
    let (c1, s1): (Weak<WidgetCore>, Weak<S>) = (Rc::downgrade(core), Rc::downgrade(state));
    let (c2, s2) = (c1.clone(), s1.clone());
    let model = ClosureModel {
        items: Box::new(move || match (c1.upgrade(), s1.upgrade()) {
            (Some(core), Some(state)) => items(&core, &state),
            _ => (AccessibleItem::new(0, crate::accessibility::AccessibleRole::Custom, ""), Vec::new()),
        }),
        act: Box::new(move |id, action| match (c2.upgrade(), s2.upgrade()) {
            (Some(core), Some(state)) => act(&core, &state, id, action),
            _ => false,
        }),
    };
    let widget = crate::core::Window { core: core.clone() };
    let _ = crate::accessibility::set_accessible_model(&widget, model);
}
