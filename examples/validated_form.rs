//! Example 17 — Complete form with field-by-field validation.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::rc::Rc;

type Validator = Box<dyn Fn(&str) -> Result<(), String>>;

struct Field {
    input: TextBox,
    message: Label,
    validate: Validator,
}

impl Field {
    /// Validates the field, updates the error message and returns whether it is valid.
    fn check(&self) -> bool {
        match (self.validate)(self.input.text().trim()) {
            Ok(()) => {
                self.message.set_text("");
                true
            }
            Err(reason) => {
                self.message.set_text(&reason);
                false
            }
        }
    }
}

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window =
        Window::builder().title("Form with validation").size(480, 420).min_size(420, 420).centered().build()?;
    let grid = GridLayout::new(&[Track::Fixed(110), Track::Star(1.0)], &[]).gaps(8, 2).padding(Thickness::all(16));

    let mut fields: Vec<Rc<Field>> = Vec::new();
    let mut add_field = |caption: &str, placeholder: &str, validate: Validator| -> GuiResult<Rc<Field>> {
        let row = grid.add_row(Track::Fixed(26));
        let message_row = grid.add_row(Track::Fixed(18));
        let label = Label::builder().text(caption).parent(&window).build()?;
        let input = TextBox::builder().placeholder(placeholder).parent(&window).build()?;
        let message = Label::builder().parent(&window).build()?;
        message.set_foreground(Color::rgb(200, 30, 30));
        message.set_font(Font::system().with_size(8.0));
        grid.add_with(&label, row, 0, LayoutParams::new().align_v(Align::Center).height(20));
        grid.add(&input, row, 1);
        grid.add(&message, message_row, 1);
        let field = Rc::new(Field { input, message, validate });
        // Validates when leaving the field and clears the error when typing again.
        let on_blur = field.clone();
        field.input.on_blur(move || {
            on_blur.check();
        });
        let on_change = field.clone();
        field.input.on_text_changed(move || on_change.message.set_text(""));
        fields.push(field.clone());
        Ok(field)
    };

    add_field(
        "&Name:",
        "Full name",
        Box::new(|v| if v.split_whitespace().count() >= 2 { Ok(()) } else { Err("Enter first and last name.".into()) }),
    )?;
    add_field(
        "&Email:",
        "name@example.com",
        Box::new(|v| {
            let valid = v
                .split_once('@')
                .map(|(user, domain)| !user.is_empty() && domain.contains('.') && !domain.ends_with('.'))
                .unwrap_or(false);
            if valid {
                Ok(())
            } else {
                Err("Invalid email.".into())
            }
        }),
    )?;
    add_field(
        "&Age:",
        "18 to 120",
        Box::new(|v| match v.parse::<u32>() {
            Ok(18..=120) => Ok(()),
            Ok(_) => Err("Age must be between 18 and 120.".into()),
            Err(_) => Err("Enter digits only.".into()),
        }),
    )?;
    add_field(
        "&ZIP code:",
        "00000",
        Box::new(|v| {
            let digits = v.chars().filter(|c| c.is_ascii_digit()).count();
            if digits == 5 && v.chars().all(|c| c.is_ascii_digit() || c == '-') {
                Ok(())
            } else {
                Err("The ZIP code must have 5 digits.".into())
            }
        }),
    )?;

    let row = grid.add_row(Track::Fixed(26));
    let plan_label = Label::builder().text("&Plan:").parent(&window).build()?;
    let plan = ComboBox::builder().items(&["Basic", "Professional", "Business"]).parent(&window).build()?;
    grid.add_with(&plan_label, row, 0, LayoutParams::new().align_v(Align::Center).height(20));
    grid.add(&plan, row, 1);
    let row = grid.add_row(Track::Fixed(18));
    let plan_message = Label::builder().parent(&window).build()?;
    plan_message.set_foreground(Color::rgb(200, 30, 30));
    grid.add(&plan_message, row, 1);

    let row = grid.add_row(Track::Fixed(24));
    let terms = CheckBox::builder().text("I have read and accept the &terms of use").parent(&window).build()?;
    grid.add(&terms, row, 1);
    let row = grid.add_row(Track::Star(1.0));
    let summary = Label::builder().parent(&window).build()?;
    grid.add_span(&summary, row, 0, 1, 2, LayoutParams::new());
    let row = grid.add_row(Track::Fixed(30));
    let submit = Button::builder().text("&Submit").default(true).parent(&window).build()?;
    grid.add_with(&submit, row, 1, LayoutParams::new().width(110).align_h(Align::End));
    window.set_layout(grid);

    let owner = window.clone();
    submit.on_click(move || {
        let mut errors = fields.iter().filter(|field| !field.check()).count();
        if plan.selected_index().is_none() {
            plan_message.set_text("Select a plan.");
            errors += 1;
        } else {
            plan_message.set_text("");
        }
        if !terms.is_checked() {
            errors += 1;
        }
        if errors > 0 {
            summary.set_foreground(Color::rgb(200, 30, 30));
            summary.set_text(&format!(
                "Fix {errors} problem(s) before submitting{}",
                if terms.is_checked() { "." } else { " (you must accept the terms)." }
            ));
            // Moves the focus to the first invalid field.
            if let Some(first) = fields.iter().find(|field| !field.message.text().is_empty()) {
                first.input.focus();
            }
            return;
        }
        summary.set_foreground(Color::rgb(22, 130, 60));
        summary.set_text("Valid form — data submitted.");
        // The screen reader announcement depends on the system version.
        let _ = accessibility::announce(&owner, "Form submitted successfully");
    });
    window.show();
    app.run()
}
