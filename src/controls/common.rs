//! Shared control infrastructure: common builder options and macros.

use crate::core::widget::{Widget, WidgetCore};
use crate::core::{GuiError, GuiResult};
use crate::ffi::user32::{WS_DISABLED, WS_TABSTOP, WS_VISIBLE};
use crate::graphics::Font;
use crate::utilities::Rect;
use std::rc::Rc;

/// Options accepted by all control builders.
#[derive(Clone, Debug)]
pub(crate) struct ControlOptions {
    pub text: String,
    pub bounds: Rect,
    pub parent: Option<Rc<WidgetCore>>,
    pub enabled: bool,
    pub visible: bool,
    pub tab_stop: Option<bool>,
    pub font: Option<Font>,
    pub tooltip: Option<String>,
}

impl ControlOptions {
    pub(crate) fn new(width: i32, height: i32) -> Self {
        Self {
            text: String::new(),
            bounds: Rect::new(0, 0, width, height),
            parent: None,
            enabled: true,
            visible: true,
            tab_stop: None,
            font: None,
            tooltip: None,
        }
    }

    /// Creates the native control window with common options applied.
    pub(crate) fn create(
        &self,
        kind: &'static str,
        class: &str,
        style: u32,
        ex_style: u32,
        focusable: bool,
        id: Option<u16>,
    ) -> GuiResult<Rc<WidgetCore>> {
        let parent = self.parent.as_ref().ok_or(GuiError::MissingParent)?;
        let mut style = style;
        if self.visible {
            style |= WS_VISIBLE;
        }
        if !self.enabled {
            style |= WS_DISABLED;
        }
        if self.tab_stop.unwrap_or(focusable) {
            style |= WS_TABSTOP;
        }
        let core = WidgetCore::create_child(kind, parent, class, &self.text, style, ex_style, self.bounds, id)?;
        if let Some(font) = &self.font {
            core.set_font(font.clone());
        }
        if let Some(tooltip) = &self.tooltip {
            super::tooltip::set_for(&core, tooltip)?;
        }
        Ok(core)
    }
}

/// Gets the parent's core from any widget.
pub(crate) fn parent_core(parent: &impl Widget) -> Rc<WidgetCore> {
    parent.core().clone()
}

/// Declares a control type: cloneable handle implementing `Widget`.
macro_rules! widget_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug)]
        pub struct $name {
            pub(crate) core: std::rc::Rc<$crate::core::widget::WidgetCore>,
        }

        impl $crate::core::widget::Widget for $name {
            fn core(&self) -> &std::rc::Rc<$crate::core::widget::WidgetCore> {
                &self.core
            }
        }
    };
}

/// Generates methods shared by all builders inside `impl Builder`. The builder must have an `opts: ControlOptions`
/// field.
macro_rules! builder_common {
    () => {
        /// Initial control text.
        pub fn text(mut self, text: &str) -> Self {
            self.opts.text = text.to_string();
            self
        }

        /// Logical position relative to parent.
        pub fn position(mut self, x: i32, y: i32) -> Self {
            self.opts.bounds.x = x;
            self.opts.bounds.y = y;
            self
        }

        /// Logical size.
        pub fn size(mut self, width: i32, height: i32) -> Self {
            self.opts.bounds.width = width;
            self.opts.bounds.height = height;
            self
        }

        /// Logical position and size.
        pub fn bounds(mut self, x: i32, y: i32, width: i32, height: i32) -> Self {
            self.opts.bounds = $crate::utilities::Rect::new(x, y, width, height);
            self
        }

        /// Hosting window or container (required).
        pub fn parent(mut self, parent: &impl $crate::core::widget::Widget) -> Self {
            self.opts.parent = Some($crate::controls::common::parent_core(parent));
            self
        }

        /// Initial enabled state (default: enabled).
        pub fn enabled(mut self, enabled: bool) -> Self {
            self.opts.enabled = enabled;
            self
        }

        /// Initial visibility (default: visible).
        pub fn visible(mut self, visible: bool) -> Self {
            self.opts.visible = visible;
            self
        }

        /// Includes or excludes the control from Tab navigation.
        pub fn tab_stop(mut self, tab_stop: bool) -> Self {
            self.opts.tab_stop = Some(tab_stop);
            self
        }

        /// Control font.
        pub fn font(mut self, font: $crate::graphics::Font) -> Self {
            self.opts.font = Some(font);
            self
        }

        /// Tooltip text.
        pub fn tooltip(mut self, text: &str) -> Self {
            self.opts.tooltip = Some(text.to_string());
            self
        }
    };
}

/// Generates an `on_*` method forwarding an event without data to a closure.
macro_rules! simple_event {
    ($(#[$meta:meta])* $method:ident, $kind:ident) => {
        $(#[$meta])*
        pub fn $method(&self, f: impl Fn() + 'static) -> $crate::core::HandlerId {
            self.core.on($crate::core::EventKind::$kind, move |_| f())
        }
    };
}

pub(crate) use {builder_common, simple_event, widget_type};

/// Declares a builder with only common options.
macro_rules! plain_builder {
    ($(#[$meta:meta])* $builder:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug)]
        pub struct $builder {
            opts: $crate::controls::common::ControlOptions,
        }

        impl $builder {
            $crate::controls::common::builder_common!();
        }
    };
}

pub(crate) use plain_builder;
