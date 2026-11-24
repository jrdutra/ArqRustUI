# Arquitetura

*English: [ARCHITECTURE.en.md](ARCHITECTURE.en.md).*

## Visão em camadas

```
┌────────────────────────────────────────────────────────────────────────┐
│ Aplicação (qualquer projeto Rust)            use arqrustui::prelude::*;  │
├────────────────────────────────────────────────────────────────────────┤
│ API pública de alto nível                                              │
│ controls · layout · menus · dialogs · graphics · themes · system       │
│ input · accessibility · i18n                                           │
├────────────────────────────────────────────────────────────────────────┤
│ core — Application, Window, Widget/WidgetCore, eventos, loop de        │
│        mensagens, despachante, timers, handles RAII, erros             │
│ core::native — padrões de chamada Win32 encapsulados (unsafe isolado)  │
├────────────────────────────────────────────────────────────────────────┤
│ ffi — declarações #[repr(C)] / extern "system" escritas à mão          │
├────────────────────────────────────────────────────────────────────────┤
│ DLLs do Windows: user32, gdi32, comctl32, comdlg32, shell32, ole32,    │
│ uxtheme, dwmapi, d2d1, dwrite, gdiplus, imm32, uiautomationcore…       │
└────────────────────────────────────────────────────────────────────────┘
```

Regra de dependência: cada camada só conhece as de baixo. `ffi` não contém lógica; os módulos de alto nível não declaram `extern`.

## Estrutura de módulos

```
src/
├── lib.rs              raiz do crate + prelude
├── ffi/                uma DLL por arquivo (tipos, constantes, extern)
│   ├── types.rs        HWND, RECT, GUID, helpers LOWORD/HIWORD…
│   ├── user32.rs  gdi32.rs  kernel32.rs  comctl32.rs  comdlg32.rs
│   ├── shell32.rs ole32.rs  uxtheme.rs   dwmapi.rs    d2d1.rs  dwrite.rs
│   ├── gdiplus.rs imm32.rs  uiautomation.rs
│   └── dynamic.rs      LoadLibrary/GetProcAddress + APIs opcionais por versão
├── core/
│   ├── application.rs  Application, estado por thread, janela de despacho
│   ├── event_loop.rs   GetMessage → menus desenhados → atalhos → IsDialogMessage → Dispatch
│   ├── widget.rs       WidgetCore, registro HWND→núcleo, window_proc, trait Widget;
│   │                   widget/dispatch.rs = despacho das mensagens (o `match` que vira eventos)
│   ├── window.rs       Window, WindowBuilder, classes de janela
│   ├── events.rs       Event, EventKind, HandlerId
│   ├── dispatcher.rs   UiDispatcher, spawn_local, run_blocking, sleep
│   ├── timer.rs        Timer
│   ├── motion.rs       animações (Easing, animate, transições)
│   ├── handles.rs      GdiObject, Icon, ImageList, MemoryDc, WindowDc, GlobalMemory
│   ├── errors.rs       GuiError, GuiResult
│   ├── messages.rs     RawMessage (API avançada)
│   └── native.rs       helpers internos (send, send_ref, estilos, DPI)
├── controls/           um arquivo por controle ou família; modern.rs = base dos controles modernos;
│                       os maiores têm submódulos por tema (datagrid/{drawing,editing,export,filtering,
│                       reorder,styling}.rs, commandbar/{access,customize,flyout,layout,painting}.rs)
├── layout/             engine.rs (cálculo puro) + mod.rs (layouts ligados a widgets)
├── menus/              mod.rs (Menu, MenuBar, ContextMenu); popup.rs + popup/{tracking,painting,access}.rs
│                       (menus desenhados); strip.rs + strip/{bar,messages}.rs (MenuStrip)
├── dialogs/  themes/
├── system/             área de transferência, impressão, bandeja, monitores…; notifications.rs +
│                       notifications/{activator,winrt}.rs (notificações do Windows)
├── graphics/           gdi.rs, gdiplus.rs, direct2d.rs, vector.rs, svg.rs + svg/{xml,parse}.rs
├── input/              teclado, mouse, ime.rs + ime/, gestures.rs, drag.rs + drag/{source,target,reorder}.rs
├── icons/              IconAsset, categorias, busca; catalog.rs com os 392 ícones
├── accessibility/      mod.rs (IAccessible/anúncios), model.rs, tree.rs (árvore UIA própria) +
│                       tree/{provider,patterns,debug}.rs (métodos COM, padrões, chamadas para os testes)
├── i18n/  utilities/
examples/               aplicações completas (uma por área)
tests/                  integração (janelas reais) + validação de layout FFI
docs/                   esta documentação
crates/                 arqrustui-icons (fachada) + arqrustui-icons-data1..7 (bytes dos ícones)
assets/icons/           origem e tratamento do pacote de ícones, prancha preview.png
.github/workflows/      CI (GitHub Actions)
```

## O núcleo: `WidgetCore`

Toda janela nativa criada pela biblioteca — janela de nível superior ou controle — tem exatamente um `WidgetCore`:

- guarda o `HWND` (anulado na destruição), os manipuladores de evento, fonte, cores, layout, cursor, menu de contexto, atalhos e recursos mantidos vivos;
- fica em um **registro por thread** `HashMap<HWND, Rc<WidgetCore>>`, que o mantém vivo enquanto a janela nativa existir;
- os tipos públicos (`Button`, `Window`, …) são apenas `struct { core: Rc<WidgetCore> }` — *handles* clonáveis.

### Um único procedimento de janela

`core::widget::window_proc` atende **todas** as janelas:

- classes próprias (`ArqRustUI.Window`, `ArqRustUI.Panel`) o usam como `WNDPROC`;
- controles do sistema (`BUTTON`, `EDIT`, `SysListView32`…) são *subclassificados* com `SetWindowLongPtrW(GWLP_WNDPROC)`; o procedimento original é guardado e chamado via `CallWindowProcW`.

Fluxo de uma mensagem:

1. `window_proc` localiza o `WidgetCore` no registro (sem ele → `DefWindowProcW`).
2. A chamada é envolvida em `catch_unwind`: um panic em um manipulador nunca atravessa a fronteira FFI; ele é guardado e relançado pelo loop de eventos. O mesmo vale para todas as outras entradas chamadas pelo Windows (métodos COM e callbacks), que rodam dentro de `core::ffi_guard::guard`/`guard_hresult` — em caso de pânico o Windows recebe `E_UNEXPECTED` ou um valor neutro.
3. `WidgetCore::handle` traduz a mensagem em `Event` e chama os manipuladores.
4. Notificações que o Windows entrega **ao pai** (`WM_COMMAND`, `WM_NOTIFY`, `WM_HSCROLL/VSCROLL`, `WM_DRAWITEM`, `WM_CTLCOLOR*`) são **refletidas** para o núcleo do controle filho que as originou. É isso que permite `button.on_click(..)` funcionar com qualquer contêiner (janela, painel, caixa de grupo, aba).
5. O restante segue para o procedimento original.

### Ciclo de vida

```
build() ──► CreateWindowExW ──► registro.insert(hwnd, core)
                                        │
                 eventos ◄──────────────┤
                                        │
destroy()/pai destruído ──► WM_NCDESTROY ──► emit(Destroyed)
                                             registro.remove(hwnd)
                                             core.hwnd = null
                                             manipuladores, layout, menus, recursos soltos
```

Depois de `WM_NCDESTROY`, os handles que a aplicação ainda guarda ficam **inertes**: métodos viram no-ops, `is_alive()` devolve `false`, operações que devolvem `GuiResult` devolvem `GuiError::WidgetDestroyed`. Soltar todos os manipuladores nesse momento quebra os ciclos `Rc` que closures do tipo `button.on_click(move || button2.…)` criam.

## Aplicação e loop de eventos

`Application::new()` (uma por thread de interface):

1. ativa DPI por monitor v2 (`SetProcessDpiAwarenessContext`, com *fallback*);
2. ativa Common Controls v6 por contexto de ativação criado em tempo de execução (manifesto embutido em `shell32.dll`), dispensando manifesto na aplicação, e só então carrega `comctl32.dll` dinamicamente;
3. inicializa OLE/COM (STA);
4. registra as classes de janela e cria a **janela de despacho** (somente-mensagem), que recebe tarefas enfileiradas, temporizadores e mensagens da bandeja.

`Application::run()` executa: `GetMessageW` → modo de menu da `MenuStrip`/menus desenhados (`menus::popup::pre_translate`: teclado e mouse enquanto um menu está aberto; fora dele, Alt, F10, Alt+letra e atalhos dos itens) → diálogos não modais → atalhos de teclado da janela → `IsDialogMessageW` (Tab, setas, mnemônicos, Enter/Esc) → `TranslateMessage`/`DispatchMessageW`. Janelas modais rodam um loop aninhado com a janela dona desabilitada.

### Complemento do tema escuro

Alguns controles do Windows ignoram as cores com estilos visuais ativos. `themes::dark` cobre esses casos sem trocar o controle: com um tema escuro na janela, `WidgetCore::handle` passa `WM_PAINT`/`WM_PRINTCLIENT` de abas, barra de status, cabeçalhos (`SysHeader32`, inclusive o da ListView, adotado como núcleo com `WidgetCore::adopt`) e `GroupBox` para uma pintura própria que lê o estado do controle nativo (itens, textos, ícones, seleção), e pinta a borda 3D depois do `WM_NCPAINT` padrão. A barra de ferramentas recebe as cores no `NM_CUSTOMDRAW`; calendário, progresso e rebar, por mensagens de cor no modo clássico. Com tema claro nada é interceptado e o desenho volta a ser 100% nativo.

### Menus desenhados (`MenuStrip`)

A faixa da `MenuStrip` é área **não cliente** da janela de topo: `WidgetCore` encaminha as mensagens da janela a `menus::strip::root_message`, que soma a altura da faixa em `WM_NCCALCSIZE`, a desenha depois do `WM_NCPAINT`/`WM_NCACTIVATE` padrão (DC da janela inteira, com double buffering) e responde `HTMENU` em `WM_NCHITTEST`. `outer_rect_for_client` inclui a faixa, de modo que `size(..)` continua sendo o tamanho da área cliente.

As listas são janelas `WS_POPUP` da classe `ArqRustUI.MenuPopup`, com `WS_EX_NOACTIVATE` e `MA_NOACTIVATE`: a janela dona continua ativa e com o foco. Não há loop modal; um único rastreador por thread recebe teclado e mouse no início do loop de mensagens enquanto houver menu aberto, e é cancelado quando a janela perde a ativação, muda de tamanho, recebe `WM_CANCELMODE` ou é destruída. Nenhum `RefCell` fica emprestado durante chamadas que enviam mensagens (criar/destruir janelas) nem durante o manipulador de um item, que só roda depois de o modo de menu terminar.

A `CommandBar` reaproveita o mesmo rastreador: um botão de menu abre a lista com `popup::open_dropdown` (ancorada abaixo do botão), recebe um aviso quando o modo de menu termina e uma função de troca, consultada quando o cursor passa fora das listas, que abre o menu de outro botão da barra.

Os itens continuam no `HMENU` do `Menu` (fonte única para a barra nativa e a desenhada): a lista lê texto, estado, tipo e submenus com `GetMenuItemInfoW` no momento de abrir, e os ícones vêm da forma original (`IconSource`), recarregados no DPI da janela.

### Acessibilidade dos controles desenhados (`accessibility::tree`)

Cada controle desenhado implementa `VirtualTree`: uma lista plana de `Node`s (papel UIA, nome, retângulo de tela, estados e quais padrões tem) e `act(id, Action)`. `TreeHost` publica isso como fragmentos UIA: um único tipo de objeto COM com uma vtable por interface (`IRawElementProviderSimple`, `…Fragment`, `…FragmentRoot` só na raiz, `IInvokeProvider`, `IToggleProvider`, `IExpandCollapseProvider`, `IValueProvider`), criados sob demanda a cada navegação e identificados pelo `RuntimeId` (`UiaAppendRuntimeId`, id). A raiz é a própria janela: devolve `UiaHostProviderFromHwnd` e deixa nome e tipo para o provedor padrão quando a árvore não os define (caso da janela com `MenuStrip`). Os elementos guardam só um `Weak` da árvore e a thread de origem; de outra thread respondem `UIA_E_ELEMENTNOTAVAILABLE`. O `WidgetCore` responde `WM_GETOBJECT` com a árvore; as listas de menu, com janela própria, fazem o mesmo em `popup_proc`. Mudanças de foco pelo teclado disparam `UIA_AutomationFocusChangedEventId` (só com clientes ouvindo); as listas também disparam `MenuOpened`/`MenuClosed` e os eventos MSAA `EVENT_SYSTEM_MENUPOPUPSTART/END`.

Além desses, `IRawElementProviderAdviseEvents` (os eventos só são disparados com clientes ouvindo) e os padrões Selection/SelectionItem, Scroll, Grid/GridItem, RangeValue e Text vivem em `accessibility/tree/patterns.rs`. A aplicação publica a própria árvore com `set_accessible_model` (`accessibility/model.rs`): um `AccessibleModel` devolve `AccessibleItem`s (convertidos em `Node`s com retângulos de tela) e recebe `AccessibleAction`s. Os controles modernos usam esse mesmo caminho por meio de `controls/modern.rs::publish`, que guarda só referências fracas ao estado do controle.

### Notificações do sistema (`system::notifications`)

`SystemNotification` monta o XML `ToastGeneric` e fala com o Windows Runtime por índices de vtable, como no resto do COM da biblioteca: `combase.dll` é resolvida dinamicamente (`ffi::winrt`), `RoActivateInstance` cria o `XmlDocument`, `RoGetActivationFactory` dá a fábrica de `ToastNotification` e o `ToastNotificationManager` (`CreateToastNotifierWithId`). Os eventos `Activated`, `Dismissed` e `Failed` recebem delegados próprios (`TypedEventHandler<…>`, IIDs calculados pela regra dos tipos parametrizados e conferidos num teste com SHA-1), com contagem atômica e `IAgileObject`: o Windows os chama numa thread qualquer, eles leem os argumentos ali e repassam o evento à thread da interface pelo `UiDispatcher`, onde ficam os callbacks (`Rc`). O *AppUserModelID* é registrado em `HKCU` e associado ao processo com `SetCurrentProcessExplicitAppUserModelID`. Para cliques com o programa fechado, `notifications/activator.rs` registra um servidor COM local por usuário (`HKCU\Software\Classes\CLSID\{…}\LocalServer32` e `ToastActivatorCLSID` no AUMID) e uma fábrica de classe (`CoRegisterClassObject`) que cria objetos `INotificationActivationCallback`; a ativação chega numa thread do COM e é repassada à interface pelo `UiDispatcher`.

### Arrastar e soltar (`input::drag`)

O módulo é dividido em `drag/source.rs` (origem OLE: `start_drag`, imagem do arraste), `drag/target.rs` (destino: `accept_drops`, `DropData`) e `drag/reorder.rs` (reordenação por captura de mouse, movimento em bloco de vários selecionados, efeitos para fora); `drag.rs` só reexporta. Dois mecanismos complementares, ambos sem dependências:

* **OLE** para tudo que cruza fronteiras de controle ou de programa. A origem monta um `IDataObject` (o do shell, `SHCreateDataObject`; sem ele — Wine, sistemas reduzidos — o objeto próprio de `input/data_object.rs`, com `IEnumFORMATETC`) a partir dos mesmos blocos do clipboard (`ClipboardContent::blocks`), e chama `DoDragDrop` com um `IDropSource` próprio. O destino é um `IDropTarget` próprio registrado com `RegisterDragDrop` e guardado no `DragState` do `WidgetCore`; `WM_DESTROY` faz o `RevokeDragDrop`. Os objetos COM são `#[repr(C)]` com vtable estática e contagem de referências em `Cell` (todos vivem na thread STA da interface). Callbacks do usuário rodam sob `catch_unwind`, e o panic é relançado pelo loop de eventos.
* **Captura de mouse** para reordenar dentro de `ListBox`/`ListView`/`DataGrid`/`TreeView`: o arraste começa pelo limiar `SM_CXDRAG` (ListBox) ou por `LVN_BEGINDRAG`/`TVN_BEGINDRAGW` (interceptados no `WM_NOTIFY` do pai); a partir daí o controle fica com a captura e um temporizador faz a rolagem automática. O item é copiado para uma seção DIB e exibido numa janela `WS_POPUP` transparente ao mouse (`ArqRustUI.DragVisual`, translúcida com `WS_EX_LAYERED` fora do Wine); a linha de inserção é outra janela da mesma classe, recortada com `SetWindowRgn`. Quando o cursor sai do controle e há `on_drag_out`, a sessão termina e vira um `DoDragDrop`, levando o desenho do item como imagem do arraste (`IDragSourceHelper`). Nenhum `RefCell` fica emprestado enquanto janelas são movidas (o Windows pode gerar um `WM_MOUSEMOVE` sintético que volta ao módulo).

### Animações (`core::motion`)

Todas as animações de uma thread compartilham um único `Timer` de ~15 ms, criado quando a primeira começa e solto quando a última termina. Cada animação guarda início, duração, curva (`Easing`) e o callback de quadro; o quadro é calculado a partir do relógio (`Instant`), não da contagem de tiques, então um atraso do loop não alonga a animação. Os callbacks são copiados para fora do `RefCell` antes de chamados (podem iniciar ou cancelar outras animações). Com "Mostrar animações" desligado no Windows (`SPI_GETCLIENTAREAANIMATION`), cada animação vai ao fim no primeiro quadro. Os controles desenhados respondem a `WM_PRINTCLIENT` (pintam no DC recebido), o que permite `AnimateWindow` e capturas.

### Vetores e SVG (`graphics::vector`, `graphics::svg`)

`vector.rs` define figuras com retas e Béziers cúbicas em `f64`, tintas sólidas e gradientes e o traço, e as desenha com a API plana do GDI+ (`GdipCreatePath`, `GdipCreateLineBrush` + `GdipSetLinePresetBlend`, `GdipCreatePathGradientFromPath`, `GdipCreatePen2`). O GDI+ repete gradientes lineares fora do eixo; o modo `Pad` do SVG é emulado estendendo o eixo até cobrir a figura e reposicionando as paradas, e o radial pinta a área fora da elipse com a última cor (recorte excluindo a elipse). Quando a transformação é uma similaridade, os pontos são transformados em Rust e a espessura do traço escalada aqui — o resultado não depende de como cada GDI+ escala canetas (o do Wine não as escala).

O SVG é lido por um leitor de XML mínimo (`svg/xml.rs`) e interpretado em `svg.rs`: cascata (atributos de apresentação → regras de `<style>` por especificidade → atributo `style`), herança, `use`, gradientes com cadeia `href` e `viewBox`/`preserveAspectRatio`. O resultado é uma lista de operações (`DrawOp`) testável sem janelas, desenhada depois pelo `VectorPainter`.

### Controles modernos e janela sem moldura

`ToggleSwitch`, `NumberBox`, `RatingControl`, `Card`, `Expander`, `BreadcrumbBar`, `Chart`, `PrintPreview`, `DockHost`/`DockPane` e `TitleBar` são painéis da classe da biblioteca com `paint` próprio; a geometria de cada um (escalas do gráfico, trilha recolhida, zonas de acoplamento, regiões da barra de título) é uma função pura com teste unitário. A `TitleBar` acrescenta um filtro de mensagens à janela: `WM_NCCALCSIZE` devolve a janela toda como área cliente (com o ajuste da borda quando maximizada) e `WM_NCHITTEST` responde bordas, `HTCAPTION` e `HTMAXBUTTON` (layouts de encaixe do Windows 11); a própria barra responde `HTTRANSPARENT` fora dos botões, de modo que arrastar, duplo clique e o menu de sistema ficam com o Windows. Flutuar um painel ou destacar uma aba troca o pai da janela do conteúdo (`SetParent`) — os controles filhos e o estado vão junto; fechar a janela flutuante devolve o conteúdo antes que ela seja destruída.

### Modal (`dialogs::modal`)

São duas janelas nativas donas da janela da aplicação. O **fundo** é um popup `WS_EX_LAYERED | WS_EX_NOACTIVATE` atualizado com `UpdateLayeredWindow` a partir de uma seção DIB pré-multiplicada: preto com a opacidade do escurecimento e uma sombra calculada em Rust (distância até o retângulo arredondado do cartão, mais forte embaixo); a animação só troca o `SourceConstantAlpha`. O **cartão** é uma janela da biblioteca sem moldura, dona = fundo (fica sempre acima dele), com o tema da janela dona, os controles normais e cantos do DWM no Windows 11. A dona é desabilitada (`EnableWindow`) como num modal comum e reabilitada antes de o cartão sumir, para a ativação voltar a ela; o foco anterior é devolvido e guardado na dona (volta mesmo se ela estiver inativa). `show` roda o loop de mensagens até a fase `Closed`. No Wine (sem compositor) a camada recebe uma cópia da tela sob a dona, escurecida e opaca.

### Gestos (`input::gestures`)

`GestureRecognizer` é lógica pura alimentada por contatos com tempo: centroide e, com dois dedos, distância e ângulo entre os dois primeiros dão deslocamento, escala e rotação; trocar o conjunto de contatos refaz a referência (sem "salto"); a velocidade é uma média exponencial usada pela inércia, que decai exponencialmente num relógio só ligado quando preciso. A ligação com o widget usa `WM_POINTER*`; sem eles (Windows 7) traduz o `WM_GESTURE` para os mesmos eventos.

## DPI e coordenadas

Toda a API pública usa **unidades lógicas (96 DPI)**. A conversão ocorre em `WidgetCore::set_bounds` e nos eventos de mouse. Em `WM_DPICHANGED`, a janela adota o retângulo sugerido e todos os descendentes têm limites e fontes reaplicados no novo DPI; em seguida o layout é refeito. O `Canvas` GDI configura o DC (`MM_ANISOTROPIC`) para que o desenho também seja em unidades lógicas; Direct2D usa DIPs nativamente.

Exceção deliberada: a **posição** de janelas de nível superior é em pixels de tela (coordenadas do desktop virtual), porque uma coordenada lógica de tela seria ambígua entre monitores com escalas diferentes.

## Layout

`layout::engine` contém funções puras (`stack`, `grid`, `flow`, `dock`, `anchor`, `split`) que recebem especificações e devolvem retângulos — testáveis sem janelas. Os tipos `StackLayout`, `GridLayout`, etc. guardam os itens (controles ou layouts aninhados) e aplicam os retângulos com `set_bounds`. Um layout associado com `set_layout` é reaplicado em `WM_SIZE` e nas mudanças de DPI.

## Gráficos

Três backends independentes, selecionados pelo tipo usado:

| Backend | Entrada | Observações |
|---|---|---|
| GDI | `Canvas` (em `on_paint`, `Bitmap::render`, impressão) | double buffering por padrão |
| GDI+ | `canvas.with_gdiplus(\|g\| …)` | antialiasing, alfa, PNG/JPEG |
| Direct2D + DirectWrite | `Direct2DSurface::attach(&painel, \|d2d\| …)` | COM por índice de vtable; recria o alvo ao perder o dispositivo |

Sobre o GDI+ ficam os vetores em ponto flutuante e o SVG (seção acima); `Bitmap::render_at` desenha em qualquer DPI numa seção DIB (usado pela pré-visualização de impressão, que desenha a mesma função de página da impressora).

## Ícones

`icons::IconAsset` guarda referências `'static` aos bytes de um `.ico` (vários tamanhos), de um `.png` e, opcionalmente, de um `.svg`; os 392 ícones do pacote são `static`s gerados em `icons/catalog.rs` (uma linha `icon!` por ícone) com `include_bytes!`, atrás do recurso `icons`. Os bytes vêm da crate `arqrustui-icons`, dependência opcional do recurso `icons`: ela só reexporta as oito crates de dados de `crates/`, cada uma com até 9 MB (o crates.io aceita até 10 MiB por pacote) e uma constante `(.ico, Option<.png>, Option<.svg>)` por ícone; a linha `icon!` pega essa constante pelo mesmo nome. Quando o PNG é cópia exata do quadro de 256 px do `.ico`, a crate de dados guarda `None` e `IconAsset::png_bytes` devolve esse quadro como um trecho do `.ico` (170 dos 392 ícones). A tabela `EXTENSIONS` do mesmo arquivo liga extensões (`pdf`, `yml`, `tar.gz` → `gz`...) aos ícones de `IconCategory::FileTypes`; `for_extension`/`for_path` só a percorrem, então com LTO entram no executável apenas esses ícones. Quem desenha ícones não recebe um `HICON` pronto, e sim um `IconSource` — implementado por `Icon` e `IconAsset` — e pede a ele o tamanho em pixels de que precisa, calculado a partir do DPI do controle. `IconAsset` escolhe a imagem do `.ico` mais próxima (`CreateIconFromResourceEx`, que aceita quadros PNG) e guarda o resultado em um cache por thread.

Dois mecanismos em `controls/icon_support.rs` levam ícones aos controles:

| Mecanismo | Controles | Como funciona |
|---|---|---|
| `WidgetIcon` no `WidgetCore` | `Button`, `Label`, `Window` | guarda a origem, o tamanho lógico e a posição; o ícone é recarregado no DPI atual a cada desenho (rótulo, botão desenhado pela biblioteca) ou reenviado ao controle (`WM_SETICON`) quando o DPI muda. No `Button` nativo (v6), ícone e texto são desenhados no `CDDS_PREPAINT` do `NM_CUSTOMDRAW`, sobre o fundo que o tema já pintou; o tamanho automático é 60% da altura |
| `ManagedImages` no `WidgetCore` | `TreeView`, `ListView`, `DataGrid`, `TabControl`, `ToolBar`, `ComboBox`, `ListBox`, `StatusBar` | uma `ImageList` no tamanho do DPI; cada origem distinta ocupa uma posição (reaproveitada por chave); na mudança de DPI a lista é refeita com as mesmas posições e reenviada ao controle |

`ComboBox` e `ListBox` com ícones são `*_OWNERDRAWFIXED`: o item guarda no seu dado (`*_SETITEMDATA`) a posição da origem + 1, de modo que o ícone acompanha o item em listas ordenadas; `WM_MEASUREITEM` é respondido pelo pai com uma altura padrão (o controle ainda não está no registro) e corrigido logo após a criação.

Caixas de combinação não editáveis são sempre `CBS_OWNERDRAWFIXED | CBS_HASSTRINGS`: o texto do campo fica centralizado em qualquer altura e os itens aceitam ícones; nas editáveis, a caixa de texto interna é recentralizada (`GetComboBoxInfo`) a cada ajuste.

Controles de uma linha que desenham o texto no alto da área cliente (`TextBox` de uma linha, `PasswordBox`, `SpinButton`, `DatePicker`, `TimePicker`, `HotKeyBox`) ligam `WidgetCore::set_centered_text`: em `WM_NCCALCSIZE` a área cliente é reduzida à altura de uma linha com faixas iguais em cima e embaixo; `WM_NCPAINT` pinta as faixas com o pincel que o pai daria ao controle (`WM_CTLCOLOREDIT`/`WM_CTLCOLORSTATIC`, o que respeita temas e o estado desabilitado) e `WM_NCHITTEST` trata as faixas como área cliente.

A altura da `ComboBox` fechada não segue a da janela: o controle nativo a calcula pela fonte. Para que a caixa fique com a altura lógica pedida (por padrão `DEFAULT_CONTROL_HEIGHT`, a mesma do `Button`), `WidgetCore` chama `combobox::fit_field_height` depois de cada `set_bounds` e de cada troca de fonte (o que cobre a mudança de DPI): a altura do campo de seleção é corrigida com `CB_SETITEMHEIGHT(-1)` pela diferença entre a altura fechada medida e a pedida, sem ficar abaixo da altura do texto.

## Decisões de projeto relevantes

| Decisão | Motivo |
|---|---|
| Handles `Clone` com `Rc`, sem lifetimes na API | Closures de evento precisam capturar controles; o estado real pertence à janela nativa |
| Registro por `HWND` em vez de ponteiro em `GWLP_USERDATA` | Nenhum ponteiro bruto para estado Rust fica guardado no Windows; elimina *use-after-free* |
| Subclassing clássico em vez de `SetWindowSubclass` | Não depende da versão do comctl32 carregada |
| `comctl32` resolvida dinamicamente | Importação estática ligaria à v5.82 (sem `TaskDialogIndirect`) em processos sem manifesto |
| APIs recentes via `GetProcAddress` | Detecção de capacidades; o executável carrega em qualquer Windows suportado |
| Sem `Send`/`Sync` em tipos visuais | Afinidade de thread do Win32 garantida pelo compilador |
| Ícones como `static`s com `include_bytes!`, atrás de um recurso | Sem arquivos para distribuir; com LTO só os ícones usados entram no executável; quem não usa o pacote o desliga |
| Controles recebem `IconSource`, não `HICON` | O controle escolhe a resolução pelo próprio DPI e consegue recarregá-la quando o DPI muda |
