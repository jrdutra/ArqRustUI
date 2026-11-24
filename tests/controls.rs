//! Controles: propriedades, eventos e estados sobre janelas nativas reais.

mod common;

use arqrustui::ffi::user32::{WM_CHAR, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE};
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn common_widget_properties() {
    with_app(|_| {
        let window = test_window();
        let button = Button::builder().text("Um").bounds(10, 20, 100, 30).parent(&window).build().unwrap();
        assert_eq!(button.text(), "Um");
        assert_eq!(button.bounds(), Rect::new(10, 20, 100, 30));
        button.set_text("Dois");
        button.set_position(30, 40);
        button.set_size(120, 32);
        assert_eq!((button.text().as_str(), button.bounds()), ("Dois", Rect::new(30, 40, 120, 32)));
        assert!(button.is_enabled());
        button.set_enabled(false);
        assert!(!button.is_enabled());
        button.set_visible(false);
        button.set_font(Font::new("Arial", 12.0).bold());
        assert_eq!(button.font().family(), "Arial");
        button.set_tooltip("Dica do botão").unwrap();
        button.set_tooltip("").unwrap();
        let hidden = Label::builder().text("x").visible(false).enabled(false).parent(&window).build().unwrap();
        assert!(!hidden.is_enabled());
    });
}

#[test]
fn handlers_can_be_removed_and_are_reentrant() {
    with_app(|_| {
        let window = test_window();
        let button = Button::builder().parent(&window).build().unwrap();
        let count = Counter::new();
        let id = button.on_click({
            let count = count.clone();
            move || count.hit()
        });
        // Um manipulador que registra outro e destrói o controle durante o despacho.
        let inner = button.clone();
        let late = Counter::new();
        let late_probe = late.clone();
        button.on_click(move || {
            let late = late_probe.clone();
            inner.on_click(move || late.hit());
        });
        button.click();
        assert_eq!((count.get(), late.get()), (1, 0));
        button.remove_handler(id);
        button.click();
        assert_eq!((count.get(), late.get()), (1, 1));
        let victim = button.clone();
        button.on_click(move || victim.destroy());
        button.click();
        assert!(!button.is_alive());
    });
}

#[test]
fn textbox_text_events_and_editing() {
    with_app(|_| {
        let window = test_window();
        let text = TextBox::builder().text("abc").max_length(10).placeholder("dica").parent(&window).build().unwrap();
        let changes = Counter::new();
        text.on_text_changed({
            let c = changes.clone();
            move || c.hit()
        });
        text.set_text("Olá, ação!");
        assert_eq!(text.text(), "Olá, ação!");
        assert!(changes.get() >= 1);
        text.set_selection(0, 3);
        assert_eq!(text.selection(), (0, 3));
        text.replace_selection("Oi");
        assert_eq!(text.text(), "Oi, ação!");
        text.append("?");
        assert!(text.text().ends_with("!?"));
        // Digitação simulada: o caractere entra pelo mesmo caminho do teclado.
        let before = changes.get();
        text.set_text("");
        send(&text, WM_CHAR, 'z' as usize, 0);
        assert_eq!(text.text(), "z");
        assert!(changes.get() > before);
        assert!(!text.is_read_only());
        text.set_read_only(true);
        assert!(text.is_read_only());

        let multi =
            TextBox::builder().multiline(true).text("a\r\nb\r\nc").size(100, 80).parent(&window).build().unwrap();
        assert_eq!(multi.line_count(), 3);

        let password = PasswordBox::builder().parent(&window).build().unwrap();
        password.set_text("s3gr3do");
        assert_eq!(password.password(), "s3gr3do");
        password.set_revealed(true);
        password.set_revealed(false);
    });
}

#[test]
fn check_radio_and_toggle_states() {
    with_app(|_| {
        let window = test_window();
        let check = CheckBox::builder().text("Aceito").parent(&window).build().unwrap();
        let seen = Rc::new(RefCell::new(Vec::new()));
        check.on_value_changed({
            let seen = seen.clone();
            move |checked| seen.borrow_mut().push(checked)
        });
        assert!(!check.is_checked());
        send(&check, arqrustui::ffi::user32::BM_CLICK, 0, 0);
        send(&check, arqrustui::ffi::user32::BM_CLICK, 0, 0);
        assert_eq!(*seen.borrow(), vec![true, false]);
        check.set_checked(true);
        assert_eq!(seen.borrow().len(), 2, "set_checked não deve disparar o evento");
        assert_eq!(check.state(), CheckState::Checked);

        let tri = CheckBox::builder().three_state(true).parent(&window).build().unwrap();
        tri.set_state(CheckState::Indeterminate);
        assert_eq!(tri.state(), CheckState::Indeterminate);
        send(&tri, arqrustui::ffi::user32::BM_CLICK, 0, 0);
        assert_eq!(tri.state(), CheckState::Unchecked);

        let a = RadioButton::builder().text("A").group_start(true).checked(true).parent(&window).build().unwrap();
        let b = RadioButton::builder().text("B").parent(&window).build().unwrap();
        assert!(a.is_checked() && !b.is_checked());
        send(&b, arqrustui::ffi::user32::BM_CLICK, 0, 0);
        assert!(!a.is_checked() && b.is_checked(), "botões de opção do mesmo grupo são exclusivos");

        let toggle = ToggleButton::builder().text("Negrito").parent(&window).build().unwrap();
        send(&toggle, arqrustui::ffi::user32::BM_CLICK, 0, 0);
        assert!(toggle.is_checked());
    });
}

/// Altura real (em pixels) da janela nativa, sem a lista aberta da ComboBox.
fn native_height(widget: &impl Widget) -> i32 {
    let mut rect = arqrustui::ffi::RECT::default();
    // SAFETY: consulta de um controle vivo da própria thread de teste.
    unsafe { arqrustui::ffi::user32::GetWindowRect(widget.handle() as _, &mut rect) };
    rect.bottom - rect.top
}

#[test]
fn combobox_has_the_same_default_height_as_a_button() {
    with_app(|_| {
        let window = test_window();
        let px = |logical: i32| logical * window.dpi() as i32 / 96;
        let button = Button::builder().text("OK").parent(&window).build().unwrap();
        let combo = ComboBox::builder().items(&["um", "dois"]).selected(0).parent(&window).build().unwrap();
        let editable = ComboBox::builder().editable(true).parent(&window).build().unwrap();
        let with_icons = ComboBox::builder().with_icons(true).parent(&window).build().unwrap();
        with_icons.add_item_with_icon("Pasta", &icons::FOLDER).unwrap();
        let expected = px(DEFAULT_CONTROL_HEIGHT);
        assert_eq!(native_height(&button), expected);
        for (name, combo) in [("lista", &combo), ("editável", &editable), ("com ícones", &with_icons)] {
            assert_eq!(native_height(combo), expected, "ComboBox {name}");
            assert_eq!(combo.bounds().height, DEFAULT_CONTROL_HEIGHT, "ComboBox {name}");
        }
        // A altura pedida vale também depois de redimensionar e de trocar a fonte.
        combo.set_size(200, 34);
        assert_eq!(native_height(&combo), px(34));
        combo.set_font(Font::new("Arial", 14.0));
        assert_eq!(native_height(&combo), px(34));
        // Pedir menos que o texto não corta o texto: o campo para na altura da fonte.
        combo.set_size(200, 4);
        assert!(native_height(&combo) > px(4));
        // Em um layout, as duas ficam com a altura natural e alinhadas.
        let row = Panel::builder().bounds(0, 0, 400, 40).parent(&window).build().unwrap();
        let ok = Button::builder().text("OK").parent(&row).build().unwrap();
        let choice = ComboBox::builder().items(&["a"]).parent(&row).build().unwrap();
        let stack = StackLayout::horizontal().spacing(8);
        stack.add_with(&choice, LayoutParams::new().align_v(Align::Center));
        stack.add_with(&ok, LayoutParams::new().align_v(Align::Center));
        row.set_layout(stack);
        assert_eq!(native_height(&choice), expected);
        assert_eq!(native_height(&choice), native_height(&ok));
    });
}

/// Altura da área cliente (onde o controle desenha o texto), em pixels.
fn client_height(widget: &impl Widget) -> i32 {
    let mut rect = arqrustui::ffi::RECT::default();
    // SAFETY: consulta de um controle vivo da própria thread de teste.
    unsafe { arqrustui::ffi::user32::GetClientRect(widget.handle() as _, &mut rect) };
    rect.bottom - rect.top
}

#[test]
fn single_line_fields_share_the_default_height_and_center_their_text() {
    with_app(|_| {
        let window = test_window();
        let px = |logical: i32| logical * window.dpi() as i32 / 96;
        let text = TextBox::builder().text("abc").parent(&window).build().unwrap();
        let password = PasswordBox::builder().parent(&window).build().unwrap();
        let date = DatePicker::builder().parent(&window).build().unwrap();
        let time = TimePicker::builder().parent(&window).build().unwrap();
        let spin = SpinButton::builder().parent(&window).build().unwrap();
        let check = CheckBox::builder().text("Opção").parent(&window).build().unwrap();
        let heights = [
            ("TextBox", native_height(&text)),
            ("PasswordBox", native_height(&password)),
            ("DatePicker", native_height(&date)),
            ("TimePicker", native_height(&time)),
            ("SpinButton", native_height(&spin)),
            ("CheckBox", native_height(&check)),
        ];
        for (name, height) in heights {
            assert_eq!(height, px(DEFAULT_CONTROL_HEIGHT), "{name}");
        }
        // A área de texto fica do tamanho de uma linha, no meio da caixa.
        for (name, client) in [("TextBox", client_height(&text)), ("DatePicker", client_height(&date))] {
            assert!(client < px(DEFAULT_CONTROL_HEIGHT) - px(6), "{name}: área cliente de {client} px");
        }
        // Continua centralizado depois de trocar a fonte e o tamanho.
        text.set_font(Font::new("Arial", 16.0));
        text.set_size(160, 48);
        assert!(client_height(&text) < px(40));
        text.set_text("novo");
        assert_eq!(text.text(), "novo");
        // Caixas de várias linhas não mudam: o texto começa no alto.
        let multi = TextBox::builder().multiline(true).bounds(0, 60, 160, 80).parent(&window).build().unwrap();
        assert!(client_height(&multi) > px(60));
    });
}

#[test]
fn combobox_and_listbox_items_and_selection() {
    with_app(|_| {
        let window = test_window();
        let combo = ComboBox::builder().items(&["um", "dois", "três"]).selected(0).parent(&window).build().unwrap();
        assert_eq!(combo.count(), 3);
        assert_eq!(combo.selected_text().as_deref(), Some("um"));
        let seen = Rc::new(RefCell::new(Vec::new()));
        combo.on_selection_changed({
            let seen = seen.clone();
            move |index| seen.borrow_mut().push(index)
        });
        // Seta para baixo: seleção feita "pelo usuário" dispara o evento.
        send(&combo, WM_KEYDOWN, Key::DOWN.0 as usize, 0);
        assert_eq!(*seen.borrow(), vec![Some(1)]);
        combo.insert_item(0, "zero");
        combo.remove_item(3);
        assert_eq!(combo.item_text(0).as_deref(), Some("zero"));
        assert_eq!(combo.item_text(99), None);
        combo.set_selected_index(None);
        assert_eq!(combo.selected_index(), None);
        combo.clear();
        assert_eq!(combo.count(), 0);

        let list = ListBox::builder().items(&["a", "b", "c"]).multi_select(true).parent(&window).build().unwrap();
        list.set_item_selected(0, true);
        list.set_item_selected(2, true);
        assert_eq!(list.selected_indices(), vec![0, 2]);
        assert_eq!(list.add_item("d"), 3);
        list.remove_item(1);
        assert_eq!(list.item_text(1).as_deref(), Some("c"));
        let single = ListBox::builder().items(&["x", "y"]).parent(&window).build().unwrap();
        let changed = Counter::new();
        single.on_selection_changed({
            let c = changed.clone();
            move |_| c.hit()
        });
        send(&single, WM_KEYDOWN, Key::DOWN.0 as usize, 0);
        assert_eq!(single.selected_index(), Some(0));
        assert_eq!(changed.get(), 1);
    });
}

#[test]
fn range_controls_values_and_events() {
    with_app(|app| {
        let window = test_window();
        let progress = ProgressBar::builder().range(0, 50).value(10).parent(&window).build().unwrap();
        progress.advance(15);
        assert_eq!(progress.value(), 25);
        progress.set_value(999);
        assert_eq!(progress.value(), 50);
        progress.set_state(ProgressState::Paused);
        progress.set_marquee(true);

        let slider = Slider::builder().range(0, 10).value(3).ticks(1).parent(&window).build().unwrap();
        let values = Rc::new(RefCell::new(Vec::new()));
        slider.on_value_changed({
            let values = values.clone();
            move |v| values.borrow_mut().push(v)
        });
        send(&slider, WM_KEYDOWN, Key::RIGHT.0 as usize, 0);
        assert_eq!(slider.value(), 4);
        assert_eq!(values.borrow().last(), Some(&4));

        let scroll = ScrollBar::builder().range(0, 100).page(10).value(5).parent(&window).build().unwrap();
        assert_eq!(scroll.value(), 5);
        scroll.set_value(40);
        assert_eq!(scroll.value(), 40);

        let spin = SpinButton::builder().range(1, 9).value(5).parent(&window).build().unwrap();
        assert_eq!(spin.value(), 5);
        spin.set_value(42);
        assert_eq!(spin.value(), 9, "o valor deve ser limitado ao intervalo");
        assert_eq!(spin.text(), "9");
        let updown = UpDown::builder().parent(&window).build().unwrap();
        updown.set_range(0, 3);
        updown.set_value(2);
        assert_eq!(updown.value(), 2);
        app.process_pending().unwrap();
    });
}

#[test]
fn date_and_time_controls() {
    with_app(|_| {
        let window = test_window();
        let date = DatePicker::builder().parent(&window).build().unwrap();
        let wanted = DateTime::new(2026, 11, 12, 0, 0, 0);
        date.set_value(&wanted);
        let got = date.value().expect("data");
        assert_eq!((got.year, got.month, got.day), (2026, 11, 12));
        date.set_format("dd/MM/yyyy");
        let time = TimePicker::builder().parent(&window).build().unwrap();
        time.set_value(&DateTime::new(2026, 1, 1, 13, 45, 0));
        let got = time.value().expect("hora");
        assert_eq!((got.hour, got.minute), (13, 45));
        let calendar = MonthCalendar::builder().parent(&window).build().unwrap();
        calendar.set_selected_date(&DateTime::date(2025, 2, 28));
        assert_eq!(calendar.selected_date(), Some(DateTime::date(2025, 2, 28)));
        let minimum = calendar.minimum_size();
        assert!(minimum.width > 50 && minimum.height > 50);
    });
}

#[test]
fn list_view_rows_columns_selection_and_sort() {
    with_app(|_| {
        let window = test_window();
        let list = ListView::builder().checkboxes(true).parent(&window).build().unwrap();
        assert_eq!(list.add_column("Nome", 120, ColumnAlign::Left), 0);
        assert_eq!(list.add_column("Qtd", 60, ColumnAlign::Right), 1);
        assert_eq!(list.column_count(), 2);
        assert_eq!(list.column_width(0), 120);
        for (name, quantity) in [("pera", "10"), ("uva", "9"), ("abacate", "100")] {
            list.add_row(&[name, quantity]);
        }
        assert_eq!(list.row_count(), 3);
        assert_eq!(list.cell(1, 0).as_deref(), Some("uva"));
        assert_eq!(list.cell(9, 0), None);
        list.set_cell(1, 1, "7");
        list.sort_by_column(1, true);
        let order: Vec<String> = (0..3).map(|row| list.cell(row, 0).unwrap()).collect();
        assert_eq!(order, ["uva", "pera", "abacate"], "ordenação numérica");
        list.sort_by_column(0, false);
        assert_eq!(list.cell(0, 0).as_deref(), Some("uva"));

        let selections = Rc::new(RefCell::new(Vec::new()));
        list.on_selection_changed({
            let selections = selections.clone();
            move |rows| selections.borrow_mut().push(rows)
        });
        list.set_row_selected(0, true);
        list.set_row_selected(2, true);
        assert_eq!(list.selected_rows(), vec![0, 2]);
        assert_eq!(selections.borrow().last(), Some(&vec![0, 2]));
        list.set_row_checked(1, true);
        assert!(list.is_row_checked(1) && !list.is_row_checked(0));
        let images = ImageList::new(16).unwrap();
        let index = images.add_icon(&Icon::system(SystemIcon::Warning).unwrap()).unwrap();
        list.set_image_list(&images);
        list.set_row_image(0, index);
        list.remove_row(0);
        assert_eq!(list.row_count(), 2);
        list.clear();
        assert_eq!(list.row_count(), 0);
    });
}

#[test]
fn tree_view_hierarchy() {
    with_app(|_| {
        let window = test_window();
        let tree = TreeView::builder().checkboxes(true).parent(&window).build().unwrap();
        let root = tree.add_root("Projeto").unwrap();
        let src = tree.add_child(root, "src").unwrap();
        let main = tree.add_child(src, "main.rs").unwrap();
        tree.add_child(src, "lib.rs").unwrap();
        tree.add_root("Outro").unwrap();
        assert_eq!(tree.count(), 5);
        assert_eq!(tree.roots().len(), 2);
        assert_eq!(tree.children(src).len(), 2);
        assert_eq!(tree.parent_of(main), Some(src));
        assert_eq!(tree.parent_of(root), None);
        assert_eq!(tree.item_path(main), ["Projeto", "src", "main.rs"]);
        tree.set_item_text(main, "principal.rs");
        assert_eq!(tree.item_text(main), "principal.rs");
        let selected = Rc::new(RefCell::new(None));
        tree.on_selection_changed({
            let selected = selected.clone();
            move |item| *selected.borrow_mut() = item
        });
        tree.expand(root);
        tree.select(Some(main));
        assert_eq!(tree.selected(), Some(main));
        assert_eq!(*selected.borrow(), Some(main));
        tree.set_checked(src, true);
        assert!(tree.is_checked(src) && !tree.is_checked(main));
        tree.remove(src);
        assert_eq!(tree.count(), 2);
        tree.clear();
        assert_eq!(tree.count(), 0);
    });
}

#[test]
fn tab_control_pages() {
    with_app(|_| {
        let window = test_window();
        let tabs = TabControl::builder().bounds(0, 0, 300, 200).parent(&window).build().unwrap();
        let first = tabs.add_tab("Geral").unwrap();
        let second = tabs.add_tab("Avançado").unwrap();
        let label = Label::builder().text("na segunda").parent(&second).build().unwrap();
        assert_eq!(tabs.tab_count(), 2);
        assert_eq!(tabs.tab_title(1), "Avançado");
        assert_eq!(tabs.selected_index(), Some(0));
        let style = |w: &Panel| arqrustui::ffi::user32::WS_VISIBLE & unsafe_style(w) != 0;
        assert!(style(&first) && !style(&second));
        tabs.set_selected_index(1);
        assert!(!style(&first) && style(&second));
        let page = second.bounds();
        assert!(page.width > 200 && page.height > 100 && page.y > 0, "página mal posicionada: {page:?}");
        tabs.set_size(400, 300);
        assert!(second.bounds().width > page.width, "as páginas devem acompanhar o redimensionamento");
        tabs.set_tab_title(0, "Início");
        assert_eq!(tabs.tab_title(0), "Início");
        tabs.remove_tab(1);
        assert!(!second.is_alive() && !label.is_alive());
        assert_eq!(tabs.tab_count(), 1);
        assert!(tabs.page(0).is_some() && tabs.page(1).is_none());
    });
}

fn unsafe_style(widget: &impl Widget) -> u32 {
    // SAFETY: leitura do estilo de uma janela viva da thread de teste.
    unsafe { arqrustui::ffi::user32::GetWindowLongPtrW(widget.handle() as _, arqrustui::ffi::user32::GWL_STYLE) as u32 }
}

#[test]
fn data_grid_memory_virtual_sort_and_edit() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let grid = DataGrid::builder().editable(true).bounds(0, 0, 380, 200).parent(&window).build().unwrap();
        grid.add_column("Nome", 150, ColumnAlign::Left);
        grid.add_column("Valor", 100, ColumnAlign::Right);
        grid.add_row(&["b", "20"]);
        grid.add_row(&["a", "100"]);
        grid.add_row(&["c", "3"]);
        assert_eq!(grid.row_count(), 3);
        grid.sort_by(1, true);
        assert_eq!(grid.rows().iter().map(|r| r[0].as_str()).collect::<Vec<_>>(), ["c", "b", "a"]);
        assert_eq!(grid.sort_state(), Some((1, true)));

        let edits = Rc::new(RefCell::new(Vec::new()));
        grid.on_cell_edited({
            let edits = edits.clone();
            move |row, column, text| edits.borrow_mut().push((row, column, text.to_string()))
        });
        grid.set_validator(|_, column, text| column != 1 || text.parse::<i32>().is_ok());
        grid.set_column_read_only(0, true);
        assert!(!grid.begin_edit(0, 0), "coluna somente leitura");
        assert!(grid.begin_edit(0, 1));
        assert!(grid.is_editing());
        grid.cancel_edit();
        assert!(!grid.is_editing() && edits.borrow().is_empty());
        assert_eq!(grid.cell(0, 1).as_deref(), Some("3"));
        // Sem editor aberto, confirmar é um no-op bem-sucedido.
        assert!(grid.commit_edit());
        app.process_pending().unwrap();

        // Fonte virtual: um milhão de linhas sem alocá-las.
        let requested = Counter::new();
        let probe = requested.clone();
        grid.set_virtual_source(1_000_000, move |row, column| {
            probe.hit();
            format!("L{row}C{column}")
        });
        assert_eq!(grid.row_count(), 1_000_000);
        assert_eq!(grid.cell(999_999, 1).as_deref(), Some("L999999C1"));
        assert!(!grid.begin_edit(0, 1), "fonte virtual não é editável");
        app.process_pending().unwrap();
        assert!(requested.get() < 10_000, "a virtualização não deve consultar todas as linhas");
        grid.set_row_selected(5, true);
        assert_eq!(grid.selected_rows(), vec![5]);
        grid.clear();
        assert_eq!(grid.row_count(), 0);
    });
}

#[test]
fn property_grid_typed_values() {
    with_app(|_| {
        let window = test_window();
        let grid = PropertyGrid::new(&window, Rect::new(0, 0, 300, 200)).unwrap();
        grid.add_category("Geral");
        grid.add_property("Nome", PropertyValue::Text("x".into()));
        grid.add_property("Ativo", PropertyValue::Bool(false));
        grid.add_property("Modo", PropertyValue::Choice { options: vec!["A".into(), "B".into()], selected: 0 });
        let changes = Rc::new(RefCell::new(Vec::new()));
        grid.on_property_changed({
            let changes = changes.clone();
            move |name, value| changes.borrow_mut().push((name.to_string(), value.clone()))
        });
        assert_eq!(grid.property_names(), ["Nome", "Ativo", "Modo"]);
        grid.set_choice_mode(ChoiceMode::Cycle);
        assert!(grid.activate("Ativo"));
        assert_eq!(grid.value("Ativo"), Some(PropertyValue::Bool(true)));
        assert!(grid.activate("Modo"));
        assert!(grid.activate("Modo"));
        assert_eq!(
            grid.value("Modo"),
            Some(PropertyValue::Choice { options: vec!["A".into(), "B".into()], selected: 0 })
        );
        assert_eq!(changes.borrow().len(), 3);
        assert!(grid.set_value("Nome", PropertyValue::Text("novo".into())));
        assert_eq!(changes.borrow().len(), 3, "set_value não dispara o evento");
        assert!(!grid.set_value("Inexistente", PropertyValue::Bool(true)));
        assert!(!grid.activate("Geral"), "categorias não são editáveis");
        grid.clear();
        assert!(grid.property_names().is_empty());
    });
}

#[test]
fn split_container_and_custom_panel_events() {
    with_app(|_| {
        let window = test_window();
        let split = SplitContainer::builder()
            .bounds(0, 0, 400, 200)
            .split_position(100)
            .min_sizes(50, 50)
            .parent(&window)
            .build()
            .unwrap();
        assert_eq!(split.first().bounds(), Rect::new(0, 0, 100, 200));
        assert_eq!(split.second().bounds(), Rect::new(106, 0, 294, 200));
        split.set_split_position(10);
        assert_eq!(split.split_position(), 50, "respeita o tamanho mínimo");
        let moved = Rc::new(RefCell::new(Vec::new()));
        split.on_split_moved({
            let moved = moved.clone();
            move |position| moved.borrow_mut().push(position)
        });
        // Arraste simulado do divisor: pressiona em x=52 e move até x=203.
        // As mensagens de mouse carregam pixels físicos: converte as coordenadas lógicas pelo DPI.
        let dpi = split.dpi() as i32;
        let lparam = |x: i32, y: i32| (((y * dpi / 96) << 16) | ((x * dpi / 96) & 0xFFFF)) as isize;
        send(&split, WM_LBUTTONDOWN, 1, lparam(52, 10));
        send(&split, WM_MOUSEMOVE, 1, lparam(203, 10));
        send(&split, WM_LBUTTONUP, 0, lparam(203, 10));
        assert_eq!(split.split_position(), 200);
        assert_eq!(moved.borrow().last(), Some(&200));
        split.set_size(300, 200);
        assert_eq!(split.second().bounds().width, 94);

        let panel = Panel::builder().bounds(0, 0, 50, 50).focusable(true).parent(&window).build().unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        panel.on_event({
            let events = events.clone();
            move |event| events.borrow_mut().push(event.kind())
        });
        send(&panel, WM_MOUSEMOVE, 0, lparam(5, 5));
        send(&panel, WM_LBUTTONDOWN, 1, lparam(5, 5));
        send(&panel, WM_LBUTTONUP, 0, lparam(5, 5));
        send(&panel, WM_KEYDOWN, Key::F5.0 as usize, 0);
        let seen = events.borrow();
        for expected in [
            EventKind::MouseEnter,
            EventKind::MouseMove,
            EventKind::MouseDown,
            EventKind::MouseUp,
            EventKind::Click,
            EventKind::KeyDown,
        ] {
            assert!(seen.contains(&expected), "evento ausente: {expected:?} em {seen:?}");
        }
    });
}

#[test]
fn bars_and_misc_native_controls() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let status = StatusBar::new(&window).unwrap();
        status.set_parts(&[100, -1]);
        status.set_part_text(0, "Pronto");
        status.set_part_text(1, "Linha 1");
        assert_eq!(status.part_text(0), "Pronto");
        assert!(status.height() > 10);

        let toolbar = ToolBar::new(&window).unwrap();
        let clicks = Counter::new();
        let save = toolbar
            .add_button("Salvar", Some(&Icon::system(SystemIcon::Information).unwrap()), {
                let c = clicks.clone();
                move || c.hit()
            })
            .unwrap();
        toolbar.add_separator().unwrap();
        let toggled = Rc::new(RefCell::new(Vec::new()));
        let bold = toolbar
            .add_toggle_button("N", None, {
                let toggled = toggled.clone();
                move |on| toggled.borrow_mut().push(on)
            })
            .unwrap();
        assert_eq!(toolbar.button_count(), 3);
        // Um clique na barra chega como WM_COMMAND à janela pai, com o HWND da barra.
        send(&window, arqrustui::ffi::user32::WM_COMMAND, save as usize, toolbar.handle());
        assert_eq!(clicks.get(), 1);
        toolbar.set_button_checked(bold, true);
        toolbar.trigger(bold);
        assert_eq!(*toggled.borrow(), vec![true]);
        toolbar.set_button_enabled(save, false);
        assert!(!toolbar.is_button_enabled(save));
        assert!(toolbar.height() > 10);

        let link = Hyperlink::builder()
            .text("Veja <a href=\"https://example.com\">o site</a>")
            .parent(&window)
            .build()
            .unwrap();
        assert!(link.is_alive());
        let header = HeaderControl::builder().parent(&window).build().unwrap();
        header.add_item("A", 80);
        header.add_item("B", 120);
        assert_eq!((header.item_count(), header.item_width(1)), (2, 120));
        header.set_sort_indicator(0, Some(true));
        let hotkey = HotKeyBox::builder().parent(&window).build().unwrap();
        let shortcut = Shortcut::ctrl_shift(Key::from_char('K').unwrap());
        hotkey.set_shortcut(Some(shortcut));
        assert_eq!(hotkey.shortcut(), Some(shortcut));
        let ip = IpAddressBox::builder().parent(&window).build().unwrap();
        assert_eq!(ip.address(), None);
        ip.set_address(std::net::Ipv4Addr::new(192, 168, 0, 10));
        assert_eq!(ip.address(), Some(std::net::Ipv4Addr::new(192, 168, 0, 10)));
        let animation = Animation::builder().parent(&window).build().unwrap();
        assert!(animation.open("Z:\\inexistente.avi").is_err());
        let group = GroupBox::builder().text("Grupo").parent(&window).build().unwrap();
        let inside = CheckBox::builder().text("dentro").parent(&group).build().unwrap();
        let inner_clicks = Counter::new();
        inside.on_click({
            let c = inner_clicks.clone();
            move || c.hit()
        });
        send(&inside, arqrustui::ffi::user32::BM_CLICK, 0, 0);
        assert_eq!(inner_clicks.get(), 1, "eventos de filhos de contêineres devem ser refletidos");
        let pager = Pager::builder().horizontal(true).parent(&window).build().unwrap();
        let wide = Panel::builder().size(800, 30).parent(&pager).build().unwrap();
        pager.set_child(&wide).unwrap();
        let rebar = Rebar::new(&window).unwrap();
        let combo = ComboBox::builder().parent(&rebar).build().unwrap();
        rebar.add_band("Filtro", &combo, false).unwrap();
        assert_eq!(rebar.band_count(), 1);
        app.process_pending().unwrap();
    });
}

#[test]
fn rich_text_formatting_and_rtf() {
    with_app(|_| {
        let window = test_window();
        let rich = RichTextBox::builder().parent(&window).build().unwrap();
        rich.set_text("Olá mundo");
        rich.set_selection(0, 3);
        rich.format_selection(&TextFormat::new().bold(true).color(Color::RED).size(14.0));
        assert_eq!(rich.selection_style(), (true, false, false));
        rich.set_selection(4, 9);
        assert!(!rich.selection_style().0);
        let rtf = rich.rtf();
        assert!(rtf.starts_with("{\\rtf"), "RTF inesperado: {rtf}");
        assert!(rtf.contains("\\b"));
        rich.set_rtf("{\\rtf1 {\\i texto} simples}").unwrap();
        assert_eq!(rich.text().trim_end(), "texto simples");
        assert_eq!(rich.find("SIMPLES", 0, false), Some(6));
        assert_eq!(rich.find("SIMPLES", 0, true), None);
        assert_eq!(rich.selection(), (6, 13));
        rich.set_alignment(TextAlign::Center);
    });
}
