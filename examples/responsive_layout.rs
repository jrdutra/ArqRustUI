//! Example 13 — Responsive layout: docking, proportional grid, flow and switching
//! arrangement according to the window width.
#![windows_subsystem = "windows"]

use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Responsive layout").size(720, 440).min_size(320, 300).centered().build()?;

    let header = Label::builder().text("  Dashboard — resize the window").parent(&window).build()?;
    header.set_font(Font::system().with_size(13.0).bold());
    header.set_background(Color::rgb(0, 95, 184));
    header.set_foreground(Color::WHITE);
    let sidebar = ListBox::builder().items(&["Home", "Reports", "Customers", "Settings"]).parent(&window).build()?;
    let footer = Label::builder().parent(&window).build()?;
    let content = Panel::builder().parent(&window).build()?;

    // Cards in a flow: they wrap to a new line when width runs out.
    let cards = FlowLayout::new().gaps(10, 10);
    for (title, color) in [
        ("Sales", Color::rgb(22, 163, 74)),
        ("Orders", Color::rgb(234, 179, 8)),
        ("Alerts", Color::rgb(220, 38, 38)),
        ("Visits", Color::rgb(37, 99, 235)),
    ] {
        let card = Panel::builder().size(150, 70).background(color).parent(&content).build()?;
        let caption = Label::builder().text(title).bounds(10, 8, 130, 22).parent(&card).build()?;
        caption.set_foreground(Color::WHITE);
        caption.set_font(Font::system().bold());
        cards.add(&card);
    }
    let search = TextBox::builder().placeholder("Search...").parent(&content).build()?;
    let go = Button::builder().text("Search").parent(&content).build()?;
    let form = GridLayout::new(&[Track::Star(3.0), Track::Star(1.0)], &[Track::Fixed(26)]).gaps(8, 0);
    form.add(&search, 0, 0);
    form.add(&go, 0, 1);
    let inner = StackLayout::vertical().spacing(12).padding(Thickness::all(12));
    inner.add_with(form, LayoutParams::new().height(26));
    inner.add_with(cards, LayoutParams::new().weight(1.0));
    content.set_layout(inner);

    // Two arrangements for the window: wide (sidebar) and narrow (bar at the top).
    let wide = DockLayout::new();
    wide.add_with(&header, Dock::Top, LayoutParams::new().height(34));
    wide.add_with(&footer, Dock::Bottom, LayoutParams::new().height(22).margin(Thickness::symmetric(8, 0)));
    wide.add_with(&sidebar, Dock::Left, LayoutParams::new().width(170).margin(Thickness::all(8)));
    wide.add(&content, Dock::Fill);
    let narrow = DockLayout::new();
    narrow.add_with(&header, Dock::Top, LayoutParams::new().height(34));
    narrow.add_with(&footer, Dock::Bottom, LayoutParams::new().height(22).margin(Thickness::symmetric(8, 0)));
    narrow.add_with(&sidebar, Dock::Top, LayoutParams::new().height(76).margin(Thickness::all(8)));
    narrow.add(&content, Dock::Fill);

    let apply = {
        let (window, footer) = (window.clone(), footer.clone());
        move |size: Size| {
            let is_wide = size.width >= 520;
            window.set_layout(if is_wide { wide.clone() } else { narrow.clone() });
            footer.set_text(&format!(
                "{} × {} logical units — {} arrangement — scale {:.0}%",
                size.width,
                size.height,
                if is_wide { "wide" } else { "narrow" },
                window.scale_factor() * 100.0
            ));
        }
    };
    apply(window.client_size());
    let mode = std::cell::Cell::new(window.client_size().width >= 520);
    window.on_resize(move |size| {
        // Only switches the layout when crossing the breakpoint; the rest is automatic.
        let is_wide = size.width >= 520;
        if mode.replace(is_wide) != is_wide {
            apply(size);
        }
    });
    window.show();
    app.run()
}
