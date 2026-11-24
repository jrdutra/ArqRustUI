//! Windows shell: opening files, known folders and associations.

use crate::core::{GuiError, GuiResult};
use crate::ffi::ole32::CoTaskMemFree;
use crate::ffi::shell32::*;
use crate::ffi::user32::SW_SHOWNORMAL;
use crate::ffi::GUID;
use crate::utilities::strings::from_wide_ptr;
use crate::utilities::{from_wide, to_wide};
use std::path::{Path, PathBuf};

fn execute(verb: &str, target: &str, parameters: Option<&str>) -> GuiResult<()> {
    let verb = to_wide(verb);
    let target = to_wide(target);
    let parameters = parameters.map(to_wide);
    // SAFETY: strings terminadas em NUL válidas durante a chamada; ponteiros opcionais nulos.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            parameters.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    // ShellExecuteW devolve um valor maior que 32 em caso de sucesso.
    if result > 32 {
        Ok(())
    } else {
        Err(GuiError::from_win32("ShellExecuteW", result.max(0) as u32))
    }
}

/// Opens file/folder/URL in associated program.
pub fn open(target: &str) -> GuiResult<()> {
    execute("open", target, None)
}

/// Opens file with given verb (`"edit"`, `"print"`, `"runas"`, etc.).
pub fn open_with_verb(target: &str, verb: &str) -> GuiResult<()> {
    execute(verb, target, None)
}

/// Opens Explorer selecting the item.
pub fn show_in_explorer(path: impl AsRef<Path>) -> GuiResult<()> {
    let argument = format!("/select,\"{}\"", path.as_ref().display());
    execute("open", "explorer.exe", Some(&argument))
}

/// Known user folders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KnownFolder {
    /// Documents.
    Documents,
    /// Desktop.
    Desktop,
    /// Downloads.
    Downloads,
    /// Pictures.
    Pictures,
    /// User profile.
    Profile,
    /// `%APPDATA%`, roaming application data.
    RoamingAppData,
    /// `%LOCALAPPDATA%`.
    LocalAppData,
}

/// Current user's known folder path.
pub fn known_folder(folder: KnownFolder) -> GuiResult<PathBuf> {
    let id: &GUID = match folder {
        KnownFolder::Documents => &FOLDERID_Documents,
        KnownFolder::Desktop => &FOLDERID_Desktop,
        KnownFolder::Downloads => &FOLDERID_Downloads,
        KnownFolder::Pictures => &FOLDERID_Pictures,
        KnownFolder::Profile => &FOLDERID_Profile,
        KnownFolder::RoamingAppData => &FOLDERID_RoamingAppData,
        KnownFolder::LocalAppData => &FOLDERID_LocalAppData,
    };
    let mut raw = std::ptr::null_mut();
    // SAFETY: GUID válido e ponteiro de saída; a string devolvida é liberada com
    // CoTaskMemFree (inclusive em caso de falha, como manda a documentação).
    unsafe {
        let hr = SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut raw);
        let path = from_wide_ptr(raw);
        CoTaskMemFree(raw as *mut _);
        GuiError::check_hresult("SHGetKnownFolderPath", hr)?;
        Ok(PathBuf::from(path))
    }
}

fn association(extension: &str, kind: i32) -> Option<String> {
    let ext = if extension.starts_with('.') { extension.to_string() } else { format!(".{extension}") };
    let wide = to_wide(&ext);
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    // SAFETY: buffer com o tamanho informado em `size`.
    let hr = unsafe { AssocQueryStringW(0, kind, wide.as_ptr(), std::ptr::null(), buffer.as_mut_ptr(), &mut size) };
    (hr >= 0).then(|| from_wide(&buffer)).filter(|s| !s.is_empty())
}

/// Executable associated with extension, e.g. `"txt"`, if any.
pub fn associated_program(extension: &str) -> Option<PathBuf> {
    association(extension, ASSOCSTR_EXECUTABLE).map(PathBuf::from)
}

/// Friendly associated program name, if any.
pub fn associated_program_name(extension: &str) -> Option<String> {
    association(extension, ASSOCSTR_FRIENDLYAPPNAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_folders_resolve() {
        let profile = known_folder(KnownFolder::Profile).expect("perfil");
        assert!(!profile.as_os_str().is_empty());
        assert!(known_folder(KnownFolder::LocalAppData).is_ok());
    }

    #[test]
    fn unknown_extension_has_no_association() {
        assert_eq!(associated_program("extensao-que-nao-existe-arqrustui"), None);
    }
}
