//! Padrões UIA adicionais: modelo acessível da aplicação (`set_accessible_model`) com
//! Toggle na raiz, Selection/SelectionItem, Grid/GridItem, RangeValue, Scroll e Text;
//! `IRawElementProviderAdviseEvents`; seleção da PropertyGrid; anúncios de avisos.

mod common;

use arqrustui::accessibility::uia_debug::{
    advise, extra_patterns, grid_cell, number_property, perform, perform_extra, selection_names, text_unit, walk,
};
use arqrustui::accessibility::{
    set_accessible_model, AccessibleAction, AccessibleItem, AccessibleModel, AccessibleRange, AccessibleRole,
    AccessibleScroll, ScrollAmount,
};
use arqrustui::prelude::*;
use common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const S_OK: i32 = 0;
const SELECTION: usize = 0;
const SELECTION_ITEM: usize = 1;
const SCROLL: usize = 2;
const GRID: usize = 3;
const GRID_ITEM: usize = 4;
const RANGE: usize = 5;
const TEXT: usize = 6;

#[derive(Default)]
struct State {
    on: Cell<bool>,
    selected: Cell<usize>,
    rating: Cell<f64>,
    scroll: Cell<f64>,
    actions: RefCell<Vec<AccessibleAction>>,
}

struct Board(Rc<State>);

impl AccessibleModel for Board {
    fn root(&self) -> AccessibleItem {
        AccessibleItem::new(0, AccessibleRole::Custom, "Painel")
            .toggle(self.0.on.get())
            .scroll(AccessibleScroll {
                horizontal: None,
                vertical: Some(self.0.scroll.get()),
                horizontal_view: 100.0,
                vertical_view: 25.0,
            })
            .text("primeira linha\nsegunda linha")
    }

    fn children(&self) -> Vec<AccessibleItem> {
        let mut items = vec![AccessibleItem::new(1, AccessibleRole::List, "Cores").selection(false, true).grid(1, 3)];
        for (index, name) in ["Vermelho", "Verde", "Azul"].iter().enumerate() {
            items.push(
                AccessibleItem::new(10 + index as u32, AccessibleRole::ListItem, name)
                    .parent(1)
                    .bounds(Rect::new(index as i32 * 30, 0, 30, 20))
                    .selected(self.0.selected.get() == index)
                    .cell(0, index as i32),
            );
        }
        items.push(AccessibleItem::new(2, AccessibleRole::Slider, "Nota").range(AccessibleRange {
            value: self.0.rating.get(),
            minimum: 0.0,
            maximum: 5.0,
            small_change: 1.0,
            large_change: 1.0,
            read_only: false,
        }));
        items
    }

    fn perform(&self, id: u32, action: AccessibleAction) -> bool {
        self.0.actions.borrow_mut().push(action.clone());
        match action {
            AccessibleAction::Toggle if id == 0 => self.0.on.set(!self.0.on.get()),
            AccessibleAction::Select if (10..13).contains(&id) => self.0.selected.set(id as usize - 10),
            AccessibleAction::SetRange(value) if id == 2 => self.0.rating.set(value),
            AccessibleAction::ScrollTo(_, Some(v)) if id == 0 => self.0.scroll.set(v),
            AccessibleAction::Scroll(_, ScrollAmount::LargeIncrement) => self.0.scroll.set(self.0.scroll.get() + 25.0),
            _ => return false,
        }
        true
    }
}

#[test]
fn application_models_publish_every_pattern() {
    with_app(|_| {
        let window = test_window();
        window.show();
        let panel = Panel::builder().bounds(10, 10, 200, 100).parent(&window).build().unwrap();
        let state = Rc::new(State::default());
        set_accessible_model(&panel, Board(state.clone())).unwrap();
        let hwnd = panel.handle();
        // SAFETY: todas as chamadas abaixo usam janelas vivas desta thread.
        unsafe {
            // Raiz: Toggle (padrões básicos), Scroll e Text.
            let root = extra_patterns(hwnd, "");
            assert!(root[SCROLL] && root[TEXT] && !root[SELECTION] && !root[RANGE], "{root:?}");
            assert_eq!(perform(hwnd, "Painel", "toggle"), -2, "a raiz não é um filho");
            let tree = walk(hwnd);
            let names: Vec<&str> = tree.iter().map(|(_, e)| e.name.as_str()).collect();
            assert_eq!(names, ["Cores", "Vermelho", "Verde", "Azul", "Nota"]);
            assert_eq!(tree[0].1.control_type, 50008);

            // Selection + Grid no contêiner, SelectionItem + GridItem nos itens.
            let list = extra_patterns(hwnd, "Cores");
            assert!(list[SELECTION] && list[GRID] && !list[SELECTION_ITEM]);
            let item = extra_patterns(hwnd, "Verde");
            assert!(item[SELECTION_ITEM] && item[GRID_ITEM]);
            assert_eq!(selection_names(hwnd, "Cores"), ["Vermelho"]);
            assert_eq!(perform_extra(hwnd, "Azul", "select"), S_OK);
            assert_eq!(selection_names(hwnd, "Cores"), ["Azul"]);
            assert_eq!(number_property(hwnd, "Azul", 30079), Some(1.0), "SelectionItem.IsSelected");
            assert_eq!(number_property(hwnd, "Cores", 30061), Some(1.0), "Selection.IsSelectionRequired");
            assert_eq!(grid_cell(hwnd, "Cores", 0, 1).as_deref(), Some("Verde"));
            assert_eq!(grid_cell(hwnd, "Cores", 1, 0), None);
            assert_eq!(number_property(hwnd, "Verde", 30065), Some(1.0), "GridItem.Column");
            assert_eq!(number_property(hwnd, "Cores", 30063), Some(3.0), "Grid.ColumnCount");

            // RangeValue: valor, limites e validação.
            assert!(extra_patterns(hwnd, "Nota")[RANGE]);
            assert_eq!(perform_extra(hwnd, "Nota", "range:4"), S_OK);
            assert_eq!(state.rating.get(), 4.0);
            assert_eq!(number_property(hwnd, "Nota", 30047), Some(4.0));
            assert_eq!(number_property(hwnd, "Nota", 30050), Some(5.0));
            assert_ne!(perform_extra(hwnd, "Nota", "range:9"), S_OK, "fora da faixa");

            // Scroll na raiz.
            assert_eq!(perform_extra(hwnd, "", "scrollto:-1,50"), S_OK);
            assert_eq!(state.scroll.get(), 50.0);
            assert_eq!(perform_extra(hwnd, "", "scroll:2,3"), S_OK);
            assert_eq!(state.scroll.get(), 75.0);
            assert_eq!(number_property(hwnd, "", 30055), Some(75.0), "VerticalScrollPercent");
            assert_eq!(number_property(hwnd, "", 30057), Some(0.0), "sem rolagem horizontal");
            assert_ne!(perform_extra(hwnd, "", "scrollto:-1,150"), S_OK, "porcentagem inválida");

            // Text: unidades de palavra e linha.
            assert_eq!(text_unit(hwnd, "", 2, 1).as_deref(), Some("linha"), "a quebra de linha não entra na palavra");
            assert_eq!(text_unit(hwnd, "", 3, 1).as_deref(), Some("segunda linha"));
            assert_eq!(text_unit(hwnd, "", 0, 3).as_deref(), Some("m"));

            // AdviseEvents na raiz.
            assert_eq!(advise(hwnd, 20005, true), [20005]);
            assert_eq!(advise(hwnd, 20012, true), [20005, 20012]);
            assert_eq!(advise(hwnd, 20005, false), [20012]);
        }
        assert!(state.actions.borrow().contains(&AccessibleAction::SetRange(4.0)));
        arqrustui::accessibility::raise_accessible_event(
            &panel,
            11,
            arqrustui::accessibility::AccessibleEvent::Selected,
        );
    });
}

#[test]
fn property_grid_rows_are_selection_items() {
    with_app(|_| {
        let window = test_window();
        let grid = PropertyGrid::new(&window, Rect::new(0, 0, 300, 200)).unwrap();
        grid.add_property("Nome", PropertyValue::Text("x".into()));
        grid.add_property("Largura", PropertyValue::Integer(3));
        let hwnd = grid.handle();
        // SAFETY: janela viva desta thread.
        unsafe {
            assert!(extra_patterns(hwnd, "")[SELECTION]);
            assert!(extra_patterns(hwnd, "Nome")[SELECTION_ITEM]);
            assert_eq!(perform_extra(hwnd, "Largura", "select"), S_OK);
            assert_eq!(selection_names(hwnd, ""), ["Largura"]);
        }
    });
}

#[test]
fn notices_announce_themselves() {
    with_app(|_| {
        let window = test_window();
        window.show();
        let bar = InfoBar::builder()
            .severity(Severity::Error)
            .title("Falhou")
            .visible(false)
            .parent(&window)
            .build()
            .unwrap();
        bar.open();
        // No Windows 10 1709+ (e no Wine 9) o anúncio é aceito; sem a API, Unsupported.
        match bar.announce() {
            Ok(()) => {}
            Err(GuiError::Unsupported(_)) => {}
            Err(other) => panic!("anúncio falhou: {other}"),
        }
        let toast = Toast::new("Salvo").severity(Severity::Success).show(&window).unwrap();
        assert!(toast.is_open());
        toast.close();
    });
}
