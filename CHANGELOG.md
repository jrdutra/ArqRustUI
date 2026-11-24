# Registro de mudanças

*English: [CHANGELOG.en.md](CHANGELOG.en.md).*

Todas as mudanças relevantes da ArqRustUI ficam neste arquivo. O formato segue o
[Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o projeto usa
[versionamento semântico](https://semver.org/lang/pt-BR/): enquanto a versão for
0.x, uma versão "menor" (0.**2**.0) pode trazer mudanças incompatíveis, sempre
listadas em **Mudado** ou **Removido** com o que fazer para atualizar.

As dez crates do repositório (`arqrustui`, a fachada `arqrustui-icons` e as oito
`arqrustui-icons-dataN`) têm sempre a mesma versão e são publicadas juntas (veja
[`docs/RELEASING.md`](docs/RELEASING.md)).

Seções de cada versão: **Adicionado**, **Mudado**, **Obsoleto**, **Removido**,
**Corrigido**, **Segurança**.

## [Não publicado]

### Adicionado

- Pacote de redes sociais fornecido em 16/11/2026: 40 desenhos disponíveis na
  categoria `Social`, com 39 novos assets e reutilização do Threads idêntico.
  O catálogo passa a ter 374 ícones. `GITHUB_SOCIAL`, `GITLAB_SOCIAL` e
  `THREADS_ALTERNATE` são variantes separadas; os logotipos anteriores continuam
  disponíveis. Os ICOs recebidos foram preservados, inclusive cortes e fragmentos
  documentados em `assets/icons/social/ORIGEM.md`.
- Oitava crate própria de dados de ícones, mantendo as partes abaixo de 9 MB.
- `i18n::system_library_language` e `i18n::use_system_library_language`: escolhem a
  tabela dos textos internos pelo idioma da interface do Windows (`"pt"` no Windows
  em português, `"en"` nos demais). O padrão continua sendo português.
- 22 chaves `LibraryText::Category*` (`category_actions`…) com os nomes das categorias
  de ícones em português e inglês, e `IconCategory::label_key`.
- Versões em inglês dos guias (`*.en.md`, ao lado de cada original): `BIBLIOTECA`,
  `AGENTS`, `CHANGELOG` e, em `docs/`, `ARCHITECTURE`, `COMPATIBILITY`, `COMPONENTS`,
  `CUSTOM_CONTROLS`, `EVENTS`, `FEATURE_MATRIX`, `MEMORY_AND_SAFETY` e `RELEASING`.

### Mudado

- Preparação para GitHub: README principal em inglês com galeria local das
  capturas fornecidas, guia das 32 demos, uso, limites e instruções de publicação.
  O campo `readme` do pacote principal passa a usar `README.md`. Regras de
  exclusão ampliadas para caches e temporários; pacotes de trabalho preservados
  fora do repositório. Código, recursos, testes e histórico Git mantidos.
- Documentação Rust (rustdoc) em inglês, incluindo módulos, APIs, contratos de
  segurança, macros de ícones, crates de dados e exemplos publicados. Guias
  continuam com versões em português e inglês; textos localizados permanecem
  nas tabelas de tradução.
- Demo `modern_controls` usa layouts responsivos e tamanho mínimo de 780×520.
  O campo numérico mantém a largura de 140 unidades lógicas.
- Corrigida a escala duplicada ao alternar de GDI para GDI+ em `Canvas`, que
  cortava o `ToggleSwitch` e as estrelas do `RatingControl` a 150%.
  Teste de pixels cobre 96, 144 e 192 DPI e a restauração do desenho GDI.
- Removido `map_or` redundante na imagem de arraste: a verificação do Clippy
  com `-D warnings` deixa de falhar com `map_or_identity`.
- Mensagens para o desenvolvedor passam a ser em inglês: `Display` de `GuiError`,
  os motivos de `InvalidArgument`/`Unsupported`, os contextos de erros Win32, os
  `expect` internos, os erros do leitor de SVG e as notas dos aliases
  `#[deprecated]`. Textos exibidos ao usuário final continuam na tabela de
  `LibraryText`. Quem comparava o texto de um erro deve comparar a variante de
  `GuiError`.
- **Incompatível:** `IconCategory::label` devolve `String` (antes `&'static str` fixo
  em português) e segue o idioma de `i18n::set_library_language`; pode ser
  substituído com `i18n::set_library_text(LibraryText::CategoryActions, ..)`.
  Para atualizar, troque `.unwrap_or("…")` por `.unwrap_or_else(|| "…".to_string())`
  onde o resultado era combinado com um `&str`.
- Descrições dos dez `Cargo.toml` (exibidas no crates.io) em inglês.
- Textos e comentários dos 32 exemplos (`examples/`) em inglês. Os que mostram
  textos da própria biblioteca chamam `i18n::set_library_language("en")`. No
  `validated_form`, o campo de CEP passou a ser um ZIP code de 5 dígitos.

- Comando Novo nas demos `demo` e `menus` usa o ícone de documento `icons::FILE`,
  na resolução pedida pelo controle, em vez do ícone genérico de aplicação.
  As duas demos continuam compilando sem o catálogo opcional de ícones.
- Redes sociais atualizadas pelo pacote `arqrustui-social-icons-bevel-3d.zip`:
  22 desenhos substituídos e 18 novos, mantendo constantes anteriores e as
  plataformas sem substituto no pacote. Agora são 392 ícones, 58 sociais e
  79 fontes SVG. PNGs novos com 512 px; `facebook_messenger` encontra `MESSENGER`.
  Arte, origem, créditos Font Awesome/CC BY 4.0 e limitação de texto em três SVGs
  documentados e incluídos nos subpacotes de dados afetados.
- Janelas sem ícone explícito agora usam o logo da biblioteca, disponível em
  `icons::DEFAULT_WINDOW_ICON` mesmo sem o recurso opcional `icons`. Ícones
  escolhidos no builder ou por `set_icon` continuam prevalecendo, inclusive nas
  atualizações de DPI. O ICO original acompanha o pacote da biblioteca.
- Os apelidos `21_github`, `22_gitlab` e `31_threads_alternate` encontram os
  desenhos correspondentes do pacote social; os nomes canônicos anteriores
  continuam encontrando seus desenhos originais.
- Nome da biblioteca e diretório atualizados para ArqRustUI; crate/import para
  `arqrustui`. Os oito subpacotes de ícones usam o prefixo `arqrustui-icons`.
  Consumidores devem atualizar as dependências por caminho e os imports.

## [0.1.0] — ainda não publicada

Primeira versão. Validada até aqui sob Wine 9 (`x86_64-pc-windows-gnu` e `i686-pc-windows-gnu`); os itens
que ainda precisam ser vistos num Windows de verdade estão marcados com ⚠ em
[`docs/FEATURE_MATRIX.md`](docs/FEATURE_MATRIX.md) e descritos em
[`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md).

### Adicionado

- Biblioteca de interface gráfica nativa para Windows 10 e 11 (x64) em Rust puro:
  só `std` e FFI escrita à mão para Win32, Common Controls v6, GDI/GDI+,
  Direct2D/DirectWrite, COM, DWM e UI Automation; nenhum crate de terceiros.
- Núcleo: `Application`, janelas com builders, eventos por closures
  (`on_<evento>`), `GuiResult`, RAII para os recursos do Windows, laço de mensagens
  com `UiDispatcher` para outras threads, temporizadores, DPI por monitor,
  estilos visuais e DPI ativados em tempo de execução (sem manifesto).
- Controles nativos (botões, campos, listas, árvores, `ListView`, `DataGrid`
  virtual, abas, `RichTextBox` com realces, seletores de data e hora, barras de
  progresso, de rolagem e de status, entre outros) e controles desenhados pela
  biblioteca: interruptor, caixa numérica com expressões, avaliação, cartões,
  expansor, trilha de navegação, gráficos, pré-visualização de impressão,
  painéis acopláveis, abas destacáveis e janela com barra de título própria.
- Menus: `MenuBar` do Windows e `MenuStrip` desenhada com os mesmos `Menu`s,
  menus de contexto, `CommandBar` com botões, alternância, menus, busca e
  personalização.
- Diálogos: os comuns do sistema (abrir, salvar, pasta, cor, fonte, impressão,
  configurar página, localizar), `MessageBox`, `TaskDialog`, `Dialog` próprio e
  `Modal` no estilo da web (fundo escurecido, cartão centralizado, animação,
  validação e foco preso).
- Layouts (pilha, grade, fluxo, encaixe, âncoras, divisão, absoluto), temas
  claro, escuro e alto contraste, animações (`motion`), SVG, gestos de toque, IME
  com janela de candidatos, arrastar e soltar (arquivos, texto e reordenação),
  área de transferência, notificações do Windows, bandeja do sistema,
  acessibilidade por UI Automation e textos da própria biblioteca traduzíveis
  (português e inglês).
- Pacote de 335 ícones em 22 categorias (recurso `icons`, ligado por padrão):
  `.ico` de 16 a 256 px, `.png` e 39 em `.svg`, com constantes
  (`icons::FOLDER`), busca por nome e apelido (`icons::find`), categorias e ícone
  pelo tipo de arquivo (`icons::for_extension("pdf")`, `icons::for_path(..)`,
  66 tipos com desenho próprio, 92 extensões reconhecidas).
- Distribuição pronta para o crates.io: os bytes dos ícones (cerca de 61 MB) ficam
  em sete crates de dados de até 9 MB (`arqrustui-icons-data1` a `-data7`),
  reunidas pela fachada `arqrustui-icons`, que é a dependência opcional do
  recurso `icons`; o pacote da `arqrustui` tem menos de 1 MB. A API
  `arqrustui::icons::*` é a mesma de antes da divisão.
- Metadados do docs.rs (documentação gerada com o alvo
  `x86_64-pc-windows-msvc`, já que o crate é `#![cfg(windows)]`).
- Integração contínua no GitHub Actions: formatação, clippy e documentação sem
  avisos, testes em Windows real (MSVC e GNU x64, MSVC x86), versão mínima do Rust
  (1.80), documentação como no docs.rs, tamanho de cada pacote abaixo do limite do
  crates.io e compilação experimental para ARM64.
- Suporte a Windows de 32 bits (x86): a suíte inteira passa no Wine de 32 bits.
- Guia completo em inglês (`README.en.md`).
- `ImeCandidates::new`, `with_selection` e `with_page`, para montar uma lista de
  candidatos fora da biblioteca.
- Testes do que vem do Windows e antes não era simulado (`tests/system_events.rs`):
  clique e Enter num link, troca de tema do sistema (`WM_SETTINGCHANGE`), arquivos
  soltos na janela (`WM_DROPFILES`), exibição do tooltip ao pousar o ponteiro, menu de
  contexto do Windows e desenhado (`WM_CONTEXTMENU`) e, na mudança de DPI, o ícone da
  janela e o retângulo sugerido.
- Testes automatizados dos diálogos do sistema (abrir, salvar, pasta, cor, fonte,
  impressão, configurar página, localizar, `MessageBox` e `TaskDialog`): um
  temporizador responde ao diálogo modal como o usuário faria (`tests/dialogs.rs`).

### Mudado

- Os maiores arquivos foram divididos em submódulos por tema, sem mudar a API nem o
  comportamento: `DataGrid` (desenho, edição, exportação, filtro e agrupamento,
  reordenação, estilos), `CommandBar` (teclado e acessibilidade, layout, desenho,
  painel do `»`), o despacho de mensagens do `WidgetCore` (`core/widget/dispatch.rs`),
  os menus desenhados (máquina de estados, desenho, acessibilidade), a `MenuStrip`
  (barra e mensagens da janela), a árvore de UI Automation (métodos COM do provedor
  e chamadas para os testes) e as notificações (infraestrutura WinRT/COM). Nenhum
  arquivo passa mais de 1.200 linhas.
- Nomes de eventos unificados numa convenção (verbo no presente para ação do
  usuário, particípio para aviso depois do fato, `_request` para pedido que pode ser
  recusado; veja "Nomes dos eventos" em `docs/EVENTS.md`). Novos nomes:
  `Window::on_close_request`, `InfoBar::on_closed`, `Toast::on_dismissed`,
  `CommandButton::on_toggled`, `Hyperlink::on_link_click`,
  `ListView::on_row_activated`, `ListView`/`DataGrid::on_row_reorder_request` e
  `on_rows_reorder_request`, `ListBox::on_item_reorder_request` e
  `on_items_reorder_request`, `TreeView::on_item_drop_request` e
  `Transition::on_finished`, `Menu::on_highlight_changed`,
  `Ime::on_candidates_changed` e `Ime::on_composition_changed`.
- As 13 structs públicas que a biblioteca produz e quem usa só lê passaram a ser
  `#[non_exhaustive]`: `KeyEvent`, `MouseEvent`, `PointerEvent`, `GestureEvent`,
  `FindEvent`, `NotificationActivation`, `FontChoice`, `TaskDialogResult`,
  `DetachedTab`, `ImeCandidates`, `RawMessage`, `Monitor` e `WindowsVersion`.
  **Ao atualizar:** os campos continuam públicos para leitura, mas uma dessas
  structs não pode mais ser montada com literal (`Tipo { .. }`) fora da biblioteca;
  para `ImeCandidates`, que também é entrada da API, use `ImeCandidates::new(itens)`
  com `with_selection`/`with_page`. Tipos de valor (`Color`, `Rect`, `Thickness`…)
  e estilos continuam montáveis.
- 59 dos 63 enums públicos passaram a ser `#[non_exhaustive]` (entre eles
  `GuiError`, `Event`/`EventKind`, `IconCategory` e `DialogResult`), para que
  variantes novas não quebrem quem usa a biblioteca. **Ao atualizar:** um `match`
  sobre esses enums fora da biblioteca precisa de um braço `_ => ..`. Ficam
  exaustivos só os conjuntos fechados `Orientation`, `CheckState` e
  `TreeDropPosition` (e `LibraryText`, que já era `#[non_exhaustive]`).
- Os PNGs dos ícones que eram cópia exata do quadro de 256 px do `.ico` (153 dos
  335) passaram a ser lidos de dentro do `.ico`: `IconAsset::png_bytes` devolve
  os mesmos bytes de antes, sem guardá-los duas vezes.

### Corrigido

- 32 bits: a biblioteca não linkava, porque a user32 de 32 bits não exporta
  `GetWindowLongPtrW`/`SetWindowLongPtrW`; agora elas apontam para
  `GetWindowLongW`/`SetWindowLongW`, como no `windows.h`.
- 32 bits: valores `double` em `VARIANT` (UI Automation: `RangeValue.Value` e outras
  propriedades numéricas) perdiam a metade alta e chegavam como 0 aos leitores de
  tela; `VARIANT::from_f64`/`as_f64` gravam e leem os 8 bytes em qualquer alvo.
- Os testes da área de transferência (um recurso do sistema inteiro) passaram a ser
  serializados; antes, rodando em paralelo, um deles falhava de vez em quando.

### Segurança

- Nenhum pânico atravessa mais a fronteira com o Windows. Antes, só os
  procedimentos de janela e alguns callbacks tinham `catch_unwind`; os métodos COM
  de UI Automation, `IDataObject`, `IDropSource`/`IDropTarget` e do ativador de
  notificações, e os callbacks de ordenação da `ListView`, fluxo do RichEdit,
  enumeração de monitores e a subclasse da reordenação (123 funções) não tinham,
  e um pânico neles era comportamento indefinido no Rust 1.80 (e abortaria o
  processo a partir do 1.81). Agora todas rodam em `core::ffi_guard`: o Windows
  recebe `E_UNEXPECTED` (COM) ou um valor neutro, e o pânico é relançado pelo laço
  de mensagens. Um teste varre o código e falha se uma entrada nova ficar sem
  proteção. O MSRV continua 1.80.

### Obsoleto

- Os nomes antigos dos eventos renomeados continuam funcionando como aliases
  `#[deprecated]` (o compilador avisa e indica o nome novo): `Window::on_close`,
  `InfoBar::on_close`, `Toast::on_dismiss`, `CommandButton::on_toggle`,
  `Hyperlink::on_link_clicked`, `ListView::on_item_activated`,
  `ListView`/`DataGrid::on_reorder` e `on_rows_reordered`, `ListBox::on_reorder` e
  `on_items_reordered`, `TreeView::on_item_drop`, `Transition::on_done`,
  `Menu::on_highlight`, `Ime::on_candidates` e `Ime::on_composition`. Serão
  removidos antes da versão 1.0.

### Problemas conhecidos

- Quase nada foi verificado num Windows de verdade: só dois itens de composição
  foram registrados como testados lá, por um trabalho anterior. O CI é o primeiro
  lugar onde a suíte inteira roda fora do Wine.

[Não publicado]: #não-publicado
[0.1.0]: #010--ainda-não-publicada
