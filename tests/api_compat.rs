//! Os nomes antigos de eventos, de antes da convenção de docs/EVENTS.md, continuam
//! funcionando: cada um é um alias `#[deprecated]` que encaminha para o nome novo.

#![allow(deprecated)]

mod common;

use arqrustui::prelude::*;
use common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[test]
fn window_on_close_still_vetoes() {
    with_app(|_| {
        let window = test_window();
        let allow = Rc::new(Cell::new(false));
        let asked = Counter::new();
        window.on_close({
            let (allow, asked) = (allow.clone(), asked.clone());
            move || {
                asked.hit();
                allow.get()
            }
        });
        window.close();
        assert!(window.is_alive(), "o nome antigo continua podendo vetar");
        allow.set(true);
        window.close();
        assert!(!window.is_alive());
        assert_eq!(asked.get(), 2);
    });
}

#[test]
fn notification_and_command_aliases_fire() {
    with_app(|app| {
        let window = test_window();
        window.show();

        let bar = InfoBar::builder().title("Aviso").message("texto").parent(&window).build().unwrap();
        let closed = Counter::new();
        let hit = closed.clone();
        bar.on_close(move || hit.hit());
        bar.close();
        assert_eq!(closed.get(), 1, "InfoBar::on_close = on_closed");

        let dismissed = Counter::new();
        let hit = dismissed.clone();
        let toast = Toast::new("Aviso").persistent().on_dismiss(move || hit.hit()).show(&window).unwrap();
        toast.close();
        assert!(pump_until(app, Duration::from_secs(2), || dismissed.get() == 1), "Toast::on_dismiss = on_dismissed");

        let commands = CommandBar::builder().parent(&window).build().unwrap();
        let states = Rc::new(RefCell::new(Vec::new()));
        let log = states.clone();
        let bold = commands
            .add_button(CommandButton::new("Negrito").toggle(false).on_toggle(move |on| log.borrow_mut().push(on)))
            .unwrap();
        bold.trigger();
        assert_eq!(*states.borrow(), vec![true], "CommandButton::on_toggle = on_toggled");

        let finished = Rc::new(Cell::new(false));
        let flag = finished.clone();
        let transition = motion::animate_value(0.0, 1.0, Duration::from_millis(60), Easing::Linear, |_| {}).unwrap();
        transition.on_done(move || flag.set(true));
        assert!(pump_until(app, Duration::from_secs(2), || finished.get()), "Transition::on_done = on_finished");
        window.destroy();
    });
}

#[test]
fn list_and_tree_aliases_register() {
    with_app(|_| {
        let window = test_window();
        // Registro pelos nomes antigos; o comportamento de cada evento é testado com
        // o nome novo em drag_drop.rs, data_grid.rs e controls.rs.
        let list_box = ListBox::builder().items(&["a", "b"]).parent(&window).build().unwrap();
        list_box.on_reorder(|_, _| true);
        list_box.on_items_reordered(|_, _| true);
        let list_view = ListView::builder().parent(&window).build().unwrap();
        list_view.on_item_activated(|_| {});
        list_view.on_reorder(|_, _| true);
        list_view.on_rows_reordered(|_, _| true);
        let grid = DataGrid::builder().parent(&window).build().unwrap();
        grid.on_reorder(|_, _| true);
        grid.on_rows_reordered(|_, _| true);
        let tree = TreeView::builder().parent(&window).build().unwrap();
        tree.on_item_drop(|_, _| true);
        let link = Hyperlink::builder().text("<a href=\"x\">x</a>").parent(&window).build().unwrap();
        link.on_link_clicked(|_| {});
        let menu = Menu::new().unwrap();
        menu.on_highlight(|_| {});
        let field = TextBox::builder().parent(&window).build().unwrap();
        Ime::on_composition(&field, |_| {});
        Ime::on_candidates(&field, |_| {});
        window.destroy();
    });
}
