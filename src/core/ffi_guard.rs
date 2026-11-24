//! Panic protection for Windows entry points.
//!
//! A panic must not cross an `extern "system"` function: undefined behavior through Rust 1.80, process abort from 1.81.
//! Every function directly called by Windows — COM methods (UI Automation, `IDataObject`, `IDropSource`/`IDropTarget`,
//! notification activator), window procedures and callbacks (sorting, RichEdit streaming, enumeration) — runs inside
//! [`guard`] or [`guard_hresult`].
//!
//! Captured panics are saved and resumed by the application message loop outside the system callback. On threads
//! without an application (e.g. notification activation COM threads), the panic hook already displayed the message and
//! the call simply returns failure.
//!
//! `every_system_entry_point_is_guarded` verifies every `extern` body outside `src/ffi` uses this protection.

use crate::core::application as app;
use crate::ffi::types::HRESULT;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// `E_UNEXPECTED`: returned to COM callers when the method panics.
pub(crate) const E_UNEXPECTED: HRESULT = 0x8000_FFFF_u32 as HRESULT;

/// Runs `f` without unwinding across FFI; on panic, saves it for the message loop and returns `fallback`.
pub(crate) fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => {
            app::store_panic(payload);
            fallback
        }
    }
}

/// COM version of [`guard`]: returns [`E_UNEXPECTED`] on panic.
pub(crate) fn guard_hresult(f: impl FnOnce() -> HRESULT) -> HRESULT {
    guard(E_UNEXPECTED, f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panics_become_the_fallback_value() {
        assert_eq!(guard(7, || 3), 3);
        assert_eq!(guard_hresult(|| 0), 0);
        let quiet = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let caught = guard(-1, || -> i32 { panic!("teste") });
        let hresult = guard_hresult(|| panic!("teste"));
        std::panic::set_hook(quiet);
        assert_eq!(caught, -1);
        assert_eq!(hresult, E_UNEXPECTED);
        assert_eq!(E_UNEXPECTED as u32, 0x8000_FFFF);
    }

    /// Every `extern "system"`/`extern "C"` body outside `src/ffi` is a Windows entry point and must use
    /// `guard`/`guard_hresult` or its own `catch_unwind`.
    #[test]
    fn every_system_entry_point_is_guarded() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        if !root.is_dir() {
            return; // código-fonte indisponível (não deve acontecer com `cargo test`)
        }
        let mut files = Vec::new();
        let mut pending = vec![root.clone()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    if path != root.join("ffi") {
                        pending.push(path);
                    }
                } else if path.extension().is_some_and(|e| e == "rs") {
                    files.push(path);
                }
            }
        }
        let mut checked = 0;
        let mut unguarded = Vec::new();
        for file in &files {
            let source = std::fs::read_to_string(file).unwrap();
            for (name, body) in extern_definitions(&source) {
                checked += 1;
                let inner = body.trim_start_matches('{').trim_start();
                let guarded = ["crate::core::ffi_guard::guard", "ffi_guard::guard", "guard(", "guard_hresult("]
                    .iter()
                    .any(|marker| inner.starts_with(marker))
                    || body.contains("catch_unwind(");
                if !guarded {
                    unguarded.push(format!("{}: {name}", file.display()));
                }
            }
        }
        assert!(checked >= 100, "a busca encontrou só {checked} entradas: o padrão mudou?");
        assert!(unguarded.is_empty(), "entradas sem proteção contra pânico:\n{}", unguarded.join("\n"));
    }

    /// `(name, body)` for every `extern "system"`/`"C"` function body in the source.
    fn extern_definitions(source: &str) -> Vec<(String, String)> {
        let mut found = Vec::new();
        let mut rest = source;
        while let Some(at) = ["extern \"system\" fn ", "extern \"C\" fn "]
            .iter()
            .filter_map(|marker| rest.find(marker).map(|i| (i, marker.len())))
            .min()
        {
            let (index, length) = at;
            let after = &rest[index + length..];
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            // Fim da lista de parâmetros, depois o `{` do corpo ou o `;` de uma declaração.
            let mut depth = 0;
            let mut end = 0;
            for (i, c) in after.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let tail = &after[end..];
            let brace = tail.find('{');
            let semicolon = tail.find(';');
            let is_type = name.is_empty();
            match (brace, semicolon) {
                (Some(b), s) if !is_type && s.map_or(true, |s| b < s) => {
                    let body_start = end + b;
                    let mut depth = 0;
                    let mut body_end = body_start;
                    for (i, c) in after[body_start..].char_indices() {
                        match c {
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    body_end = body_start + i + 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    found.push((name, after[body_start..body_end].to_string()));
                    rest = &after[body_end..];
                }
                _ => rest = &after[end.max(1)..],
            }
        }
        found
    }
}
