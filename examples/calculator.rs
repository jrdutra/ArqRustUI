//! Example 5 — Calculator with a button grid and keyboard input.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Calculator {
    display: String,
    accumulator: Option<f64>,
    pending: Option<char>,
    fresh: bool,
}

impl Calculator {
    fn press(&mut self, key: char) {
        match key {
            '0'..='9' | ',' => {
                if self.fresh {
                    self.display.clear();
                    self.fresh = false;
                }
                if key == ',' && self.display.contains(',') {
                    return;
                }
                if key == ',' && self.display.is_empty() {
                    self.display.push('0');
                }
                self.display.push(key);
            }
            '+' | '-' | '*' | '/' => {
                self.evaluate();
                self.pending = Some(key);
                self.fresh = true;
            }
            '=' => {
                self.evaluate();
                self.pending = None;
                self.fresh = true;
            }
            'C' => *self = Calculator::default(),
            _ => {}
        }
    }

    fn value(&self) -> f64 {
        self.display.replace(',', ".").parse().unwrap_or(0.0)
    }

    fn evaluate(&mut self) {
        let current = self.value();
        let result = match (self.accumulator, self.pending) {
            (Some(left), Some('+')) => left + current,
            (Some(left), Some('-')) => left - current,
            (Some(left), Some('*')) => left * current,
            (Some(left), Some('/')) if current != 0.0 => left / current,
            (Some(_), Some('/')) => {
                self.display = "Division by zero".into();
                self.accumulator = None;
                return;
            }
            _ => current,
        };
        self.accumulator = Some(result);
        self.display = format!("{result}").replace('.', ",");
    }

    fn text(&self) -> String {
        if self.display.is_empty() {
            "0".into()
        } else {
            self.display.clone()
        }
    }
}

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Calculator").size(260, 330).resizable(false).centered().build()?;

    let display = TextBox::builder().text("0").read_only(true).align(TextAlign::Right).parent(&window).build()?;
    display.set_font(Font::new("Consolas", 16.0));

    let state = Rc::new(RefCell::new(Calculator::default()));
    let press = {
        let (state, display) = (state.clone(), display.clone());
        move |key: char| {
            state.borrow_mut().press(key);
            display.set_text(&state.borrow().text());
        }
    };

    let keys = ["7", "8", "9", "/", "4", "5", "6", "*", "1", "2", "3", "-", "0", ",", "=", "+"];
    let grid = GridLayout::new(
        &[Track::Star(1.0); 4],
        &[Track::Fixed(40), Track::Star(1.0), Track::Star(1.0), Track::Star(1.0), Track::Star(1.0), Track::Fixed(34)],
    )
    .gaps(6, 6)
    .padding(Thickness::all(10));
    grid.add_span(&display, 0, 0, 1, 4, LayoutParams::new());
    for (index, key) in keys.iter().enumerate() {
        let button = Button::builder().text(key).tab_stop(false).parent(&window).build()?;
        let (press, key) = (press.clone(), key.chars().next().unwrap_or(' '));
        button.on_click(move || press(key));
        grid.add(&button, 1 + index / 4, index % 4);
    }
    let clear = Button::builder().text("C").tab_stop(false).parent(&window).build()?;
    let on_clear = press.clone();
    clear.on_click(move || on_clear('C'));
    grid.add_span(&clear, 5, 0, 1, 4, LayoutParams::new());
    window.set_layout(grid);

    // Keyboard: digits, operators, Enter (=) and Esc (clear).
    let typed = press.clone();
    window.on_char(move |c| match c {
        '0'..='9' | '+' | '-' | '*' | '/' | '=' | ',' => typed(c),
        '.' => typed(','),
        '\r' => typed('='),
        '\u{1b}' => typed('C'),
        _ => {}
    });
    window.show();
    window.focus();
    app.run()
}
