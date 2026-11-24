# Gerenciamento de memória e segurança

*English: [MEMORY_AND_SAFETY.en.md](MEMORY_AND_SAFETY.en.md).*

## Princípios

1. **`unsafe` isolado**: declarações em `ffi/`; padrões de chamada em `core/native.rs`; o restante usa blocos `unsafe` pequenos, cada um com um comentário `// SAFETY:` explicando por que a chamada é válida.
2. **Funções públicas seguras**: as únicas `pub unsafe fn` são utilitários de baixo nível (`from_wide_ptr`, `GdiObject::from_raw`, `ComPtr::from_raw`/`method`, `Library::symbol`), todas com seção `# Safety`.
3. **Sem ponteiros para estado Rust guardados no Windows**: a associação janela → estado é um registro `HashMap<HWND, Rc<WidgetCore>>` por thread. Ponteiros só cruzam a fronteira durante chamadas síncronas (contextos de callbacks de ordenação, enumeração e *stream*).
4. **Afinidade de thread no sistema de tipos**: todo tipo visual contém `Rc`/ponteiro bruto e portanto é `!Send + !Sync`. `UiDispatcher`, `GuiError`, `Font` e `Theme` são os tipos pensados para cruzar threads. Há testes `compile_fail` garantindo isso.

## Ownership de handles — uma regra por tipo

| Handle | Wrapper | Quem libera | Como |
|---|---|---|---|
| `HWND` | `WidgetCore` (registro) | o sistema de janelas (pai destrói filhos) ou `destroy()` | `DestroyWindow`; o núcleo se desliga em `WM_NCDESTROY` |
| `HFONT`, `HBRUSH`, `HPEN`, `HBITMAP`, `HRGN` | `GdiObject`, `Bitmap` | o wrapper | `DeleteObject` no `Drop` |
| `HFONT` de controles | cache `(Font, dpi) → Weak<GdiObject>` | último controle que a usa | `Rc` compartilhado |
| `HICON`/`HCURSOR` de sistema | `Icon`/`Cursor` (não próprios) | ninguém (recurso compartilhado) | — |
| `HICON`/`HCURSOR` de arquivo | `Icon`/`Cursor` (próprios) | último clone | `DestroyIcon`/`DestroyCursor` |
| `HDC` de `BeginPaint` | interno em `WM_PAINT` | a biblioteca | `EndPaint` |
| `HDC` de memória | `MemoryDc` | o wrapper | `DeleteDC` |
| `HDC` de janela/tela | `WindowDc` | o wrapper | `ReleaseDC` |
| `HDC` de impressora | `PrinterContext` | o wrapper | `DeleteDC` |
| `HMENU` | `Menu`/`MenuBar` | o wrapper **até** ser anexado; depois, o Windows | `DestroyMenu` só se não anexado (`system_owned`) |
| `HIMAGELIST` | `ImageList` (`Rc`) | último usuário; controles guardam um `Rc` | `ImageList_Destroy` |
| `HGLOBAL` | `GlobalMemory` | o wrapper, ou o clipboard após `SetClipboardData`, ou o objeto de dados OLE após `SetData` | `GlobalFree` / `into_raw` |
| Interfaces COM | `ComPtr` | cada referência | `Release` no `Drop`, `AddRef` no `Clone` |
| Strings `CoTaskMem` | lidas e liberadas no mesmo bloco | a função que as recebeu | `CoTaskMemFree` |
| `HICON` criado de bytes (`.ico`, PNG, versão desabilitada) | `Icon` (próprio) | último clone; os ícones do pacote ficam no cache por thread até o fim dela | `DestroyIcon` |
| `HIMAGELIST` gerenciada (ícones por item, botões) | `ManagedImages` / `WidgetIcon` no `WidgetCore` | o núcleo do controle, solta em `WM_NCDESTROY` | `ImageList_Destroy` (`ListView` e `DataGrid` usam `LVS_SHAREIMAGELISTS` para não destruí-la também) |
| `IStream` de imagem em memória | `PlusImage` | a imagem, pois o GDI+ lê o fluxo enquanto ela existir | `Release` |

## Garantias

| Risco | Como é evitado |
|---|---|
| Use-after-free de janela | `HWND` anulado em `WM_NCDESTROY`; métodos em handles inertes são no-ops ou devolvem `WidgetDestroyed` |
| Double-destroy | `destroy()` verifica o `HWND`; o registro é removido uma única vez |
| Callback após destruição | manipuladores são soltos em `WM_NCDESTROY`; mensagens tardias caem em `DefWindowProcW` |
| Panic atravessando FFI | Toda função que o Windows chama diretamente (procedimentos de janela, os 100 métodos COM de UI Automation, `IDataObject`, `IDropSource`/`IDropTarget` e ativador de notificações, e os callbacks de ordenação, fluxo do RichEdit e enumeração) roda o corpo em `core::ffi_guard::guard`/`guard_hresult` (ou num `catch_unwind` próprio); o pânico é guardado e relançado pelo loop de eventos, e o Windows recebe `E_UNEXPECTED` (COM) ou um valor neutro. O teste `every_system_entry_point_is_guarded` varre `src/` e falha se uma função `extern` com corpo ficar sem proteção |
| Reentrância (`RefCell` já emprestado) | listas de manipuladores são clonadas antes de chamar; nenhum `borrow` fica ativo durante callbacks |
| Vazamento por ciclos `Rc` | todos os closures do núcleo são soltos na destruição; modelos de `DataGrid`/`PropertyGrid` soltam seus ganchos no evento `Destroyed` |
| Tarefas postadas após o fim da aplicação | `UiDispatcher::post` devolve `GuiError::ApplicationGone` |
| Estruturas FFI com layout errado | `tests/ffi_layout.rs` compara `size_of` de ~80 estruturas com os valores do Windows SDK |
| APIs ausentes em versões antigas | resolvidas por `GetProcAddress`; a ausência vira `GuiError::Unsupported` |

## Tratamento de erros

- Operações que podem falhar devolvem `GuiResult<T>`; `GuiError::Win32` carrega o código de `GetLastError` e a mensagem do sistema, `GuiError::HResult` carrega o `HRESULT`.
- Diálogos devolvem `Ok(None)` para "usuário cancelou" e `Err` para falha real — os dois casos nunca se confundem.
- *Setters* simples (`set_text`, `set_bounds`, `set_enabled`…) não devolvem `Result`: em um controle destruído são no-ops documentados, como no próprio Win32. Use `is_alive()` quando precisar distinguir.

## Testes relacionados

`tests/lifecycle.rs` cobre: destruição idempotente, handles inertes, destruição em cascata, `Drop` da aplicação, liberação de estado com ciclos de referência (via `Weak`), propagação de panic e contagem de objetos GDI/USER após 200 ciclos de criação e destruição.

`tests/icons.rs` carrega os 392 ícones em quatro tamanhos e decodifica todos os PNGs; `tests/ffi_layout.rs` confere `MEASUREITEMSTRUCT`, `BUTTON_IMAGELIST` e `ICONINFO`; o diretório de `.ico` é validado antes de qualquer ponteiro ser passado ao Windows (testes unitários com arquivos truncados).
