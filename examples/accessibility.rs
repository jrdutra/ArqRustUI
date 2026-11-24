//! Example 19 — Accessibility: accessible names, mnemonics, tab order,
//! a custom control exposed to UI Automation, high contrast and text scale.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use arqrustui::system::accessibility as system_accessibility;
use std::cell::Cell;
use std::rc::Rc;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Accessibility").size(480, 360).centered().build()?;
    // In high contrast, the theme respects the system colors.
    Theme::system().apply(&window);

    let report = format!(
        "High contrast: {}  ·  Screen reader: {}  ·  Text scale: {:.0}%",
        if system_accessibility::high_contrast() { "yes" } else { "no" },
        if system_accessibility::screen_reader_active() { "yes" } else { "no" },
        system_accessibility::text_scale_factor() * 100.0
    );
    Label::builder().text(&report).bounds(16, 12, 448, 20).parent(&window).build()?;

    // Labels with mnemonics (Alt+letter) move the focus to the next field.
    Label::builder().text("&Name:").bounds(16, 48, 80, 20).parent(&window).build()?;
    let name = TextBox::builder().bounds(100, 44, 240, DEFAULT_CONTROL_HEIGHT).parent(&window).build()?;
    Label::builder().text("S&earch:").bounds(16, 82, 80, 20).parent(&window).build()?;
    let search = TextBox::builder().bounds(100, 78, 240, 24).parent(&window).build()?;
    // A symbol-only button needs an explicit accessible name.
    let go = Button::builder().text("➜").bounds(346, 78, 32, 24).tooltip("Search").parent(&window).build()?;
    let note = Label::builder().bounds(16, 112, 448, 20).parent(&window).build()?;
    let mut notes = Vec::new();
    if let Err(error) = go.set_accessible_name("Search") {
        notes.push(format!("accessible name: {error}"));
    }
    if let Err(error) = search.set_accessible_description("Type a term and press Enter") {
        notes.push(format!("description: {error}"));
    }

    // Custom control: a "switch" drawn by the application.
    let on = Rc::new(Cell::new(false));
    let switch = Panel::builder().bounds(16, 150, 64, 30).focusable(true).parent(&window).build()?;
    Label::builder().text("&Silent mode (Space toggles)").bounds(90, 155, 300, 20).parent(&window).build()?;
    if let Err(error) = accessibility::set_custom_role(&switch, AccessibleRole::CheckBox, "Silent mode: off") {
        notes.push(format!("UI Automation: {error}"));
    }
    let (state, probe) = (on.clone(), switch.clone());
    switch.on_paint(move |canvas| {
        let active = state.get();
        let track = if active { Color::rgb(0, 120, 212) } else { Color::rgb(150, 150, 150) };
        canvas.round_rect(Rect::new(2, 3, 58, 22), 11, None, Some(track));
        let knob = if active { 38 } else { 6 };
        canvas.ellipse(Rect::new(knob, 6, 16, 16), None, Some(Color::WHITE));
        // Visual focus indicator for keyboard users.
        if probe.has_focus() {
            canvas.focus_rect(Rect::new(0, 0, 62, 28));
        }
    });
    let toggle = {
        let (state, target, owner) = (on.clone(), switch.clone(), window.clone());
        move || {
            state.set(!state.get());
            let text = if state.get() { "Silent mode: on" } else { "Silent mode: off" };
            // Updates the exposed name and announces the change; both depend on the system.
            let _ = target.set_accessible_name(text);
            let _ = accessibility::announce(&owner, text);
            target.invalidate();
        }
    };
    switch.on_click(toggle.clone());
    switch.on_key_down(move |event| {
        if event.key == Key::SPACE {
            toggle()
        }
    });
    let target = switch.clone();
    switch.on_focus(move || target.invalidate());
    let target = switch.clone();
    switch.on_blur(move || target.invalidate());

    let ok = Button::builder().text("&OK").bounds(276, 310, 90, 30).default(true).parent(&window).build()?;
    let cancel = Button::builder().text("&Cancel").bounds(374, 310, 90, 30).cancel(true).parent(&window).build()?;
    let target = window.clone();
    ok.on_click(move || target.close());
    let target = window.clone();
    cancel.on_click(move || target.close());

    // Explicit tab order, independent of the creation order.
    accessibility::set_tab_order(&[&name, &search, &go, &switch, &ok, &cancel]);
    accessibility::show_focus_indicators(&window);
    note.set_text(&if notes.is_empty() { "Accessibility features active.".to_string() } else { notes.join(" · ") });
    name.focus();
    window.show();
    app.run()
}
