# Guia de componentes

*English: [COMPONENTS.en.md](COMPONENTS.en.md).*

Referência resumida. A referência completa, com todos os métodos, está no rustdoc (`cargo doc --open`).

O rustdoc é escrito em inglês, inclusive exemplos e documentação gerada por macros.
`Canvas::with_gdiplus` aplica a escala de DPI uma vez e preserva recorte e
transformação do canvas. A demo `modern_controls` usa layouts que acompanham o
redimensionamento; o comando New das demos `demo` e `menus` usa `icons::FILE`.

## Convenções válidas para todos os controles

```rust
let controle = Tipo::builder()      // todo controle com opções tem um builder
    .text("…")                      // texto inicial
    .bounds(x, y, largura, altura)  // ou .position(x, y) + .size(l, a) — unidades lógicas
    .parent(&janela_ou_container)   // OBRIGATÓRIO
    .enabled(true).visible(true).tab_stop(true)
    .font(Font::new("Segoe UI", 10.0)).tooltip("dica")
    .build()?;                      // GuiResult<Tipo>
```

Métodos de `Widget` (todos os controles e `Window`): `text`/`set_text`, `bounds`/`set_bounds`, `position`/`set_position`, `size`/`set_size`, `client_size`, `show`/`hide`/`set_visible`/`is_visible`, `set_enabled`/`is_enabled`, `focus`/`has_focus`, `set_tab_stop`, `set_font`/`font`, `set_foreground`/`set_background`, `set_cursor`, `set_tooltip`, `set_context_menu`, `invalidate`, `dpi`/`scale_factor`, `set_accessible_name`/`set_accessible_description`, `destroy`/`is_alive`, `handle`, e os eventos `on_*` descritos em [EVENTS.md](EVENTS.md).

## Janela

Sem `.icon(...)`, toda `Window` usa `icons::DEFAULT_WINDOW_ICON`, inclusive janelas
secundárias e diálogos próprios. O logo permanece disponível sem o recurso
`icons`; `.icon(&outro)` ou `set_icon(&outro)` prevalecem sobre esse padrão e
continuam sendo reaplicados após mudanças de DPI. Recursos de ícone do arquivo
executável pertencem à compilação de cada aplicativo.

| Tipo | Construção | Principais métodos |
|---|---|---|
| `Window` | `Window::builder().title().size().position().centered().resizable().minimizable().maximizable().borderless().topmost().tool_window().dialog_frame().opacity().owner().icon().min_size().max_size().accept_files().close_on_escape().background().right_to_left().build()?` | `close`, `maximize`/`minimize`/`restore`, `set_fullscreen`, `center`, `move_to_monitor`, `monitor`, `set_opacity`, `set_topmost`, `set_icon`, `set_menu`, `set_menu_strip`/`remove_menu_strip`/`menu_strip`, `system_menu`, `set_layout`/`relayout`, `set_min_size`/`set_max_size`, `add_shortcut`, `show_modal`, `on_close_request` (`false` cancela), `on_paint`, efeitos DWM (`set_dark_title_bar`, `set_corner_style`, `set_border_color`, `set_caption_color`, `set_backdrop`, `extend_frame_into_client`) |

## Controles fornecidos pelo Windows

| Tipo | Classe nativa | Opções do builder | Métodos próprios |
|---|---|---|---|
| `Button` | `BUTTON` | `default`, `cancel`, `style`; altura padrão `DEFAULT_CONTROL_HEIGHT` | `click`, `set_style`; ícone: builder `icon`, `icon_size` (padrão: automático, 60% da altura), `icon_position`; `set_icon`, `clear_icon`, `set_icon_position`, `set_icon_size(Some(n) \| None)`, `icon_size()` (lado efetivo), `is_icon_native`. Ícone e texto formam um grupo centralizado, com espaço de ~1/3 do ícone entre eles e margem de 8 da borda; texto que não cabe vira reticências |
| `Label` | `STATIC` | `align`, `no_prefix`, `single_line` (uma linha centralizada na altura, para alinhar com campos) | —; ícone: builder `icon`, `icon_size`, `icon_position`; `set_icon`, `clear_icon` |
| `TextBox` | `EDIT` | altura padrão `DEFAULT_CONTROL_HEIGHT` com o texto centralizado (uma linha); `multiline`, `word_wrap`, `read_only`, `numeric`, `align`, `placeholder`, `max_length` | `set_placeholder`, `select_all`, `selection`/`set_selection`, `replace_selection`, `append`, `set_read_only`, `undo`, `cut`/`copy`/`paste`, `is_modified`/`set_modified`, `line_count` |
| `PasswordBox` | `EDIT` + `ES_PASSWORD` | `placeholder`, `max_length` | `password`, `set_revealed` + métodos de edição |
| `RichTextBox` | `RICHEDIT50W` | `read_only`, `word_wrap` | `format_selection`, `format_all`, `selection_style`, `set_alignment`, `set_list_style(ListStyle)`/`list_style` (marcadores, números, letras, romanos), `insert_rtf`, `insert_table(linhas, larguras, cabeçalho)`, `insert_image(&PlusImage, l, a)`, `rtf`/`set_rtf`, `find` + métodos de edição |
| `CheckBox` | `BUTTON` | `checked`, `three_state` | `is_checked`/`set_checked`, `state`/`set_state` |
| `RadioButton` | `BUTTON` | `checked`, `group_start` | `is_checked`/`set_checked` |
| `ToggleButton` | `BUTTON` + `BS_PUSHLIKE` | `checked` | `is_checked`/`set_checked` |
| `ComboBox` | `COMBOBOX` | `editable`, `sorted`, `items`, `selected`, `with_icons` (só para as editáveis); altura padrão `DEFAULT_CONTROL_HEIGHT` (28, a mesma do `Button`), respeitada em qualquer fonte e DPI, com o texto centralizado | `add_item`, `insert_item`, `remove_item`, `clear`, `count`, `item_text`, `selected_index`/`set_selected_index`, `selected_text`; ícones (não editáveis, ou editáveis com `with_icons(true)`): `add_item_with_icon`, `set_item_icon`, `clear_item_icon`, `item_image` |
| `ListBox` | `LISTBOX` | `multi_select`, `sorted`, `items` | `add_item`, `insert_item`, `remove_item`, `clear`, `count`, `item_text`, `selected_index`, `selected_indices`, `set_selected_index`, `set_item_selected`; com `with_icons(true)`: `add_item_with_icon`, `set_item_icon`, `clear_item_icon`, `item_image` |
| `GroupBox` | `BUTTON` + `BS_GROUPBOX` | — | `set_layout` (aceita filhos) |
| `ProgressBar` | `msctls_progress32` | `range`, `value`, `vertical`, `marquee` | `set_range`, `value`/`set_value`, `advance`, `set_marquee`, `set_state` |
| `Slider` | `msctls_trackbar32` | `range`, `value`, `vertical`, `ticks` | `set_range`, `value`/`set_value`, `set_page_size` |
| `ScrollBar` | `SCROLLBAR` | `vertical`, `range`, `page`, `value` | `configure`, `value`/`set_value` |
| `UpDown` | `msctls_updown32` | — | `set_range`, `value`/`set_value` |
| `DatePicker` / `TimePicker` | `SysDateTimePick32` | `long_format` (data); altura padrão `DEFAULT_CONTROL_HEIGHT`, texto centralizado | `value`/`set_value`, `set_format` |
| `MonthCalendar` | `SysMonthCal32` | — | `selected_date`/`set_selected_date`, `minimum_size` |
| `Hyperlink` | `SysLink` | texto com `<a href="…">` | `on_link_click`, `open_links_in_browser` |
| `Tooltip` | `tooltips_class32` | — | `Tooltip::set`, `remove`, `set_enabled` (ou `widget.set_tooltip`) |
| `StatusBar` | `msctls_statusbar32` | `StatusBar::new(&janela)?` | `set_parts`, `set_part_text`, `part_text`, `height`, `set_part_icon`, `clear_part_icon`, `has_part_icon` |
| `ToolBar` | `ToolbarWindow32` | `ToolBar::new(&janela)?` | `add_button`, `add_toggle_button`, `add_separator`, `set_button_enabled`, `set_button_checked`, `trigger`, `height` (ícone: `Some(&icons::X)` ou `Some(&Icon)`) |
| `ListView` | `SysListView32` | `single_selection`, `grid_lines`, `checkboxes` | `add_column`, `column_width`, `add_row`/`insert_row`, `cell`/`set_cell`, `remove_row`, `clear`, `row_count`, `selected_rows`, `set_row_selected`, `is_row_checked`, `set_image_list`, `set_row_image`, `sort_by_column`, `set_row_icon`, `clear_row_icon`, `row_image`, `set_icon_size` |
| `TreeView` | `SysTreeView32` | `checkboxes` | `add_root`, `add_child`, `remove`, `clear`, `roots`, `children`, `parent_of`, `item_text`, `item_path`, `expand`/`collapse`, `selected`/`select`, `is_checked`/`set_checked`, `set_image_list`, `set_item_image`, `set_item_icon`, `set_item_icons`, `clear_item_icon`, `item_image`, `set_icon_size` |
| `TabControl` | `SysTabControl32` | — | `add_tab` (devolve o `Panel` da aba), `remove_tab`, `tab_count`, `page`, `tab_title`/`set_tab_title`, `selected_index`/`set_selected_index`, `set_tab_icon`, `clear_tab_icon`, `tab_image`, `tab_at`, `tab_bounds`; abas destacáveis: `set_detachable(true)` (arrastar para fora), `detach_tab(i)` → `DetachedTab { title, page, window }`, `attach_tab(&janela)`, `detached_tabs`, `on_tab_detached`, `on_tab_attached` (fechar a janela devolve a aba) |
| `HeaderControl` | `SysHeader32` | — | `add_item`, `remove_item`, `item_count`, `item_width`, `set_sort_indicator` |
| `Rebar` | `ReBarWindow32` | `Rebar::new(&janela)?` | `add_band`, `remove_band`, `band_count`, `height` |
| `Pager` | `SysPager` | `horizontal` | `set_child`, `scroll_position`/`set_scroll_position` |
| `HotKeyBox` | `msctls_hotkey32` | — | `shortcut`/`set_shortcut` |
| `IpAddressBox` | `SysIPAddress32` | — | `address`/`set_address`, `clear` |
| `Animation` | `SysAnimate32` | — | `open`, `play`, `stop` |

## Controles de implementação própria

| Tipo | Como é construído | Principais métodos |
|---|---|---|
| `Panel` | classe de janela da biblioteca | `set_layout`, `on_paint`, `set_double_buffered`; builder: `border`, `background`, `focusable` |
| `SpinButton` | `EDIT` + `UpDown` acoplados | `set_range`, `value`/`set_value`, `on_value_changed` |
| `PictureBox` | painel com desenho próprio | builder `icon`, `icon_size`, `background`; `set_icon`, `set_image(PlusImage)`, `set_svg(SvgImage)` (nítido em qualquer tamanho), `set_icon_size`, `clear`, `has_content` (acima de 256 px desenha o `.png` do ícone) |
| `SplitContainer` | dois `Panel`s + divisor arrastável | `first()`, `second()`, `split_position`/`set_split_position`, `set_min_sizes`, `on_split_moved`; builder: `stacked`, `split_position`, `min_sizes` |
| `DataGrid` | ListView virtual + modelo em Rust + editor sobreposto | `add_column`, `set_rows`, `add_row`, `remove_row`, `clear`, `set_virtual_source`, `cell`/`set_cell`, `rows`, `sort_by`, `selected_rows`, `set_column_read_only`, `set_validator`, `begin_edit`/`commit_edit`/`cancel_edit`, `on_cell_edited`; builder: `editable`, `sortable`, `single_selection`; ícones: `set_row_icon` (acompanha a ordenação), `set_icon_provider`, `set_icon_size`, `row_image`; estilos: `set_cell_style(linha, coluna, CellStyle::new().foreground().background().bold().italic())`, `set_row_style`, `set_cell_style_provider`, `clear_styles`; `set_column_kind(c, ColumnKind::Check)` + `toggle_check`, `parse_check`; fonte virtual editável: `set_virtual_editor`; `set_filter`/`set_text_filter`/`has_filter`; `set_group_by(Some(coluna))`, `groups`, `set_group_collapsed`, `on_group_toggled`; `to_csv`/`to_tsv`/`to_html`, `export_csv(caminho)`, `copy_to_clipboard` |
| `CommandBar` | `Panel` desenhado pela biblioteca + `EDIT` hospedado na busca; menus com o desenho da `MenuStrip` | `CommandBar::builder().parent(&w).build()?` (altura padrão `COMMAND_BAR_HEIGHT` = 36; builder `style(MenuStyle)`, `bottom_border`); `add_button(CommandButton)`, `add_separator`, `add_spacer` (empurra o resto para a direita), `add_label`, `add_search(texto_de_espera, largura, on_search)` → `TextBox` (Enter busca, Esc limpa, `×` limpa), `add_widget(&controle_filho, largura)`, `clear`, `len`, `height`, `set_style`/`style`, `overflow_count`, `open_overflow`. `CommandButton::new(texto)` → `icon`, `icon_only`, `tooltip`, `on_click`, `toggle(marcado)` + `on_toggled`, `menu(&Menu)` (botão `▾`), `split(&Menu)` (ação + seta), `enabled`. `CommandItem` → `set_enabled`, `set_checked`/`is_checked`, `set_text`/`text`, `set_icon`/`clear_icon`, `set_tooltip`, `set_visible`, `is_in_overflow`, `trigger`, `open_menu`. O que não cabe vai para o botão `»`; com o menu de um botão aberto, passar o cursor sobre outro botão de menu troca a lista; Tab até a barra, setas, Enter/Espaço e Baixo/F4 pelo teclado. Personalização: `customize()` (caixa com Adicionar/Remover/Para cima/Para baixo/Restaurar padrão), `layout()`/`set_layout(&CommandBarLayout)` (texto salvável), `set_default_layout`, `reset_layout`; `set_reorderable(true)` arrasta botões. Controles hospedados que não cabem vão para o `»`. |
| `InfoBar` | `Panel` desenhado pela biblioteca + `Button`s de ação | builder `severity(Severity)`, `title`, `message`, `closable` (altura padrão `INFO_BAR_HEIGHT` = 44); `add_action(texto, on_click)` → `Button`; `open`, `open_with(gravidade, título, mensagem)`, `close`, `is_open`, `on_closed`; `set_severity`, `set_title`, `set_message`, `set_closable`. Abrir/fechar mostra/esconde e refaz o layout do contêiner. O nome acessível anuncia gravidade, título e mensagem. |
| `Toast` | janela `WS_POPUP` própria, sem ativação, translúcida só durante o esmaecimento | `Toast::new(título)` → `message`, `severity`, `duration`, `persistent`, `action(texto, f)`, `on_dismissed`, `show(&janela)` → `ToastHandle` (`close`, `is_open`); `Toast::dismiss_all(&janela)`, `Toast::count(&janela)`. Canto inferior direito, empilhados, acompanham a janela; o tempo pausa com o cursor por cima. |
| `PropertyGrid` | `DataGrid` de duas colunas com valores tipados | `PropertyGrid::new(&pai, Rect)?`, `add_category`, `add_property`, `value`/`set_value`, `property_names`, `activate`, `on_property_changed`, `set_choice_mode(ChoiceMode::DropDown \| Cycle)`, `set_bool_labels(Some(("Ligado", "Desligado")))` |
| `ToggleSwitch` | painel desenhado (botão deslizante animado) | builder `text` (título), `on`, `state_texts(ligado, desligado)`; `is_on`/`set_on`/`toggle`, `set_header`, `state_text`, `on_toggled(\|on\| ..)`; Espaço/Enter; UIA Toggle |
| `NumberBox` | `EDIT` hospedado + botões ▲/▼ desenhados | builder `value`, `range`, `step`, `large_step`, `decimals`, `decimal_comma`; `value`/`set_value`, `set_range`/`range`, `commit`, `text_box`, `on_value_changed`; ↑/↓, PgUp/PgDn, Enter avalia expressões (`numberbox::evaluate`); UIA RangeValue + Value |
| `RatingControl` | painel desenhado (estrelas) | builder `maximum`, `value`, `allow_half`, `read_only`; `value`/`set_value`, `maximum`, `set_read_only`, `star_bounds`, `on_value_changed`; setas, Home/End, 0–9; UIA RangeValue |
| `Card` | painel desenhado (cantos arredondados) | builder `title`, `subtitle`, `clickable`, `padding`; `content_bounds` (onde pôr os filhos), `set_title`/`set_subtitle`, `on_click` |
| `Expander` | cabeçalho desenhado + `Panel` de conteúdo | builder `header`, `expanded`, `content_height`; `content()`, `is_expanded`/`set_expanded`/`toggle` (altura animada, refaz o layout do pai), `preferred_height`, `set_content_height`, `on_expanded_changed`; UIA ExpandCollapse |
| `BreadcrumbBar` | painel desenhado | builder `items`; `items`, `set_items`, `push`, `truncate`, `len`, `item_bounds`, `hidden_items`, `on_item_click(\|índice\| ..)`; sem espaço, os primeiros itens vão para "…" (menu) |
| `Chart` | painel desenhado (GDI+) | builder `kind(ChartKind::{Line, Area, Bar, StackedBar, Pie})`, `title`, `categories`, `series(ChartSeries::new(nome, &valores).with_color(..))`, `legend`, `animated`; `set_kind`, `set_series`, `add_series`, `set_values`, `set_categories`, `set_value_format`, `hit_test`, `item_center`, `summary` (descrição acessível), `on_point_click(\|série, índice\| ..)` |
| `TitleBar` | painel desenhado + área não cliente da janela | `TitleBar::attach(&janela)?` (tira a moldura do sistema); `height`/`set_height`, `add_control(&filho, largura)`, `press(CaptionButton)`, `button_bounds`; ponha-a no topo do layout com `LayoutParams::new().height(bar.height())` |
| `DockHost` / `DockPane` | painéis desenhados + `Panel`s de conteúdo | `DockHost::builder()...build()?`, `center()`, `add_pane(título, DockSide::{Left, Right, Bottom})` → `DockPane` (`content`, `float`, `dock`, `dock_to`, `toggle_floating`, `close`, `show`, `is_floating`/`is_docked`/`is_closed`, `floating_window`); `zone_size`/`set_zone_size`, `splitter_bounds`, `arrange` |
| `PrintPreview` / `PrintPreviewDialog` | painel desenhado / `Dialog` | `PrintPreview::builder().document(doc).zoom(PreviewZoom::FitPage)`; `set_document`, `refresh`, `page`/`set_page`, `next_page`/`previous_page`, `zoom`/`set_zoom`, `zoom_in`/`zoom_out`, `page_bounds`, `on_page_changed`; `PrintPreviewDialog::new(doc).show(&janela)?` → páginas impressas (`create` devolve `PrintPreviewWindow` para exibir sem bloquear) |

## Ícones (`arqrustui::icons`, recurso `icons`)

392 ícones embutidos em 22 categorias (de arquivos, pastas e 66 tipos de arquivo por extensão a redes sociais, emojis e gestos de mãos), cada um em `.ico` (16, 24, 32, 48, 64, 128 e 256 px) e `.png`; 79 também em `.svg`. Prancha com todos: `assets/icons/preview.png`.

| Item | Uso |
|---|---|
| Constantes | `icons::FOLDER`, `icons::SAVE`, `icons::DATABASE`… (nome canônico em maiúsculas) |
| Redes sociais | `icons::YOUTUBE`, `INSTAGRAM`, `FACEBOOK`, `WHATSAPP`, `X_TWITTER`, `YOUTUBE_SHORTS`…; 58 desenhos em `IconCategory::Social`. `GITHUB_SOCIAL`, `GITLAB_SOCIAL` e `THREADS_ALTERNATE` preservam variantes separadas dos logotipos anteriores. Origem e limitações visuais em `assets/icons/social/ORIGEM.md`. |
| Busca | `icons::find("pasta-aberta")` aceita nome, apelido e nomes de arquivo originais (`"05_banco_de_dados"`); `icons::all()`, `icons::in_category(IconCategory::Files)` |
| Tipo de arquivo | `icons::for_path("fotos/praia.JPG")` → `JPG_FILE`, `icons::for_extension(".pdf")` → `PDF_FILE` (sem diferenciar maiúsculas, com ou sem ponto; `htm`, `yml`, `tif`, `mjs`, `hpp`, `tgz`… usam o ícone da variante comum); `None` quando não há ícone — `.unwrap_or(&icons::FILE)` |
| `IconAsset` | `name`, `category`, `aliases`, `ico_bytes`/`png_bytes`/`svg_bytes`, `sizes`, `png_size`, `icon(px)` (cache por thread), `icon_for(&widget, lógico)`, `image()` → `PlusImage`, `save(caminho)` (formato pela extensão), `IconAsset::new(..)` para ícones próprios |
| `IconSource` | aceito por todos os métodos que exibem ícones; implementado por `Icon` e `IconAsset` |
| `Icon` | `from_ico_bytes(bytes, px)`, `disabled()`, `size()` além de `system`/`from_file` |
| `ImageList` | `add(&impl IconSource)`, `icon_size`, `len` |
| `IconPosition` | `Left`, `Right`, `Top`, `Bottom`, `Center` (botões e rótulos) |

## Layouts (`arqrustui::layout`)

| Tipo | Uso |
|---|---|
| `StackLayout::vertical()` / `::horizontal()` | pilha; `.spacing(n).padding(Thickness)`, `add`, `add_with(item, LayoutParams)`; valores recomendados: `layout::SPACING` (8) entre controles e `layout::MARGIN` (12) nas bordas |
| `GridLayout::new(&colunas, &linhas)` | grade com `Track::Fixed(n)`, `Track::Auto`, `Track::Star(peso)`; `.gaps(c, l)`, `add(item, linha, coluna)`, `add_span`, `add_row` |
| `FlowLayout::new()` | fluxo com quebra de linha; `.gaps(h, v)` |
| `DockLayout::new()` | encaixe: `add(item, Dock::Top/Bottom/Left/Right/Fill)` |
| `AnchorLayout::new(tamanho_de_projeto)` | âncoras estilo Windows Forms: `add(&widget, Anchors::BOTTOM_RIGHT)` |
| `AbsoluteLayout::new()` | retângulos fixos (escalados por DPI) |
| `SplitLayout::horizontal(a, b)` / `::vertical(a, b)` | divisão proporcional fixa: `.ratio(0.3).gap(6)` |
| `LayoutParams::new()` | `.width()`, `.height()`, `.size()`, `.min_size()`, `.max_size()`, `.margin()`, `.weight()`, `.align_h()`, `.align_v()` |
| `Spacer` | item vazio (com `weight` empurra os vizinhos) |

Layouts aninham: `externo.add_with(interno, LayoutParams::new().height(30))`. Controles ocultos não ocupam espaço. Associe com `window.set_layout(layout)`, `panel.set_layout(layout)` ou `group.set_layout(layout)`.

## Menus (`arqrustui::menus`)

`Menu::new()?` → `on_highlight_changed(\|item\| ..)` (item destacado pelo mouse/teclado; `None` ao sair), `add_item`, `add_item_with_shortcut`, `add_check_item`, `add_radio_item` (itens de opção consecutivos formam um grupo), `add_separator`, `add_submenu`, `add_submenu_with_icon`, `remove`, `clear`, `on_open(|menu| ..)` (chamado antes de exibir o menu; também antes de verificar um atalho na `MenuStrip`). `MenuItem` → `set_enabled`, `set_checked`, `set_text`, `text`, `set_icon`, `trigger`. `window.system_menu()?` devolve o menu de sistema para receber itens.

Os mesmos `Menu`s servem a duas barras:

| Barra | Criação | Aparência |
|---|---|---|
| `MenuBar` | `MenuBar::new()?` → `add_menu`; `window.set_menu(&bar)?` | barra do Windows (`HMENU`), cores do sistema |
| `MenuStrip` | `MenuStrip::new()?` → `add_menu`, `add_menu_with_icon`; `window.set_menu_strip(&strip)?` | desenhada pela biblioteca: tema claro/escuro/alto contraste, ícones em qualquer DPI |

`MenuStrip` → `set_menu_text`/`menu_text`, `set_menu_enabled`/`is_menu_enabled`, `set_menu_icon`/`clear_menu_icon`, `remove_menu`, `clear`, `len`, `set_style(Option<MenuStyle>)`/`style`, `height`, `open(índice)`, `close`, `is_open`, `overflow_count`. `window.remove_menu_strip()`, `window.menu_strip()`. A faixa fica na área não cliente (a área cliente e os layouts não mudam). Teclado: Alt ou F10 entram no modo de menu; Alt+letra abre um menu; setas, Home/End, Enter, Esc e letras sublinhadas navegam. Menus que não cabem vão para o botão `»`. `MenuStyle::{light, dark, high_contrast, system, from_theme(&tema), with_accent(cor)}`; sem estilo explícito, a barra segue o tema da janela.

`ContextMenu::new()?` (mesmos métodos de `Menu`) → `widget.set_context_menu(&menu)` ou `menu.show(&widget, ponto)`; `set_drawn(true)` usa o desenho da `MenuStrip` (não bloqueia; `close()` fecha).

## Diálogos (`arqrustui::dialogs`)

| Tipo | Uso | Retorno |
|---|---|---|
| `MessageBox` | `info`, `warning`, `error`, `confirm`, `show(&dono, título, texto, botões, ícone)` | `()`, `bool` ou `MessageResult` |
| `OpenFileDialog` | `.title().filter(nome, "*.ext").folder().file_name().show(&dono)?` / `.show_multiple` | `Option<PathBuf>` / `Vec<PathBuf>` |
| `SaveFileDialog` | idem + `.default_extension()` | `Option<PathBuf>` |
| `FolderPicker` | `.title().folder().show(&dono)?` | `Option<PathBuf>` |
| `ColorDialog` | `.initial(cor).show(&dono)?` | `Option<Color>` |
| `FontDialog` | `.initial(&fonte).color(cor).show(&dono)?` | `Option<FontChoice>` |
| `PrintDialog` | `.show(&dono)?` → `printer.print("doc", \|página, canvas\| …)`; `.show_with(&dono, &PrintOptions::new().page_count(n).allow_selection(true))?` → `printer.range()`, `printer.print_document(&PrintDocument)` | `Option<PrinterContext>` |
| `PageSetupDialog` | `.margins().show(&dono)?` | `Option<PageSetup>` |
| `FindDialog` | `FindDialog::find(&dono, texto, \|evento\| …)?` / `::replace` — **não modal** | `FindDialog` |
| `TaskDialog` | `.title().heading().content().icon().button(id, texto).command_links().verification().show(&dono)?` | `TaskDialogResult` |
| `Dialog` | `Dialog::new(&dono, título, l, a)?`; controles com `.parent(dialog.window())`; `close_with(resultado)`; `show_modal()?` ou `show_modeless(\|r\| …)` | `DialogResult` |
| `Modal` (estilo web: fundo escurecido + cartão) | `Modal::builder().title().message().content_height(n).width(l).button(texto, DialogResult::Cancel).primary_button(texto, DialogResult::Ok).close_on_escape().close_on_backdrop().close_button().dim(0.4).build(&dono)?`; controles em `modal.content()`; `show()?` (bloqueia) ou `show_then(\|r\| ..)?`; `close_with`, `on_close_request(\|r\| bool)` (validação), `on_closed`, `buttons`, `close_button`, `card`/`backdrop`, `card_bounds`, `shake`; atalhos `Modal::alert(&dono, título, texto)?`, `Modal::confirm(..)? -> bool` | `DialogResult` |

`Ok(None)` (ou vetor vazio) significa que o usuário cancelou.

## Gráficos (`arqrustui::graphics`)

- `Canvas` (GDI): `clear`, `line`, `rect`, `fill_rect`, `round_rect`, `ellipse`, `polygon`, `polyline`, `path`, `text`, `text_in_rect`, `text_centered`, `text_line` (uma linha com reticências), `measure_text`, `gradient_rect`, `draw_bitmap`, `draw_bitmap_alpha`, `draw_icon`, `focus_rect`, `clip_rect`, `clip_path`, `reset_clip`, `save`/`restore`, `translate`/`rotate`/`scale`/`reset_transform`, `pixel`, `with_gdiplus`.
- `PlusGraphics` (GDI+): `clear`, `line`, `draw_rect`, `fill_rect`, `draw_ellipse`, `fill_ellipse`, `draw_polygon`, `fill_polygon`, `draw_path`, `fill_path`, `gradient_rect`, `draw_image`, `text`, `translate`/`rotate`/`scale`, `clip_rect`.
- `Direct2DSurface::attach(&painel, |d2d| …)?`; `D2DContext`: `clear`, `line`, `fill_rect`, `draw_rect`, `rounded_rect`, `ellipse`, `path`, `gradient_rect`, `text`, `push_transform`/`pop_transform`, `translate`/`rotate`/`scale`, `push_clip`/`pop_clip`, `set_antialias`.
- Vetores (`graphics::vector`): `Figure::{new, rect, ellipse, pie}` + `Segment::{Line, Cubic}`, `Paint::{Solid, Linear, Radial}`, `Stroke { width, join, cap, miter_limit, dashes }`, `Matrix::{translate, scale, rotate, multiply, inverse}`; `PlusGraphics::{fill_figures, stroke_figures, with_matrix}`.
- SVG: `SvgImage::{parse, from_bytes, from_file}` → `size`, `view_box`, `with_current_color`, `render(l, a)` → `PlusImage`, `draw(&PlusGraphics, x, y, l, a)`; `canvas.draw_svg(&svg, Rect)`; `icons::X.svg_image()`.
- Animações (`motion`, no prelúdio): `Easing::{Linear, EaseIn, EaseOut, EaseInOut, Overshoot, CubicBezier(..)}`, `motion::animate(duração, easing, \|t\| ..)` / `animate_value(de, até, ..)` → `Transition` (`on_finished`, `cancel`, `finish`, `is_running`), `animate_bounds(&w, Rect, ..)`, `fade_window`, `show_animated`/`hide_animated(&w, ShowEffect::{Fade, SlideDown, ..}, duração)`, `lerp`/`lerp_rect`/`lerp_color`, `animations_enabled`/`set_animations_enabled`.
- `Font::new(família, pontos).bold().italic()`, `Font::system()`, `Font::load_file(caminho)`; `Pen::new(cor, largura)`; `Path::new().move_to().line_to().bezier_to().close()`; `Bitmap::from_file`, `Bitmap::render`, `Bitmap::from_icon`; `PlusImage::from_file`, `PlusImage::from_bytes`, `PlusImage::to_icon`; `canvas.draw_icon(&impl IconSource, origem, tamanho)`.

## Temas (`arqrustui::themes`)

`Theme::light()`, `Theme::dark()`, `Theme::system()`, `.with_font()`, `.with_accent()`, `theme.apply(&window)`; `SystemTheme::current()`; `Style::new().font().foreground().background().apply_to(&widget)`; `ButtonStyle::from_accent(cor, texto)`.

## Sistema (`arqrustui::system`)

`Clipboard::{set, set_text, text, has_text, set_html, html, has_html, set_image, image, has_image, png, metafile, has_metafile, set_metafile, set_files, files, set_data, data, has_format, format_names, sequence_number, clear}` com `ClipboardContent::new().text().html().image()?.image_pixels()?.metafile().files().custom()` (vários formatos de uma vez; imagens também em `CF_DIBV5` e `PNG`) · `Metafile::{record(l, a, \|canvas\| ..), from_bytes, bytes, size, render}` · `ClipboardWatcher::new(\|\| ..)?` · `SystemNotification::new(título).body().attribution().image().hero_image().button(rótulo, argumento).text_input(id, dica).selection_input(id, opções).button_for_input(..).progress(..).tag().group().header().scenario(NotificationScenario::..).suppress_popup().launch_argument().silent().long_duration().on_activated_with(\|NotificationActivation\| ..).on_dismissed().show()?` → `SystemNotificationHandle::{hide, update_progress}`; `.show_at(quando)?` → `ScheduledNotification::cancel`; `SystemNotification::{set_app_id, app_id, is_supported, remove, remove_group, clear_all, launched_by_notification}`; `NotificationActivator::register(clsid, \|ativação\| ..)` (clique com o programa fechado) · `TrayIcon::new(&ícone, dica)?` (`on_click`, `set_menu`, `show_notification`) · `shell::{open, open_with_verb, show_in_explorer, known_folder, associated_program}` · `Monitor::{all, primary, from_point}` · `Cursor::{system, from_file, position}` · `WindowsVersion::current()` · `accessibility::{high_contrast, screen_reader_active, text_scale_factor, animations_enabled}` · `version::{touch_available, max_touch_points}`.

## Acessibilidade, entrada e i18n

- Controles desenhados acessíveis sem configuração: `MenuStrip` (barra de menus → itens com ExpandCollapse; listas → itens com Invoke/Toggle/ExpandCollapse), `CommandBar` (botões com Invoke, Toggle ou ExpandCollapse; botão dividido como SplitButton), `PropertyGrid` (categorias como grupos; propriedades com Value, booleanos com Toggle). `set_accessible_name`/`set_accessible_description` trocam o nome/descrição da raiz.

- `accessibility::{set_custom_role, announce, announce_with, set_tab_order, show_focus_indicators}`; `widget.set_accessible_name(..)`.
- Modelo acessível de controles desenhados pela aplicação: `set_accessible_model(&w, impl AccessibleModel)` com `root()`, `children()` (`AccessibleItem::new(id, papel, nome).parent().bounds().invokable().toggle().expanded().value().range(AccessibleRange).selection().selected().grid().cell().scroll(AccessibleScroll).text().live()`) e `perform(id, AccessibleAction)`; `raise_accessible_event(&w, id, AccessibleEvent::..)`.
- `Key`, `Modifiers`, `Shortcut::{key, ctrl, ctrl_shift, alt, ctrl_char}`, `MouseEvent`, `PointerEvent`, `Ime::{set_enabled, is_open, set_composition_position, on_composition_changed, use_candidate_window, on_candidates_changed, candidates, select_candidate, set_candidate_page, complete, cancel, set_custom_candidates, set_custom_composition, simulate_candidates}`, `ImeCandidateWindow`, `ImeCandidates`.
- Gestos (`gestures`, no prelúdio): `gestures::on_gesture(&w, \|g: &GestureEvent\| ..)?` / `on_gesture_with(&w, GestureOptions { inertia, rotation, scaling, mouse_pan, ctrl_wheel_zoom }, ..)` → `GestureSubscription::stop`; `GestureKind::{Tap, DoubleTap, Hold, Manipulation}`, `GesturePhase::{Started, Changed, Inertia, Completed}`; `GestureRecognizer` (lógica pura, para testes).
- Arrastar e soltar (`input::drag`, OLE): `start_drag(&w, &ClipboardContent, &[DropEffect])`, `start_drag_with_image(.., &PlusImage, hotspot)` → `DropEffect`; `accept_drops(&w, DropHandler::new().on_over().on_drop().on_leave())`, `stop_accepting_drops`; `DropData::{position, modifiers, allows, suggested_effect, has_text/text, has_html/html, has_image/image, has_files/files, has_format/data, format_names, from_content}`.
- Reordenação por arraste: `ListBox`/`ListView`/`DataGrid` — `set_reorderable`, `on_item_reorder_request(de, para) -> bool` (`ListBox`) / `on_row_reorder_request(de, para) -> bool` (`ListView`, `DataGrid`), `on_items_reorder_request` / `on_rows_reorder_request` para blocos e `move_rows` (`DataGrid`) para vários selecionados em bloco, `set_drag_out_effects(&[DropEffect::Copy, DropEffect::Move])` + `on_drag_out_finished`, `on_drag_out(índice) -> Option<ClipboardContent>`, `move_item`/`move_row`, `index_at`/`row_at`; `TreeView` — `set_drag_enabled`, `on_item_drop(nó, TreeDropPosition) -> bool`, `on_item_moved`, `on_drag_out`, `move_item(nó, TreeDropPosition::{Inside, Before, After})`, `item_at`, `TreeItem::handle`. O item segue o cursor, uma linha de inserção na cor de destaque mostra o destino (na árvore, soltar no meio do nó o realça e coloca dentro), há rolagem automática nas bordas, nós fechados se abrem após ~0,7 s e `Esc` cancela.
- Textos internos da biblioteca: `i18n::{LibraryText, library_text, library_format, set_library_text, set_library_language("en"), use_system_library_language(), system_library_language(), apply_library_translations(&Translations)}`; nomes das categorias de ícones por `IconCategory::label()`.
- `Locale::{user_default, named}` → `format_date`, `format_long_date`, `format_time`, `format_number`, `format_currency`, `is_right_to_left`; `DateTime`; `Translations` (catálogo `chave = valor` por idioma).

### Texto por trechos
StyledText::new(fonte, pontos), push(texto, TextFormat) e to_rtf() constroem documentos Unicode sem RTF manual. Estilos são isolados por trecho; pode ser usado em segundo plano. Exemplo: examples/styled_text.rs, incluindo OpenFileDialog e SaveFileDialog nativos.

Widget::invalidate() também solicita pintura dos descendentes de um contêiner, sem execução síncrona de callbacks.

### Composição, rodapé e rolagem
- StatusBarLayout: conteúdo separado da barra real, altura preferida, limites e fit_height. O ajuste de altura preserva a geometria de janelas maximizadas, minimizadas e em tela cheia; redimensiona apenas no estado normal.
- ScrollViewport: viewport/content, barra vertical automática, layout_content, scroll_to, scroll_wheel e ensure_visible.
- StyledText::push_mapped: estilos por caractere Unicode com agrupamento adjacente.
Detalhes em [COMPOSITION.md](COMPOSITION.md); exemplo composition.

### Estilos visuais por intervalo
`RichTextBox::apply_highlights(&base, &[(range, format)]) -> GuiResult<()>` aplica `TextFormat` a intervalos UTF-16 do RichEdit (uma quebra CRLF ocupa uma posição), sem substituir o texto, sem criar etapas de desfazer e sem emitir eventos de edição. Preserva seleção, posição de rolagem e indicador de modificação. Valida todos os intervalos antes de alterar o documento. A aplicação fornece os intervalos e estilos; nenhum parser ou regra de linguagem é embutido no controle. Requer ITextDocument do RichEdit e retorna erro explícito quando indisponível. Exemplo: `examples/rich_highlight.rs`.

### Alinhamento do conteúdo de botões
`Button::builder().content_alignment(TextAlign::Left | Center | Right)` alinha horizontalmente o conjunto ícone + texto, preservando a margem interna de 8 unidades lógicas e a posição relativa definida por `icon_position`. O padrão continua centralizado. `button.set_content_alignment(..)` altera o alinhamento em execução; `button.content_alignment()` consulta o valor. Textos longos cedem espaço ao ícone e são truncados com reticências. Exemplo independente: `examples/button_alignment.rs`.
