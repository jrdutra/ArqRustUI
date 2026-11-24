//! Modern file dialogs (`IFileDialog`): open, save and folder selection.

use crate::core::application as app;
use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult};
use crate::ffi::ole32::{CoCreateInstance, CoTaskMemFree, ComPtr, CLSCTX_INPROC_SERVER};
use crate::ffi::shell32::*;
use crate::ffi::{GUID, HRESULT, HWND};
use crate::utilities::strings::from_wide_ptr;
use crate::utilities::to_wide;
use std::ffi::c_void;
use std::path::{Path, PathBuf};

type Raw = *mut c_void;

// Índices de vtable conforme shobjidl_core.h.
const MODAL_SHOW: usize = 3;
const DIALOG_SET_FILE_TYPES: usize = 4;
const DIALOG_SET_OPTIONS: usize = 9;
const DIALOG_GET_OPTIONS: usize = 10;
const DIALOG_SET_FOLDER: usize = 12;
const DIALOG_SET_FILE_NAME: usize = 15;
const DIALOG_SET_TITLE: usize = 17;
const DIALOG_GET_RESULT: usize = 20;
const DIALOG_SET_DEFAULT_EXTENSION: usize = 22;
const OPEN_GET_RESULTS: usize = 27;
const ITEM_GET_DISPLAY_NAME: usize = 5;
const ARRAY_GET_COUNT: usize = 7;
const ARRAY_GET_ITEM_AT: usize = 8;
/// `HRESULT_FROM_WIN32(ERROR_CANCELLED)`: user closed the dialog.
const CANCELLED: HRESULT = 0x8007_04C7_u32 as i32;

#[derive(Clone, Debug, Default)]
struct Options {
    title: Option<String>,
    filters: Vec<(String, String)>,
    folder: Option<PathBuf>,
    file_name: Option<String>,
    default_extension: Option<String>,
}

fn item_path(item: &ComPtr) -> GuiResult<PathBuf> {
    let mut name: *mut u16 = std::ptr::null_mut();
    // SAFETY: IShellItem[5] é GetDisplayName(SIGDN, LPWSTR*); a string devolvida é
    // liberada com CoTaskMemFree.
    unsafe {
        let get: unsafe extern "system" fn(Raw, u32, *mut *mut u16) -> HRESULT = item.method(ITEM_GET_DISPLAY_NAME);
        let hr = get(item.as_ptr(), SIGDN_FILESYSPATH, &mut name);
        let path = from_wide_ptr(name);
        CoTaskMemFree(name as Raw);
        GuiError::check_hresult("IShellItem::GetDisplayName", hr)?;
        Ok(PathBuf::from(path))
    }
}

struct Dialog {
    com: ComPtr,
}

impl Dialog {
    fn create(clsid: &GUID, iid: &GUID, options: &Options, extra_flags: u32) -> GuiResult<Dialog> {
        app::require()?;
        let mut raw: Raw = std::ptr::null_mut();
        // SAFETY: CLSID/IID válidos e ponteiro de saída; a referência devolvida passa ao ComPtr.
        let com = unsafe {
            let hr = CoCreateInstance(clsid, std::ptr::null_mut(), CLSCTX_INPROC_SERVER, iid, &mut raw);
            GuiError::check_hresult("CoCreateInstance(FileDialog)", hr)?;
            ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("IFileDialog"))?
        };
        let dialog = Dialog { com };
        dialog.configure(options, extra_flags)?;
        Ok(dialog)
    }

    fn set_string(&self, index: usize, text: &str, context: &'static str) -> GuiResult<()> {
        let wide = to_wide(text);
        // SAFETY: os métodos SetTitle/SetFileName/SetDefaultExtension de IFileDialog
        // recebem um único LPCWSTR.
        let hr = unsafe {
            let set: unsafe extern "system" fn(Raw, *const u16) -> HRESULT = self.com.method(index);
            set(self.com.as_ptr(), wide.as_ptr())
        };
        GuiError::check_hresult(context, hr)
    }

    fn configure(&self, options: &Options, extra_flags: u32) -> GuiResult<()> {
        // SAFETY: IFileDialog[10] GetOptions(DWORD*) e [9] SetOptions(DWORD).
        unsafe {
            let mut flags = 0u32;
            let get: unsafe extern "system" fn(Raw, *mut u32) -> HRESULT = self.com.method(DIALOG_GET_OPTIONS);
            GuiError::check_hresult("IFileDialog::GetOptions", get(self.com.as_ptr(), &mut flags))?;
            let set: unsafe extern "system" fn(Raw, u32) -> HRESULT = self.com.method(DIALOG_SET_OPTIONS);
            let hr = set(self.com.as_ptr(), flags | FOS_FORCEFILESYSTEM | extra_flags);
            GuiError::check_hresult("IFileDialog::SetOptions", hr)?;
        }
        if let Some(title) = &options.title {
            self.set_string(DIALOG_SET_TITLE, title, "IFileDialog::SetTitle")?;
        }
        if let Some(name) = &options.file_name {
            self.set_string(DIALOG_SET_FILE_NAME, name, "IFileDialog::SetFileName")?;
        }
        if let Some(extension) = &options.default_extension {
            self.set_string(
                DIALOG_SET_DEFAULT_EXTENSION,
                extension.trim_start_matches('.'),
                "IFileDialog::SetDefaultExtension",
            )?;
        }
        if !options.filters.is_empty() {
            let wide: Vec<(Vec<u16>, Vec<u16>)> =
                options.filters.iter().map(|(name, pattern)| (to_wide(name), to_wide(pattern))).collect();
            let specs: Vec<COMDLG_FILTERSPEC> = wide
                .iter()
                .map(|(name, pattern)| COMDLG_FILTERSPEC { pszName: name.as_ptr(), pszSpec: pattern.as_ptr() })
                .collect();
            // SAFETY: IFileDialog[4] é SetFileTypes(UINT, const COMDLG_FILTERSPEC*); o array
            // e as strings vivem até o fim da chamada (o diálogo copia os dados).
            let hr = unsafe {
                let set: unsafe extern "system" fn(Raw, u32, *const COMDLG_FILTERSPEC) -> HRESULT =
                    self.com.method(DIALOG_SET_FILE_TYPES);
                set(self.com.as_ptr(), specs.len() as u32, specs.as_ptr())
            };
            GuiError::check_hresult("IFileDialog::SetFileTypes", hr)?;
        }
        if let Some(folder) = &options.folder {
            let wide = to_wide(&folder.to_string_lossy());
            let mut item: Raw = std::ptr::null_mut();
            // SAFETY: string terminada em NUL, IID e ponteiro de saída válidos;
            // IFileDialog[12] é SetFolder(IShellItem*).
            unsafe {
                let hr = SHCreateItemFromParsingName(wide.as_ptr(), std::ptr::null_mut(), &IID_IShellItem, &mut item);
                // Uma pasta inicial inexistente não deve impedir o diálogo de abrir.
                if hr >= 0 {
                    if let Some(item) = ComPtr::from_raw(item) {
                        let set: unsafe extern "system" fn(Raw, Raw) -> HRESULT = self.com.method(DIALOG_SET_FOLDER);
                        set(self.com.as_ptr(), item.as_ptr());
                    }
                }
            }
        }
        Ok(())
    }

    /// Shows dialog; `Ok(false)` means canceled.
    fn show(&self, owner: HWND) -> GuiResult<bool> {
        // SAFETY: IModalWindow[3] é Show(HWND).
        let hr = unsafe {
            let show: unsafe extern "system" fn(Raw, HWND) -> HRESULT = self.com.method(MODAL_SHOW);
            show(self.com.as_ptr(), owner)
        };
        if hr == CANCELLED {
            return Ok(false);
        }
        GuiError::check_hresult("IFileDialog::Show", hr)?;
        Ok(true)
    }

    fn result(&self) -> GuiResult<PathBuf> {
        let mut raw: Raw = std::ptr::null_mut();
        // SAFETY: IFileDialog[20] é GetResult(IShellItem**).
        let item = unsafe {
            let get: unsafe extern "system" fn(Raw, *mut Raw) -> HRESULT = self.com.method(DIALOG_GET_RESULT);
            GuiError::check_hresult("IFileDialog::GetResult", get(self.com.as_ptr(), &mut raw))?;
            ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("IShellItem"))?
        };
        item_path(&item)
    }

    fn results(&self) -> GuiResult<Vec<PathBuf>> {
        let mut raw: Raw = std::ptr::null_mut();
        // SAFETY: IFileOpenDialog[27] é GetResults(IShellItemArray**); IShellItemArray[7]
        // é GetCount(DWORD*) e [8] é GetItemAt(DWORD, IShellItem**).
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut Raw) -> HRESULT = self.com.method(OPEN_GET_RESULTS);
            GuiError::check_hresult("IFileOpenDialog::GetResults", get(self.com.as_ptr(), &mut raw))?;
            let array = ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("IShellItemArray"))?;
            let mut count = 0u32;
            let get_count: unsafe extern "system" fn(Raw, *mut u32) -> HRESULT = array.method(ARRAY_GET_COUNT);
            GuiError::check_hresult("IShellItemArray::GetCount", get_count(array.as_ptr(), &mut count))?;
            let get_item: unsafe extern "system" fn(Raw, u32, *mut Raw) -> HRESULT = array.method(ARRAY_GET_ITEM_AT);
            let mut paths = Vec::with_capacity(count as usize);
            for index in 0..count {
                let mut item: Raw = std::ptr::null_mut();
                GuiError::check_hresult("IShellItemArray::GetItemAt", get_item(array.as_ptr(), index, &mut item))?;
                if let Some(item) = ComPtr::from_raw(item) {
                    paths.push(item_path(&item)?);
                }
            }
            Ok(paths)
        }
    }
}

macro_rules! common_options {
    () => {
        /// Dialog window title.
        pub fn title(mut self, title: &str) -> Self {
            self.options.title = Some(title.to_string());
            self
        }

        /// Initially displayed folder.
        pub fn folder(mut self, folder: impl AsRef<Path>) -> Self {
            self.options.folder = Some(folder.as_ref().to_path_buf());
            self
        }
    };
}

macro_rules! file_options {
    () => {
        common_options!();

        /// Adds a type filter, e.g. `("Images", "*.png;*.jpg")`.
        pub fn filter(mut self, name: &str, pattern: &str) -> Self {
            self.options.filters.push((name.to_string(), pattern.to_string()));
            self
        }

        /// Suggested filename.
        pub fn file_name(mut self, name: &str) -> Self {
            self.options.file_name = Some(name.to_string());
            self
        }
    };
}

/// Open file dialog.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// if let Some(path) = OpenFileDialog::new().filter("Text", "*.txt").filter("All", "*.*").show(window)? {
///     println!("chosen: {}", path.display());
/// }
/// # Ok(()) }
/// ```
#[derive(Clone, Debug, Default)]
pub struct OpenFileDialog {
    options: Options,
}

impl OpenFileDialog {
    /// Creates with default options.
    pub fn new() -> Self {
        Self::default()
    }

    file_options!();

    fn build(&self, flags: u32) -> GuiResult<Dialog> {
        Dialog::create(&CLSID_FileOpenDialog, &IID_IFileOpenDialog, &self.options, flags)
    }

    /// Selects one file; `Ok(None)` on cancellation.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<Option<PathBuf>> {
        let dialog = self.build(FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST)?;
        if !dialog.show(owner.core().hwnd())? {
            return Ok(None);
        }
        dialog.result().map(Some)
    }

    /// Selects multiple files; empty on cancellation.
    pub fn show_multiple(&self, owner: &impl Widget) -> GuiResult<Vec<PathBuf>> {
        let dialog = self.build(FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST | FOS_ALLOWMULTISELECT)?;
        if !dialog.show(owner.core().hwnd())? {
            return Ok(Vec::new());
        }
        dialog.results()
    }
}

/// Save As dialog.
#[derive(Clone, Debug, Default)]
pub struct SaveFileDialog {
    options: Options,
}

impl SaveFileDialog {
    /// Creates with default options.
    pub fn new() -> Self {
        Self::default()
    }

    file_options!();

    /// Extension added if omitted, e.g. `"txt"`.
    pub fn default_extension(mut self, extension: &str) -> Self {
        self.options.default_extension = Some(extension.to_string());
        self
    }

    /// Shows dialog; `Ok(None)` on cancellation. System asks before overwriting existing files.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<Option<PathBuf>> {
        let dialog = Dialog::create(&CLSID_FileSaveDialog, &IID_IFileSaveDialog, &self.options, FOS_OVERWRITEPROMPT)?;
        if !dialog.show(owner.core().hwnd())? {
            return Ok(None);
        }
        dialog.result().map(Some)
    }
}

/// Folder selection dialog.
#[derive(Clone, Debug, Default)]
pub struct FolderPicker {
    options: Options,
}

impl FolderPicker {
    /// Creates with default options.
    pub fn new() -> Self {
        Self::default()
    }

    common_options!();

    /// Shows dialog; `Ok(None)` on cancellation.
    pub fn show(&self, owner: &impl Widget) -> GuiResult<Option<PathBuf>> {
        let dialog = Dialog::create(
            &CLSID_FileOpenDialog,
            &IID_IFileOpenDialog,
            &self.options,
            FOS_PICKFOLDERS | FOS_PATHMUSTEXIST,
        )?;
        if !dialog.show(owner.core().hwnd())? {
            return Ok(None);
        }
        dialog.result().map(Some)
    }
}
