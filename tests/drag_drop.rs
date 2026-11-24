//! Arrastar e soltar: dados OLE, destinos e reordenação de listas e árvores.

mod common;

use arqrustui::ffi::comctl32::{NMLISTVIEW, NMTREEVIEWW, TVITEMW};
use arqrustui::ffi::user32::{SendMessageW, NMHDR};
use arqrustui::ffi::{POINT, RECT};
use arqrustui::prelude::*;
use common::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

const WM_NOTIFY: u32 = 0x004E;
const WM_KEYDOWN: u32 = 0x0100;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const MK_LBUTTON: usize = 1;
const LB_GETITEMHEIGHT: u32 = 0x01A1;
const LVM_GETITEMRECT: u32 = 0x100E;
const TVM_GETITEMRECT: u32 = 0x1104;
const LVN_BEGINDRAG: i32 = -109;
const TVN_BEGINDRAGW: i32 = -456;

fn lparam(x: i32, y: i32) -> isize {
    ((y as u16 as isize) << 16) | x as u16 as isize
}

fn listbox_items(list: &ListBox) -> Vec<String> {
    (0..list.count()).map(|i| list.item_text(i).unwrap()).collect()
}

/// Arrasta, com mensagens de mouse, o item 0 até o ponto `y` e solta.
fn drag_listbox(list: &ListBox, to_y: i32, cancel: bool) {
    let height = send(list, LB_GETITEMHEIGHT, 0, 0) as i32;
    send(list, WM_LBUTTONDOWN, MK_LBUTTON, lparam(10, height / 2));
    send(list, WM_MOUSEMOVE, MK_LBUTTON, lparam(10, height / 2 + 12));
    send(list, WM_MOUSEMOVE, MK_LBUTTON, lparam(10, to_y));
    if cancel {
        send(list, WM_KEYDOWN, 0x1B, 0);
    }
    send(list, WM_LBUTTONUP, 0, lparam(10, to_y));
}

#[test]
fn listbox_reorders_by_dragging_and_respects_veto_and_escape() {
    with_app(|_| {
        let window = test_window();
        let list =
            ListBox::builder().items(&["a", "b", "c", "d"]).bounds(0, 0, 200, 200).parent(&window).build().unwrap();
        window.show();
        list.set_reorderable(true);
        let moves = Rc::new(RefCell::new(Vec::new()));
        let allow = Rc::new(std::cell::Cell::new(true));
        list.on_item_reorder_request({
            let moves = moves.clone();
            let allow = allow.clone();
            move |from, to| {
                moves.borrow_mut().push((from, to));
                allow.get()
            }
        });
        let height = send(&list, LB_GETITEMHEIGHT, 0, 0) as i32;

        // Solto logo acima da divisa entre "c" e "d": "a" vai para a posição 2.
        drag_listbox(&list, 3 * height - 2, false);
        assert_eq!(*moves.borrow(), vec![(0, 2)]);
        assert_eq!(listbox_items(&list), ["b", "c", "a", "d"]);
        assert_eq!(list.selected_index(), Some(2), "a seleção acompanha o item");

        // Recusado pelo callback: nada muda.
        allow.set(false);
        drag_listbox(&list, 4 * height - 2, false);
        assert_eq!(moves.borrow().last(), Some(&(0, 3)));
        assert_eq!(listbox_items(&list), ["b", "c", "a", "d"]);

        // Esc cancela antes de soltar.
        allow.set(true);
        drag_listbox(&list, 4 * height - 2, true);
        assert_eq!(moves.borrow().len(), 2);
        assert_eq!(listbox_items(&list), ["b", "c", "a", "d"]);

        // Sem reordenação ligada, arrastar não faz nada.
        list.set_reorderable(false);
        drag_listbox(&list, 4 * height - 2, false);
        assert_eq!(moves.borrow().len(), 2);

        list.move_item(3, 0);
        assert_eq!(listbox_items(&list), ["d", "b", "c", "a"]);
        assert_eq!(list.index_at(Point::new(5, 1)), Some(0));
        assert_eq!(list.index_at(Point::new(5, 500)), None);
    });
}

fn list_view_with_rows(window: &Window) -> ListView {
    let list = ListView::builder().bounds(0, 0, 300, 220).checkboxes(true).parent(window).build().unwrap();
    list.add_column("Nome", 120, ColumnAlign::Left);
    list.add_column("Valor", 80, ColumnAlign::Right);
    for (name, value) in [("um", "1"), ("dois", "2"), ("três", "3"), ("quatro", "4")] {
        list.add_row(&[name, value]);
    }
    list
}

fn column(list: &ListView, column: usize) -> Vec<String> {
    (0..list.row_count()).map(|r| list.cell(r, column).unwrap()).collect()
}

#[test]
fn list_view_move_row_keeps_cells_icon_and_check_state() {
    with_app(|_| {
        let window = test_window();
        let list = list_view_with_rows(&window);
        list.set_row_icon(0, &icons::FILE).unwrap();
        list.set_row_checked(0, true);
        let image = list.row_image(0);
        list.move_row(0, 3);
        assert_eq!(column(&list, 0), ["dois", "três", "quatro", "um"]);
        assert_eq!(column(&list, 1), ["2", "3", "4", "1"]);
        assert!(list.is_row_checked(3) && !list.is_row_checked(0));
        assert_eq!(list.row_image(3), image);
        list.move_row(3, 1);
        assert_eq!(column(&list, 0), ["dois", "um", "três", "quatro"]);
        list.move_row(9, 0);
        assert_eq!(list.row_count(), 4, "índices inválidos são ignorados");
    });
}

fn item_rect(widget: &impl Widget, message: u32, wparam: usize, mut rect: RECT) -> RECT {
    // SAFETY: mensagem de geometria com um RECT local, enviada à janela viva do teste.
    unsafe { SendMessageW(widget.handle() as _, message, wparam, &mut rect as *mut RECT as isize) };
    rect
}

fn notify_begin_drag<T>(window: &Window, mut info: T) {
    // SAFETY: estrutura de notificação local, válida durante o envio síncrono.
    unsafe { SendMessageW(window.handle() as _, WM_NOTIFY, 0, &mut info as *mut T as isize) };
}

#[test]
fn list_view_reorders_from_its_begin_drag_notification() {
    with_app(|_| {
        let window = test_window();
        let list = list_view_with_rows(&window);
        window.show();
        list.set_reorderable(true);
        let moved = Rc::new(RefCell::new(None));
        list.on_row_reorder_request({
            let moved = moved.clone();
            move |from, to| {
                *moved.borrow_mut() = Some((from, to));
                true
            }
        });
        let first = item_rect(&list, LVM_GETITEMRECT, 0, RECT::default());
        let last = item_rect(&list, LVM_GETITEMRECT, 3, RECT::default());
        notify_begin_drag(
            &window,
            NMLISTVIEW {
                hdr: NMHDR { hwndFrom: list.handle() as _, idFrom: 0, code: LVN_BEGINDRAG as u32 },
                iItem: 0,
                iSubItem: 0,
                uNewState: 0,
                uOldState: 0,
                uChanged: 0,
                ptAction: POINT { x: 20, y: (first.top + first.bottom) / 2 },
                lParam: 0,
            },
        );
        // Abaixo da última linha: vai para o fim.
        send(&list, WM_MOUSEMOVE, MK_LBUTTON, lparam(20, last.bottom - 2));
        send(&list, WM_LBUTTONUP, 0, lparam(20, last.bottom - 2));
        assert_eq!(*moved.borrow(), Some((0, 3)));
        assert_eq!(column(&list, 0), ["dois", "três", "quatro", "um"]);
        assert_eq!(list.row_at(Point::new(20, 500)), None);
    });
}

#[test]
fn data_grid_moves_rows_in_memory_and_clears_sorting() {
    with_app(|_| {
        let window = test_window();
        let grid = DataGrid::builder().bounds(0, 0, 320, 200).sortable(true).parent(&window).build().unwrap();
        grid.add_column("Nome", 120, ColumnAlign::Left);
        for name in ["c", "a", "b"] {
            grid.add_row(&[name]);
        }
        grid.sort_by(0, true);
        assert_eq!(grid.sort_state(), Some((0, true)));
        grid.move_row(0, 2);
        let names: Vec<String> = grid.rows().into_iter().map(|r| r[0].clone()).collect();
        assert_eq!(names, ["b", "c", "a"]);
        assert_eq!(grid.sort_state(), None);
        assert_eq!(grid.selected_rows(), vec![2]);

        grid.set_virtual_source(100, |row, _| row.to_string());
        grid.move_row(0, 5);
        assert_eq!(grid.cell(0, 0).as_deref(), Some("0"), "fonte virtual não é reordenada");
    });
}

fn texts(tree: &TreeView, items: &[TreeItem]) -> Vec<String> {
    items.iter().map(|i| tree.item_text(*i)).collect()
}

#[test]
fn tree_moves_whole_subtrees_and_refuses_cycles() {
    with_app(|_| {
        let window = test_window();
        let tree = TreeView::builder().bounds(0, 0, 240, 260).checkboxes(true).parent(&window).build().unwrap();
        let docs = tree.add_root("Documentos").unwrap();
        let photos = tree.add_root("Fotos").unwrap();
        let music = tree.add_root("Música").unwrap();
        let trip = tree.add_child(photos, "Viagem").unwrap();
        tree.add_child(trip, "praia.jpg").unwrap();
        tree.add_child(photos, "gato.png").unwrap();
        tree.set_checked(trip, true);
        tree.set_item_icon(trip, &icons::FOLDER).unwrap();
        tree.expand(photos);
        tree.expand(trip);

        // Não pode ir para dentro de si mesmo nem de um descendente.
        assert!(tree.move_item(photos, TreeDropPosition::Inside(trip)).is_err());
        assert!(tree.move_item(photos, TreeDropPosition::Inside(photos)).is_err());

        let moved = tree.move_item(trip, TreeDropPosition::Inside(docs)).unwrap();
        assert_eq!(texts(&tree, &tree.children(docs)), ["Viagem"]);
        assert_eq!(texts(&tree, &tree.children(moved)), ["praia.jpg"], "filhos vão junto");
        assert_eq!(texts(&tree, &tree.children(photos)), ["gato.png"]);
        assert!(tree.is_checked(moved), "estado da caixa preservado");
        assert!(tree.item_image(moved).is_some(), "ícone preservado");
        assert_eq!(tree.parent_of(moved), Some(docs));

        let music = tree.move_item(music, TreeDropPosition::Before(docs)).unwrap();
        assert_eq!(texts(&tree, &tree.roots()), ["Música", "Documentos", "Fotos"]);
        tree.move_item(music, TreeDropPosition::After(photos)).unwrap();
        assert_eq!(texts(&tree, &tree.roots()), ["Documentos", "Fotos", "Música"]);
        assert_eq!(tree.count(), 6);
    });
}

#[test]
fn tree_drag_drops_inside_a_node_from_its_begin_drag_notification() {
    with_app(|_| {
        let window = test_window();
        let tree = TreeView::builder().bounds(0, 0, 240, 260).parent(&window).build().unwrap();
        let docs = tree.add_root("Documentos").unwrap();
        let photos = tree.add_root("Fotos").unwrap();
        tree.add_root("Música").unwrap();
        window.show();
        tree.set_drag_enabled(true);
        let dropped = Rc::new(RefCell::new(None));
        tree.on_item_drop_request({
            let dropped = dropped.clone();
            move |item, position| {
                *dropped.borrow_mut() = Some((item, position));
                true
            }
        });
        let moved = Rc::new(RefCell::new(None));
        tree.on_item_moved({
            let moved = moved.clone();
            move |item| *moved.borrow_mut() = Some(item)
        });
        let rect_of = |item: TreeItem| {
            // O HTREEITEM vai nos primeiros bytes do RECT (convenção de TVM_GETITEMRECT).
            let mut rect = RECT::default();
            // SAFETY: o RECT tem espaço para um ponteiro.
            unsafe { *(&mut rect as *mut RECT as *mut isize) = item.handle() };
            item_rect(&tree, TVM_GETITEMRECT, 1, rect)
        };
        let target = rect_of(docs);
        let source = rect_of(photos);
        let mut item = blank_tv_item();
        item.hItem = photos.handle() as _;
        notify_begin_drag(
            &window,
            NMTREEVIEWW {
                hdr: NMHDR { hwndFrom: tree.handle() as _, idFrom: 0, code: TVN_BEGINDRAGW as u32 },
                action: 0,
                itemOld: blank_tv_item(),
                itemNew: item,
                ptDrag: POINT { x: source.left + 4, y: (source.top + source.bottom) / 2 },
            },
        );
        let middle = (target.top + target.bottom) / 2;
        send(&tree, WM_MOUSEMOVE, MK_LBUTTON, lparam(target.left + 4, middle));
        send(&tree, WM_LBUTTONUP, 0, lparam(target.left + 4, middle));
        assert_eq!(*dropped.borrow(), Some((photos, TreeDropPosition::Inside(docs))));
        let new = moved.borrow().expect("nó movido");
        assert_eq!(tree.parent_of(new), Some(docs));
        assert_eq!(texts(&tree, &tree.roots()), ["Documentos", "Música"]);
        assert_eq!(tree.item_at(Point::new(1, 1000)), None);
    });
}

fn blank_tv_item() -> TVITEMW {
    TVITEMW {
        mask: 0,
        hItem: std::ptr::null_mut(),
        state: 0,
        stateMask: 0,
        pszText: std::ptr::null_mut(),
        cchTextMax: 0,
        iImage: 0,
        iSelectedImage: 0,
        cChildren: 0,
        lParam: 0,
    }
}

#[test]
fn drop_data_reads_every_format_offered_by_the_source() {
    with_app(|_| {
        let pixels: Vec<u8> = (0..2 * 2).flat_map(|i| [i as u8 * 40, 10, 220, 255]).collect();
        let content = ClipboardContent::new()
            .text("olá")
            .html("<p>parágrafo</p>")
            .image_pixels(2, 2, &pixels)
            .unwrap()
            .files(&["C:\\a\\um.txt", "C:\\a\\dois.txt"])
            .custom("ArqRustUI.Arraste", &[9, 8, 7]);
        let data = DropData::from_content(&content, Point::new(12, 34)).unwrap();
        assert_eq!(data.position(), Point::new(12, 34));
        assert!(data.has_text() && data.has_html() && data.has_image() && data.has_files());
        assert_eq!(data.text().as_deref(), Some("olá"));
        assert_eq!(data.html().as_deref(), Some("<p>parágrafo</p>"));
        assert_eq!(data.files(), vec![PathBuf::from("C:\\a\\um.txt"), PathBuf::from("C:\\a\\dois.txt")]);
        assert_eq!(data.data("ArqRustUI.Arraste"), Some(vec![9, 8, 7]));
        assert!(!data.has_format("ArqRustUI.Outro"));
        assert_eq!(data.image().unwrap().unwrap().to_pixels().unwrap(), pixels);
        assert!(data.format_names().iter().any(|n| n == "ArqRustUI.Arraste"));
        assert_eq!(data.suggested_effect(), DropEffect::Copy);
        assert!(data.allows(DropEffect::Move));

        let text_only = DropData::from_content(&ClipboardContent::new().text("x"), Point::default()).unwrap();
        assert!(!text_only.has_files() && !text_only.has_image() && text_only.files().is_empty());
        assert!(DropData::from_content(&ClipboardContent::new(), Point::default()).is_err());
    });
}

#[test]
fn widgets_register_and_revoke_drop_targets() {
    with_app(|_| {
        let window = test_window();
        let zone = Panel::builder().bounds(10, 10, 100, 100).parent(&window).build().unwrap();
        accept_drops(&zone, DropHandler::new().on_drop(|_, _| {})).unwrap();
        // Registrar de novo substitui o anterior.
        accept_drops(
            &zone,
            DropHandler::new().on_over(|data| if data.has_files() { DropEffect::Copy } else { DropEffect::None }),
        )
        .unwrap();
        stop_accepting_drops(&zone);
        stop_accepting_drops(&zone);
        accept_drops(&window, DropHandler::new()).unwrap();
        // Destruir a janela desfaz o registro sem erro.
        window.close();
    });
}
