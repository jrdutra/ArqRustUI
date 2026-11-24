# Compatibilidade, validação e limitações conhecidas

*English: [COMPATIBILITY.en.md](COMPATIBILITY.en.md).*

## Verificação de maximização em Windows — 14/11/2026

Com MSVC e escala de 144 DPI, foi reproduzido um encolhimento de 1.550 para 92 pixels de altura ao chamar `StatusBarLayout::fit_height` depois de ocultar conteúdo numa janela maximizada. O Windows mantinha o estado maximizado, mas o ajuste de conteúdo substituía seu retângulo. O método agora só redimensiona a janela no estado normal.

Os quatro testes de `tests/window_maximize.rs` passaram: ciclos de maximizar/restaurar com atualização de conteúdo, preservação da área útil do monitor e do rodapé, minimização, tela cheia e reaplicação de DPI. A mensagem de DPI é simulada com a escala atual; troca física entre monitores com escalas distintas continua pendente. Os três testes de composição passaram. Os exemplos `demo` e `composition` foram abertos; a maximização e o rodapé foram conferidos visualmente, inclusive ao mostrar/ocultar conteúdo em `composition`.

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo build --release` passaram. A suíte completa (`cargo test --no-fail-fast`) teve 223 testes aprovados (incluindo 35 doctests) e 12 falhas em caminhos que não usam o método alterado: associação de arquivos, cliente UIA, pintura de tema/ícones, estado de árvore após arraste, InfoBar, notificações e clipboard. Notificações e clipboard retornaram acesso negado nesta sessão; as outras divergências precisam de investigação própria. A falha de associação de arquivos também ocorreu antes da correção: o Windows devolve `OpenWith.exe` para extensão desconhecida. Esses resultados complementam, sem substituir, a validação histórica sob Wine descrita abaixo.

## Plataformas

| Item | Situação |
|---|---|
| Windows 10 e 11, x86_64 | alvo principal |
| Toolchains | `x86_64-pc-windows-msvc` e `x86_64-pc-windows-gnu` (as DLLs são ligadas por `#[link(name = "…")]`, disponível nos dois) |
| x86 (32 bits) | suportado: a suíte inteira passa no Wine de 32 bits (`i686-pc-windows-gnu`; o CI roda `i686-pc-windows-msvc`). Estruturas dependentes de ponteiro têm `cfg` onde o layout difere (`TBBUTTON`, `PRINTDLGW`); `GetWindowLongPtrW`/`SetWindowLongPtrW` apontam para `GetWindowLongW`/`SetWindowLongW`, como no `windows.h`. A conferência de layout de `tests/ffi_layout.rs` só roda em 64 bits |
| ARM64 | a camada FFI não usa nada específico de x86_64; compila (`cargo check`, e no CI como alvo experimental), **nunca executado** |
| Outros sistemas | o crate compila vazio (`#![cfg(windows)]`) |

## Recursos que dependem da versão do Windows

Nenhuma API recente é importada estaticamente: tudo abaixo é resolvido em tempo de execução, e a ausência vira `GuiError::Unsupported` (ou um *fallback*), nunca falha de carregamento.

| Recurso | Requisito | Sem o requisito |
|---|---|---|
| DPI por monitor v2 | Windows 10 1703 | DPI de sistema (`SetProcessDPIAware`) |
| `GetDpiForWindow`, `AdjustWindowRectExForDpi` | Windows 10 1607 | DPI do sistema / `AdjustWindowRectEx` |
| Estilos visuais (Common Controls v6) | sempre presente no Windows 10/11 | controles com aparência clássica; `TaskDialog` devolve `Unsupported` |
| Barra de título escura | Windows 10 build 19041 | `Unsupported` |
| Cantos, cor de borda/título | Windows 11 | `Unsupported` |
| Mica/Acrílico | Windows 11 22H2 (build 22621) | `Unsupported` |
| `accessibility::announce` | Windows 10 1709 | `Unsupported` |
| Eventos de ponteiro (toque/caneta) | Windows 8 | `PointerKind::Other` / eventos de mouse |
| Direct2D/DirectWrite | Windows 7 SP1+ | `Direct2DSurface::attach` devolve erro |
| Ícone em botão nativo (`NM_CUSTOMDRAW` do botão) | Common Controls v6 | o botão passa a ser desenhado pela biblioteca, com as cores do sistema |

Use `WindowsVersion::current()` e seus métodos `supports_*` para decidir antes de chamar.

## Como esta versão foi validada

O ambiente em que a biblioteca foi escrita não executava Windows. A validação foi:

| Verificação | Resultado |
|---|---|
| Compilação para `x86_64-pc-windows-gnu` (biblioteca, 20 exemplos, testes) | sem erros nem avisos |
| `cargo fmt --check` | limpo |
| `cargo clippy --all-targets` | sem avisos |
| `cargo test` sob **Wine 9.0**, com os Common Controls v6 ativos | 129 testes (64 unitários, 59 de integração com janelas reais, 6 de layout FFI) + 28 doctests — todos passando |
| Mesma suíte com o sistema em **144 DPI** (150 %) | todos passando |
| `cargo build --release` | ok (LTO só pôde ser exercitado desligado no ambiente de validação) |
| Execução visual dos exemplos sob Wine, com cliques e digitação simulados | hello_world, demo, dark_theme, direct2d, responsive_layout, custom_drawing, calculator, text_form, validated_form, tabs, listview_columns, treeview, file_dialogs, icons (96 e 144 DPI) |

### O que **ainda precisa** de validação em Windows 10/11 reais

Wine é uma reimplementação: comporta-se como o Windows na grande maioria das APIs usadas, mas não é o Windows. Antes de considerar a versão estável, rode no Windows:

```powershell
cargo test
cargo run --example demo
```

e confira especificamente:

1. **Aparência com estilos visuais** (Common Controls v6): no ambiente de validação não havia tema instalado, então os controles apareceram no estilo clássico. O carregamento da v6 foi confirmado indiretamente (o `TaskDialogIndirect`, exclusivo da v6, foi resolvido e exibido).
2. **Tema escuro em controles temáticos**: cores via `WM_CTLCOLOR*`, ListView/TreeView/RichEdit, botões, abas, barra de status, cabeçalhos, grupo, bordas 3D, barra de ferramentas e calendário foram verificados no Wine (os quatro primeiros pintados pela biblioteca também por pixels nos testes); a combinação com os estilos visuais reais do Windows (caixas de seleção, barras de rolagem `DarkMode_Explorer`, `DarkMode_ItemsView` no cabeçalho) não.
3. **Efeitos do DWM**: `set_dark_title_bar` devolveu sucesso; cantos/bordas/Mica devolveram `Unsupported` (a versão simulada era Windows 10). O efeito visual não pôde ser observado.
4. **Texto DirectWrite**: as chamadas `CreateTextFormat`/`DrawText` foram confirmadas no rastreamento do Wine com os argumentos corretos, mas o ambiente não tinha fontes instaladas para o DirectWrite renderizar. Formas, gradientes, caminhos e transformações do Direct2D renderizaram corretamente.
5. **Nome acessível em controles nativos** (`set_accessible_name` via `IAccPropServices`): o Wine devolve `E_NOTIMPL`. O provedor UIA de controles personalizados e `announce` devolveram sucesso, mas não foram testados com um leitor de tela (Narrador/NVDA). Os elementos virtuais de `MenuStrip`, `CommandBar` e `PropertyGrid` foram lidos pelo cliente `IUIAutomation` do Wine (tipo e nomes); confira no Windows com o Narrador/Accessibility Insights que os controles nativos da janela continuam listados ao lado da barra de menus desenhada (o cliente do Wine não lista janelas filhas) e que os anúncios de foco acompanham as setas.
6. **Mudança de DPI em tempo de execução** (`WM_DPICHANGED` ao arrastar entre monitores): implementada; só o DPI fixo (96 e 144) foi exercitado.
7. **Bandeja e notificações**: ícone adicionado e removido com sucesso; a exibição da notificação e o menu no clique direito não foram observados. `SystemNotification` não pôde ser exibida (o Wine não tem `Windows.UI.Notifications`): confira no Windows 10/11 a notificação com botões, o nome/ícone registrados no `AppUserModelID`, `on_activated` com o argumento do botão e `on_dismissed` ao fechar/expirar.
8. **Diálogos**: abrir arquivo e TaskDialog foram exibidos; cor, fonte, impressão, configuração de página e localizar/substituir compilaram e seguem os layouts verificados em `tests/ffi_layout.rs`, mas não foram operados interativamente. No Wine o TaskDialog não mostrou botões personalizados/links de comando (limitação do Wine).
9. **Arrastar e soltar entre programas**: a reordenação, o arrastar para fora e o destino OLE foram exercitados no Wine com o mouse (inclusive um arraste completo de uma lista para outro controle). Confira no Windows a imagem de arraste do shell (`IDragSourceHelper`/`IDropTargetHelper`), o fantasma translúcido sob o cursor e soltar no Explorador, Word e Bloco de Notas.
10. **Impressão em papel**, **toque/caneta reais**, **IME real**, **arrastar arquivos do Explorador**, **`Animation` com um AVI real**, **vários monitores**: sem dispositivo ou sem meio de simular.
11. **Toolchain MSVC**: só a variante GNU foi compilada.
12. **Ícones em botões nativos e texto centralizado**: os estilos visuais são ativados pelo manifesto embutido no `shell32.dll`; quando ele não traz a v6 (caso do Wine), a biblioteca grava um manifesto equivalente na pasta temporária, e é assim que os testes rodam sob Wine com a v6. Confira no Windows: o ícone e o texto desenhados sobre o fundo do tema no `NM_CUSTOMDRAW` do botão (`Button::is_icon_native` verdadeiro), o texto centralizado nas caixas de texto e seletores (faixas não cliente) e a altura da `ComboBox` (`CB_SETITEMHEIGHT(-1)`).

13. **Recursos da segunda etapa só vistos no Wine**: confira no Windows a aparência das listas e tabelas do `RichTextBox` e, principalmente, as imagens (`insert_image`: o RichEdit do Wine guarda o `\pict` mas não desenha metarquivos); a pré-visualização e a impressão com intervalo de páginas numa impressora real; a lista de candidatos própria com um IME chinês/japonês (seleção pelo clique e pelas teclas do IME); o antialiasing do SVG e dos gráficos e os gradientes radiais com várias paradas (o GDI+ do Wine não suaviza caminhos dentro de imagens e só usa as cores do centro e da borda); gestos com uma tela de toque real (pinça, rotação, inércia) e o `WM_GESTURE` no Windows 7; a janela com `TitleBar` (sombra do DWM, maximizar sem passar da tela, layouts de encaixe do Windows 11 sobre o botão maximizar, Alt+Espaço); flutuar/acoplar painéis e destacar abas com o mouse real; as notificações com campos, progresso, agendamento e o ativador COM (clique com o programa fechado); `AnimateWindow` nos controles desenhados; o `Modal` no Windows (transparência real do fundo sobre a janela viva, sombra, cantos arredondados do Windows 11, a animação de entrada/saída e o retorno do foco ao trocar de programa com o modal aberto).

## Limitações conhecidas (por projeto ou por ora)

| Área | Limitação |
|---|---|
| Arrastar e soltar | A reordenação move todos os itens selecionados em bloco (`ListBox`, `ListView`, `DataGrid` também com fonte virtual, `TreeView` um nó por vez, `CommandBar`), mas não funciona em listas `sorted`. Arrastar para fora oferece copiar e, com `set_drag_out_effects`, mover (a aplicação apaga o item no fim). No Wine a imagem do item não é translúcida e fica ao lado do cursor (sem compositor); `SHCreateDataObject` não existe no Wine, então o objeto de dados próprio da biblioteca é usado ali. |
| Notificações | `SystemNotification` (Windows 10+) registra um *AppUserModelID* em `HKCU\Software\Classes\AppUserModelId` na primeira notificação; com `NotificationActivator::register` o clique abre o programa fechado (registro COM por usuário, `LocalServer32`). Campos de texto/seleção, progresso, agendamento e grupos seguem o esquema dos toasts do Windows 10; nada disso pôde ser exibido (o Wine não tem `Windows.UI.Notifications`). No Windows 7 e no Wine devolve `Unsupported` — use `TrayIcon::show_notification`. |
| UI Automation | `MenuStrip`, listas dos menus desenhados, `CommandBar`, `PropertyGrid`, `DataGrid`, os controles modernos, `TitleBar`, `DockPane` e os modelos de `set_accessible_model` publicam elementos virtuais (Invoke, Toggle, ExpandCollapse, Value, RangeValue, Selection, Scroll, Grid, Text) e aceitam `IRawElementProviderAdviseEvents`. As listas de menu continuam filhas da área de trabalho (como os menus nativos). Nada foi ouvido com Narrador/NVDA. |
| Clipboard | Imagens são gravadas como `CF_DIB`, `CF_DIBV5` (com alfa) e `PNG` e lidas do melhor formato disponível; metarquivos como `CF_ENHMETAFILE`. `ClipboardWatcher` usa `AddClipboardFormatListener` (Vista+). |
| Tema escuro | Com tema escuro, a biblioteca pinta abas (só abas no topo; `TCS_BOTTOM`/verticais/botões ficam nativas), barra de status, cabeçalhos de coluna, `GroupBox`, as bordas 3D (`WS_EX_CLIENTEDGE`), o campo do `DatePicker`/`TimePicker` e o `Slider` (repintura sobre o desenho nativo); sem realce de hover nas abas e nos cabeçalhos. Calendário, barra de progresso e barra de ferramentas passam ao desenho clássico com as cores do tema. A `MenuBar` nativa é trocada automaticamente por uma `MenuStrip` com as cores do tema (desligue com `set_menu_bar_follows_theme(false)`). Barras de rolagem dependem de `DarkMode_Explorer` (Windows 10 1809+). No Wine, barras de rolagem e o trilho da barra de progresso ficam claros. |
| `InfoBar` / `Toast` | O toast usa `WS_EX_LAYERED` só durante o esmaecimento (com o Windows configurado para reduzir animações, aparece e some sem efeito). Para a Central de Ações use `SystemNotification`. Os dois anunciam o texto aos leitores de tela (`UiaRaiseNotificationEvent`, Windows 10 1709+); o anúncio não foi ouvido. |
| `CommandBar` | Itens saem pelo fim da barra (os da direita primeiro); controles hospedados que não cabem vão para o `»` com o rótulo deles. A personalização troca ordem e visibilidade (`CommandBarLayout`), não cria comandos. |
| Menus | Trocar a barra de menus de uma janela não destrói a anterior. `MenuStrip`: sem arrastar a barra, sem itens à direita e sem animação de abertura; a sombra das listas (`CS_DROPSHADOW`) e os cantos arredondados do Windows 11 não foram observados (o Wine não os desenha). `on_highlight_changed` na barra nativa depende de `WM_MENUSELECT`. |
| `DataGrid` | Edição de células em linha (sem editores de lista suspensa ou data); agrupamento por uma coluna; exportação em texto (CSV/TSV/HTML), sem formatos binários. |
| `PropertyGrid` | Tipos de valor: texto, número, booleano e escolha; sem editores personalizados (cor, arquivo). |
| `RichTextBox` | Listas, tabelas e imagens são inseridas como RTF no cursor e editadas pelo próprio RichEdit (sem redimensionar colunas ou imagens pela API). As imagens viram metarquivos EMF dentro do documento. |
| `Rebar`, `Pager`, `Animation`, `HotKeyBox` | Cobertura do essencial de cada controle. |
| Impressão | `PrintDlgW` (um intervalo de páginas `De/Até`, sem várias faixas como o `PrintDlgEx`); a pré-visualização desenha cada página num bitmap na escala da tela; cópias são feitas pelo driver quando ele suporta. |
| Toque | Gestos reconhecidos pela biblioteca a partir de `WM_POINTER*` (Windows 8+) e, sem eles, do `WM_GESTURE` (Windows 7). Sem gestos de três ou mais dedos, sem "deslizar para trás". O Wine informa `PT_MOUSE` para ponteiros sintéticos. |
| IME | Lista de candidatos própria (`Ime::use_candidate_window`) e controle da composição por `ImmNotifyIME`; a janela de composição pode ser escondida, mas o texto em composição é desenhado pela aplicação. Sem suporte a TSF (Text Services Framework) avançado. |
| Internacionalização | Formatação regional e RTL de janela (`right_to_left`); os textos internos da biblioteca vêm em português e inglês (`set_library_language`, ou `use_system_library_language` pelo idioma do Windows) e aceitam tradução por catálogo (`apply_library_translations`). |
| Fontes | Tamanhos fracionários são arredondados para décimos de ponto. |
| Posição de janelas | Em pixels físicos de tela (o tamanho é lógico) — ver ARCHITECTURE.md. |
| Texto com NUL interno | Truncado pelo Windows no primeiro NUL. |
| SVG | Sem texto, imagens embutidas, `clipPath`, `mask`, `pattern`, filtros, marcadores e animações; a `opacity` de grupo é aplicada a cada elemento. |
| Gráficos (`Chart`) | Uma escala de valores (sem segundo eixo), categorias como texto (sem eixo de datas/números), pizza com a primeira série. |
| Animações | Dirigidas por um temporizador de ~15 ms na thread de interface; `show_animated`/`hide_animated` usam `AnimateWindow` (síncrono, bloqueia até o fim). |
| Janela sem moldura (`TitleBar`) | Não combina com `MenuStrip` na área não cliente da mesma janela (ponha a `MenuStrip`/`CommandBar` abaixo da barra). Sombra e cantos dependem do DWM. |
| Painéis acopláveis | Mudam de lado por `dock_to` ou pelo botão; não há alvos de acoplamento ao arrastar a janela flutuante. Uma zona empilha os painéis (sem abas dentro da zona). |
| `Modal` | O fundo cobre a área cliente da janela dona (a barra de título dela fica visível e desabilitada, como em qualquer modal do Windows). Transparência por pixel com `UpdateLayeredWindow` (Windows 2000+, sem depender do DWM); no Wine, sem compositor, o fundo vira uma foto escurecida e opaca da janela, tirada ao abrir. Os cantos arredondados do cartão vêm do DWM (Windows 11); antes dele o cartão é retangular. |
| Arquiteturas | x64 e x86 (32 bits) testados (no Wine); ARM64 só compila. Ver "Plataformas" acima. |
| Abas destacáveis | A aba volta sempre no fim da faixa; ícones da aba não acompanham a janela destacada. |
| Ícones | ComboBox e ListBox exibem ícones apenas se criadas com `with_icons(true)`. Ícones do pacote carregados ficam em cache até o fim da thread. O SVG é desenhado por um subconjunto próprio do SVG 1.1 (ver `graphics::svg`). |
| Tamanho do executável com o recurso `icons` | Sem LTO, a toolchain GNU (mingw) não descarta ícones não usados (`function-sections` desligado no alvo) e embute o pacote inteiro (~66 MB); compilações `debug` de qualquer toolchain também. Em `release` com LTO (perfil do repositório) ou MSVC com `/OPT:REF`, só os ícones usados. Desligue com `default-features = false` se não usar o pacote. |
