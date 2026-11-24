# Custom controls and Win32 integration

*Português: [CUSTOM_CONTROLS.md](CUSTOM_CONTROLS.md).*

There are three levels of extension, from the simplest to the lowest level. Always use the highest one that solves the problem.

## 1. Composition in the application (without changing the library)

A custom control is usually a `Panel` with its own drawing and events:

```rust
use std::cell::Cell;
use std::rc::Rc;
use arqrustui::prelude::*;

/// Circular progress indicator — example of a control made in the application.
#[derive(Clone)]
pub struct Gauge {
    panel: Panel,
    value: Rc<Cell<f32>>,
}

impl Gauge {
    pub fn new(parent: &impl Widget, bounds: Rect) -> GuiResult<Gauge> {
        let panel = Panel::builder().bounds(bounds.x, bounds.y, bounds.width, bounds.height).parent(parent).build()?;
        let value = Rc::new(Cell::new(0.0f32));
        let shown = value.clone();
        panel.on_paint(move |canvas| {
            let size = canvas.size();
            let area = Rect::new(4, 4, size.width - 8, size.height - 8);
            canvas.ellipse(area, Some(&Pen::new(Color::GRAY, 2)), None);
            let filled = (area.height as f32 * shown.get()) as i32;
            canvas.clip_rect(Rect::new(area.x, area.bottom() - filled, area.width, filled));
            canvas.ellipse(area, None, Some(Color::BLUE));
        });
        // Screen readers now see the control as a progress bar.
        accessibility::set_custom_role(&panel, AccessibleRole::ProgressBar, "Progress")?;
        Ok(Gauge { panel, value })
    }

    pub fn set_value(&self, value: f32) {
        self.value.set(value.clamp(0.0, 1.0));
        self.panel.invalidate(); // schedules a redraw
    }

    /// Exposes the panel for layouts, positioning and events.
    pub fn widget(&self) -> &Panel {
        &self.panel
    }
}
```

Points to watch:

- **State** in `Rc<Cell<_>>`/`Rc<RefCell<_>>`, shared between the type and the closures.
- **Redrawing**: change the state and call `invalidate()`; never draw outside `on_paint`.
- **Input**: `Panel::builder().focusable(true)` to receive the keyboard; use `on_mouse_down`, `on_mouse_move`, `on_key_down`…
- **Accessibility**: `accessibility::set_custom_role` + `set_accessible_name`.
- For animation or heavy vector work, replace `on_paint` with `Direct2DSurface::attach`.
- **Icons**: `canvas.draw_icon(&icons::WARNING, Point::new(x, y), 16)` draws at the surface's DPI resolution; for large images, `icons::X.image()` + `canvas.with_gdiplus(|g| g.draw_image(..))`.

## 2. New control inside the library

To wrap a native class that is not covered yet (or to create a reusable composite), follow the pattern of the files in `src/controls/`:

```rust
// src/controls/my_control.rs
use super::common::{builder_common, widget_type, ControlOptions};
use crate::core::native;
use crate::core::widget::Reflected;
use crate::core::{Event, EventKind, GuiResult, HandlerId};

widget_type!(
    /// Public documentation of the control (required: `#![warn(missing_docs)]`).
    MyControl
);

/// Builder for [`MyControl`].
#[derive(Clone, Debug)]
pub struct MyControlBuilder {
    opts: ControlOptions,
    // specific options…
}

impl MyControlBuilder {
    builder_common!(); // text, position, size, bounds, parent, enabled, visible, tab_stop, font, tooltip

    /// Creates the control.
    pub fn build(self) -> GuiResult<MyControl> {
        // kind, native class, style, extended style, focusable, optional id
        let core = self.opts.create("MyControl", "NATIVE_CLASS", 0, 0, true, None)?;
        // Notifications that Windows sends to the parent arrive here, already reflected:
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Command { code, .. } if *code == 0 => {
                core.emit(&Event::Click);
                Some(0)
            }
            Reflected::Notify { code, lparam } => {
                let _ = (code, lparam); // convert `lparam` to the NM* structure in a documented unsafe block
                None
            }
            _ => None,
        });
        Ok(MyControl { core })
    }
}

impl MyControl {
    /// Starts building (give the default size).
    pub fn builder() -> MyControlBuilder {
        MyControlBuilder { opts: ControlOptions::new(120, 24) }
    }

    /// Properties use the `core::native` helpers instead of direct `unsafe`.
    pub fn value(&self) -> i32 {
        native::send(self.core.hwnd(), 0x0400, 0, 0) as i32
    }

    /// Typed events forward to `core.on`.
    pub fn on_value_changed(&self, f: impl Fn(i32) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.value()))
    }
}
```

Checklist when adding a control:

1. New constants, structures and functions go into the file of the corresponding DLL in `src/ffi/`, with the names from Microsoft's documentation; add the `size_of` of new structures to `tests/ffi_layout.rs`.
2. A file in `src/controls/`, `pub mod` + `pub use` in `src/controls/mod.rs` (and in the `prelude` in `src/lib.rs`, if it is commonly used).
3. Every public item documented; every `unsafe` block with `// SAFETY:`.
4. API coordinates in logical units (`to_pixels`/`to_logical` with `core.dpi()`).
5. If the control has its own colors (like ListView/TreeView), handle its `kind` in `themes::style_control`.
6. An integration test in `tests/controls.rs` and an example or a use in `examples/demo.rs`.
7. Update `docs/COMPONENTS.md`, `docs/FEATURE_MATRIX.md` and `AGENTS.md` (and their `.en.md` versions).
8. **Generic**: no business rules or names of consuming applications.
9. If the control shows icons, accept `&impl IconSource` and reuse `WidgetIcon` or `ManagedImages` from `controls/icon_support.rs` (DPI and lifetime already handled).

## 3. Raw messages and FFI (advanced API)

When a specific feature is missing, the application can go down one level without changing the library:

```rust
use arqrustui::ffi::user32::{SendMessageW, WM_SETREDRAW};

// Suspend redrawing during a large load.
// SAFETY: message without pointers, to a live window of this thread.
unsafe { SendMessageW(list.handle() as _, WM_SETREDRAW, 0, 0) };
/* … insert thousands of rows … */
// SAFETY: same as above.
unsafe { SendMessageW(list.handle() as _, WM_SETREDRAW, 1, 0) };
list.invalidate();
```

- `widget.handle()` returns the `HWND` as `isize` (zero if destroyed).
- `widget.on_raw_message(|msg| …)` intercepts messages before the library (one hook per widget).
- `arqrustui::ffi` exposes the existing declarations; `ffi::dynamic::Library` loads DLLs and optional symbols.

If the feature is of general interest, the right path is to promote it to a safe library API (level 2) instead of spreading `unsafe` across applications.
