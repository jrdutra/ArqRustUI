# Memory management and safety

*Português: [MEMORY_AND_SAFETY.md](MEMORY_AND_SAFETY.md).*

## Principles

1. **Isolated `unsafe`**: declarations in `ffi/`; calling patterns in `core/native.rs`; the rest uses small `unsafe` blocks, each with a `// SAFETY:` comment explaining why the call is valid.
2. **Safe public functions**: the only `pub unsafe fn` are low-level utilities (`from_wide_ptr`, `GdiObject::from_raw`, `ComPtr::from_raw`/`method`, `Library::symbol`), all with a `# Safety` section.
3. **No pointers to Rust state stored in Windows**: the window → state association is a per-thread `HashMap<HWND, Rc<WidgetCore>>` registry. Pointers only cross the boundary during synchronous calls (contexts of sort, enumeration and *stream* callbacks).
4. **Thread affinity in the type system**: every visual type contains `Rc`/a raw pointer and is therefore `!Send + !Sync`. `UiDispatcher`, `GuiError`, `Font` and `Theme` are the types meant to cross threads. `compile_fail` tests guarantee this.

## Handle ownership — one rule per type

| Handle | Wrapper | Who releases it | How |
|---|---|---|---|
| `HWND` | `WidgetCore` (registry) | the window system (a parent destroys its children) or `destroy()` | `DestroyWindow`; the core detaches on `WM_NCDESTROY` |
| `HFONT`, `HBRUSH`, `HPEN`, `HBITMAP`, `HRGN` | `GdiObject`, `Bitmap` | the wrapper | `DeleteObject` in `Drop` |
| Controls' `HFONT` | cache `(Font, dpi) → Weak<GdiObject>` | the last control that uses it | shared `Rc` |
| System `HICON`/`HCURSOR` | `Icon`/`Cursor` (not owned) | nobody (shared resource) | — |
| `HICON`/`HCURSOR` from a file | `Icon`/`Cursor` (owned) | the last clone | `DestroyIcon`/`DestroyCursor` |
| `HDC` from `BeginPaint` | internal in `WM_PAINT` | the library | `EndPaint` |
| Memory `HDC` | `MemoryDc` | the wrapper | `DeleteDC` |
| Window/screen `HDC` | `WindowDc` | the wrapper | `ReleaseDC` |
| Printer `HDC` | `PrinterContext` | the wrapper | `DeleteDC` |
| `HMENU` | `Menu`/`MenuBar` | the wrapper **until** it is attached; after that, Windows | `DestroyMenu` only if not attached (`system_owned`) |
| `HIMAGELIST` | `ImageList` (`Rc`) | the last user; controls keep an `Rc` | `ImageList_Destroy` |
| `HGLOBAL` | `GlobalMemory` | the wrapper, or the clipboard after `SetClipboardData`, or the OLE data object after `SetData` | `GlobalFree` / `into_raw` |
| COM interfaces | `ComPtr` | each reference | `Release` in `Drop`, `AddRef` in `Clone` |
| `CoTaskMem` strings | read and freed in the same block | the function that received them | `CoTaskMemFree` |
| `HICON` created from bytes (`.ico`, PNG, disabled version) | `Icon` (owned) | the last clone; pack icons stay in the per-thread cache until the thread ends | `DestroyIcon` |
| Managed `HIMAGELIST` (per-item icons, buttons) | `ManagedImages` / `WidgetIcon` in `WidgetCore` | the control's core, released on `WM_NCDESTROY` | `ImageList_Destroy` (`ListView` and `DataGrid` use `LVS_SHAREIMAGELISTS` so they do not destroy it too) |
| In-memory image `IStream` | `PlusImage` | the image, since GDI+ reads the stream while it exists | `Release` |

## Guarantees

| Risk | How it is avoided |
|---|---|
| Window use-after-free | `HWND` cleared on `WM_NCDESTROY`; methods on inert handles are no-ops or return `WidgetDestroyed` |
| Double destroy | `destroy()` checks the `HWND`; the registry entry is removed only once |
| Callback after destruction | handlers are released on `WM_NCDESTROY`; late messages fall through to `DefWindowProcW` |
| Panic crossing FFI | Every function that Windows calls directly (window procedures, the 100 UI Automation COM methods, `IDataObject`, `IDropSource`/`IDropTarget` and the notification activator, and the sort, RichEdit stream and enumeration callbacks) runs its body inside `core::ffi_guard::guard`/`guard_hresult` (or its own `catch_unwind`); the panic is stored and re-raised by the event loop, and Windows receives `E_UNEXPECTED` (COM) or a neutral value. The `every_system_entry_point_is_guarded` test scans `src/` and fails if an `extern` function with a body is left unprotected |
| Reentrancy (`RefCell` already borrowed) | handler lists are cloned before calling; no `borrow` stays active during callbacks |
| Leaks through `Rc` cycles | all core closures are released on destruction; `DataGrid`/`PropertyGrid` models release their hooks on the `Destroyed` event |
| Tasks posted after the application ends | `UiDispatcher::post` returns `GuiError::ApplicationGone` |
| FFI structures with a wrong layout | `tests/ffi_layout.rs` compares the `size_of` of ~80 structures with the Windows SDK values |
| APIs missing on older versions | resolved with `GetProcAddress`; their absence becomes `GuiError::Unsupported` |

## Error handling

- Operations that can fail return `GuiResult<T>`; `GuiError::Win32` carries the `GetLastError` code and the system message, `GuiError::HResult` carries the `HRESULT`.
- Dialogs return `Ok(None)` for "the user cancelled" and `Err` for a real failure — the two cases are never confused.
- Simple setters (`set_text`, `set_bounds`, `set_enabled`…) do not return `Result`: on a destroyed control they are documented no-ops, as in Win32 itself. Use `is_alive()` when you need to tell the difference.

## Related tests

`tests/lifecycle.rs` covers: idempotent destruction, inert handles, cascading destruction, the application's `Drop`, release of state with reference cycles (through `Weak`), panic propagation and the count of GDI/USER objects after 200 create/destroy cycles.

`tests/icons.rs` loads the 392 icons at four sizes and decodes every PNG; `tests/ffi_layout.rs` checks `MEASUREITEMSTRUCT`, `BUTTON_IMAGELIST` and `ICONINFO`; the `.ico` directory is validated before any pointer is passed to Windows (unit tests with truncated files).
