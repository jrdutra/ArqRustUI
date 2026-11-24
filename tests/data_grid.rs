//! DataGrid: filtros, agrupamento, estilos por célula, caixas de seleção, listas de
//! opções, edição em fonte virtual, editor que acompanha a rolagem, exportação e
//! reordenação de várias linhas; PropertyGrid com lista suspensa e rótulos próprios.

mod common;

use arqrustui::ffi::comctl32::{LVM_GETITEMRECT, LVM_SCROLL, LVN_BEGINDRAG, NMLISTVIEW};
use arqrustui::ffi::user32::{SendMessageW, NMHDR, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NOTIFY};
use arqrustui::ffi::{POINT, RECT};
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::rc::Rc;

const MK_LBUTTON: usize = 1;

fn lparam(x: i32, y: i32) -> isize {
    ((y as u16 as isize) << 16) | x as u16 as isize
}

fn names(grid: &DataGrid) -> Vec<String> {
    grid.rows().into_iter().map(|r| r[0].clone()).collect()
}

fn grid_with(window: &Window, rows: &[[&str; 3]]) -> DataGrid {
    let grid = DataGrid::builder().editable(true).bounds(0, 0, 380, 220).parent(window).build().unwrap();
    grid.add_column("Nome", 120, ColumnAlign::Left);
    grid.add_column("Grupo", 100, ColumnAlign::Left);
    grid.add_column("Ativo", 60, ColumnAlign::Center);
    grid.set_rows(rows.iter().map(|r| r.iter().map(|c| c.to_string()).collect()).collect());
    grid
}

fn edits_of(grid: &DataGrid) -> Rc<RefCell<Vec<(usize, usize, String)>>> {
    let edits = Rc::new(RefCell::new(Vec::new()));
    grid.on_cell_edited({
        let edits = edits.clone();
        move |row, column, text| edits.borrow_mut().push((row, column, text.to_string()))
    });
    edits
}

#[test]
fn filters_and_groups_change_the_visible_rows() {
    with_app(|app| {
        let window = test_window();

        let grid =
            grid_with(&window, &[["ana", "A", "true"], ["bia", "B", "false"], ["caio", "A", "1"], ["davi", "C", "0"]]);
        grid.set_text_filter("a", Some(0));
        assert_eq!(names(&grid), ["ana", "bia", "caio", "davi"]);
        grid.set_text_filter("ai", None);
        assert_eq!(names(&grid), ["caio"]);
        assert_eq!(grid.total_row_count(), 4);
        grid.set_filter(Some(|cells: &[String]| cells[1] != "C"));
        assert_eq!(grid.row_count(), 3);
        assert!(grid.has_filter());
        assert_eq!(grid.add_row(&["edu", "B", "x"]), 3);
        grid.set_filter(None::<fn(&[String]) -> bool>);
        assert_eq!(names(&grid), ["ana", "bia", "caio", "davi", "edu"]);

        grid.set_group_by(Some(1));
        app.process_pending().unwrap();
        assert_eq!(names(&grid), ["ana", "caio", "bia", "edu", "davi"]);
        assert_eq!(
            grid.groups(),
            [("A".to_string(), 2, false), ("B".to_string(), 2, false), ("C".to_string(), 1, false)]
        );
        // Linha 2 ("bia") é o 5º item do ListView (cabeçalho A, ana, caio, cabeçalho B, bia).
        grid.set_row_selected(2, true);
        assert_eq!(grid.selected_rows(), [2]);
        let toggled = Rc::new(RefCell::new(Vec::new()));
        grid.on_group_toggled({
            let toggled = toggled.clone();
            move |key, collapsed| toggled.borrow_mut().push((key.to_string(), collapsed))
        });
        grid.set_group_collapsed("A", true);
        assert_eq!(grid.groups()[0], ("A".to_string(), 2, true));
        assert_eq!(grid.row_count(), 5, "linhas de grupos recolhidos continuam existindo");
        assert_eq!(grid.cell(0, 0).as_deref(), Some("ana"));
        assert!(!grid.begin_edit(0, 0), "linha recolhida não abre editor");
        grid.set_group_by(None);
        assert!(grid.groups().is_empty());
        assert_eq!(names(&grid), ["ana", "bia", "caio", "davi", "edu"]);
    });
}

#[test]
fn styles_check_boxes_and_choice_columns() {
    with_app(|app| {
        let window = test_window();

        let grid = grid_with(&window, &[["ana", "A", "true"], ["bia", "B", "false"]]);
        let edits = edits_of(&grid);
        grid.set_row_style(0, Some(CellStyle::new().background(Color::YELLOW)));
        grid.set_cell_style(0, 1, CellStyle::new().foreground(Color::RED).bold());
        let style = grid.cell_style(0, 1);
        assert_eq!((style.foreground, style.background, style.bold), (Some(Color::RED), Some(Color::YELLOW), true));
        grid.set_cell_style_provider(Some(|_row: usize, column: usize, text: &str| {
            (column == 0 && text.starts_with('b')).then(|| CellStyle::new().italic())
        }));
        grid.sort_by(0, false);
        assert_eq!(names(&grid), ["bia", "ana"]);
        assert_eq!(grid.cell_style(1, 1).foreground, Some(Color::RED), "estilo acompanha a linha");
        window.invalidate();
        app.process_pending().unwrap();

        grid.set_column_kind(2, ColumnKind::CheckBox);
        assert_eq!(grid.column_kind(2), ColumnKind::CheckBox);
        assert!(grid.toggle_check(0, 2));
        assert_eq!(grid.cell(0, 2).as_deref(), Some("true"));
        assert!(!grid.toggle_check(0, 0), "coluna de texto não alterna");
        assert!(grid.begin_edit(1, 2), "editar uma caixa de seleção alterna");
        assert_eq!(grid.cell(1, 2).as_deref(), Some("false"));
        assert!(!grid.is_editing());
        app.process_pending().unwrap();

        grid.set_column_kind(1, ColumnKind::Choice(vec!["A".into(), "B".into(), "C".into()]));
        assert!(grid.begin_edit(0, 1));
        assert!(grid.is_editing());
        assert_eq!(grid.editing_cell(), Some((0, 1)));
        let editor = child_of_class(&grid, "ComboBox").expect("lista suspensa");
        send_raw(editor, 0x014E /* CB_SETCURSEL */, 2, 0);
        assert!(grid.commit_edit());
        assert_eq!(grid.cell(0, 1).as_deref(), Some("C"));
        assert_eq!(*edits.borrow(), [(0, 2, "true".to_string()), (1, 2, "false".to_string()), (0, 1, "C".to_string())]);
    });
}

#[test]
fn virtual_sources_are_editable_through_the_application() {
    with_app(|_| {
        let window = test_window();

        let grid = DataGrid::builder().editable(true).bounds(0, 0, 300, 200).parent(&window).build().unwrap();
        grid.add_column("Valor", 120, ColumnAlign::Left);
        grid.add_column("Marcado", 60, ColumnAlign::Center);
        grid.set_column_kind(1, ColumnKind::CheckBox);
        let store = Rc::new(RefCell::new(vec![("zero".to_string(), false); 50]));
        let read = store.clone();
        grid.set_virtual_source(50, move |row, column| {
            let data = read.borrow();
            if column == 0 {
                data[row].0.clone()
            } else {
                data[row].1.to_string()
            }
        });
        assert!(!grid.begin_edit(3, 0), "sem editor virtual a fonte é somente leitura");
        let write = store.clone();
        grid.set_virtual_editor(move |row, column, text| {
            if text.is_empty() {
                return false;
            }
            let mut data = write.borrow_mut();
            if column == 0 {
                data[row].0 = text.to_string();
            } else {
                data[row].1 = parse_check(text);
            }
            true
        });
        let edits = edits_of(&grid);
        assert!(grid.begin_edit(3, 0));
        let editor = child_of_class(&grid, "Edit").expect("editor");
        set_native_text(editor, "três");
        assert!(grid.commit_edit());
        assert_eq!(store.borrow()[3].0, "três");
        assert_eq!(grid.cell(3, 0).as_deref(), Some("três"));
        assert!(grid.begin_edit(4, 0));
        set_native_text(child_of_class(&grid, "Edit").expect("editor"), "");
        assert!(!grid.commit_edit(), "o editor virtual recusou");
        assert_eq!(store.borrow()[4].0, "zero");
        assert!(grid.toggle_check(7, 1));
        assert!(store.borrow()[7].1);
        assert_eq!(edits.borrow().len(), 2);
        grid.set_virtual_row_count(10);
        assert_eq!(grid.row_count(), 10);
    });
}

#[test]
fn editor_follows_scrolling() {
    with_app(|app| {
        let window = test_window();
        window.show();
        let grid = DataGrid::builder().editable(true).bounds(0, 0, 300, 160).parent(&window).build().unwrap();
        grid.add_column("Nome", 200, ColumnAlign::Left);
        grid.set_rows((0..60).map(|i| vec![format!("linha {i}")]).collect());
        assert!(grid.begin_edit(1, 0));
        let editor = child_of_class(&grid, "Edit").expect("editor");
        let before = native_rect(editor);
        // SAFETY: rolagem de uma janela viva da thread de teste.
        unsafe { SendMessageW(grid.handle() as _, LVM_SCROLL, 0, 40) };
        // SAFETY: idem (WM_VSCROLL com SB_LINEDOWN) — dispara o acompanhamento do editor.
        unsafe { SendMessageW(grid.handle() as _, 0x0115, 1, 0) };
        app.process_pending().unwrap();
        let after = native_rect(editor);
        // SAFETY: consulta simples de uma janela viva.
        let visible = unsafe { arqrustui::ffi::user32::IsWindowVisible(editor as _) } != 0;
        assert!(after.top < before.top || !visible, "editor acompanhou a rolagem: {} → {}", before.top, after.top);
        assert!(grid.is_editing());
        grid.cancel_edit();
        assert!(!grid.is_editing());
    });
}

#[test]
fn export_to_csv_tsv_and_html() {
    with_app(|_| {
        let window = test_window();
        let grid = grid_with(&window, &[["ana; maria", "A", "true"], ["bi\"a", "<B>", "false"]]);
        grid.set_cell_style(1, 0, CellStyle::new().foreground(Color::rgb(255, 0, 0)).bold());
        let csv = grid.to_csv(';');
        assert_eq!(csv, "Nome;Grupo;Ativo\r\n\"ana; maria\";A;true\r\n\"bi\"\"a\";<B>;false\r\n");
        assert!(grid.to_tsv().starts_with("Nome\tGrupo\tAtivo\r\nana; maria\tA\ttrue"));
        let html = grid.to_html();
        assert!(html.contains("<th>Grupo</th>"));
        assert!(html.contains("<td>&lt;B&gt;</td>"));
        assert!(html.contains("<td style=\"color:#ff0000;font-weight:bold;\">bi&quot;a</td>"));
        let path = std::env::temp_dir().join("arqrustui-grade.csv");
        grid.export_csv(&path, ',').unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
        assert!(String::from_utf8_lossy(&bytes[3..]).starts_with("Nome,Grupo,Ativo"));
        let _ = std::fs::remove_file(path);
        grid.set_text_filter("bi", None);
        assert_eq!(grid.to_csv(',').lines().count(), 2, "exporta só as linhas exibidas");
    });
}

fn begin_drag(window: &Window, grid: &DataGrid, item: i32) {
    let mut rect = RECT { left: 0, ..RECT::default() };
    // SAFETY: geometria de uma linha da grade viva, num RECT local.
    unsafe { SendMessageW(grid.handle() as _, LVM_GETITEMRECT, item as usize, &mut rect as *mut RECT as isize) };
    let mut info = NMLISTVIEW {
        hdr: NMHDR { hwndFrom: grid.handle() as _, idFrom: 0, code: LVN_BEGINDRAG as u32 },
        iItem: item,
        iSubItem: 0,
        uNewState: 0,
        uOldState: 0,
        uChanged: 0,
        ptAction: POINT { x: 20, y: (rect.top + rect.bottom) / 2 },
        lParam: 0,
    };
    // SAFETY: notificação local, válida durante o envio síncrono à janela do teste.
    unsafe { SendMessageW(window.handle() as _, WM_NOTIFY, 0, &mut info as *mut NMLISTVIEW as isize) };
}

fn bottom_of(grid: &DataGrid, item: usize) -> i32 {
    let mut rect = RECT { left: 0, ..RECT::default() };
    // SAFETY: geometria de uma linha da grade viva, num RECT local.
    unsafe { SendMessageW(grid.handle() as _, LVM_GETITEMRECT, item, &mut rect as *mut RECT as isize) };
    rect.bottom
}

#[test]
fn several_selected_rows_move_together() {
    with_app(|_| {
        let window = test_window();
        window.show();
        let grid = grid_with(&window, &[["a", "", ""], ["b", "", ""], ["c", "", ""], ["d", "", ""], ["e", "", ""]]);
        grid.set_reorderable(true);
        let blocks = Rc::new(RefCell::new(Vec::new()));
        grid.on_rows_reorder_request({
            let blocks = blocks.clone();
            move |rows, slot| {
                blocks.borrow_mut().push((rows.to_vec(), slot));
                true
            }
        });
        grid.set_row_selected(0, true);
        grid.set_row_selected(2, true);
        begin_drag(&window, &grid, 0);
        let y = bottom_of(&grid, 4) - 2;
        send(&grid, WM_MOUSEMOVE, MK_LBUTTON, lparam(20, y));
        send(&grid, WM_LBUTTONUP, 0, lparam(20, y));
        assert_eq!(*blocks.borrow(), [(vec![0, 2], 5)]);
        assert_eq!(names(&grid), ["b", "d", "e", "a", "c"]);
        assert_eq!(grid.selected_rows(), [3, 4], "as linhas movidas continuam selecionadas");
        assert_eq!(grid.move_rows(&[3, 4], 0), Some(0));
        assert_eq!(names(&grid), ["a", "c", "b", "d", "e"]);
    });
}

#[test]
fn list_box_moves_a_multiple_selection_as_a_block() {
    with_app(|_| {
        let window = test_window();
        let list = ListBox::builder().multi_select(true).bounds(0, 0, 200, 200).parent(&window).build().unwrap();
        for item in ["um", "dois", "três", "quatro"] {
            list.add_item(item);
        }
        list.set_reorderable(true);
        list.set_drag_out_effects(&[DropEffect::Copy, DropEffect::Move]);
        list.on_drag_out_finished(|_, _| {});
        list.set_item_selected(0, true);
        list.set_item_selected(1, true);
        let height = send(&list, 0x01A1 /* LB_GETITEMHEIGHT */, 0, 0) as i32;
        // Pressionar sobre a seleção não a desfaz; arrastar leva o bloco para o fim.
        send(&list, 0x0201 /* WM_LBUTTONDOWN */, MK_LBUTTON, lparam(10, height / 2));
        assert_eq!(list.selected_indices(), [0, 1]);
        send(&list, WM_MOUSEMOVE, MK_LBUTTON, lparam(10, height / 2 + 12));
        send(&list, WM_MOUSEMOVE, MK_LBUTTON, lparam(10, height * 4 - 2));
        send(&list, WM_LBUTTONUP, 0, lparam(10, height * 4 - 2));
        let items: Vec<String> = (0..list.count()).filter_map(|i| list.item_text(i)).collect();
        assert_eq!(items, ["três", "quatro", "um", "dois"]);
        assert_eq!(list.selected_indices(), [2, 3]);
        // Clique simples (sem arrastar) sobre a seleção: seleciona só o item.
        send(&list, 0x0201 /* WM_LBUTTONDOWN */, MK_LBUTTON, lparam(10, height * 2 + height / 2));
        send(&list, WM_LBUTTONUP, 0, lparam(10, height * 2 + height / 2));
        assert_eq!(list.selected_indices(), [2]);
    });
}

#[test]
fn property_grid_opens_a_drop_down_for_choices_and_uses_custom_labels() {
    with_app(|_| {
        let window = test_window();

        let grid = PropertyGrid::new(&window, Rect::new(0, 0, 300, 200)).unwrap();
        grid.add_property("Ativo", PropertyValue::Bool(true));
        grid.add_property("Modo", PropertyValue::Choice { options: vec!["A".into(), "B".into()], selected: 0 });
        let changes = Rc::new(RefCell::new(Vec::new()));
        grid.on_property_changed({
            let changes = changes.clone();
            move |name, value| changes.borrow_mut().push((name.to_string(), value.clone()))
        });
        grid.set_bool_labels(Some(("Ligado", "Desligado")));
        assert!(grid.activate("Modo"));
        assert!(grid.is_editing(), "Choice abre a lista suspensa");
        let combo = child_of_class(&grid, "ComboBox").expect("lista");
        send_raw(combo, 0x014E /* CB_SETCURSEL */, 1, 0);
        // Enter confirma a escolha.
        send_raw(combo, 0x0100 /* WM_KEYDOWN */, 0x0D, 0);
        assert!(!grid.is_editing());
        assert_eq!(
            grid.value("Modo"),
            Some(PropertyValue::Choice { options: vec!["A".into(), "B".into()], selected: 1 })
        );
        assert_eq!(changes.borrow().len(), 1);
        assert!(grid.activate("Ativo"));
        assert_eq!(grid.value("Ativo"), Some(PropertyValue::Bool(false)));
    });
}
