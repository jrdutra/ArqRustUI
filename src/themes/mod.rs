//! Appearance: light/dark themes, system detection, reusable styles and DWM effects.
//!
//! Official Windows support used here:
//!
//! * classic control background/text colors (`WM_CTLCOLOR*`);
//! * ListView/TreeView/RichEdit colors through control messages;
//! * version-dependent DWM dark title, corners, border and backdrop (see [`dwm`]);
//! * library-drawn buttons (`BS_OWNERDRAW`) with [`ButtonStyle`].
//!
//! **Optional cosmetic** addition: undocumented Microsoft `DarkMode_Explorer` visual class for dark scrollbars via
//! SetWindowTheme. If absent, nothing else is affected.

pub(crate) mod dark;
pub mod dwm;

pub use dwm::{Backdrop, CornerStyle};

use crate::core::native;
use crate::core::widget::{all_cores, Widget, WidgetCore};
use crate::core::Window;
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;
use crate::ffi::uxtheme::SetWindowTheme;
use crate::graphics::Font;
use crate::system::accessibility::{high_contrast, read_user_dword};
use crate::utilities::{to_wide, Color};
use std::rc::Rc;

/// Current system theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SystemTheme {
    /// Light application mode.
    Light,
    /// Dark application mode.
    Dark,
    /// High-contrast theme active.
    HighContrast,
}

impl SystemTheme {
    /// Detects current application system theme.
    pub fn current() -> SystemTheme {
        if high_contrast() {
            return SystemTheme::HighContrast;
        }
        let key = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";
        match read_user_dword(key, "AppsUseLightTheme") {
            Some(0) => SystemTheme::Dark,
            _ => SystemTheme::Light,
        }
    }
}

/// Button visual state colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateColors {
    /// Background.
    pub background: Color,
    /// Text.
    pub foreground: Color,
    /// Border.
    pub border: Color,
}

/// Custom button appearance for normal, hover, pressed, focused and disabled states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonStyle {
    /// Idle state.
    pub normal: StateColors,
    /// Hovered.
    pub hover: StateColors,
    /// Pressed.
    pub pressed: StateColors,
    /// Disabled.
    pub disabled: StateColors,
    /// Keyboard focus indicator color.
    pub focus_ring: Color,
    /// Logical corner radius.
    pub corner_radius: i32,
}

impl ButtonStyle {
    /// Derives button style from accent color.
    pub fn from_accent(accent: Color, foreground: Color) -> ButtonStyle {
        let state = |background: Color| StateColors { background, foreground, border: background };
        ButtonStyle {
            normal: state(accent),
            hover: state(accent.mix(Color::WHITE, 0.15)),
            pressed: state(accent.mix(Color::BLACK, 0.2)),
            disabled: StateColors {
                background: accent.mix(Color::GRAY, 0.7),
                foreground: foreground.mix(Color::GRAY, 0.5),
                border: accent.mix(Color::GRAY, 0.7),
            },
            focus_ring: foreground,
            corner_radius: 4,
        }
    }
}

impl ButtonStyle {
    /// System button colors (`COLOR_BTNFACE`, `COLOR_BTNTEXT`, etc.) when library must draw a normally native button.
    pub fn system() -> ButtonStyle {
        // SAFETY: consultas de cores do sistema.
        let color = |index| Color::from_colorref(unsafe { crate::ffi::user32::GetSysColor(index) });
        use crate::ffi::user32::{COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_HIGHLIGHT};
        let face = color(COLOR_BTNFACE);
        let text = color(COLOR_BTNTEXT);
        let border = face.mix(Color::BLACK, 0.35);
        let state = |background: Color, border: Color| StateColors { background, foreground: text, border };
        ButtonStyle {
            normal: state(face, border),
            hover: state(face.mix(color(COLOR_HIGHLIGHT), 0.12), color(COLOR_HIGHLIGHT)),
            pressed: state(face.mix(Color::BLACK, 0.12), border),
            disabled: StateColors {
                background: face,
                foreground: color(COLOR_GRAYTEXT),
                border: face.mix(Color::BLACK, 0.2),
            },
            focus_ring: text,
            corner_radius: 3,
        }
    }
}

/// Colors/font applied to window and all controls.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Theme name.
    pub name: String,
    /// `true` for dark themes, enabling dark title bar.
    pub dark: bool,
    /// Window/panel/label/check box background.
    pub background: Color,
    /// Text on `background`.
    pub foreground: Color,
    /// Text field/list background.
    pub input_background: Color,
    /// Text field/list text.
    pub input_foreground: Color,
    /// Accent color.
    pub accent: Color,
    /// Button appearance (`None` retains native system button).
    pub button: Option<ButtonStyle>,
    /// Default control font (`None` uses system font).
    pub font: Option<Font>,
}

impl Theme {
    /// Light theme with default native controls.
    pub fn light() -> Theme {
        Theme {
            name: "Claro".into(),
            dark: false,
            background: Color::rgb(243, 243, 243),
            foreground: Color::rgb(26, 26, 26),
            input_background: Color::WHITE,
            input_foreground: Color::rgb(26, 26, 26),
            accent: Color::rgb(0, 95, 184),
            button: None,
            font: None,
        }
    }

    /// Dark theme.
    pub fn dark() -> Theme {
        let surface = Color::rgb(55, 55, 55);
        let text = Color::rgb(240, 240, 240);
        let state = |background: Color| StateColors { background, foreground: text, border: Color::rgb(95, 95, 95) };
        Theme {
            name: "Escuro".into(),
            dark: true,
            background: Color::rgb(32, 32, 32),
            foreground: text,
            input_background: Color::rgb(45, 45, 45),
            input_foreground: text,
            accent: Color::rgb(96, 205, 255),
            button: Some(ButtonStyle {
                normal: state(surface),
                hover: state(Color::rgb(70, 70, 70)),
                pressed: state(Color::rgb(40, 40, 40)),
                disabled: StateColors {
                    background: Color::rgb(42, 42, 42),
                    foreground: Color::rgb(120, 120, 120),
                    border: Color::rgb(60, 60, 60),
                },
                focus_ring: Color::rgb(96, 205, 255),
                corner_radius: 4,
            }),
            font: None,
        }
    }

    /// System-following dark/light theme, using system colors in high contrast.
    pub fn system() -> Theme {
        match SystemTheme::current() {
            SystemTheme::Dark => Theme::dark(),
            SystemTheme::Light => Theme::light(),
            SystemTheme::HighContrast => {
                // SAFETY: consultas de cores do sistema.
                let color = |index| Color::from_colorref(unsafe { GetSysColor(index) });
                Theme {
                    name: "Alto contraste".into(),
                    dark: color(COLOR_WINDOW).luminance() < 0.5,
                    background: color(COLOR_WINDOW),
                    foreground: color(COLOR_WINDOWTEXT),
                    input_background: color(COLOR_WINDOW),
                    input_foreground: color(COLOR_WINDOWTEXT),
                    accent: color(COLOR_HIGHLIGHT),
                    button: None,
                    font: None,
                }
            }
        }
    }

    /// Same palette with another default font.
    pub fn with_font(mut self, font: Font) -> Theme {
        self.font = Some(font);
        self
    }

    /// Same palette with another accent and derived buttons.
    pub fn with_accent(mut self, accent: Color) -> Theme {
        self.accent = accent;
        let text = if accent.luminance() > 0.6 { Color::BLACK } else { Color::WHITE };
        self.button = Some(ButtonStyle::from_accent(accent, text));
        self
    }

    /// Applies to window/existing controls; later controls inherit automatically.
    pub fn apply(&self, window: &Window) {
        let root = window.core();
        if !root.is_alive() {
            return;
        }
        *root.theme.borrow_mut() = Some(Rc::new(self.clone()));
        // A barra de título escura depende da versão do sistema; a ausência dela
        // não impede o restante do tema.
        let _ = window.set_dark_title_bar(self.dark);
        crate::menus::strip::sync_native_bar(window, self.dark && !crate::system::accessibility::high_contrast());
        let root_hwnd = root.hwnd();
        root.refresh_font();
        for core in all_cores() {
            if !core.is_top_level() && native::root_of(core.hwnd()) == root_hwnd {
                core.refresh_font();
                style_control(&core, self);
            }
        }
        // SAFETY: redesenha a janela viva e seus filhos.
        unsafe {
            RedrawWindow(
                root_hwnd,
                std::ptr::null(),
                std::ptr::null_mut(),
                RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME,
            )
        };
    }
}

/// Reusable control font/color style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    /// Font.
    pub font: Option<Font>,
    /// Text color.
    pub foreground: Option<Color>,
    /// Background color.
    pub background: Option<Color>,
}

impl Style {
    /// Empty style.
    pub fn new() -> Style {
        Style::default()
    }

    /// Sets font.
    pub fn font(mut self, font: Font) -> Style {
        self.font = Some(font);
        self
    }

    /// Sets text color.
    pub fn foreground(mut self, color: Color) -> Style {
        self.foreground = Some(color);
        self
    }

    /// Sets background color.
    pub fn background(mut self, color: Color) -> Style {
        self.background = Some(color);
        self
    }

    /// Applies style to control.
    pub fn apply_to(&self, widget: &impl Widget) {
        if let Some(font) = &self.font {
            widget.set_font(font.clone());
        }
        if let Some(color) = self.foreground {
            widget.set_foreground(color);
        }
        if let Some(color) = self.background {
            widget.set_background(color);
        }
    }
}

fn set_visual_class(core: &WidgetCore, class: Option<&str>) {
    let hwnd = core.hwnd();
    // SAFETY: strings terminadas em NUL; a falha (classe inexistente) é inofensiva.
    unsafe {
        match class {
            Some(name) => {
                let wide = to_wide(name);
                let empty = [0u16];
                if name.is_empty() {
                    SetWindowTheme(hwnd, empty.as_ptr(), empty.as_ptr());
                } else {
                    SetWindowTheme(hwnd, wide.as_ptr(), std::ptr::null());
                }
            }
            None => {
                SetWindowTheme(hwnd, std::ptr::null(), std::ptr::null());
            }
        }
    }
}

/// Applies theme colors via control-specific messages.
fn style_control(core: &Rc<WidgetCore>, theme: &Theme) {
    let hwnd = core.hwnd();
    let background = core.background.get().unwrap_or(theme.input_background).to_colorref() as isize;
    let foreground = core.foreground.get().unwrap_or(theme.input_foreground).to_colorref() as isize;
    match core.kind {
        "ListView" | "DataGrid" | "PropertyGrid" => {
            native::send(hwnd, LVM_SETBKCOLOR, 0, background);
            native::send(hwnd, LVM_SETTEXTBKCOLOR, 0, background);
            native::send(hwnd, LVM_SETTEXTCOLOR, 0, foreground);
            set_visual_class(core, if theme.dark { Some("DarkMode_Explorer") } else { Some("Explorer") });
        }
        "TreeView" => {
            native::send(hwnd, TVM_SETBKCOLOR, 0, background);
            native::send(hwnd, TVM_SETTEXTCOLOR, 0, foreground);
            set_visual_class(core, if theme.dark { Some("DarkMode_Explorer") } else { Some("Explorer") });
        }
        "RichTextBox" => {
            native::send(hwnd, EM_SETBKGNDCOLOR, 0, background);
            crate::controls::richtext::set_default_text_color(hwnd, Color::from_colorref(foreground as u32));
        }
        // Com estilos visuais ativos, estes controles ignoram a cor de texto de
        // WM_CTLCOLOR*; em temas escuros voltam ao desenho clássico para respeitá-la.
        "CheckBox" | "RadioButton" | "GroupBox" | "ToggleButton" => {
            set_visual_class(core, if theme.dark { Some("") } else { None });
        }
        "Button" => crate::controls::button::refresh_draw_mode(core),
        "TextBox" | "PasswordBox" | "ListBox" | "ComboBox" => {
            set_visual_class(core, if theme.dark { Some("DarkMode_Explorer") } else { None });
        }
        _ => {}
    }
    dark::style_native(core, theme);
}

/// Inherits window theme during control creation.
pub(crate) fn apply_to_new_control(core: &Rc<WidgetCore>) {
    let theme = core.root().and_then(|root| root.theme.borrow().clone());
    if let Some(theme) = theme {
        style_control(core, &theme);
    }
}

/// Called when custom control colors change.
pub(crate) fn refresh_control_colors(core: &Rc<WidgetCore>) {
    let theme = core.root().and_then(|root| root.theme.borrow().clone());
    let base = theme.map(|t| (*t).clone()).unwrap_or_else(|| {
        // SAFETY: consultas de cores do sistema.
        let color = |index| Color::from_colorref(unsafe { GetSysColor(index) });
        Theme { input_background: color(COLOR_WINDOW), input_foreground: color(COLOR_WINDOWTEXT), ..Theme::light() }
    });
    if matches!(core.kind, "ListView" | "DataGrid" | "PropertyGrid" | "TreeView" | "RichTextBox") {
        let hwnd = core.hwnd();
        let background = core.background.get().unwrap_or(base.input_background).to_colorref() as isize;
        let foreground = core.foreground.get().unwrap_or(base.input_foreground).to_colorref() as isize;
        match core.kind {
            "TreeView" => {
                native::send(hwnd, TVM_SETBKCOLOR, 0, background);
                native::send(hwnd, TVM_SETTEXTCOLOR, 0, foreground);
            }
            "RichTextBox" => {
                native::send(hwnd, EM_SETBKGNDCOLOR, 0, background);
                crate::controls::richtext::set_default_text_color(hwnd, Color::from_colorref(foreground as u32));
            }
            _ => {
                native::send(hwnd, LVM_SETBKCOLOR, 0, background);
                native::send(hwnd, LVM_SETTEXTBKCOLOR, 0, background);
                native::send(hwnd, LVM_SETTEXTCOLOR, 0, foreground);
            }
        }
    }
    core.invalidate();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_themes_have_readable_contrast() {
        for theme in [Theme::light(), Theme::dark(), Theme::system()] {
            let contrast = (theme.background.luminance() - theme.foreground.luminance()).abs();
            assert!(contrast > 0.4, "{}: contraste insuficiente", theme.name);
            let input = (theme.input_background.luminance() - theme.input_foreground.luminance()).abs();
            assert!(input > 0.4, "{}: contraste de campos insuficiente", theme.name);
        }
        assert!(Theme::dark().dark && !Theme::light().dark);
    }

    #[test]
    fn accent_derives_button_states() {
        let theme = Theme::light().with_accent(Color::rgb(0, 120, 60));
        let style = theme.button.expect("estilo de botão");
        assert_ne!(style.normal.background, style.hover.background);
        assert_ne!(style.normal.background, style.pressed.background);
        assert_eq!(style.normal.foreground, Color::WHITE);
        let _ = SystemTheme::current();
    }
}
