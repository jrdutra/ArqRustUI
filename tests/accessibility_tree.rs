//! Elementos UIA virtuais dos controles desenhados: MenuStrip (e suas listas),
//! CommandBar e PropertyGrid — papéis, nomes, estados e padrões Invoke, Toggle,
//! ExpandCollapse e Value.

mod common;

use arqrustui::accessibility::uia_debug::{menu_popups, perform, walk, ElementInfo};
use arqrustui::menus::debug_state;
use arqrustui::prelude::*;
use common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const BUTTON: i32 = 50000;
const LIST_ITEM: i32 = 50007;
const LIST: i32 = 50008;
const MENU_BAR: i32 = 50010;
const MENU_ITEM: i32 = 50011;
const TEXT: i32 = 50020;
const GROUP: i32 = 50026;
const SPLIT_BUTTON: i32 = 50031;
const SEPARATOR: i32 = 50038;
const S_OK: i32 = 0;
const INVOKE: usize = 0;
const TOGGLE: usize = 1;
const EXPAND: usize = 2;
const VALUE: usize = 3;

fn tree(widget: &impl Widget) -> Vec<(usize, ElementInfo)> {
    // SAFETY: janela viva desta thread.
    unsafe { walk(widget.handle()) }
}

fn element<'a>(items: &'a [(usize, ElementInfo)], name: &str) -> &'a ElementInfo {
    &items.iter().find(|(_, e)| e.name == name).unwrap_or_else(|| panic!("elemento {name:?} ausente: {items:#?}")).1
}

fn act(hwnd: isize, name: &str, action: &str) -> i32 {
    // SAFETY: janela viva desta thread.
    unsafe { perform(hwnd, name, action) }
}

fn depth() -> usize {
    debug_state().map(|(depth, _)| depth).unwrap_or(0)
}

#[test]
fn command_bar_exposes_buttons_with_their_patterns() {
    with_app(|_| {
        let window = test_window();
        let bar = CommandBar::builder().bounds(0, 0, 400, COMMAND_BAR_HEIGHT).parent(&window).build().unwrap();
        let clicks = Counter::new();
        let toggled = Rc::new(Cell::new(None));
        let menu = Menu::new().unwrap();
        let picked = Counter::new();
        let hit = picked.clone();
        menu.add_item("&Primeiro", move || hit.hit()).unwrap();
        let hit = clicks.clone();
        bar.add_button(
            CommandButton::new("&Salvar").icon(&icons::SAVE).tooltip("Salvar (Ctrl+S)").on_click(move || hit.hit()),
        )
        .unwrap();
        let seen = toggled.clone();
        bar.add_button(CommandButton::new("Negrito").toggle(false).on_toggled(move |on| seen.set(Some(on)))).unwrap();
        bar.add_separator().unwrap();
        bar.add_button(CommandButton::new("Ferramentas").menu(&menu)).unwrap();
        bar.add_button(CommandButton::new("Colar").split(&menu)).unwrap();
        bar.add_label("Pronto").unwrap();
        window.show();

        let items = tree(&bar);
        assert!(items.iter().all(|(depth, _)| *depth == 0), "elementos planos dentro da barra");
        let save = element(&items, "Salvar");
        assert_eq!((save.control_type, save.patterns), (BUTTON, [true, false, false, false]));
        let bold = element(&items, "Negrito");
        assert_eq!((bold.patterns[TOGGLE], bold.states.0), (true, Some(0)));
        let tools = element(&items, "Ferramentas");
        assert_eq!((tools.patterns[EXPAND], tools.states.1), (true, Some(0)));
        let paste = element(&items, "Colar");
        assert_eq!(paste.control_type, SPLIT_BUTTON);
        assert!(paste.patterns[INVOKE] && paste.patterns[EXPAND]);
        assert_eq!(element(&items, "Pronto").control_type, TEXT);
        assert!(items.iter().any(|(_, e)| e.control_type == SEPARATOR));

        let hwnd = bar.handle();
        assert_eq!(act(hwnd, "Salvar", "invoke"), S_OK);
        assert_eq!(clicks.get(), 1);
        assert_eq!(act(hwnd, "Negrito", "toggle"), S_OK);
        assert_eq!(toggled.get(), Some(true));
        assert_eq!(element(&tree(&bar), "Negrito").states.0, Some(1));
        assert_ne!(act(hwnd, "Pronto", "invoke"), S_OK, "rótulo não tem Invoke");

        assert_eq!(act(hwnd, "Ferramentas", "expand"), S_OK);
        assert_eq!(depth(), 1, "o menu abriu");
        assert_eq!(element(&tree(&bar), "Ferramentas").states.1, Some(1));
        let popup = menu_popups()[0];
        assert_eq!(act(popup, "Primeiro", "invoke"), S_OK);
        assert_eq!((picked.get(), depth()), (1, 0), "item executado e menu fechado");

        assert_eq!(act(hwnd, "Salvar", "focus"), S_OK);
        assert!(element(&tree(&bar), "Salvar").focused);
    });
}

#[test]
fn menu_strip_is_a_menu_bar_whose_lists_are_menus() {
    with_app(|_| {
        let window = test_window();
        let created = Counter::new();
        let wrap = Rc::new(Cell::new(false));
        let file = Menu::new().unwrap();
        let hit = created.clone();
        file.add_item_with_shortcut("&Novo", Shortcut::ctrl(Key(b'N' as u16)), move || hit.hit()).unwrap();
        file.add_separator().unwrap();
        let seen = wrap.clone();
        file.add_check_item("&Quebra de linha", false, move |on| seen.set(on)).unwrap();
        let recent = Menu::new().unwrap();
        recent.add_item("um.txt", || {}).unwrap();
        file.add_submenu("&Recentes", &recent).unwrap();
        let strip = MenuStrip::new().unwrap();
        strip.add_menu("&Arquivo", &file).unwrap();
        strip.add_menu("&Editar", &Menu::new().unwrap()).unwrap();
        window.set_menu_strip(&strip).unwrap();
        window.show();

        let items = tree(&window);
        assert_eq!(items[0], (0, ElementInfo { control_type: MENU_BAR, ..items[0].1.clone() }));
        let arquivo = element(&items, "Arquivo");
        assert_eq!((arquivo.control_type, arquivo.patterns[EXPAND], arquivo.states.1), (MENU_ITEM, true, Some(0)));
        assert!(items.iter().filter(|(d, _)| *d == 1).count() >= 2, "menus dentro da barra");

        assert_eq!(act(window.handle(), "Arquivo", "expand"), S_OK);
        assert_eq!(depth(), 1);
        assert_eq!(element(&tree(&window), "Arquivo").states.1, Some(1));
        let popup = menu_popups()[0];
        // SAFETY: `popup` é a janela de menu viva desta thread, devolvida por `menu_popups`.
        let list = unsafe { walk(popup) };
        let novo = element(&list, "Novo");
        assert_eq!((novo.control_type, novo.patterns[INVOKE]), (MENU_ITEM, true));
        assert!(novo.focused, "aberto pelo teclado: o primeiro item tem o foco");
        assert!(list.iter().any(|(_, e)| e.control_type == SEPARATOR));
        assert!(element(&list, "Recentes").patterns[EXPAND]);

        assert_eq!(act(popup, "Recentes", "expand"), S_OK);
        assert_eq!(depth(), 2, "submenu aberto");
        // SAFETY: o submenu aberto acima é uma janela de menu viva desta thread.
        assert_eq!(unsafe { walk(menu_popups()[1]) }[0].1.name, "um.txt");
        assert_eq!(act(popup, "Recentes", "collapse"), S_OK);
        assert_eq!(depth(), 1);

        assert_eq!(act(popup, "Quebra de linha", "invoke"), S_OK);
        assert!(wrap.get(), "item marcável alternado");
        assert_eq!(depth(), 0);

        assert_eq!(act(window.handle(), "Arquivo", "expand"), S_OK);
        assert_eq!(act(window.handle(), "Arquivo", "collapse"), S_OK);
        assert_eq!(depth(), 0);
        assert_eq!(created.get(), 0);

        window.remove_menu_strip();
        assert!(tree(&window).is_empty(), "sem faixa, sem elementos");
    });
}

#[test]
fn property_grid_rows_expose_value_and_toggle() {
    with_app(|_| {
        let window = test_window();
        let grid = PropertyGrid::new(&window, Rect::new(0, 0, 320, 240)).unwrap();
        grid.add_category("Aparência");
        grid.add_property("Título", PropertyValue::Text("Relatório".into()));
        grid.add_property("Largura", PropertyValue::Integer(800));
        grid.add_property("Visível", PropertyValue::Bool(true));
        grid.add_property(
            "Alinhamento",
            PropertyValue::Choice { options: vec!["Esquerda".into(), "Centro".into()], selected: 0 },
        );
        let changes = Rc::new(RefCell::new(Vec::new()));
        let log = changes.clone();
        grid.on_property_changed(move |name, value| log.borrow_mut().push((name.to_string(), value.clone())));
        window.show();

        let items = tree(&grid);
        assert_eq!(items[0], (0, ElementInfo { control_type: GROUP, ..items[0].1.clone() }));
        assert!(items[1..].iter().all(|(depth, _)| *depth == 1), "propriedades dentro da categoria");
        let width = element(&items, "Largura");
        assert_eq!((width.control_type, width.value.as_deref()), (LIST_ITEM, Some("800")));
        assert!(width.patterns[VALUE] && !width.patterns[TOGGLE]);
        let visible = element(&items, "Visível");
        assert_eq!((visible.patterns[TOGGLE], visible.states.0), (true, Some(1)));

        let hwnd = grid.handle();
        assert_eq!(act(hwnd, "Largura", "set:1024"), S_OK);
        assert_eq!(grid.value("Largura"), Some(PropertyValue::Integer(1024)));
        assert_ne!(act(hwnd, "Largura", "set:abc"), S_OK, "texto inválido é recusado");
        assert_eq!(act(hwnd, "Visível", "toggle"), S_OK);
        assert_eq!(grid.value("Visível"), Some(PropertyValue::Bool(false)));
        assert_ne!(act(hwnd, "Visível", "set:Sim"), S_OK, "booleano é somente leitura no Value");
        assert_eq!(act(hwnd, "Alinhamento", "set:centro"), S_OK);
        assert_eq!(element(&tree(&grid), "Alinhamento").value.as_deref(), Some("Centro"));
        assert_eq!(changes.borrow().len(), 3);

        assert_eq!(act(hwnd, "Título", "focus"), S_OK);
        assert!(element(&tree(&grid), "Título").focused);
        let _ = (LIST, BUTTON);
    });
}

#[test]
fn drawn_controls_answer_the_uia_root_request() {
    with_app(|_| {
        let window = test_window();
        let bar = CommandBar::builder().bounds(0, 0, 300, COMMAND_BAR_HEIGHT).parent(&window).build().unwrap();
        bar.add_button(CommandButton::new("Abrir").on_click(|| {})).unwrap();
        // WM_GETOBJECT com UiaRootObjectId: o provedor é entregue ao UI Automation.
        let result = send(&bar, 0x003D, 0, -25);
        assert_ne!(result, 0, "provedor UIA publicado");
    });
}

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *mut std::ffi::c_void, flags: u32) -> i32;
    fn CoUninitialize();
}

/// Lê, com um cliente UI Automation de verdade (IUIAutomation) numa thread própria —
/// como um leitor de tela faria —, o tipo da janela e os nomes dos filhos.
fn uia_client_read(hwnd: isize) -> Result<(i32, Vec<String>), String> {
    use arqrustui::ffi::ole32::{CoCreateInstance, ComPtr, CLSCTX_INPROC_SERVER};
    use arqrustui::ffi::GUID;
    use std::ffi::c_void;
    const CLSID_CUIAUTOMATION: GUID =
        GUID::new(0xFF48DBA4, 0x60EF, 0x4201, [0xAA, 0x87, 0x54, 0x10, 0x3E, 0xEF, 0x59, 0x4E]);
    const IID_IUIAUTOMATION: GUID =
        GUID::new(0x30CBE57D, 0xD9D0, 0x452A, [0xAB, 0x13, 0x7A, 0xC5, 0xAC, 0x48, 0x25, 0xEE]);
    // SAFETY: chamadas COM pelos índices de vtable de uiautomationclient.h; cada
    // referência obtida passa a um ComPtr, liberado antes do CoUninitialize.
    unsafe {
        CoInitializeEx(std::ptr::null_mut(), 0);
        let result = (|| {
            let mut raw: *mut c_void = std::ptr::null_mut();
            let hr = CoCreateInstance(
                &CLSID_CUIAUTOMATION,
                std::ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_IUIAUTOMATION,
                &mut raw,
            );
            let uia = ComPtr::from_raw(raw).filter(|_| hr >= 0).ok_or(format!("IUIAutomation: {hr:#x}"))?;
            let from_handle: unsafe extern "system" fn(*mut c_void, isize, *mut *mut c_void) -> i32 = uia.method(6);
            let mut element: *mut c_void = std::ptr::null_mut();
            let hr = from_handle(uia.as_ptr(), hwnd, &mut element);
            let element = ComPtr::from_raw(element).ok_or(format!("ElementFromHandle: {hr:#x}"))?;
            let control_type: unsafe extern "system" fn(*mut c_void, *mut i32) -> i32 = element.method(21);
            let mut kind = 0;
            control_type(element.as_ptr(), &mut kind);
            let true_condition: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32 = uia.method(21);
            let mut condition: *mut c_void = std::ptr::null_mut();
            true_condition(uia.as_ptr(), &mut condition);
            let condition = ComPtr::from_raw(condition).ok_or("CreateTrueCondition")?;
            let find_all: unsafe extern "system" fn(*mut c_void, i32, *mut c_void, *mut *mut c_void) -> i32 =
                element.method(6);
            let mut array: *mut c_void = std::ptr::null_mut();
            let hr = find_all(element.as_ptr(), 2, condition.as_ptr(), &mut array);
            let array = ComPtr::from_raw(array).ok_or(format!("FindAll: {hr:#x}"))?;
            let length: unsafe extern "system" fn(*mut c_void, *mut i32) -> i32 = array.method(3);
            let get: unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> i32 = array.method(4);
            let mut count = 0;
            length(array.as_ptr(), &mut count);
            let mut names = Vec::new();
            for index in 0..count {
                let mut child: *mut c_void = std::ptr::null_mut();
                get(array.as_ptr(), index, &mut child);
                let child = ComPtr::from_raw(child).ok_or("GetElement")?;
                let name: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> i32 = child.method(23);
                let mut text: *mut u16 = std::ptr::null_mut();
                name(child.as_ptr(), &mut text);
                names.push(arqrustui::utilities::strings::from_wide_ptr(text));
                arqrustui::ffi::ole32::SysFreeString(text);
            }
            Ok((kind, names))
        })();
        CoUninitialize();
        result
    }
}

#[test]
fn uia_client_sees_the_command_bar_buttons() {
    with_app(|app| {
        let window = test_window();
        let bar = CommandBar::builder().bounds(0, 0, 300, COMMAND_BAR_HEIGHT).parent(&window).build().unwrap();
        bar.add_button(CommandButton::new("Abrir").on_click(|| {})).unwrap();
        bar.add_button(CommandButton::new("Fechar").on_click(|| {})).unwrap();
        window.show();
        let hwnd = bar.handle();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(uia_client_read(hwnd));
        });
        // A thread da interface continua processando mensagens: é por ela que o UIA
        // chega aos provedores.
        let result = RefCell::new(None);
        pump_until(app, std::time::Duration::from_secs(20), || {
            if let Ok(value) = receiver.try_recv() {
                *result.borrow_mut() = Some(value);
            }
            result.borrow().is_some()
        });
        match result.into_inner().expect("o cliente UIA respondeu") {
            Ok((kind, names)) => {
                assert_eq!(kind, 50021, "a barra aparece como ToolBar");
                assert_eq!(names, ["Abrir", "Fechar"]);
            }
            Err(error) if error.starts_with("IUIAutomation") => eprintln!("{error}; teste ignorado"),
            Err(error) => panic!("{error}"),
        }
    });
}

#[test]
fn uia_client_sees_the_menu_bar_in_the_window() {
    with_app(|app| {
        let window = test_window();
        let strip = MenuStrip::new().unwrap();
        strip.add_menu("&Arquivo", &Menu::new().unwrap()).unwrap();
        window.set_menu_strip(&strip).unwrap();
        Button::builder().text("Confirmar").bounds(10, 10, 100, 28).parent(&window).build().unwrap();
        window.show();
        let hwnd = window.handle();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(uia_client_read(hwnd));
        });
        let result = RefCell::new(None);
        pump_until(app, std::time::Duration::from_secs(20), || {
            if let Ok(value) = receiver.try_recv() {
                *result.borrow_mut() = Some(value);
            }
            result.borrow().is_some()
        });
        match result.into_inner().expect("o cliente UIA respondeu") {
            Ok((_, names)) => {
                assert!(names.iter().any(|n| n == "Barra de menus"), "barra desenhada: {names:?}");
                // O cliente UIA do Wine não lista janelas filhas nem sem provedor próprio;
                // no Windows o provedor padrão do HWND as acrescenta (não verificado aqui).
            }
            Err(error) if error.starts_with("IUIAutomation") => eprintln!("{error}; teste ignorado"),
            Err(error) => panic!("{error}"),
        }
    });
}
