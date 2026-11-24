# Matriz de funcionalidades

*English: [FEATURE_MATRIX.en.md](FEATURE_MATRIX.en.md).*

Validação local em Windows, 16/11/2026: rustdoc em inglês gerado sem avisos;
Clippy com `-D warnings` aprovado. Os cinco testes GDI, incluindo escala única
GDI/GDI+ a 96/144/192 DPI e preservação de transformação/recorte, passaram.
Suíte completa: 356 passaram, 32 falharam e um teste de diálogo de impressão
foi excluído por travamento já observado. As 32 falhas coincidem exatamente com
o registro anterior em `target/bevel-import/test-final.log`; as contagens
históricas abaixo não substituem essa execução. Capturas e detalhes em
[CORRECOES_DEMOS_E_RUSTDOC.md](CORRECOES_DEMOS_E_RUSTDOC.md).

Estados, em ordem crescente:

| Estado | Significado |
|---|---|
| Não iniciado | sem código |
| Em desenvolvimento | cobertura parcial; a coluna de observações diz o que falta |
| Implementado | código completo, sem teste automatizado próprio |
| Testado | implementado + teste automatizado passando |
| Documentado | testado + documentação pública (rustdoc e guias) |
| Concluído | documentado + exemplo funcional + tratamento de erros + sem dependências externas |

**Testado em Windows** (dois itens, na seção 4.14): além do teste automatizado, o item foi verificado num Windows real. O registro é de um trabalho anterior a esta revisão e ainda não foi repetido pelo CI.

> **Ressalva válida para toda a tabela:** "teste passando" significa execução sob Wine 9 (96 e 144 DPI), não em Windows real. Os itens cuja verificação depende do Windows de verdade estão marcados com ⚠ e detalhados em [COMPATIBILITY.md](COMPATIBILITY.md). Nenhum item usa crates externos.

## 4.1 Gerenciamento de janelas

| Recurso | Estado | Observações |
|---|---|---|
| Criação/destruição, principais e secundárias | Concluído | `tests/lifecycle.rs`, exemplos 1 e 18 |
| Modais e não modais | Concluído | `Window::show_modal`, `Dialog` |
| Janelas filhas / pai e filho / dono | Concluído | `Panel`, `WindowBuilder::owner` |
| Sem bordas, redimensionável/fixa, sempre visível | Concluído | exemplo 18 |
| Transparência (opacidade da janela) | Concluído | `set_opacity`; sem transparência por pixel |
| Tela cheia, maximizar, minimizar, centralizar | Concluído | |
| Múltiplos monitores | Documentado ⚠ | `Monitor`, `move_to_monitor`; testado com um monitor |
| Ícones de aplicação e de janela | Concluído | `Icon::system`, `Icon::from_file` |
| Logo padrão das janelas | Testado | `icons::DEFAULT_WINDOW_ICON`, disponível sem o catálogo opcional; janelas e diálogos próprios; ícone explícito no builder ou em `set_icon` prevalece. Três testes nativos em Windows x64 em `tests/default_window_icon.rs`, repetidos com e sem `icons`; DPI simulado, sem troca real de monitor |
| Barra de título, menu de sistema | Concluído | `system_menu()` |
| Área de cliente personalizada | Concluído | `on_paint`, `on_raw_message` |
| Fechamento com veto, eventos de tamanho/movimento | Concluído | |
| Escalas de DPI | Concluído | suíte inteira repetida em 144 DPI |
| Mudança de DPI em execução | Documentado ⚠ | `WM_DPICHANGED` simulado (`tests/window_maximize.rs`, `tests/system_events.rs`): retângulo sugerido, reescala, ícone da janela e `on_dpi_changed`; a troca real de DPI entre monitores não foi vista |
| Foco, atalhos de teclado | Concluído | |
| Arrastar arquivos para a janela | Documentado ⚠ | `on_drop_files`; `WM_DROPFILES` com um `HDROP` montado como o Explorer monta (`tests/system_events.rs`); arrastar de verdade a partir do Explorer não foi visto |
| Bordas, sombras e efeitos do DWM | Documentado ⚠ | efeito visual não observado |

## 4.2 Componentes básicos

| Componente | Estado | Observações |
|---|---|---|
| Button, Label, TextBox, PasswordBox | Concluído | |
| CheckBox (2 e 3 estados), RadioButton, ToggleButton | Concluído | |
| ComboBox, ListBox, GroupBox | Concluído | |
| ProgressBar, Slider (TrackBar), ScrollBar | Concluído | |
| SpinButton, UpDown | Concluído | |
| DatePicker, TimePicker, MonthCalendar | Concluído | |
| RichTextBox | Concluído ⚠ | formatação de caractere/parágrafo, RTF, busca; listas (`ListStyle`), tabelas e imagens (EMF) inseridas no cursor, `insert_rtf` — listas e tabelas exibidas pelo RichEdit do Wine (exemplo `styled_text`); a imagem entra no RTF (conferido) mas o RichEdit do Wine não a desenha |
| Hyperlink | Documentado | clique do mouse (`NM_CLICK`) e Enter (`NM_RETURN`) simulados, com o endereço do link (`tests/system_events.rs`) |
| Tooltip | Documentado | criação/remoção testadas; aparece sozinho quando o ponteiro pousa no controle (`tests/system_events.rs`) |
| StatusBar, ToolBar | Concluído | |
| CommandBar (barra de comandos desenhada) | Testado ⚠ | botões de ícone/texto, alternância, menu `▾`, dividido, separadores, espaço flexível, rótulos, busca, controles hospedados (também no `»`), dicas por botão, teclado, reordenar arrastando, caixa de personalização (`customize`, `CommandBarLayout`); validada no Wine (96 e 144 DPI), não em Windows real |
| ToggleSwitch, NumberBox, RatingControl, Card (próprios) | Testado ⚠ | interruptor animado; caixa numérica com limites, passo e expressões (`2*3+1`); estrelas com meias notas; cartão clicável; teclado, tema e UIA (Toggle, RangeValue, Value, Invoke) verificados no Wine (`tests/modern_controls.rs`) |
| Expander, BreadcrumbBar (próprios) | Testado ⚠ | altura animada e ExpandCollapse; trilha com recolhimento em "…" e menu; mensagens sintéticas no Wine |

## 4.3 Componentes avançados

| Componente | Estado | Observações |
|---|---|---|
| ListView (colunas, ordenação, seleção múltipla, caixas, imagens) | Concluído | |
| TreeView | Concluído | |
| TabControl | Concluído | |
| Modal no estilo da web (`Modal`) | Testado ⚠ | fundo em camadas (`UpdateLayeredWindow`, transparência por pixel e sombra suave) sobre a área cliente da dona + cartão nativo centralizado; dona bloqueada, Esc/Enter/✕/clique no fundo, foco preso e devolvido, validação antes de fechar, `show` bloqueante e `show_then`, acompanha a dona, animação, tema escuro; no Wine o fundo é uma foto escurecida (sem compositor) |
| Abas destacáveis | Testado ⚠ | `set_detachable`, `detach_tab`/`attach_tab`: arrastar a aba para fora a move para uma janela própria; fechar a janela a devolve; arrasto com mensagens sintéticas no Wine |
| Painéis acopláveis (`DockHost`, `DockPane`) | Testado ⚠ | esquerda/direita/embaixo, divisórias arrastáveis, flutuar em janela própria, acoplar, fechar e reabrir; sem soltar arrastando sobre alvos de acoplamento |
| Gráficos (`Chart`) | Testado ⚠ | linha, área, barras, barras empilhadas e pizza; eixo com passos redondos, legenda, realce e dica, clique, transição animada, tema; cores conferidas por pixels no Wine |
| Pré-visualização de impressão (`PrintPreview`, `PrintPreviewDialog`) | Testado ⚠ | páginas de um `PrintDocument` em qualquer escala, navegação, zoom, cache; o botão Imprimir usa o intervalo escolhido — impressão em si não exercida (sem impressora) |
| HeaderControl | Documentado | usado nos testes; sem exemplo dedicado |
| Rebar, Pager | Testado | criação e operações básicas; cobertura do essencial |
| HotKeyBox, IpAddressBox | Documentado | presentes no exemplo `demo` |
| Animation | Em desenvolvimento ⚠ | só o caminho de erro foi testado (falta um AVI real) |
| SplitContainer (próprio) | Concluído | arraste simulado nos testes |
| DataGrid (próprio): virtualização, ordenação, seleção múltipla, edição, colunas redimensionáveis | Concluído | 1 000 000 de linhas virtuais no teste; edição real verificada no `demo` |
| DataGrid: estilo por célula/linha, coluna de caixas de seleção, edição virtual (`set_virtual_editor`), editor que acompanha a rolagem, filtros, agrupamento recolhível, exportação CSV/TSV/HTML | Testado ⚠ | `tests/data_grid.rs`; validado no Wine |
| PropertyGrid (próprio) | Concluído | lista suspensa para `Choice` (`set_choice_mode`), rótulos de Sim/Não configuráveis, seleção acessível |
| Imagens associadas a itens | Concluído | `ImageList` em ListView/TreeView/ToolBar |
| Menus contextuais em controles | Documentado | `set_context_menu`; `WM_CONTEXTMENU` abre o menu do Windows (item escolhido pelo teclado no laço modal) e o desenhado, também pela tecla de menu (`tests/system_events.rs`) |

## 4.4 Menus

| Recurso | Estado | Observações |
|---|---|---|
| MenuBar, MenuItem, SubMenu, separadores | Concluído | |
| Itens habilitados/desabilitados e marcáveis | Concluído | |
| Atalhos de teclado | Concluído | itens acrescentados depois de `set_menu` também respondem |
| Menus dinâmicos | Concluído | `remove`, `clear` |
| Menus com ícones | Testado | `MenuItem::set_icon` |
| ContextMenu | Documentado | menu do Windows ou desenhado (`set_drawn`) |
| Eventos de seleção | Concluído | clique no item e `Menu::on_highlight_changed` (item destacado pelo mouse ou teclado, também na barra nativa) |
| Menu de sistema | Concluído | |
| Itens de opção (grupos) | Testado | `add_radio_item` |
| Menu preenchido na abertura | Testado | `Menu::on_open` (barra nativa, `MenuStrip` e contexto) |
| Barra desenhada (`MenuStrip`) | Testado ⚠ | área não cliente, tema claro/escuro/alto contraste, ícones por DPI, submenus, rolagem, `»`; validada no Wine (96 e 144 DPI), não em Windows real |
| Teclado na `MenuStrip` | Testado | Alt, F10, Alt+letra, setas, Home/End, Enter, Esc, mnemônicos; atalhos respeitam item desabilitado |

## 4.5 Diálogos

| Diálogo | Estado | Observações |
|---|---|---|
| MessageBox | Concluído | título, texto, dona bloqueada e respostas Sim/Não verificados (`tests/dialogs.rs`) |
| OpenFileDialog, SaveFileDialog, FolderPicker | Testado ⚠ | `IFileDialog`; `tests/dialogs.rs` abre, confere o título e a dona bloqueada, confirma (caminho do arquivo existente / nome sugerido na pasta pedida) e cancela (`None`); visual não conferido em Windows |
| ColorDialog, FontDialog | Testado ⚠ | `tests/dialogs.rs`: OK devolve a cor inicial e a fonte inicial (família, tamanho e cor), cancelar devolve `None` |
| PrintDialog, PageSetupDialog | Testado ⚠ | `show_with(PrintOptions)`: intervalo de páginas, seleção, cópias (campos do `PRINTDLGW` testados); `tests/dialogs.rs` confere que abrir e cancelar volta sem travar; sem impressora no ambiente, o resultado com impressora não foi visto |
| FindDialog / Replace (não modal) | Testado ⚠ | `tests/dialogs.rs`: não bloqueia a dona, mostra o texto inicial, "Localizar próxima" chega como `FindEvent`, `close` fecha |
| TaskDialog | Testado ⚠ | `tests/dialogs.rs`: título, botão personalizado (`TaskButton::Custom`) e caixa de verificação |
| Diálogos personalizados, modais e assíncronos | Concluído | `Dialog` |

## 4.6 Layouts

| Recurso | Estado |
|---|---|
| StackLayout vertical/horizontal | Concluído |
| GridLayout (fixo, automático, proporcional, spans) | Concluído |
| FlowLayout, DockLayout, AbsoluteLayout, AnchorLayout, SplitLayout | Concluído |
| Margens, padding, espaçamento, alinhamento | Concluído |
| Tamanhos mínimos/máximos, dimensionamento proporcional | Concluído |
| Responsivo ao tamanho da janela, atualização automática | Concluído |
| DPI por monitor | Concluído (fixo) / Implementado ⚠ (mudança em execução) |

## 4.7 Eventos

| Recurso | Estado |
|---|---|
| `on_click`, `on_double_click`, `on_mouse_move/enter/leave`, `on_key_down/up` | Concluído |
| `on_text_changed`, `on_focus`, `on_blur`, `on_resize`, `on_move`, `on_close_request` | Concluído |
| `on_selection_changed`, `on_value_changed` | Concluído |
| Convenção de nomes dos eventos (presente / particípio / `_request`), nomes antigos como aliases `#[deprecated]` (`tests/api_compat.rs`) | Concluído |
| Enums públicos `#[non_exhaustive]` (59 de 63; exaustivos só `Orientation`, `CheckState`, `TreeDropPosition`) | Concluído |
| Despacho seguro (reentrância, destruição, panic) | Concluído |
| Agendamento na thread de UI, `async` sem runtime externo | Concluído |

## 4.8 Desenho

| Recurso | Estado | Observações |
|---|---|---|
| GDI: linhas, retângulos, elipses, polígonos, caminhos, gradientes, bitmaps, texto, fontes | Concluído | pixels verificados em teste |
| Regiões de recorte, transformações, double buffering | Concluído | |
| GDI+: antialiasing, transparência, imagens | Concluído | |
| Vetores em ponto flutuante (`Figure`, `Paint` sólido/linear/radial, `Stroke`, `Matrix`) | Testado ⚠ | `PlusGraphics::fill_figures`/`stroke_figures`; o GDI+ do Wine não suaviza caminhos dentro de imagens e ignora paradas intermediárias de gradientes radiais |
| Desenho de SVG (`SvgImage`) | Testado ⚠ | subconjunto do SVG 1.1 (formas, caminhos com arcos, gradientes, `use`, transformações, estilos e `<style>` simples); 39 SVG anteriores comparados visualmente com PNGs. O lote social acrescenta 40 fontes; BeReal, KakaoTalk e Lemon8 contêm texto não suportado. Sem texto, máscaras, recortes e filtros |
| Animações e transições (`motion`) | Testado ⚠ | curvas de suavização, `animate`, `animate_bounds`, `fade_window`, `show_animated`/`hide_animated` (`AnimateWindow`), respeita "Mostrar animações" do Windows |
| Direct2D: formas, caminhos, gradientes, transformações, recorte | Concluído | exemplo 16 renderizado |
| DirectWrite (texto) | Implementado ⚠ | chamadas confirmadas; sem fontes no ambiente |
| Seleção de backend | Concluído | três tipos independentes |
| Fontes personalizadas de arquivo | Testado | caminho de erro testado |

## 4.9 Aparência

| Recurso | Estado | Observações |
|---|---|---|
| Tema claro e escuro, cores e fontes personalizadas | Concluído | exemplo 12 |
| Detecção do tema do sistema e de mudanças | Documentado ⚠ | leitura testada; `WM_SETTINGCHANGE` com `ImmersiveColorSet`/`WindowsThemeElement` dispara `on_theme_changed`, outras áreas não (`tests/system_events.rs`); a troca real no Windows não foi vista |
| Estilos reutilizáveis, escala de texto | Concluído | |
| Estados hover/pressed/focused/disabled | Concluído | `ButtonStyle` (botões) |
| Ícones nativos | Concluído | |
| Visual moderno, cantos e sombras | Documentado ⚠ | depende de Windows 11 |
| Tema escuro em todos os controles | Testado ⚠ | abas, barra de status, cabeçalhos, grupo e bordas 3D pintados pela biblioteca; barra de ferramentas por `NM_CUSTOMDRAW`; calendário, calendário suspenso, progresso e rebar por mensagens de cor; DatePicker/TimePicker e Slider repintados; `MenuBar` nativa trocada pela `MenuStrip` com o tema (`set_menu_bar_follows_theme`); barras de rolagem por `DarkMode_Explorer` (só Windows 10 1809+; não observado no Wine) |
| Janela sem moldura com barra de título própria (`TitleBar`) | Testado ⚠ | `WM_NCCALCSIZE`/`WM_NCHITTEST` (bordas, arrastar, `HTMAXBUTTON` para os layouts do Windows 11), controles na barra, botões acessíveis; sombra do DWM e maximizar não observados (Wine/Xvfb) |

## 4.10 Integração com o Windows

| Recurso | Estado | Observações |
|---|---|---|
| Bandeja, ícones, notificações | Documentado ⚠ | exemplo 14 |
| Notificações do sistema (Central de Ações, `SystemNotification`) | Implementado ⚠ | WinRT à mão: título, texto, imagens, até 5 botões, campos de texto e seleção, barra de progresso atualizável, agendamento (`show_at`), grupos e etiquetas (`remove_group`, `clear_all`), cenários, ativador COM para cliques com o programa fechado (`NotificationActivator`); XML, IIDs e registro testados; o Wine não tem `Windows.UI.Notifications` (devolve `Unsupported`), exibição não observada |
| Avisos dentro da janela (`InfoBar`, `Toast`) | Testado ⚠ | quatro gravidades, ações, empilhamento, expiração com pausa, tema; validados no Wine (esmaecimento não observado: o Wine sem compositor não mostra janelas translúcidas) |
| Clipboard (texto, HTML, imagens, arquivos, formatos próprios) | Testado | vários formatos de uma vez; ida e volta verificada nos testes (pixels, HTML, `CF_HDROP`); imagens também como `CF_DIBV5` (alfa) e `PNG`; metarquivos (`Metafile`, `CF_ENHMETAFILE`); `ClipboardWatcher` |
| Drag and Drop (OLE) | Testado ⚠ | `start_drag`, `accept_drops`, `DropData`; reordenação em `ListBox`/`ListView`/`DataGrid` (também virtual)/`TreeView`/`CommandBar`, vários itens de uma vez, mover para fora (`DropEffect::Move`); validados no Wine com mouse real (exemplo 25). Imagem de arraste do shell e soltar em outros programas não observados |
| Abrir arquivos, associações, Explorador, pastas conhecidas | Testado | `system::shell` |
| Cursores padrão e personalizados | Testado | |
| Monitores, resolução, DPI | Concluído | |
| Informações de acessibilidade | Concluído | |
| Impressão | Implementado ⚠ | `PrintDocument` (A4/Carta, `with_page_setup`), `PrintRange`, `print_document` com intervalo e cópias, pré-visualização; impressão real não exercida |
| Teclado e mouse | Concluído | |
| Toque | Testado ⚠ | eventos de ponteiro e gestos (`gestures::on_gesture`): toque, toque duplo, manter, pinça, rotação, arrasto com inércia; `WM_GESTURE` no Windows 7; Ctrl+roda como pinça; reconhecedor testado com mensagens sintéticas (sem tela de toque) |
| IME | Testado ⚠ | lista de candidatos própria (`Ime::use_candidate_window`, `ImeCandidateWindow`, `on_candidates_changed`) com tema e anúncio; candidatos simulados — sem IME no ambiente |

## 4.11 Acessibilidade

| Recurso | Estado | Observações |
|---|---|---|
| Recursos nativos dos controles (UIA/MSAA) | Concluído | fornecidos pelo Windows |
| Navegação por teclado, tabulação, mnemônicos | Concluído | |
| Nomes e descrições acessíveis | Documentado ⚠ | `IAccPropServices`; não suportado pelo Wine |
| Provedor UIA para controles personalizados | Testado ⚠ | `set_accessible_model` (`AccessibleModel`, `AccessibleItem`): papel, estado e padrões Invoke, Toggle, ExpandCollapse, Value, RangeValue, Selection/SelectionItem, Scroll, Grid/GridItem e Text; `IRawElementProviderAdviseEvents`; `raise_accessible_event`; `set_custom_role` continua para o caso simples |
| Elementos UIA dos controles desenhados (`MenuStrip`, listas dos menus, `CommandBar`, `PropertyGrid`, `DataGrid`, controles modernos, `TitleBar`, `DockPane`) | Testado ⚠ | fragmentos com Invoke, Toggle, ExpandCollapse e Value, foco e eventos; verificados chamando os provedores e com o cliente `IUIAutomation` do Wine; não testados com Narrador/NVDA |
| Anúncios para leitores de tela | Documentado ⚠ | `announce`; `Toast`, `InfoBar` e a lista de candidatos do IME anunciam sozinhos; não ouvido com Narrador/NVDA |
| Alto contraste, escala de texto, indicadores de foco | Concluído | |

## 4.12 Internacionalização

| Recurso | Estado |
|---|---|
| Unicode, UTF-8 ↔ UTF-16 | Concluído |
| Formatação regional (datas, horas, números, moeda) | Concluído |
| Textos configuráveis (`Translations`) | Concluído |
| Textos internos da biblioteca traduzíveis (`LibraryText`, `set_library_language`, `apply_library_translations`) | Concluído |
| Direção de texto (RTL) | Testado (detecção) / Implementado (janela espelhada) |

## 4.13 Ícones

| Recurso | Estado | Observações |
|---|---|---|
| Pacote embutido (392 ícones em 22 categorias, ICO/PNG, 79 SVG), constantes, busca por nome/apelido, categorias | Concluído | `tests/icons.rs`, exemplo 21; todos os ícones carregados em 4 tamanhos e o PNG decodificado em teste |
| Redes sociais (58 desenhos) | Integrado | Atualização com bisel: 22 desenhos substituídos, 18 incluídos, com ICO/PNG 512/SVG; 18 desenhos anteriores preservados. Testes de catálogo e carregamento; origem, créditos e limites SVG em `assets/icons/social/ORIGEM.md` |
| Ícone pelo tipo de arquivo (`for_extension`, `for_path`; 66 extensões com desenho próprio, 92 extensões reconhecidas) | Concluído | teste unitário da tabela; conferido numa `ListView` a 16, 32 e 48 px no Wine |
| `Icon` a partir de `.ico` em memória, versão desabilitada, cache por DPI | Concluído | |
| PNG em memória (`PlusImage::from_bytes`, `to_icon`) | Concluído | |
| Botão com ícone | Concluído ⚠ | ícone com 60% da altura e grupo ícone + texto centralizado, desenhado sobre o botão nativo no `NM_CUSTOMDRAW` (v6); sem a v6 o botão é desenhado pela biblioteca; os dois caminhos testados sob Wine |
| Altura padrão única e texto centralizado | Concluído ⚠ | `DEFAULT_CONTROL_HEIGHT` em botões, caixas de combinação, campos, seletores e caixas de seleção; faixas não cliente centralizam o texto de uma linha |
| Rótulo com ícone (`SS_OWNERDRAW`) | Concluído | pixels verificados em teste |
| `PictureBox` (ícone/imagem redimensionada) | Concluído | |
| Árvore, lista, grade (memória e virtual), abas | Concluído | listas de imagens gerenciadas, uma posição por origem |
| ComboBox / ListBox com ícones (desenho próprio) | Concluído | exigem `with_icons(true)` na criação |
| Barra de ferramentas, barra de status, menus, janela, bandeja, TaskDialog | Concluído ⚠ | bandeja e TaskDialog com ícone próprio não observados visualmente |
| Recarregar ícones na mudança de DPI em execução | Documentado ⚠ | o ícone da janela é recolocado em `WM_DPICHANGED` (`tests/system_events.rs`); os ícones de controles e menus num DPI de fato diferente não foram vistos |
| Desenho de SVG | Testado ⚠ | `IconAsset::svg_image`, `PictureBox::set_svg`, `Canvas::draw_svg` (ver 4.8) |

## 4.14 Composição e texto formatado

| Recurso | Estado | Detalhes |
|---|---|---|
| Texto formatado por trechos | Documentado | StyledText: Unicode, fonte, cor, destaque e estilos; testes de serialização e RichEdit nativo; exemplo styled_text. |
| Redesenho de contêineres | Implementado | invalidate inclui descendentes e apagamento de fundo; pintura assíncrona. |
| Rodapé e altura por conteúdo | Testado em Windows | StatusBarLayout, medição nativa e fit_height; preserva maximização, minimização e tela cheia; testes composition e window_maximize (144 DPI). |
| Viewport com rolagem vertical | Documentado | ScrollViewport: limites, medição responsiva, foco e vida útil; testes composition. |
| Formatação por posição Unicode | Documentado | StyledText::push_mapped; agrupamento de estilos adjacentes e teste Unicode. |
| Estilos visuais por intervalos (`RichTextBox::apply_highlights`) | Testado em Windows | Preserva texto, seleção, rolagem, indicador de modificação e histórico de desfazer; não emite eventos de edição; intervalos UTF-16 e API independente de linguagem |

## 4.15 Conteúdo de botões

| Recurso | Estado | Detalhes |
|---|---|---|
| Alinhamento horizontal do conjunto ícone/texto | Testado | `content_alignment`, `set_content_alignment`; esquerda/centro/direita, margem interna e truncamento; desenho nativo e personalizado |

## Qualidade

| Item | Estado |
|---|---|
| Zero dependências de terceiros (`Cargo.lock` só com `arqrustui` e as crates próprias do pacote de ícones) | Concluído |
| Pacotes para o crates.io: `arqrustui` + `arqrustui-icons` + 8 crates de dados (limite de 10 MiB por pacote) | Verificação local | Dez pacotes; a importação social acrescentou a parte 8. Publicação não executada; ver resultado da atualização em `docs/REDES_SOCIAIS_BEVEL.md` |
| Documentação no docs.rs (`[package.metadata.docs.rs]`, alvo `x86_64-pc-windows-msvc`) | Concluído — gerada sem avisos com esse alvo em Linux, como faz o docs.rs |
| CI (GitHub Actions) e `CHANGELOG.md` | Implementado ⚠ — workflow validado como YAML e com o passo de tamanho dos pacotes executado localmente; ainda não rodou no GitHub (o repositório não tem remoto) |
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets` | limpos |
| Testes | 145 unitários + 176 de integração + 60 doctests; sob Wine 9/Xvfb passam todos menos `window_maximize` (precisa de um gerenciador de janelas que maximize), em 64 e em 32 bits |
| Alvos | x64 (MSVC e GNU) e x86/i686 (suíte inteira no Wine de 32 bits); ARM64 só compila |
| Blocos `unsafe` com `// SAFETY:` | todos |
| Nenhum pânico atravessa o Windows (`catch_unwind` em todas as 128 funções `extern` chamadas pelo sistema) | Concluído — verificado por teste que varre o código |
| Validação em Windows real | **Parcial** — dois itens registrados como testados em Windows (seção 4.14); o restante só sob Wine. O CI (`.github/workflows/ci.yml`) roda a suíte em Windows quando o repositório for publicado no GitHub |
