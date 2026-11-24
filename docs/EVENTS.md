# Sistema de eventos, threads e `async`

*English: [EVENTS.en.md](EVENTS.en.md).*

## Registrando manipuladores

Todo controle implementa `Widget`, que oferece os eventos comuns. Controles específicos acrescentam eventos tipados.

```rust
button.on_click(|| println!("clicado"));
text.on_text_changed({ let text = text.clone(); move || println!("{}", text.text()) });
slider.on_value_changed(|value: i32| println!("{value}"));
window.on_resize(|size| println!("{}x{}", size.width, size.height));
window.on_close_request(|| MessageBox::confirm("Sair", "Fechar a janela?")); // false cancela
```

### Nomes dos eventos

Todo evento é `on_<algo>` e o nome diz **quando** ele acontece:

| Forma | Quando | O callback | Exemplos |
|---|---|---|---|
| verbo no presente | ação direta do usuário, no momento em que acontece | não devolve nada | `on_click`, `on_double_click`, `on_key_down`, `on_item_click`, `on_column_click`, `on_link_click` |
| particípio (`_changed`, `_toggled`, `_closed`…) | aviso **depois** que algo mudou | não devolve nada | `on_value_changed`, `on_selection_changed`, `on_highlight_changed`, `on_toggled`, `on_closed`, `on_dismissed`, `on_row_activated`, `on_item_moved`, `on_finished` |
| `_request` | pedido **antes** de algo acontecer, que pode ser recusado | devolve `bool` (`false` cancela) | `on_close_request`, `on_item_reorder_request`, `on_rows_reorder_request`, `on_item_drop_request` |

Exceções deliberadas: os eventos básicos do `Widget` e da `Window` mantêm os nomes
clássicos do Windows (`on_resize`, `on_move`, `on_focus`, `on_blur`, `on_destroy`,
`on_paint`); `Menu::on_open` roda logo antes de o menu aparecer, para prepará-lo;
`on_gesture` e `ImeCandidateWindow::on_pick` (o candidato clicado) são ações do
usuário, no presente; e os callbacks que **fornecem** algo à biblioteca têm o nome da ação
(`on_drag_out` devolve o conteúdo a arrastar, `DropHandler::on_over` devolve o
efeito aceito).

O substantivo segue o vocabulário do controle: `item` na `ListBox` e na `TreeView`,
`row` na `ListView` e na `DataGrid`.

Nomes antigos que fugiam da convenção continuam funcionando como aliases
`#[deprecated]` (o compilador avisa e indica o nome novo):

| Tipo | Antigo | Novo |
|---|---|---|
| `Window` | `on_close` | `on_close_request` |
| `InfoBar` | `on_close` | `on_closed` |
| `Toast` | `on_dismiss` | `on_dismissed` |
| `CommandButton` | `on_toggle` | `on_toggled` |
| `Hyperlink` | `on_link_clicked` | `on_link_click` |
| `ListView` | `on_item_activated` | `on_row_activated` |
| `ListView`, `DataGrid` | `on_reorder` | `on_row_reorder_request` |
| `ListView`, `DataGrid` | `on_rows_reordered` | `on_rows_reorder_request` |
| `ListBox` | `on_reorder` | `on_item_reorder_request` |
| `ListBox` | `on_items_reordered` | `on_items_reorder_request` |
| `TreeView` | `on_item_drop` | `on_item_drop_request` |
| `Transition` | `on_done` | `on_finished` |
| `Menu` | `on_highlight` | `on_highlight_changed` |
| `Ime` | `on_candidates` | `on_candidates_changed` |
| `Ime` | `on_composition` | `on_composition_changed` |

### Eventos comuns (`Widget`)

| Método | Dados | Origem Win32 |
|---|---|---|
| `on_click` | — | `BN_CLICKED`, `STN_CLICKED`, clique em classes próprias |
| `on_double_click` | — | `WM_LBUTTONDBLCLK`, `BN_DBLCLK`, `NM_DBLCLK` |
| `on_mouse_move` / `on_mouse_down` / `on_mouse_up` / `on_mouse_wheel` | `&MouseEvent` | `WM_MOUSE*`, `WM_*BUTTON*` |
| `on_mouse_enter` / `on_mouse_leave` | — | `TrackMouseEvent` + `WM_MOUSELEAVE` |
| `on_key_down` / `on_key_up` | `&KeyEvent` | `WM_KEYDOWN/UP`, `WM_SYSKEYDOWN/UP` |
| `on_char` | `char` | `WM_CHAR` |
| `on_focus` / `on_blur` | — | `WM_SETFOCUS` / `WM_KILLFOCUS` |
| `on_resize` | `Size` | `WM_SIZE` |
| `on_destroy` | — | `WM_NCDESTROY` |
| `on_context_menu` | `Point` | `WM_CONTEXTMENU` |
| `on_pointer` | `&PointerEvent` | `WM_POINTERDOWN/UPDATE/UP` |
| `on_event` | `&Event` | todos |

### Eventos específicos

| Tipo | Eventos |
|---|---|
| `Window` | `on_close_request() -> bool`, `on_move`, `on_activated`, `on_deactivated`, `on_drop_files`, `on_dpi_changed`, `on_theme_changed`, `on_paint` |
| `TextBox`, `PasswordBox`, `RichTextBox` | `on_text_changed` (+ `on_selection_changed` no RichTextBox) |
| `CheckBox`, `RadioButton`, `ToggleButton` | `on_value_changed(bool)` |
| `ComboBox` | `on_selection_changed(Option<usize>)`, `on_text_changed` |
| `ListBox` | `on_selection_changed(Option<usize>)`, `on_item_activated(usize)`, `on_item_reorder_request(de, para) -> bool`, `on_items_reorder_request(&[usize], para) -> bool`, `on_drag_out(usize)` |
| `Slider`, `ScrollBar`, `SpinButton`, `UpDown` | `on_value_changed(i32)` |
| `DatePicker`, `TimePicker` | `on_value_changed(Option<DateTime>)` |
| `ListView`, `DataGrid` | `on_selection_changed(Vec<usize>)`, `on_row_activated(usize)`, `on_column_click(usize)`, `on_row_reorder_request(de, para) -> bool`, `on_rows_reorder_request(&[usize], para) -> bool`, `on_drag_out(usize)`; `on_check_changed` (`ListView`); `on_cell_edited`, `on_group_toggled` (`DataGrid`) |
| `TreeView` | `on_selection_changed(Option<TreeItem>)`, `on_item_drop_request(TreeItem, TreeDropPosition) -> bool`, `on_item_moved(TreeItem)`, `on_drag_out(TreeItem)` |
| Qualquer widget (destino OLE) | `accept_drops(&w, DropHandler::new().on_over(..).on_drop(..).on_leave(..))` |
| `TabControl` | `on_selection_changed(usize)` |

Os eventos que devolvem um `HandlerId` aceitam vários manipuladores; use `widget.remove_handler(id)` para remover um. Os que devolvem `()` estão documentados um a um (os pedidos `_request` e o `on_paint`, por exemplo, guardam um manipulador só).

## Regras de despacho

- **Eventos disparam por ação do usuário**, não por chamadas de código: `set_checked`, `set_value`, `set_selected_index` não geram `on_value_changed`/`on_selection_changed`. (Exceção herdada do Windows: `set_text` em campos de edição gera `on_text_changed`.)
- **Reentrância segura**: a lista de manipuladores é copiada antes do despacho. Um manipulador pode registrar/remover manipuladores, destruir o próprio controle ou fechar a janela.
- **Depois de destruído**, um controle não recebe mais eventos; `Destroyed` é o último.
- **Panics**: capturados no procedimento de janela e em toda outra entrada chamada pelo Windows (métodos COM, callbacks) e relançados por `Application::run()` / `process_pending()` — nunca atravessam código do Windows. Numa thread sem aplicação (a thread COM que entrega a ativação de uma notificação, por exemplo) o gancho de pânico mostra a mensagem e a chamada devolve um valor de falha.

## Capturando controles em closures

Os controles são handles `Clone`. Clone o que a closure precisa:

```rust
let label = Label::builder().parent(&window).build()?;
let target = label.clone();
button.on_click(move || target.set_text("clicado"));
```

Ciclos de referência (`a` captura `b`, `b` captura `a`) não vazam: todos os manipuladores são soltos quando a janela nativa é destruída.

## Teclado

- `Tab`/`Shift+Tab`, setas em grupos de rádio e mnemônicos (`&Salvar` → Alt+S) funcionam automaticamente.
- `Button::builder().default(true)` responde a Enter; `.cancel(true)` responde a Esc; `WindowBuilder::close_on_escape(true)` fecha a janela com Esc.
- Atalhos: `window.add_shortcut(Shortcut::ctrl(Key(b'S' as u16)), || …)` ou `menu.add_item_with_shortcut(..)`.
- Ordem de tabulação: ordem de criação, ou `accessibility::set_tab_order(&[&a, &b, &c])`.

## Threads

A interface pertence à thread que criou a `Application`. Nenhum tipo visual é `Send`/`Sync` — levar um controle para outra thread é erro de compilação.

```rust
let ui = app.dispatcher();              // Send + Sync + Clone
let label = label.clone();              // fica na thread de UI
std::thread::spawn(move || {
    let total = trabalho_demorado();
    // `label` não pode vir para cá; só o resultado volta para a UI:
    let _ = ui.post(move || println!("{total}"));
});
```

Para devolver o resultado a um controle, prefira as formas abaixo, em que a closure de conclusão já roda na thread de UI.

## Trabalho em segundo plano e `async`

```rust
// Callback: `work` roda em outra thread; `done` roda na UI e pode capturar controles.
spawn_background(|| ler_arquivo_grande(), move |dados| label.set_text(&dados))?;

// async/await sem runtime externo: o future é executado dentro do loop de eventos.
spawn_local(async move {
    progress.set_marquee(true);
    let resultado = run_blocking(|| calcular()).await;   // outra thread
    sleep(Duration::from_millis(200)).await;             // não bloqueia a UI
    label.set_text(&resultado);
    progress.set_marquee(false);
})?;
```

| Função | Uso |
|---|---|
| `defer(f)` | roda `f` na próxima volta do loop (thread de UI) |
| `UiDispatcher::post(f)` | roda `f` na UI a partir de qualquer thread |
| `spawn_local(fut)` | executa um `Future` (não precisa ser `Send`) na UI |
| `run_blocking(f)` | executa `f` em uma thread de trabalho; devolve um future |
| `sleep(d)` | future que completa após `d` |
| `spawn_background(work, done)` | atalho: thread + retorno à UI |
| `Timer::interval(d, f)` / `Timer::once(d, f)` | temporizadores na UI (`interval` para ao ser solto) |

**Nunca** bloqueie a thread de UI (`thread::sleep`, E/S longa, `join`): a janela para de responder.

## Mensagens brutas (API avançada)

```rust
widget.on_raw_message(|msg| {
    if msg.message == 0x0084 /* WM_NCHITTEST */ { return Some(2 /* HTCAPTION */); }
    None // deixa a biblioteca e o Windows tratarem
});
```

Há um gancho por widget; `Direct2DSurface::attach` e o editor de células do `DataGrid` também o utilizam.
