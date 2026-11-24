# ArqRustUI — guia para agentes de IA

*English: [AGENTS.en.md](AGENTS.en.md).*

Este arquivo é a fonte de verdade para qualquer agente de IA que vá **usar**, **integrar** ou **estender** a ArqRustUI. Leia-o inteiro antes de gerar código. Ele não depende de nenhum projeto consumidor.

---

## 1. O que é

ArqRustUI é uma biblioteca de GUI **nativa para Windows**, em **Rust puro**:

- crate `arqrustui` (edição 2021, Rust ≥ 1.80), **sem nenhuma dependência de terceiros** — só `std` e FFI escrita à mão para as DLLs do Windows; os bytes do pacote de ícones vêm das crates próprias de `crates/` (`arqrustui-icons` e `arqrustui-icons-data1` a `-data8`);
- controles nativos do sistema (Win32 / Common Controls v6), mais alguns compostos escritos em Rust;
- API de alto nível: builders, closures, `Result`, RAII;
- biblioteca **genérica e autônoma**: não conhece nem deve conhecer nenhuma aplicação que a utilize.

Só funciona em Windows (`#![cfg(windows)]`; em outros sistemas o crate compila vazio).

## 2. Integração em um projeto

```toml
# Cargo.toml do projeto consumidor
[dependencies]
arqrustui = { path = "../ArqRustUI" }   # biblioteca local em diretório irmão
```

```rust
#![windows_subsystem = "windows"]   // 1ª linha de main.rs: sem janela de console
use arqrustui::prelude::*;            // importa tudo o que uma aplicação típica precisa
```

Não é necessário manifesto, `build.rs`, recurso `.rc` nem inicialização extra: `Application::new()` ativa estilos visuais, DPI por monitor e COM.

Toda `Window` sem ícone explícito usa o logo `icons::DEFAULT_WINDOW_ICON`, mesmo
com `default-features = false`. `WindowBuilder::icon(&icone)` e
`window.set_icon(&icone)` substituem o padrão, também após mudanças de DPI.
Diálogos próprios criados com `Dialog` seguem a mesma regra. O logo não integra
o catálogo opcional de 392 ícones. O ícone do arquivo executável é um recurso de
compilação do aplicativo, definido pelo próprio aplicativo.

## 3. Esqueleto obrigatório de toda aplicação

```rust
#![windows_subsystem = "windows"]
use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;                       // 1. sempre primeiro, uma vez por thread de UI
    let window = Window::builder().title("App").size(640, 400).centered().build()?; // 2. janela (nasce oculta)

    let name = TextBox::builder().placeholder("Nome").bounds(16, 16, 240, 24).parent(&window).build()?;
    let greet = Button::builder().text("&Saudar").bounds(16, 52, 100, 28).default(true).parent(&window).build()?;
    let output = Label::builder().bounds(16, 92, 400, 20).parent(&window).build()?;

    // 3. eventos: clone os handles que a closure usa
    let (input, target) = (name.clone(), output.clone());
    greet.on_click(move || target.set_text(&format!("Olá, {}!", input.text())));

    window.show();                                       // 4. mostrar
    app.run()                                            // 5. loop de eventos (bloqueia até a última janela fechar)
}
```

## 4. Modelo mental — regras que geram código correto

1. **`Application` primeiro.** Qualquer construção antes dela devolve `GuiError::NotInitialized`. Mantenha `app` vivo até o fim de `main`.
2. **Uma thread de UI.** Tipos visuais não são `Send`/`Sync` (erro de compilação ao movê-los para outra thread). Para trabalho em segundo plano use `spawn_local` + `run_blocking`, `spawn_background` ou `app.dispatcher()` (seção 9).
3. **Controles são handles `Clone`.** Clonar não duplica o controle; soltar o handle não o destrói. O controle vive enquanto a janela nativa existir. Não é preciso guardar controles em structs para mantê-los vivos.
4. **Todo controle precisa de `.parent(&algo)`** — `Window`, `Panel`, `GroupBox`, página de `TabControl`, `dialog.window()`. Sem pai: `GuiError::MissingParent`.
5. **Builders devolvem `GuiResult<T>`**: use `?`. Setters simples (`set_text`, `set_bounds`…) não devolvem `Result`.
6. **Unidades lógicas (96 DPI)** em tamanhos, posições de controles, eventos de mouse e desenho. Nunca multiplique por DPI manualmente. Exceção: a posição de janelas de nível superior é em pixels de tela.
7. **`Window::builder().size(l, a)` é o tamanho da área cliente**, não o tamanho externo.
8. **Closures de evento são `'static`**: capture clones (`let x = x.clone(); move || …`). Estado mutável compartilhado: `Rc<Cell<T>>` / `Rc<RefCell<T>>`.
9. **Eventos disparam por ação do usuário**, não por `set_*` (exceto `set_text` em campos de edição, que gera `on_text_changed`).
10. **Nunca bloqueie a thread de UI** (`thread::sleep`, E/S longa, laços demorados).
11. **Diálogos**: `Ok(None)` = usuário cancelou; `Err(_)` = falha real.
12. **Recurso dependente de versão** devolve `GuiError::Unsupported` — trate como opcional (`let _ = window.set_corner_style(..);` com um comentário, ou `if let Err(e) = …`).
13. **Não use `arqrustui::ffi` nem `unsafe`** em código de aplicação, salvo necessidade real e documentada (seção 12).

## 5. Mapa rápido: "preciso de X" → use Y

| Necessidade | API |
|---|---|
| Janela principal | `Window::builder()…build()?` |
| Diálogo modal próprio | `Dialog::new(&dono, título, l, a)?` + `show_modal()?` |
| Modal no estilo da web (fundo escurecido, cartão centralizado) | `Modal::builder().title(..).message(..).button(..).primary_button(..).build(&janela)?.show()?`; formulário: controles em `modal.content()` + `on_close_request` para validar; atalhos `Modal::confirm` / `Modal::alert` |
| Mensagem / confirmação | `MessageBox::info/warning/error/confirm`, `MessageBox::show` |
| Botão / rótulo / campo de texto | `Button`, `Label`, `TextBox` (`.multiline(true)`), `PasswordBox` |
| Texto formatado | `RichTextBox` |
| Opções | `CheckBox`, `RadioButton`, `ToggleButton`, `ComboBox`, `ListBox` |
| Número | `NumberBox` (limites, passo, decimais, expressões), `SpinButton`, `Slider`, `ProgressBar`, `ScrollBar` |
| Liga/desliga | `ToggleSwitch::builder().text("Wi-Fi").on(true)` (+ `on_toggled`) |
| Avaliação por estrelas | `RatingControl::builder().maximum(5).allow_half(true)` |
| Seção recolhível | `Expander::builder().header(..).content_height(n)` → filhos em `expander.content()` |
| Cartão | `Card::builder().title(..).subtitle(..).clickable(true)` → filhos em `card.content_bounds()` |
| Trilha de navegação | `BreadcrumbBar::builder().items(&[..])` + `on_item_click` / `truncate` |
| Gráfico de linha/área/barras/pizza | `Chart::builder().kind(ChartKind::Bar).categories(&[..]).series(ChartSeries::new(..))` |
| Data / hora | `DatePicker`, `TimePicker`, `MonthCalendar` |
| Tabela pequena | `ListView` |
| Tabela grande, editável ou virtual | `DataGrid` |
| Árvore | `TreeView` |
| Pares nome/valor editáveis | `PropertyGrid` |
| Abas | `TabControl` (`add_tab` devolve o `Panel` da aba; `set_detachable(true)` para arrastar abas para janelas próprias) |
| Painéis acopláveis (estilo IDE) | `DockHost` + `add_pane(título, DockSide::Left)` → `DockPane::content()`; `float()`/`dock()` |
| Janela sem moldura com barra própria | `TitleBar::attach(&janela)?` (no topo do layout) |
| Agrupar / conter controles | `Panel`, `GroupBox`, `SplitContainer` |
| Barra de menus / contexto | `MenuBar` + `Menu` (nativa) ou `MenuStrip` + `Menu` (desenhada, segue o tema), `ContextMenu` (`set_drawn(true)` para o mesmo desenho) |
| Barra de ferramentas / status | `ToolBar::new(&janela)?` (nativa), `StatusBar::new(&janela)?` |
| Barra de comandos com ícones, menus `▾`, botão dividido e busca | `CommandBar::builder().parent(&janela).build()?` + `add_button(CommandButton::new(..).icon(..).menu(&m))`, `add_separator`, `add_spacer`, `add_search(..)`, `add_widget(..)` |
| Posicionamento automático | `StackLayout`, `GridLayout`, `DockLayout`, `FlowLayout`, `AnchorLayout`, `SplitLayout` |
| Abrir / salvar / pasta | `OpenFileDialog`, `SaveFileDialog`, `FolderPicker` |
| Cor / fonte / impressão | `ColorDialog`, `FontDialog`, `PrintDialog` (`show_with(PrintOptions)` para intervalo de páginas) |
| Documento com várias páginas, pré-visualização | `PrintDocument::new(nome, páginas, \|i, canvas\| ..)` + `PrintPreviewDialog::new(doc).show(&janela)?` ou `printer.print_document(&doc)` |
| Pergunta com botões próprios | `TaskDialog` |
| Desenho próprio | `Panel::on_paint(\|canvas\| …)`; antialiasing: `canvas.with_gdiplus` (curvas e gradientes: `fill_figures`/`stroke_figures`); animação: `Direct2DSurface::attach` |
| Imagem SVG | `SvgImage::parse(..)?` → `render(l, a)?` / `canvas.draw_svg(..)` / `PictureBox::set_svg`; ícones: `icons::X.svg_image()` |
| Animar valores, posições, aparecer/sumir | `motion::animate_value(de, até, Duration, Easing::EaseOut, \|v\| ..)?`, `motion::animate_bounds(&w, Rect, ..)`, `motion::show_animated(&w, ShowEffect::SlideDown, ..)` |
| Pinça, rotação, arrasto com inércia (toque) | `gestures::on_gesture(&w, \|g\| ..)?` (`g.scale`, `g.rotation`, `g.translation`, `g.phase`) |
| Lista de candidatos do IME com o tema | `Ime::use_candidate_window(&campo)?` |
| Controle desenhado acessível | `accessibility::set_accessible_model(&painel, impl AccessibleModel)` |
| Tema claro/escuro | `Theme::dark().apply(&window)`, `Theme::system()`, `SystemTheme::current()` |
| Ícone na bandeja / notificação | `TrayIcon` |
| Notificação da Central de Ações (com botões) | `SystemNotification::new("Título").body(..).button("Abrir", "abrir").on_activated(\|arg\| ..).show()?` |
| Aviso dentro da janela | `InfoBar::builder().severity(Severity::Warning).title(..).message(..).parent(&w).build()?` (+ `add_action`, `open`/`close`) |
| Aviso temporário no canto | `Toast::new("Salvo").message(..).severity(Severity::Success).show(&janela)?` |
| Área de transferência | `Clipboard::set_text` / `Clipboard::text`; mudanças: `ClipboardWatcher::new(\|\| ..)?`; desenho vetorial: `Metafile::record(..)` + `ClipboardContent::metafile`; vários formatos de uma vez: `Clipboard::set(&ClipboardContent::new().text(..).html(..).image(&img)?)`; `Clipboard::{html, image, files, data("MeuApp.Itens")}` |
| Reordenar arrastando | `list.set_reorderable(true)` (+ `on_item_reorder_request(\|de, para\| true)` na `ListBox`, `on_row_reorder_request` na `ListView`/`DataGrid`); `tree.set_drag_enabled(true)` (+ `on_item_drop_request`, `on_item_moved`) |
| Arrastar para fora | `list.on_drag_out(\|i\| Some(ClipboardContent::new().text(..)))`; de qualquer controle: `start_drag(&w, &conteúdo, &[DropEffect::Copy])?` / `start_drag_with_image` |
| Receber arrastes (arquivos, texto, imagens) | `accept_drops(&w, DropHandler::new().on_over(\|d\| ..).on_drop(\|d, efeito\| ..))?`; `d.files()`, `d.text()`, `d.image()?`, `d.position()` |
| Abrir arquivo/URL no sistema | `arqrustui::system::shell::open(..)` |
| Atalho de teclado | `window.add_shortcut(Shortcut::ctrl(Key(b'S' as u16)), \|\| …)` |
| Temporizador | `Timer::interval(d, f)?` (guarde o valor) / `Timer::once(d, f)?` |
| Trabalho em segundo plano | `spawn_local(async { run_blocking(..).await })`, `spawn_background` |
| Datas/números regionais | `Locale::user_default().format_date(..)` etc. |
| Textos traduzíveis | `arqrustui::i18n::Translations`; textos internos da biblioteca: `i18n::set_library_language("en")`, `i18n::use_system_library_language()` (idioma do Windows) ou `apply_library_translations` |
| Ícone pronto (392 no pacote) | `icons::FOLDER`, `icons::SAVE`… (constantes); `icons::find("nome")`; prancha em `assets/icons/preview.png` |
| Redes sociais (58 desenhos) | `icons::YOUTUBE`, `WHATSAPP`, `X_TWITTER`, `BEREAL`, `KICK`, `KUAISHOU_KWAI`…; `IconCategory::Social`. Nomes anteriores preservados; lote com bisel em ICO/PNG 512/SVG, origem, atribuição e limites em `assets/icons/social/ORIGEM.md` |
| Ícone pelo tipo de arquivo (lista de arquivos, explorador) | `icons::for_path(&caminho).unwrap_or(&icons::FILE)` ou `icons::for_extension("pdf")`; constantes `icons::PDF_FILE`, `icons::XLSX_FILE`… (categoria `IconCategory::FileTypes`) |
| Ícone em botão / rótulo | `Button::builder().icon(&icons::SAVE)`, `Label::builder().icon(..)`; `set_icon`, `clear_icon`, `icon_position` |
| Ícone em árvore / lista / grade / abas | `tree.set_item_icon`, `list.set_row_icon`, `grid.set_row_icon` / `grid.set_icon_provider`, `tabs.set_tab_icon` |
| Ícone em lista suspensa / lista | `ComboBox::builder().with_icons(true)` + `add_item_with_icon` (idem `ListBox`) |
| Ícone em barra, menu, janela, bandeja, diálogo | `toolbar.add_button(t, Some(&icons::X), ..)`, `status.set_part_icon`, `item.set_icon`, `window.set_icon`, `TrayIcon::new(&icons::X, ..)`, `TaskDialog::custom_icon` |
| Imagem/ícone grande | `PictureBox::builder().icon(&icons::X)` (acima de 256 px usa o PNG) |
| Ícone próprio da aplicação | `IconAsset::new("nome", include_bytes!("a.ico"), include_bytes!("a.png"), None)` |

Assinaturas completas: `docs/COMPONENTS.md` (resumo) e rustdoc (`cargo doc --open`). Os programas em `examples/` são modelos prontos — **consulte o exemplo mais próximo antes de escrever uma tela nova**.

## 6. Padrão de builder (igual em todos os controles)

```rust
Tipo::builder()
    .text("…")                          // texto inicial
    .bounds(x, y, largura, altura)      // ou .position(x, y) e/ou .size(l, a)
    .parent(&container)                 // obrigatório
    .enabled(true).visible(true).tab_stop(true)
    .font(Font::new("Segoe UI", 10.0).bold())
    .tooltip("dica")
    /* opções específicas do tipo */
    .build()?
```

Exceções sem builder (construtor direto): `StatusBar::new(&w)?`, `ToolBar::new(&w)?`, `Rebar::new(&w)?`, `PropertyGrid::new(&pai, Rect)?`, `Dialog::new(..)?`, `Menu::new()?`, `MenuBar::new()?`, `MenuStrip::new()?`, `ContextMenu::new()?`, `TrayIcon::new(..)?`.

Métodos comuns a todo `Widget`: `text/set_text`, `bounds/set_bounds`, `set_position`, `set_size`, `client_size`, `show/hide/set_visible`, `set_enabled`, `focus`, `set_font`, `set_foreground/set_background`, `set_tooltip`, `set_context_menu`, `set_cursor`, `invalidate`, `destroy`, `is_alive`, `on_click`, `on_double_click`, `on_mouse_*`, `on_key_down/up`, `on_char`, `on_focus/on_blur`, `on_resize`, `on_destroy`, `on_context_menu`.

## 7. Layout — prefira layouts a coordenadas fixas

```rust
let window = Window::builder().title("Layout").size(480, 320).min_size(360, 240).build()?;
let editor = TextBox::builder().multiline(true).parent(&window).build()?;
let ok = Button::builder().text("OK").default(true).parent(&window).build()?;
let cancel = Button::builder().text("Cancelar").cancel(true).parent(&window).build()?;

let buttons = StackLayout::horizontal().spacing(8);
buttons.add_with(Spacer, LayoutParams::new().weight(1.0));          // empurra os botões para a direita
buttons.add_with(&ok, LayoutParams::new().width(90));
buttons.add_with(&cancel, LayoutParams::new().width(90));

let root = StackLayout::vertical().spacing(8).padding(Thickness::all(12));
root.add_with(&editor, LayoutParams::new().weight(1.0));            // ocupa o espaço restante
root.add_with(buttons, LayoutParams::new().height(28));             // layouts aninham
window.set_layout(root);                                            // reaplicado em todo redimensionamento
```

- Controles passam por referência (`&ok`); layouts e `Spacer` passam por valor.
- Sem `weight`, o item usa `width`/`height` de `LayoutParams` ou o tamanho atual do controle.
- Grade de formulário: `GridLayout::new(&[Track::Auto, Track::Star(1.0)], &[Track::Fixed(26), …]).gaps(8, 8)`; `grid.add(&label, linha, 0)`.
- Janela com barra de ferramentas/status: reserve a altura com padding — `DockLayout::new().padding(Thickness::new(0, toolbar.height(), 0, status.height()))`.
- Conteúdo de aba/painel: `painel.set_layout(..)`.
- Depois de adicionar itens a um layout já associado: `window.relayout()`.

## 8. Eventos — exemplos canônicos

```rust
button.on_click(|| { /* … */ });
text.on_text_changed({ let t = text.clone(); move || println!("{}", t.text()) });
check.on_value_changed(|checked: bool| { /* … */ });
combo.on_selection_changed(|index: Option<usize>| { /* … */ });
slider.on_value_changed(|value: i32| { /* … */ });
list_view.on_selection_changed(|rows: Vec<usize>| { /* … */ });
tree.on_selection_changed(|item: Option<TreeItem>| { /* … */ });
tabs.on_selection_changed(|index: usize| { /* … */ });
grid.on_cell_edited(|row, column, text: &str| { /* … */ });
window.on_close_request(|| true);         // devolva false para impedir o fechamento
window.on_resize(|size: Size| { /* … */ });
window.on_drop_files(|files| { /* &[PathBuf]; requer .accept_files(true) */ });
list.on_row_reorder_request(|from, to| true); // ListView/DataGrid; devolva false para recusar
tree.on_item_drop_request(|item, position: TreeDropPosition| true);
panel.on_mouse_down(|e: &MouseEvent| { /* e.position, e.button */ });
panel.on_key_down(|e: &KeyEvent| { if e.key == Key::ESCAPE { /* … */ } });
```

Estado compartilhado entre manipuladores:

```rust
let total = Rc::new(Cell::new(0u32));
let (counter, label) = (total.clone(), label.clone());
button.on_click(move || {
    counter.set(counter.get() + 1);
    label.set_text(&counter.get().to_string());
});
```

Remoção: `let id = w.on_click(..); w.remove_handler(id);`. Detalhes: `docs/EVENTS.md`.

## 9. Threads e operações demoradas

```rust
// Padrão recomendado: async sem runtime externo.
let (label, progress) = (label.clone(), progress.clone());
spawn_local(async move {
    progress.set_marquee(true);
    let texto = run_blocking(|| std::fs::read_to_string("grande.txt")).await; // roda em outra thread
    progress.set_marquee(false);
    match texto {
        Ok(conteudo) => label.set_text(&format!("{} bytes", conteudo.len())),
        Err(erro) => MessageBox::error("Erro", &erro.to_string()),
    }
})?;
```

- A closure de `run_blocking` precisa ser `Send`: **não capture controles nela** — capture só dados; use os controles depois do `.await`.
- De uma thread qualquer: `let ui = app.dispatcher(); ui.post(move || …)` (a closure roda na UI; também não pode capturar controles criados fora dela — passe dados e busque os controles por outro meio, ou use `spawn_local`).
- Pausa sem bloquear: `sleep(Duration::from_millis(100)).await`.

## 10. Receitas frequentes

**Diálogo modal com resultado**

```rust
let dialog = Dialog::new(&window, "Renomear", 320, 110)?;
let field = TextBox::builder().bounds(12, 12, 296, 24).parent(dialog.window()).build()?;
let ok = Button::builder().text("OK").bounds(212, 70, 96, 28).default(true).parent(dialog.window()).build()?;
let handle = dialog.clone();
ok.on_click(move || handle.close_with(DialogResult::Ok));
if dialog.show_modal()? == DialogResult::Ok { let _novo = field.text(); }
```

**Menus com atalhos**

```rust
let file = Menu::new()?;
file.add_item_with_shortcut("&Abrir...", Shortcut::ctrl(Key(b'O' as u16)), || { /* … */ })?;
file.add_separator()?;
let target = window.clone();
file.add_item("Sai&r", move || target.close())?;
let bar = MenuBar::new()?;
bar.add_menu("&Arquivo", &file)?;
window.set_menu(&bar)?;            // monte os menus ANTES de set_menu
```

**Barra de menus desenhada (acompanha o tema)**

```rust
// Mesmos `Menu`s; a faixa fica na área não cliente (área cliente e layouts não mudam).
file.add_radio_item("&Lista", true, || { /* … */ })?;          // grupo: itens de opção consecutivos
edit.on_open({ let colar = colar.clone(); move |_| colar.set_enabled(tem_texto()) }); // antes de abrir
let strip = MenuStrip::new()?;
strip.add_menu("&Arquivo", &file)?;
strip.add_menu_with_icon("&Ferramentas", &icons::TOOLS, &tools)?;
window.set_menu_strip(&strip)?;    // atalhos dos itens valem já, inclusive os acrescentados depois
Theme::dark().apply(&window);      // a barra e as listas acompanham; ou strip.set_style(Some(MenuStyle::dark()))
context.set_drawn(true);           // menu de contexto com o mesmo desenho
```

**Barra de comandos (ícones, menus, busca)**

```rust
let bar = CommandBar::builder().parent(&window).build()?;            // controle comum: reserve bar.height()
bar.add_button(CommandButton::new("Voltar").icon(&icons::BACK).icon_only().on_click(|| { /* … */ }))?;
bar.add_button(CommandButton::new("Início").icon(&icons::HOME).split(&paginas).on_click(|| { /* … */ }))?;
bar.add_separator()?;
bar.add_button(CommandButton::new("Ferramentas").icon(&icons::TOOLS).menu(&ferramentas))?; // ▾ abre o Menu
bar.add_spacer()?;                                                   // o que vem depois fica à direita
let busca = bar.add_search("Buscar", 220, |texto| { /* Enter */ })?; // TextBox; Esc limpa
let root = StackLayout::vertical();
root.add_with(&bar, LayoutParams::new().height(bar.height()));
```

**Abrir e salvar arquivo**

```rust
if let Some(path) = OpenFileDialog::new().filter("Texto", "*.txt").filter("Todos", "*.*").show(&window)? {
    editor.set_text(&std::fs::read_to_string(&path).map_err(|e| GuiError::InvalidArgument(e.to_string()))?);
}
if let Some(path) = SaveFileDialog::new().default_extension("txt").filter("Texto", "*.txt").show(&window)? { /* gravar */ }
```

**Tabela**

```rust
let list = ListView::builder().grid_lines(true).parent(&window).build()?;
list.add_column("Nome", 200, ColumnAlign::Left);
list.add_column("Tamanho", 90, ColumnAlign::Right);
list.add_row(&["a.txt", "12"]);
let sorter = list.clone();
list.on_column_click(move |column| sorter.sort_by_column(column, true));
```

**Grade grande / editável**

```rust
let grid = DataGrid::builder().editable(true).parent(&window).build()?;
grid.add_column("Chave", 160, ColumnAlign::Left);
grid.add_column("Valor", 160, ColumnAlign::Left);
grid.set_rows(vec![vec!["a".into(), "1".into()]]);
// ou, para milhões de linhas: grid.set_virtual_source(1_000_000, |linha, coluna| format!("{linha}:{coluna}"));
```

**Desenho personalizado**

```rust
let canvas_panel = Panel::builder().background(Color::WHITE).parent(&window).build()?;
canvas_panel.on_paint(|canvas| {
    let size = canvas.size();
    canvas.line(Point::new(0, 0), Point::new(size.width, size.height), &Pen::new(Color::RED, 2));
    canvas.text("Olá", Point::new(10, 10), &Font::system(), Color::BLACK);
});
// Para redesenhar após mudar o estado: canvas_panel.invalidate();
```

**Tema**

```rust
Theme::system().apply(&window);                     // acompanha claro/escuro/alto contraste
let target = window.clone();
window.on_theme_changed(move || Theme::system().apply(&target));
```

**Aplicação na bandeja**

```rust
app.set_quit_on_last_window_closed(false);
let tray = TrayIcon::new(&Icon::system(SystemIcon::Application)?, "Meu app")?;   // mantenha `tray` vivo
let menu = ContextMenu::new()?;
menu.add_item("Sair", Application::quit)?;
tray.set_menu(&menu);
```

**Gráfico, SVG e animação**

```rust
let chart = Chart::builder()
    .kind(ChartKind::Bar)
    .categories(&["T1", "T2", "T3"])
    .series(ChartSeries::new("Vendas", &[12.0, 18.0, 9.0]))
    .parent(&window)
    .build()?;
chart.set_values(0, &[14.0, 20.0, 11.0]);                        // transição animada
let logo = PictureBox::builder().size(96, 96).parent(&window).build()?;
logo.set_svg(SvgImage::parse(r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="teal"/></svg>"#)?);
motion::animate_bounds(&logo, Rect::new(200, 40, 96, 96), Duration::from_millis(250), Easing::EaseOut)?;
```

**Controles modernos e janela sem moldura**

```rust
let bar = TitleBar::attach(&window)?;                              // tira a moldura do sistema
let host = DockHost::builder().parent(&window).build()?;
let root = StackLayout::vertical();
root.add_with(&bar, LayoutParams::new().height(bar.height()));
root.add_with(&host, LayoutParams::new().weight(1.0));
window.set_layout(root);
let files = host.add_pane("Arquivos", DockSide::Left)?;            // filhos em files.content()
let wifi = ToggleSwitch::builder().text("Wi-Fi").parent(&host.center()).build()?;
wifi.on_toggled(|on| println!("{on}"));
let qtd = NumberBox::builder().range(0.0, 99.0).parent(&host.center()).build()?;
```

**Ícones**

```rust
// Constantes de `arqrustui::icons` (recurso `icons`, ligado por padrão): só os ícones usados vão para o
// executável em release com LTO. Todo método que exibe ícone aceita `&impl IconSource`
// (um `IconAsset` do pacote ou um `Icon`) e carrega a resolução certa para o DPI do controle.
let salvar = Button::builder().text("&Salvar").icon(&icons::SAVE).parent(&window).build()?;
let pasta = tree.add_root("Projetos")?;
tree.set_item_icons(pasta, &icons::FOLDER, &icons::FOLDER_OPEN)?;     // fechada / selecionada
list.set_row_icon(0, &icons::JSON_FILE)?;
grid.set_icon_provider(Some(|linha: usize| (linha % 2 == 0).then_some(&icons::FILE)))?; // fonte virtual
let tipos = ComboBox::builder().with_icons(true).parent(&window).build()?;
tipos.add_item_with_icon("Python", &icons::PYTHON)?;
canvas.draw_icon(&icons::WARNING, Point::new(8, 8), 32);              // em on_paint
```

### Texto formatado por trechos
Use StyledText::new(fonte, pontos), push(texto, TextFormat) e RichTextBox::set_rtf(documento.to_rtf()). A serialização Unicode e as tabelas RTF pertencem à biblioteca. Exemplo completo com diálogos nativos: examples/styled_text.rs. Não monte sintaxe RTF manual na aplicação.

### Composição reutilizável
Use StatusBarLayout para reservar a barra de status real e fit_height após mostrar/ocultar conteúdo. Use ScrollViewport para conteúdo vertical rolável: viewport no layout, controles em content, medição em layout_content. Use StyledText::push_mapped para destacar por posição Unicode. Referência: docs/COMPOSITION.md; exemplo independente: examples/composition.rs. As mensagens, ícones e critérios de destaque pertencem à aplicação.

`StatusBarLayout::fit_height` só redimensiona janelas no estado normal. Maximizada, minimizada ou em tela cheia, preserva a geometria e atualiza limites/layout; após restaurar, chame novamente para ajustar a altura. Regressões de maximização e rodapé: `tests/window_maximize.rs`, executadas em Windows a 144 DPI.

### Destaque visual em editores
Use `RichTextBox::apply_highlights(&base, &[(intervalo_utf16, TextFormat)])` para aplicar estilos sem substituir o conteúdo, mover a seleção ou acrescentar etapas ao desfazer. CRLF conta como uma posição no RichEdit; caracteres fora do BMP ocupam duas unidades UTF-16. A aplicação é responsável pela análise do texto, pela paleta e por descartar resultados obsoletos de tarefas assíncronas. A biblioteca não conhece formatos de dados. Exemplo: `examples/rich_highlight.rs`; teste nativo: `tests/rich_highlight.rs`.

### Alinhamento de botões
Use `Button::builder().content_alignment(TextAlign::Right)` para alinhar o grupo ícone/texto à direita. `icon_position` controla apenas a ordem relativa; não o alinhamento do grupo. A margem interna mínima usual é de 8 unidades lógicas e o texto recebe reticências quando não cabe. O padrão permanece centralizado. Exemplo: `examples/button_alignment.rs`; teste nativo: `tests/button_alignment.rs`.

## 11. Erros comuns de geração de código — evite

| Errado | Certo |
|---|---|
| Criar controles antes de `Application::new()` | `Application::new()?` é a primeira linha de `main` |
| Esquecer `.parent(&window)` | todo controle tem pai |
| Usar um controle dentro da closure sem clonar (`move` leva o original) | `let c = c.clone();` antes da closure |
| `button.on_click(\|\| label.set_text(..))` capturando por referência | closures são `'static`: use `move` com clones |
| `std::thread::spawn(move \|\| label.set_text(..))` | não compila; use `spawn_local`/`run_blocking`/`dispatcher` |
| `thread::sleep` na UI para "esperar" | `Timer::once`, ou `sleep(..).await` em `spawn_local` |
| Soltar o `Timer` de `Timer::interval` logo após criar | guarde o valor enquanto quiser os tiques |
| Soltar o `TrayIcon` | o ícone some; mantenha-o vivo até o fim |
| Multiplicar coordenadas por DPI | a biblioteca já escala |
| Esperar `on_value_changed` após `set_value` | eventos só para ações do usuário; chame sua lógica diretamente |
| Tratar `Ok(None)` de diálogo como erro | é "cancelado" |
| `unwrap()` em chamadas de GUI | propague com `?` (`main` devolve `GuiResult<()>`) |
| Adicionar crates de GUI/Win32 (`windows`, `winapi`, `winit`…) | proibido: tudo passa pela ArqRustUI |
| Posicionar tudo com coordenadas fixas em janela redimensionável | use layouts |
| Esquecer `window.show()` | janelas nascem ocultas |
| `icons::find("folder")` para um ícone fixo | `icons::FOLDER` (`find`/`all` embutem o pacote inteiro) |
| `add_item_with_icon` numa `ComboBox`/`ListBox` comum | crie com `.with_icons(true)` (o desenho próprio é definido na criação) |
| Carregar ícone com tamanho multiplicado pelo DPI | passe o tamanho lógico; `icon_for(&widget, 16)` ou o próprio controle escolhe a resolução |
| Esperar que o `.svg` seja desenhado | o Win32 não desenha SVG: use o `.ico`/`.png`; `svg_bytes`/`save` só exportam |

## 12. Quando a biblioteca não tem o recurso

1. Procure em `docs/COMPONENTS.md`, no rustdoc e em `examples/`.
2. Confira `docs/FEATURE_MATRIX.md` e as limitações em `docs/COMPATIBILITY.md`.
3. Veja se dá para **compor** com o que existe (ex.: controle próprio = `Panel` + `on_paint` + eventos — `docs/CUSTOM_CONTROLS.md`, nível 1).
4. Se faltar mesmo, **implemente na ArqRustUI de forma genérica** (seção 13) e use a nova API pública no projeto. Não contorne com `unsafe`/FFI espalhados pela aplicação.
5. Só como último recurso e de forma pontual: `widget.handle()`, `on_raw_message` e `arqrustui::ffi` (API avançada), com comentário `// SAFETY:`.

## 13. Estendendo a ArqRustUI (para agentes que alteram a biblioteca)

### Organização

```
src/ffi/            declarações Win32 por DLL — só tipos, constantes e extern
src/core/           Application, Window, WidgetCore/Widget, eventos, loop, handles, erros
src/core/native.rs  helpers que encapsulam unsafe recorrente (send, send_ref, send_mut, send_text…)
src/controls/       um arquivo por controle; common.rs tem as macros widget_type!, builder_common!, plain_builder!
                    controles grandes se dividem em submódulos por tema, com `impl Controle { .. }` próprio e `use super::*`
                    (datagrid/, commandbar/, menus/popup/, menus/strip/, accessibility/tree/,
                    system/notifications/); helpers chamados pelo pai ficam `pub(super)`
src/layout/         engine.rs = cálculo puro (testável); mod.rs = tipos de layout
src/{menus,dialogs,graphics,themes,system,input,accessibility,i18n,utilities}/
src/icons/          IconAsset, IconCategory, find/all; catalog.rs = uma linha icon!(..) por ícone
src/controls/icon_support.rs  ícone do controle (WidgetIcon) e listas de imagens gerenciadas (ManagedImages)
src/controls/modern.rs  base dos controles modernos desenhados (paleta do tema, modelo acessível por closures)
src/core/motion.rs  animações (Easing, animate, transições) dirigidas por um único Timer
src/graphics/vector.rs  figuras/tintas/traço em ponto flutuante desenhados pelo GDI+; svg.rs + svg/{xml,parse}.rs
src/input/{gestures.rs, ime.rs + ime/candidates.rs, drag.rs + drag/{source,target,reorder}.rs}
crates/arqrustui-icons/        fachada: reexporta as crates de dados (dependência opcional do recurso `icons`)
crates/arqrustui-icons-dataN/  .ico/.png/.svg com include_bytes!, até 9 MB por crate (limite do crates.io)
assets/icons/       README (origem, tratamento, como acrescentar) e preview.png
.github/workflows/ci.yml  CI: fmt, clippy, doc, testes em Windows (MSVC/GNU), MSRV, docs.rs, tamanho dos pacotes
examples/  tests/  docs/
```

Arquitetura completa: `docs/ARCHITECTURE.md`. Modelo de controle novo: `docs/CUSTOM_CONTROLS.md` (nível 2).

### Regras invioláveis

1. **Sem dependências de terceiros.** Atualmente todas as seções são vazias: apenas std e implementações próprias. Eventuais dependências ArqRust devem ser próprias, locais e irmãs, usando `path = "../ArqRustNome"`. A regra inclui desenvolvimento, compilação, opcionais, plataformas e dependências transitivas. Não use registry, Git, código de terceiros copiado nem dependências circulares. Consulte [BIBLIOTECA.md](BIBLIOTECA.md).
2. **Somente APIs nativas e documentadas do Windows**, declaradas à mão em `src/ffi/` com os nomes da Microsoft. Nada de GTK/Qt/WebView/HTML, nem crates `windows`/`winapi`/`winit`.
3. **Genérica e desacoplada.** Nenhum nome, tipo, texto, fluxo ou regra de negócio de aplicação consumidora. Uma necessidade específica de um projeto vira um recurso **geral** (parâmetros e callbacks), útil a qualquer aplicação. Exemplos e testes também não mencionam consumidores.
4. **`unsafe` isolado e justificado.** Fica em `ffi/`, `core/native.rs` e blocos mínimos; todo bloco tem `// SAFETY:` explicando a validade; toda `pub unsafe fn` tem `# Safety`.
5. **Sem `Send`/`Sync` manuais** em tipos ligados a janelas.
6. **Nenhum ponteiro para estado Rust guardado no Windows** além da duração de uma chamada síncrona.
7. **Ownership correto por tipo de handle** (tabela em `docs/MEMORY_AND_SAFETY.md`); todo recurso nativo novo ganha um wrapper RAII ou entra em um existente.
8. **Erros explícitos**: `GuiResult`; nada de engolir falhas com valores padrão; recurso ausente ⇒ `GuiError::Unsupported`.
9. **APIs que não existem em todo Windows 10/11** são resolvidas em `ffi/dynamic.rs` (`GetProcAddress`), nunca importadas estaticamente.
10. **API pública em unidades lógicas**, documentada (`#![warn(missing_docs)]`), sem `HWND`/`WPARAM`/`LPARAM` nas assinaturas (exceto API avançada explícita).
11. **Nenhum pânico atravessa o Windows.** Toda função `extern "system"` com corpo (método COM, procedimento de janela, callback) começa com `crate::core::ffi_guard::guard_hresult(move || { .. })` (COM) ou `guard(valor_de_falha, move || { .. })`; o teste `every_system_entry_point_is_guarded` falha se faltar.
12. **Sem placeholders**: nada de funções vazias, `todo!()` ou recursos anunciados sem implementação e teste.

### Convenções

- Nomes de tipos/métodos e documentação do código (rustdoc, inclusive macros e exemplos publicados) em inglês; guias em português com versão em inglês ao lado (`*.en.md`); mensagens para o desenvolvedor (`GuiError`, `expect`, `#[deprecated]`) e textos e comentários dos `examples/` em inglês; textos exibidos ao usuário final somente pela tabela `LibraryText` (pt e en), nunca fixos no código.
- Controle = handle `Clone` com `core: Rc<WidgetCore>` + `XBuilder` + `X::builder()`.
- Eventos: `on_<evento>(impl Fn(..) + 'static) -> HandlerId`, encaminhando para `core.on(EventKind::…, …)`; notificações do pai tratadas em `core.set_reflect(..)`.
- Nome do evento (detalhes em `docs/EVENTS.md`, "Nomes dos eventos"): verbo no presente para ação direta do usuário (`on_click`, `on_item_click`); particípio para aviso depois do fato (`on_value_changed`, `on_toggled`, `on_closed`); `on_<ação>_request` com callback `-> bool` para pedido que pode ser recusado (`on_close_request`). Substantivo do vocabulário do controle (`item` na `ListBox`/`TreeView`, `row` na `ListView`/`DataGrid`). Nunca renomeie um evento existente sem deixar o nome antigo como alias `#[deprecated]`.
- Toda `pub struct` de saída nova (eventos, resultados de diálogos, informações que a biblioteca devolve e quem usa só lê) leva `#[non_exhaustive]`; se quem usa também precisar montá-la, ofereça um construtor (como `ImeCandidates::new`). Tipos de valor (`Color`, `Rect`, `Thickness`…) e estilos montados com `..Default::default()` não levam.
- Todo `pub enum` novo leva `#[non_exhaustive]`, exceto um conjunto fechado por natureza (como `Orientation`, `CheckState`, `TreeDropPosition`); dentro da biblioteca isso não muda nada, mas exemplos e testes (que são outros crates) precisam de braço `_` ao fazer `match`.
- Setters não disparam eventos de usuário.
- Ícones: métodos públicos aceitam `&impl IconSource`; controles com imagem por item usam `ManagedImages` (uma posição por origem distinta, refeita quando o DPI muda).
- Formatação: `rustfmt.toml` do repositório (`max_width = 120`).

### Definição de pronto para qualquer alteração

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets   # zero avisos
cargo test                      # tudo passando (cria janelas reais)
cargo build --release
```

O mesmo roda no CI. Mudança visível para quem usa a biblioteca entra em `CHANGELOG.md` (seção `[Não publicado]`) e em `CHANGELOG.en.md` (`[Unreleased]`). Além disso: teste novo (unitário para lógica pura; integração em `tests/` para janelas), exemplo ou uso no `examples/demo.rs` quando aplicável, e atualização de `docs/COMPONENTS.md`, `docs/FEATURE_MATRIX.md` e deste arquivo. Os guias têm uma versão em inglês ao lado (`*.en.md`): `AGENTS`, `BIBLIOTECA`, `CHANGELOG` e, em `docs/`, `ARCHITECTURE`, `COMPATIBILITY`, `COMPONENTS`, `CUSTOM_CONTROLS`, `EVENTS`, `FEATURE_MATRIX`, `MEMORY_AND_SAFETY` e `RELEASING`. Toda alteração num desses arquivos é feita também na outra versão, no mesmo trabalho, com o mesmo conteúdo.

## 14. Estado atual e ressalvas

- Validação local de 16/11/2026: Clippy rigoroso e rustdoc em inglês sem avisos;
  60 doctests e cinco testes GDI aprovados. Suíte completa: 356 passaram,
  32 falhas iguais às do registro anterior e um diálogo de impressão excluído
  por travamento prévio. Demos conferidas nos tamanhos padrão e mínimo a 150%.
  Veja `docs/CORRECOES_DEMOS_E_RUSTDOC.md`; os números abaixo são históricos.
- Versão 0.1.0. 145 testes unitários + 176 de integração + 60 doctests, em 64 e em 32 bits; sob Wine 9/Xvfb todos passam menos `window_maximize` (exige um gerenciador de janelas que maximize).
- **A execução foi validada sob Wine 9 (96 e 144 DPI), não em Windows real** (exceto dois itens de composição registrados como testados em Windows por um trabalho anterior; veja a seção 4.14 de `docs/FEATURE_MATRIX.md`). Ao trabalhar em uma máquina Windows, rode `cargo test` e `cargo run --example demo` logo no início e registre qualquer divergência; a lista do que mais precisa de conferência está em `docs/COMPATIBILITY.md`.
- Recursos parciais ou só validados no Wine (não os apresente como completos nem como "testados no Windows"): arrastar e soltar (imagem de arraste do shell e soltar em outros programas não observados), notificações do sistema (`SystemNotification` com campos, progresso, agendamento e ativador COM: implementadas, não observadas — o Wine não tem o recurso), UIA (padrões verificados chamando os provedores, nunca com Narrador/NVDA), impressão (pré-visualização testada; impressão em papel não), IME (lista de candidatos com candidatos simulados), toque (gestos com mensagens sintéticas; sem tela de toque), SVG (subconjunto; o Wine não suaviza caminhos em imagens), `TitleBar` (sombra do DWM, maximizar e layouts do Windows 11 não observados), `DockHost` e abas destacáveis (sem alvos de acoplamento ao arrastar), tema escuro (barras de rolagem dependem do Windows 10 1809+), ícone em botão nativo (sem Common Controls v6 o botão passa a ser desenhado pela biblioteca), `MenuStrip`/`CommandBar`/controles modernos (validados só no Wine), `Modal` (no Wine o fundo é uma foto escurecida da janela, porque o Wine sem compositor não compõe transparência por pixel; a transparência real, a sombra sobre a janela viva e os cantos do Windows 11 não foram observados). Detalhes em `docs/FEATURE_MATRIX.md` e `docs/COMPATIBILITY.md`.
