//! Embedded icon bundle and types for icons throughout the UI.
//!
//! 392 retro glossy icons, each in ICO (16, 24, 32, 48, 64, 128, 256 px), transparent PNG and sometimes SVG. Bytes from
//! own `arqrustui-icons` crate (collecting repository `crates/arqrustui-icons-data1` through `-data8`) are embedded
//! only when used.
//!
//! ```no_run
//! # #[cfg(feature = "icons")] {
//! use arqrustui::prelude::*;
//!
//! # fn demo(window: &Window) -> GuiResult<()> {
//! // Constants: compiler checks names and embeds only used icons.
//! let save = Button::builder().text("Save").icon(&icons::SAVE).parent(window).build()?;
//!
//! // Any icon-displaying control accepts an `IconAsset`.
//! let tree = TreeView::builder().parent(window).build()?;
//! let docs = tree.add_root("Documents")?;
//! tree.set_item_icon(docs, &icons::FOLDER)?;
//!
//! // Name/alias lookup embeds the entire catalog.
//! if let Some(icon) = icons::find("database") {
//!     window.set_icon(icon);
//! }
//! # Ok(()) }
//! # }
//! ```
//!
//! ## Format usage
//!
//! | Format | Use |
//! |---|---|
//! | `.ico` | Default [`Icon`] source: nearest size selected per control DPI. |
//! | `.png` | [`IconAsset::image`] returns [`PlusImage`] for high-resolution drawing, e.g. [`PictureBox`](crate::controls::PictureBox) above 256 px. |
//! | `.svg` | [`IconAsset::svg_image`] for scalable drawing; [`IconAsset::svg_bytes`] and [`IconAsset::save`]. |
//!
//! ## `icons` feature and executable size
//!
//! The `icons` feature is enabled by default. Files total about 66 MB; linked size depends on unused-data elimination:
//!
//! | Build | Embedded data |
//! |---|---|
//! | `release` with LTO (repository profile) | Only used icons. |
//! | `release` without LTO, MSVC (`/OPT:REF`) | Only used icons. |
//! | `release` without LTO, GNU (mingw) | Entire bundle. |
//! | `debug`, any toolchain | Entire bundle. |
//!
//! Disable the bundle with `arqrustui = { path = "…", default-features = false }`; [`IconAsset`]/[`IconSource`] remain
//! available for custom icons. Even with LTO, [`all`], [`find`] and [`in_category`] reference/embed all icons. Prefer
//! constants such as [`FOLDER`] for compile-time-known sets.
//!
//! ## Custom icons
//!
//! [`IconAsset::new`] creates an icon from `'static` bytes, e.g. `include_bytes!`, with bundled-icon behavior.

#[cfg(feature = "icons")]
mod catalog;

#[cfg(feature = "icons")]
pub use catalog::*;

use crate::core::handles::{ico_directory, Icon, IconSource};
use crate::core::{GuiError, GuiResult, Widget};
use crate::graphics::PlusImage;
use crate::utilities::{to_pixels, Size};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// Default window logo, available without `icons`.
///
/// [`WindowBuilder::icon`](crate::core::WindowBuilder::icon) and [`Window::set_icon`](crate::core::Window::set_icon)
/// override it. ICO contains 16, 24, 32, 48, 64, 96, 128, 256 px frames; PNG uses its 256 px frame without embedding
/// the optional catalog.
pub static DEFAULT_WINDOW_ICON: IconAsset = IconAsset {
    name: "arqrust_logo",
    category: None,
    aliases: &[],
    ico: include_bytes!("../../assets/branding/arqrust-logo.ico"),
    png: None,
    svg: None,
};

/// Thematic bundled icon category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IconCategory {
    /// Commands: add, save, copy, arrows, refresh, etc.
    Actions,
    /// Status/signaling: information, warning, favorites, flags, etc.
    Status,
    /// Files/folders.
    Files,
    /// File types by extension: documents, spreadsheets, images, audio, video, archives, executables and source (see
    /// [`for_extension`]).
    FileTypes,
    /// Office/productivity: calendar, notes, lists, tools, etc.
    Office,
    /// Communication: email, chat, contacts, notifications, etc.
    Communication,
    /// People/accessibility.
    People,
    /// Devices: computers, phones, printers, etc.
    Devices,
    /// Components/peripherals: storage, keyboard, battery, etc.
    Hardware,
    /// Network, internet and cloud.
    Network,
    /// Data, databases and charts.
    Data,
    /// Security: locks, keys, shields, certificates, etc.
    Security,
    /// Images, video, audio and physical media.
    Media,
    /// Design/colors.
    Design,
    /// Software development: code, terminal, API, debugging, etc.
    Development,
    /// Language/platform/tool logos.
    Technologies,
    /// Commerce: cart, payment, delivery, barcodes, etc.
    Commerce,
    /// Location/navigation.
    Navigation,
    /// Games/achievements.
    Entertainment,
    /// Social network and communication app logos.
    Social,
    /// Face emojis.
    Emoji,
    /// Hand gestures.
    Gestures,
}

impl IconCategory {
    /// All categories in display order.
    pub const ALL: [IconCategory; 22] = [
        IconCategory::Actions,
        IconCategory::Status,
        IconCategory::Files,
        IconCategory::FileTypes,
        IconCategory::Office,
        IconCategory::Communication,
        IconCategory::People,
        IconCategory::Devices,
        IconCategory::Hardware,
        IconCategory::Network,
        IconCategory::Data,
        IconCategory::Security,
        IconCategory::Media,
        IconCategory::Design,
        IconCategory::Development,
        IconCategory::Technologies,
        IconCategory::Commerce,
        IconCategory::Navigation,
        IconCategory::Entertainment,
        IconCategory::Social,
        IconCategory::Emoji,
        IconCategory::Gestures,
    ];

    /// Category display name in library language ([`i18n::set_library_language`](crate::i18n::set_library_language));
    /// override via [`i18n::set_library_text`](crate::i18n::set_library_text).
    pub fn label(self) -> String {
        crate::i18n::library_text(self.label_key())
    }

    /// Category name key in the internal text table.
    pub fn label_key(self) -> crate::i18n::LibraryText {
        use crate::i18n::LibraryText;
        match self {
            IconCategory::Actions => LibraryText::CategoryActions,
            IconCategory::Status => LibraryText::CategoryStatus,
            IconCategory::Files => LibraryText::CategoryFiles,
            IconCategory::FileTypes => LibraryText::CategoryFileTypes,
            IconCategory::Office => LibraryText::CategoryOffice,
            IconCategory::Communication => LibraryText::CategoryCommunication,
            IconCategory::People => LibraryText::CategoryPeople,
            IconCategory::Devices => LibraryText::CategoryDevices,
            IconCategory::Hardware => LibraryText::CategoryHardware,
            IconCategory::Network => LibraryText::CategoryNetwork,
            IconCategory::Data => LibraryText::CategoryData,
            IconCategory::Security => LibraryText::CategorySecurity,
            IconCategory::Media => LibraryText::CategoryMedia,
            IconCategory::Design => LibraryText::CategoryDesign,
            IconCategory::Development => LibraryText::CategoryDevelopment,
            IconCategory::Technologies => LibraryText::CategoryTechnologies,
            IconCategory::Commerce => LibraryText::CategoryCommerce,
            IconCategory::Navigation => LibraryText::CategoryNavigation,
            IconCategory::Entertainment => LibraryText::CategoryEntertainment,
            IconCategory::Social => LibraryText::CategorySocial,
            IconCategory::Emoji => LibraryText::CategoryEmoji,
            IconCategory::Gestures => LibraryText::CategoryGestures,
        }
    }
}

/// Multiresolution/multiformat icon embedded in the executable.
///
/// Cheap to copy (only `'static` references). Implements [`IconSource`] for any icon control, loading resolution at the
/// control's DPI.
#[derive(Clone, Copy)]
pub struct IconAsset {
    name: &'static str,
    category: Option<IconCategory>,
    aliases: &'static [&'static str],
    ico: &'static [u8],
    /// `None`: PNG is the ICO's largest PNG frame, stored once.
    png: Option<&'static [u8]>,
    svg: Option<&'static [u8]>,
}

impl std::fmt::Debug for IconAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconAsset")
            .field("name", &self.name)
            .field("category", &self.category)
            .field("ico", &self.ico.len())
            .field("png", &self.png_bytes().len())
            .field("svg", &self.svg.map(<[u8]>::len))
            .finish()
    }
}

impl PartialEq for IconAsset {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && std::ptr::eq(self.ico, other.ico)
            && std::ptr::eq(self.png_bytes(), other.png_bytes())
    }
}

impl Eq for IconAsset {}

impl IconAsset {
    /// Creates a custom icon from `'static` bytes, e.g. `include_bytes!`.
    ///
    /// `ico` is ICO content and `png` is PNG content; either may be empty but not both (error on loading). Optional
    /// `svg` supports arbitrary sizes through [`svg_image`](Self::svg_image).
    pub const fn new(name: &'static str, ico: &'static [u8], png: &'static [u8], svg: Option<&'static [u8]>) -> Self {
        IconAsset { name, category: None, aliases: &[], ico, png: Some(png), svg }
    }

    /// Bundled icon: `(.ico, .png, .svg)` bytes from `arqrustui-icons`.
    #[cfg(feature = "icons")]
    pub(crate) const fn builtin(
        name: &'static str,
        category: IconCategory,
        aliases: &'static [&'static str],
        bytes: (&'static [u8], Option<&'static [u8]>, Option<&'static [u8]>),
    ) -> Self {
        let (ico, png, svg) = bytes;
        IconAsset { name, category: Some(category), aliases, ico, png, svg }
    }

    /// Canonical lowercase name with underscore-separated words.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Category (`None` for [`IconAsset::new`] icons).
    pub fn category(&self) -> Option<IconCategory> {
        self.category
    }

    /// [`find`] aliases: synonyms and original bundle filenames.
    pub fn aliases(&self) -> &'static [&'static str] {
        self.aliases
    }

    /// ICO file content.
    pub fn ico_bytes(&self) -> &'static [u8] {
        self.ico
    }

    /// PNG file content.
    ///
    /// If bundled PNG equals the ICO's 256 px frame, stores it once and returns that ICO slice.
    pub fn png_bytes(&self) -> &'static [u8] {
        match self.png {
            Some(png) => png,
            None => largest_png_frame(self.ico),
        }
    }

    /// SVG file content, if available.
    pub fn svg_bytes(&self) -> Option<&'static [u8]> {
        self.svg
    }

    /// Parsed SVG ready for any size ([`SvgImage`](crate::graphics::SvgImage)); `None` without SVG.
    pub fn svg_image(&self) -> Option<GuiResult<crate::graphics::SvgImage>> {
        self.svg.map(crate::graphics::SvgImage::from_bytes)
    }

    /// Available ICO sizes in pixels, ascending.
    pub fn sizes(&self) -> Vec<u32> {
        let mut sizes: Vec<u32> =
            ico_directory(self.ico).map(|e| e.iter().map(|e| e.width).collect()).unwrap_or_default();
        sizes.sort_unstable();
        sizes.dedup();
        sizes
    }

    /// PNG dimensions, zero if invalid/absent.
    pub fn png_size(&self) -> Size {
        png_dimensions(self.png_bytes()).unwrap_or_default()
    }

    /// Icon at `size`×`size` physical pixels.
    ///
    /// Loaded icons are cached per thread; repeat requests for a size cheaply return the same `HICON`.
    pub fn icon(&self, size: i32) -> GuiResult<Icon> {
        if size <= 0 {
            return Err(GuiError::InvalidArgument("the icon size must be positive".into()));
        }
        let key = (self.source_key(), size);
        if let Some(icon) = CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
            return Ok(icon);
        }
        let icon = if !self.ico.is_empty() {
            Icon::from_ico_bytes(self.ico, size)?
        } else if !self.png_bytes().is_empty() {
            self.image()?.to_icon(size)?
        } else {
            return Err(GuiError::InvalidArgument(format!("icon `{}` has neither .ico nor .png", self.name)));
        };
        CACHE.with(|cache| cache.borrow_mut().insert(key, icon.clone()));
        Ok(icon)
    }

    /// Icon of `size` logical units at `widget`'s monitor resolution.
    pub fn icon_for(&self, widget: &impl Widget, size: i32) -> GuiResult<Icon> {
        self.icon(to_pixels(size, widget.dpi()))
    }

    /// Decodes PNG, falling back to ICO, for GDI+ drawing.
    pub fn image(&self) -> GuiResult<PlusImage> {
        let png = self.png_bytes();
        if !png.is_empty() {
            PlusImage::from_bytes(png)
        } else {
            PlusImage::from_bytes(self.ico)
        }
    }

    /// Saves to a file, format from `.ico`/`.png`/`.svg` extension.
    pub fn save(&self, path: impl AsRef<Path>) -> GuiResult<()> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let bytes = match extension.as_str() {
            "ico" => self.ico,
            "png" => self.png_bytes(),
            "svg" => self.svg.ok_or(GuiError::Unsupported("this icon has no SVG version"))?,
            _ => return Err(GuiError::InvalidArgument(format!("unsupported extension: {}", path.display()))),
        };
        if bytes.is_empty() {
            return Err(GuiError::InvalidArgument(format!("icon `{}` has no .{extension}", self.name)));
        }
        std::fs::write(path, bytes).map_err(|e| GuiError::InvalidArgument(format!("{}: {e}", path.display())))
    }

    #[cfg(feature = "icons")]
    fn matches(&self, normalized: &str) -> bool {
        self.name == normalized || self.aliases.contains(&normalized)
    }
}

impl IconSource for IconAsset {
    fn icon_pixels(&self, size: i32) -> GuiResult<Icon> {
        self.icon(size)
    }

    fn source_key(&self) -> usize {
        if self.ico.is_empty() {
            self.png_bytes().as_ptr() as usize
        } else {
            self.ico.as_ptr() as usize
        }
    }

    fn to_shared(&self) -> Rc<dyn IconSource> {
        Rc::new(*self)
    }

    fn large_image(&self) -> Option<GuiResult<PlusImage>> {
        (!self.png_bytes().is_empty()).then(|| self.image())
    }
}

/// Largest ICO PNG frame, empty if absent.
fn largest_png_frame(ico: &'static [u8]) -> &'static [u8] {
    ico_directory(ico)
        .ok()
        .and_then(|entries| entries.into_iter().filter(|e| e.png).max_by_key(|e| e.width))
        .map_or(&[], |e| &ico[e.offset..e.offset + e.length])
}

thread_local! {
    static CACHE: RefCell<HashMap<(usize, i32), Icon>> = RefCell::new(HashMap::new());
}

/// Normalizes lookup: lowercase; hyphens/spaces become underscores.
#[cfg(feature = "icons")]
fn normalize(name: &str) -> String {
    name.trim().to_lowercase().chars().map(|c| if c == '-' || c.is_whitespace() { '_' } else { c }).collect()
}

/// All bundled icons alphabetically by name.
///
/// Referencing this function embeds the entire bundle.
#[cfg(feature = "icons")]
pub fn all() -> &'static [&'static IconAsset] {
    &catalog::ALL
}

/// Case-insensitive canonical/alias lookup; hyphens/spaces equal underscores (`"zoom-in"` finds [`ZOOM_IN`]).
///
/// Aliases include original filenames such as `"05_banco_de_dados"`, including merged duplicate icons.
#[cfg(feature = "icons")]
pub fn find(name: &str) -> Option<&'static IconAsset> {
    let wanted = normalize(name);
    let all = all();
    all.iter().find(|icon| icon.name == wanted).or_else(|| all.iter().find(|icon| icon.matches(&wanted))).copied()
}

/// Extension icon, case-insensitive with optional dot: `"pdf"`, `".PDF"`, `"Pdf"` yield [`PDF_FILE`]. Variants share
/// icons (`"htm"` → [`HTML_FILE`], `"yml"` → [`YAML_FILE`], `"tif"` → [`TIFF_FILE`], `"mjs"` → [`JAVASCRIPT_FILE`]).
/// Unknown extensions yield `None`; use `.unwrap_or(&icons::FILE)` for a generic icon.
///
/// With LTO, embeds only file-type icons, not the whole bundle.
///
/// ```no_run
/// # #[cfg(feature = "icons")] {
/// use arqrustui::prelude::*;
///
/// assert_eq!(icons::for_extension("pdf").map(|i| i.name()), Some("pdf_file"));
/// let icon = icons::for_path("photos/beach.JPG").unwrap_or(&icons::FILE);
/// # let _ = icon;
/// # }
/// ```
#[cfg(feature = "icons")]
pub fn for_extension(extension: &str) -> Option<&'static IconAsset> {
    let wanted = extension.trim().trim_start_matches('.').to_ascii_lowercase();
    catalog::EXTENSIONS.iter().find(|(ext, _)| *ext == wanted).map(|(_, icon)| *icon)
}

/// Path icon by extension (`report.docx` → [`DOCX_FILE`], `backup.tar.gz` → [`GZ_FILE`]); see [`for_extension`].
#[cfg(feature = "icons")]
pub fn for_path(path: impl AsRef<Path>) -> Option<&'static IconAsset> {
    path.as_ref().extension().and_then(|ext| ext.to_str()).and_then(for_extension)
}

/// Category icons alphabetically.
#[cfg(feature = "icons")]
pub fn in_category(category: IconCategory) -> impl Iterator<Item = &'static IconAsset> {
    all().iter().copied().filter(move |icon| icon.category == Some(category))
}

/// Width/height from PNG `IHDR`.
pub(crate) fn png_dimensions(png: &[u8]) -> Option<Size> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if png.len() < 24 || png[..8] != SIGNATURE || &png[12..16] != b"IHDR" {
        return None;
    }
    let read = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
    Some(Size::new(read(16) as i32, read(20) as i32))
}

/// Chooses the best ICO frame for `size` pixels (internal testing API).
#[cfg(all(test, feature = "icons"))]
fn chosen_size(ico: &[u8], size: u32) -> Option<u32> {
    let entries = ico_directory(ico).ok()?;
    crate::core::handles::best_ico_entry(&entries, size).map(|entry| entry.width)
}

#[cfg(all(test, feature = "icons"))]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_names_and_aliases_are_unique_and_normalized() {
        let mut seen = HashSet::new();
        for icon in all() {
            assert_eq!(normalize(icon.name()), icon.name());
            assert!(seen.insert(icon.name()), "nome repetido: {}", icon.name());
        }
        for icon in all() {
            for alias in icon.aliases() {
                assert_eq!(&normalize(alias), alias);
                assert!(!seen.contains(alias), "apelido `{alias}` de `{}` colide com um nome", icon.name());
                assert_eq!(find(alias).map(|i| i.name()), Some(icon.name()), "apelido ambíguo: {alias}");
            }
        }
        assert_eq!(all().len(), 392);
        let mut sorted: Vec<_> = all().iter().map(|i| i.name()).collect();
        sorted.sort_unstable();
        assert!(all().iter().map(|i| i.name()).eq(sorted));
    }

    #[test]
    fn every_asset_has_all_ico_sizes_and_a_png() {
        for icon in all() {
            assert_eq!(icon.sizes(), vec![16, 24, 32, 48, 64, 128, 256], "{}", icon.name());
            let png = icon.png_size();
            assert!(png.width >= 256 && png.width == png.height, "{}: {png:?}", icon.name());
            if let Some(svg) = icon.svg_bytes() {
                assert!(std::str::from_utf8(svg).unwrap().contains("<svg"), "{}", icon.name());
            }
            assert!(icon.category().is_some());
        }
        assert_eq!(all().iter().filter(|i| i.svg_bytes().is_some()).count(), 79);
    }

    #[test]
    fn find_accepts_aliases_original_names_and_separators() {
        assert_eq!(find("FOLDER"), Some(&FOLDER));
        assert_eq!(find("zoom-in"), Some(&ZOOM_IN));
        assert_eq!(find("Zoom In"), Some(&ZOOM_IN));
        assert_eq!(find("05_banco_de_dados"), Some(&DATABASE));
        // nome de arquivo de um duplicado unificado
        assert_eq!(find("02_04_banco_dados"), Some(&DATABASE));
        assert_eq!(find("trash"), Some(&DELETE));
        assert_eq!(find("nao-existe"), None);
        assert_eq!(in_category(IconCategory::Security).count(), 9);
        assert_eq!(in_category(IconCategory::Social).count(), 58);
        assert_eq!(in_category(IconCategory::Emoji).count(), 20);
        assert_eq!(in_category(IconCategory::Gestures).count(), 22);
        // A variante social tem seu próprio desenho; os ícones anteriores continuam disponíveis.
        assert_eq!(find("31_threads_alternate"), Some(&THREADS_ALTERNATE));
        assert_eq!(find("29_robo"), Some(&ROBOT));
        assert_eq!(find("22_gitlab"), Some(&GITLAB_SOCIAL));
        assert_eq!(find("10_apontando_esquerda"), Some(&HAND_POINT_LEFT));
        // ícones defeituosos retirados não existem mais
        assert_eq!(find("jwt"), None);
        assert_eq!(find("route"), None);
        // retirados na auditoria de bordas cortadas; os arquivos defeituosos que tinham
        // uma duplicata íntegra continuam como apelido dela
        assert_eq!(find("ssl_certificate"), None);
        assert_eq!(find("vimeo"), Some(&VIMEO));
        assert_eq!(find("06_07_cadeado"), Some(&LOCK));
        assert_eq!(find("08_03_smartphone"), Some(&SMARTPHONE));
        assert_eq!(find("01_positivo"), Some(&HAND_THUMBS_UP));
        assert_eq!(find("29_mao_na_boca"), Some(&EMOJI_HAND_OVER_MOUTH));
        let total: usize = IconCategory::ALL.iter().map(|c| in_category(*c).count()).sum();
        assert_eq!(total, all().len());
    }

    #[test]
    fn file_type_icons_and_extensions() {
        assert_eq!(in_category(IconCategory::FileTypes).count(), 66);
        assert_eq!(in_category(IconCategory::Files).count(), 7);
        // os ícones que já existiam com o mesmo significado receberam o desenho novo
        for icon in [&CSS_FILE, &HTML_FILE, &JAVASCRIPT_FILE, &JSON_FILE, &XML_FILE] {
            assert_eq!(icon.category(), Some(IconCategory::FileTypes));
            assert_eq!(icon.png_size().width, 512, "{}", icon.name());
        }
        assert_eq!(find("pdf"), Some(&PDF_FILE));
        assert_eq!(find("js_file"), Some(&JAVASCRIPT_FILE));
        assert_eq!(for_extension("js"), Some(&JAVASCRIPT_FILE));
        assert_eq!(find("rar"), Some(&RAR_FILE));
        assert_eq!(find("compressed_file"), Some(&COMPRESSED_FILES));
        assert_eq!(find("7z"), Some(&SEVEN_ZIP_FILE));
        // `cpp`, `java` e `js` continuam sendo os logotipos; a extensão dá o arquivo
        assert_eq!(find("cpp"), Some(&CPP));
        assert_eq!(find("java"), Some(&JAVA));
        assert_eq!(find("js"), Some(&JAVASCRIPT));
        assert_eq!(for_extension("cpp"), Some(&CPP_FILE));
        assert_eq!(for_extension("java"), Some(&JAVA_FILE));
        assert_eq!(for_extension(".PDF"), Some(&PDF_FILE));
        assert_eq!(for_extension(" yml "), Some(&YAML_FILE));
        assert_eq!(for_extension("htm"), Some(&HTML_FILE));
        assert_eq!(for_extension("txt"), None);
        assert_eq!(for_extension(""), None);
        assert_eq!(for_path("fotos/Praia.JPG"), Some(&JPG_FILE));
        assert_eq!(for_path("backup.tar.gz"), Some(&GZ_FILE));
        assert_eq!(for_path("LEIAME"), None);
        // a tabela é ordenada, sem repetição, e cobre todos os ícones de tipo de arquivo
        let extensions: Vec<_> = catalog::EXTENSIONS.iter().map(|(e, _)| *e).collect();
        assert!(extensions.windows(2).all(|w| w[0] < w[1]));
        for (extension, icon) in catalog::EXTENSIONS.iter() {
            assert_eq!(extension.to_ascii_lowercase(), *extension);
            assert_eq!(icon.category(), Some(IconCategory::FileTypes), "{extension}");
        }
        for icon in in_category(IconCategory::FileTypes) {
            assert!(catalog::EXTENSIONS.iter().any(|(_, i)| std::ptr::eq(*i, icon)), "{} sem extensão", icon.name());
        }
    }

    #[test]
    fn best_entry_prefers_the_smallest_that_is_large_enough() {
        let ico = FOLDER.ico_bytes();
        assert_eq!(chosen_size(ico, 16), Some(16));
        assert_eq!(chosen_size(ico, 20), Some(24));
        assert_eq!(chosen_size(ico, 40), Some(48));
        assert_eq!(chosen_size(ico, 300), Some(256));
    }

    #[test]
    fn invalid_ico_data_is_rejected() {
        assert!(ico_directory(&[]).is_err());
        assert!(ico_directory(&[0, 0, 2, 0, 1, 0]).is_err()); // tipo 2 = cursor
        assert!(ico_directory(&[0, 0, 1, 0, 1, 0]).is_err()); // diretório truncado
        let mut truncated = FOLDER.ico_bytes()[..200].to_vec();
        truncated[4] = 1;
        assert!(ico_directory(&truncated).is_err()); // dados fora do arquivo
        assert_eq!(png_dimensions(b"nada"), None);
    }

    #[test]
    fn custom_assets_work_like_builtin_ones() {
        static MINE: IconAsset = IconAsset::new("meu", arqrustui_icons::ADD.0, &[], None);
        assert_eq!(MINE.category(), None);
        assert_eq!(MINE.sizes().len(), 7);
        assert_eq!(MINE.png_size(), Size::default());
        assert!(MINE.save(std::env::temp_dir().join("arqrustui-meu.svg")).is_err());
    }
}
