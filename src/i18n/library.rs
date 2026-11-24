//! Translatable internal library text: labels, accessible names, tooltips.
//!
//! Library-drawn controls display/announce internal text: [`PropertyGrid`](crate::controls::PropertyGrid) Yes/No,
//! [`CommandBar`](crate::controls::CommandBar) overflow More options, severity names, etc. Defaults to Portuguese;
//! [`set_library_language`] selects built-in `"pt"`/`"en"` tables, and [`set_library_text`] overrides individual
//! entries, e.g. from application [`Translations`](super::Translations).
//!
//! Process-wide table. Drawing/accessibility text changes on next repaint; creation-time text, such as PropertyGrid
//! headers, affects subsequently created controls.
//!
//! ```
//! use arqrustui::i18n::{library_text, set_library_language, set_library_text, LibraryText};
//!
//! assert!(set_library_language("en-US"));
//! assert_eq!(library_text(LibraryText::Yes), "Yes");
//! set_library_text(LibraryText::Yes, "Ja");
//! assert_eq!(library_text(LibraryText::Yes), "Ja");
//! set_library_language("pt");
//! assert_eq!(library_text(LibraryText::Yes), "Sim");
//! ```

use std::collections::HashMap;
use std::sync::Mutex;

/// Internal library text key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LibraryText {
    /// True value (Yes).
    Yes,
    /// False value (No).
    No,
    /// PropertyGrid name column header (Property).
    PropertyColumn,
    /// PropertyGrid value column header (Value).
    ValueColumn,
    /// PropertyGrid accessible name (Properties).
    PropertiesName,
    /// CommandBar accessible name (Command bar).
    CommandBarName,
    /// CommandBar `»` accessible name (More options).
    MoreOptions,
    /// CommandBar `»` tooltip (More commands).
    MoreCommands,
    /// MenuStrip `»` name (More menus).
    MoreMenus,
    /// MenuStrip accessible name (Menu bar).
    MenuBarName,
    /// Information severity.
    Information,
    /// Success severity.
    Success,
    /// Warning severity.
    Warning,
    /// Error severity.
    Error,
    /// Close button/action.
    Close,
    /// Switch on state (On).
    On,
    /// Switch off state (Off).
    Off,
    /// Expand action.
    Expand,
    /// Collapse action.
    Collapse,
    /// Increase value.
    Increase,
    /// Decrease value.
    Decrease,
    /// Rating: "{value} of {max}".
    RatingValue,
    /// CommandBar customization dialog title (Customize command bar).
    CustomizeTitle,
    /// Available commands.
    AvailableCommands,
    /// Commands on the bar.
    CurrentCommands,
    /// Add →.
    Add,
    /// ← Remove.
    Remove,
    /// Move up.
    MoveUp,
    /// Move down.
    MoveDown,
    /// Restore defaults.
    ResetDefaults,
    /// Confirm (OK).
    Ok,
    /// Cancel.
    Cancel,
    /// Separator.
    Separator,
    /// Print preview.
    PrintPreview,
    /// Print.
    Print,
    /// Page: "Page {page} of {count}".
    PageOf,
    /// Previous page.
    Previous,
    /// Next page.
    Next,
    /// Zoom in.
    ZoomIn,
    /// Zoom out.
    ZoomOut,
    /// DataGrid group without value ((empty)).
    EmptyGroup,
    /// Minimize.
    Minimize,
    /// Maximize.
    Maximize,
    /// Restore.
    Restore,
    /// Title bar.
    TitleBar,
    /// IME candidates.
    Candidates,
    /// Navigation path.
    Breadcrumb,
    /// Detach tab (Open in new window).
    DetachTab,
    /// Float dockable pane.
    Float,
    /// Dock pane.
    Dock,
    /// Icon category [`IconCategory::Actions`](crate::icons::IconCategory::Actions).
    CategoryActions,
    /// Icon category [`IconCategory::Status`](crate::icons::IconCategory::Status).
    CategoryStatus,
    /// Icon category [`IconCategory::Files`](crate::icons::IconCategory::Files).
    CategoryFiles,
    /// Icon category [`IconCategory::FileTypes`](crate::icons::IconCategory::FileTypes).
    CategoryFileTypes,
    /// Icon category [`IconCategory::Office`](crate::icons::IconCategory::Office).
    CategoryOffice,
    /// Icon category [`IconCategory::Communication`](crate::icons::IconCategory::Communication).
    CategoryCommunication,
    /// Icon category [`IconCategory::People`](crate::icons::IconCategory::People).
    CategoryPeople,
    /// Icon category [`IconCategory::Devices`](crate::icons::IconCategory::Devices).
    CategoryDevices,
    /// Icon category [`IconCategory::Hardware`](crate::icons::IconCategory::Hardware).
    CategoryHardware,
    /// Icon category [`IconCategory::Network`](crate::icons::IconCategory::Network).
    CategoryNetwork,
    /// Icon category [`IconCategory::Data`](crate::icons::IconCategory::Data).
    CategoryData,
    /// Icon category [`IconCategory::Security`](crate::icons::IconCategory::Security).
    CategorySecurity,
    /// Icon category [`IconCategory::Media`](crate::icons::IconCategory::Media).
    CategoryMedia,
    /// Icon category [`IconCategory::Design`](crate::icons::IconCategory::Design).
    CategoryDesign,
    /// Icon category [`IconCategory::Development`](crate::icons::IconCategory::Development).
    CategoryDevelopment,
    /// Icon category [`IconCategory::Technologies`](crate::icons::IconCategory::Technologies).
    CategoryTechnologies,
    /// Icon category [`IconCategory::Commerce`](crate::icons::IconCategory::Commerce).
    CategoryCommerce,
    /// Icon category [`IconCategory::Navigation`](crate::icons::IconCategory::Navigation).
    CategoryNavigation,
    /// Icon category [`IconCategory::Entertainment`](crate::icons::IconCategory::Entertainment).
    CategoryEntertainment,
    /// Icon category [`IconCategory::Social`](crate::icons::IconCategory::Social).
    CategorySocial,
    /// Icon category [`IconCategory::Emoji`](crate::icons::IconCategory::Emoji).
    CategoryEmoji,
    /// Icon category [`IconCategory::Gestures`](crate::icons::IconCategory::Gestures).
    CategoryGestures,
}

const ALL: &[LibraryText] = &[
    LibraryText::Yes,
    LibraryText::No,
    LibraryText::PropertyColumn,
    LibraryText::ValueColumn,
    LibraryText::PropertiesName,
    LibraryText::CommandBarName,
    LibraryText::MoreOptions,
    LibraryText::MoreCommands,
    LibraryText::MoreMenus,
    LibraryText::MenuBarName,
    LibraryText::Information,
    LibraryText::Success,
    LibraryText::Warning,
    LibraryText::Error,
    LibraryText::Close,
    LibraryText::On,
    LibraryText::Off,
    LibraryText::Expand,
    LibraryText::Collapse,
    LibraryText::Increase,
    LibraryText::Decrease,
    LibraryText::RatingValue,
    LibraryText::CustomizeTitle,
    LibraryText::AvailableCommands,
    LibraryText::CurrentCommands,
    LibraryText::Add,
    LibraryText::Remove,
    LibraryText::MoveUp,
    LibraryText::MoveDown,
    LibraryText::ResetDefaults,
    LibraryText::Ok,
    LibraryText::Cancel,
    LibraryText::Separator,
    LibraryText::PrintPreview,
    LibraryText::Print,
    LibraryText::PageOf,
    LibraryText::Previous,
    LibraryText::Next,
    LibraryText::ZoomIn,
    LibraryText::ZoomOut,
    LibraryText::EmptyGroup,
    LibraryText::Minimize,
    LibraryText::Maximize,
    LibraryText::Restore,
    LibraryText::TitleBar,
    LibraryText::Candidates,
    LibraryText::Breadcrumb,
    LibraryText::DetachTab,
    LibraryText::Float,
    LibraryText::Dock,
    LibraryText::CategoryActions,
    LibraryText::CategoryStatus,
    LibraryText::CategoryFiles,
    LibraryText::CategoryFileTypes,
    LibraryText::CategoryOffice,
    LibraryText::CategoryCommunication,
    LibraryText::CategoryPeople,
    LibraryText::CategoryDevices,
    LibraryText::CategoryHardware,
    LibraryText::CategoryNetwork,
    LibraryText::CategoryData,
    LibraryText::CategorySecurity,
    LibraryText::CategoryMedia,
    LibraryText::CategoryDesign,
    LibraryText::CategoryDevelopment,
    LibraryText::CategoryTechnologies,
    LibraryText::CategoryCommerce,
    LibraryText::CategoryNavigation,
    LibraryText::CategoryEntertainment,
    LibraryText::CategorySocial,
    LibraryText::CategoryEmoji,
    LibraryText::CategoryGestures,
];

impl LibraryText {
    /// All keys in declaration order.
    pub fn all() -> &'static [LibraryText] {
        ALL
    }

    /// Stable catalog key name for `key = value`, e.g. `"yes"`, `"more_options"`.
    pub fn key(self) -> &'static str {
        self.entry().0
    }

    /// Built-in text for `"pt"`/`"en"`; other languages fall back to Portuguese.
    pub fn builtin(self, language: &str) -> &'static str {
        let (_, pt, en) = self.entry();
        if language.split(['-', '_']).next().is_some_and(|l| l.eq_ignore_ascii_case("en")) {
            en
        } else {
            pt
        }
    }

    fn entry(self) -> (&'static str, &'static str, &'static str) {
        use LibraryText::*;
        match self {
            Yes => ("yes", "Sim", "Yes"),
            No => ("no", "Não", "No"),
            PropertyColumn => ("property_column", "Propriedade", "Property"),
            ValueColumn => ("value_column", "Valor", "Value"),
            PropertiesName => ("properties_name", "Propriedades", "Properties"),
            CommandBarName => ("command_bar_name", "Barra de comandos", "Command bar"),
            MoreOptions => ("more_options", "Mais opções", "More options"),
            MoreCommands => ("more_commands", "Mais comandos", "More commands"),
            MoreMenus => ("more_menus", "Mais menus", "More menus"),
            MenuBarName => ("menu_bar_name", "Barra de menus", "Menu bar"),
            Information => ("information", "Informação", "Information"),
            Success => ("success", "Sucesso", "Success"),
            Warning => ("warning", "Aviso", "Warning"),
            Error => ("error", "Erro", "Error"),
            Close => ("close", "Fechar", "Close"),
            On => ("on", "Ligado", "On"),
            Off => ("off", "Desligado", "Off"),
            Expand => ("expand", "Expandir", "Expand"),
            Collapse => ("collapse", "Recolher", "Collapse"),
            Increase => ("increase", "Aumentar", "Increase"),
            Decrease => ("decrease", "Diminuir", "Decrease"),
            RatingValue => ("rating_value", "{value} de {max}", "{value} of {max}"),
            CustomizeTitle => ("customize_title", "Personalizar barra de comandos", "Customize command bar"),
            AvailableCommands => ("available_commands", "Comandos disponíveis", "Available commands"),
            CurrentCommands => ("current_commands", "Comandos na barra", "Commands on the bar"),
            Add => ("add", "Adicionar →", "Add →"),
            Remove => ("remove", "← Remover", "← Remove"),
            MoveUp => ("move_up", "Para cima", "Move up"),
            MoveDown => ("move_down", "Para baixo", "Move down"),
            ResetDefaults => ("reset_defaults", "Restaurar padrão", "Reset"),
            Ok => ("ok", "OK", "OK"),
            Cancel => ("cancel", "Cancelar", "Cancel"),
            Separator => ("separator", "Separador", "Separator"),
            PrintPreview => ("print_preview", "Visualizar impressão", "Print preview"),
            Print => ("print", "Imprimir", "Print"),
            PageOf => ("page_of", "Página {page} de {count}", "Page {page} of {count}"),
            Previous => ("previous", "Anterior", "Previous"),
            Next => ("next", "Próxima", "Next"),
            ZoomIn => ("zoom_in", "Ampliar", "Zoom in"),
            ZoomOut => ("zoom_out", "Reduzir", "Zoom out"),
            EmptyGroup => ("empty_group", "(vazio)", "(empty)"),
            Minimize => ("minimize", "Minimizar", "Minimize"),
            Maximize => ("maximize", "Maximizar", "Maximize"),
            Restore => ("restore", "Restaurar", "Restore"),
            TitleBar => ("title_bar", "Barra de título", "Title bar"),
            Candidates => ("candidates", "Candidatos", "Candidates"),
            Breadcrumb => ("breadcrumb", "Caminho", "Path"),
            DetachTab => ("detach_tab", "Abrir em nova janela", "Open in new window"),
            Float => ("float", "Flutuar", "Float"),
            Dock => ("dock", "Acoplar", "Dock"),
            CategoryActions => ("category_actions", "Ações", "Actions"),
            CategoryStatus => ("category_status", "Estados", "Status"),
            CategoryFiles => ("category_files", "Arquivos", "Files"),
            CategoryFileTypes => ("category_file_types", "Tipos de arquivo", "File types"),
            CategoryOffice => ("category_office", "Escritório", "Office"),
            CategoryCommunication => ("category_communication", "Comunicação", "Communication"),
            CategoryPeople => ("category_people", "Pessoas", "People"),
            CategoryDevices => ("category_devices", "Dispositivos", "Devices"),
            CategoryHardware => ("category_hardware", "Hardware", "Hardware"),
            CategoryNetwork => ("category_network", "Rede e nuvem", "Network and cloud"),
            CategoryData => ("category_data", "Dados", "Data"),
            CategorySecurity => ("category_security", "Segurança", "Security"),
            CategoryMedia => ("category_media", "Mídia", "Media"),
            CategoryDesign => ("category_design", "Design", "Design"),
            CategoryDevelopment => ("category_development", "Desenvolvimento", "Development"),
            CategoryTechnologies => ("category_technologies", "Tecnologias", "Technologies"),
            CategoryCommerce => ("category_commerce", "Comércio", "Commerce"),
            CategoryNavigation => ("category_navigation", "Navegação", "Navigation"),
            CategoryEntertainment => ("category_entertainment", "Entretenimento", "Entertainment"),
            CategorySocial => ("category_social", "Redes sociais", "Social networks"),
            CategoryEmoji => ("category_emoji", "Emojis", "Emoji"),
            CategoryGestures => ("category_gestures", "Gestos", "Gestures"),
        }
    }
}

struct Table {
    language: String,
    overrides: HashMap<LibraryText, String>,
}

static TABLE: Mutex<Option<Table>> = Mutex::new(None);

fn with_table<R>(f: impl FnOnce(&mut Table) -> R) -> R {
    // Um panic com o lock tomado não deixa a tabela inconsistente: só recupera o valor.
    let mut guard = TABLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let table = guard.get_or_insert_with(|| Table { language: "pt".into(), overrides: HashMap::new() });
    f(table)
}

/// Current key text from [`set_library_text`] override or language table.
pub fn library_text(key: LibraryText) -> String {
    with_table(|t| t.overrides.get(&key).cloned().unwrap_or_else(|| key.builtin(&t.language).to_string()))
}

/// Like [`library_text`], replacing `{name}` placeholders with values.
pub fn library_format(key: LibraryText, arguments: &[(&str, &str)]) -> String {
    let mut text = library_text(key);
    for (name, value) in arguments {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// Overrides a key's text process-wide.
pub fn set_library_text(key: LibraryText, text: &str) {
    with_table(|t| {
        t.overrides.insert(key, text.to_string());
    });
}

/// Selects built-in `"pt"`/`"en"` table with optional region, discarding [`set_library_text`] overrides. Returns
/// `false` and retains Portuguese for unsupported languages.
pub fn set_library_language(language: &str) -> bool {
    let base = language.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    let known = matches!(base.as_str(), "pt" | "en");
    with_table(|t| {
        t.language = if known { base } else { "pt".into() };
        t.overrides.clear();
    });
    known
}

/// Built-in table matching Windows UI language: `"pt"` for Portuguese, `"en"` for all others without their own table.
pub fn system_library_language() -> &'static str {
    // SAFETY: função sem argumentos, presente em todas as versões suportadas do Windows.
    let langid = unsafe { crate::ffi::kernel32::GetUserDefaultUILanguage() };
    language_for_langid(langid)
}

/// Selects built-in table from Windows UI language (see [`system_library_language`]), returning its name. Discards
/// previous overrides like [`set_library_language`].
pub fn use_system_library_language() -> &'static str {
    let language = system_library_language();
    set_library_language(language);
    language
}

/// Table language for Windows `LANGID` (primary language is low 10 bits).
fn language_for_langid(langid: u16) -> &'static str {
    const LANG_PORTUGUESE: u16 = 0x16;
    if langid & 0x3FF == LANG_PORTUGUESE {
        "pt"
    } else {
        "en"
    }
}

/// Applies present application catalog entries by [`LibraryText::key`] in its active language. Returns count applied.
pub fn apply_library_translations(catalog: &super::Translations) -> usize {
    let mut applied = 0;
    for &key in ALL {
        let text = catalog.get(key.key());
        if text != key.key() {
            set_library_text(key, &text);
            applied += 1;
        }
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tabela é global: um único teste exercita a sequência inteira sem disputar com outros.
    #[test]
    fn table_switches_languages_and_overrides() {
        let keys: std::collections::HashSet<_> = ALL.iter().map(|k| k.key()).collect();
        assert_eq!(keys.len(), ALL.len(), "chaves duplicadas");
        for key in ALL {
            assert!(!key.builtin("pt").is_empty() && !key.builtin("en").is_empty());
        }
        assert_eq!(LibraryText::No.builtin("en-GB"), "No");
        assert_eq!(LibraryText::No.builtin("fr"), "Não");

        assert!(!set_library_language("fr"));
        assert_eq!(library_text(LibraryText::MoreOptions), "Mais opções");
        assert!(set_library_language("en_US"));
        assert_eq!(library_text(LibraryText::MoreOptions), "More options");
        assert_eq!(library_format(LibraryText::PageOf, &[("page", "2"), ("count", "9")]), "Page 2 of 9");

        let mut catalog = super::super::Translations::new("de");
        catalog.load("de", "yes = Ja\nno = Nein\nunrelated = x");
        assert_eq!(apply_library_translations(&catalog), 2);
        assert_eq!(library_text(LibraryText::No), "Nein");
        assert_eq!(library_text(LibraryText::Close), "Close");

        assert!(set_library_language("pt-BR"));
        assert_eq!(library_text(LibraryText::No), "Não");
    }

    #[test]
    fn langid_maps_to_builtin_table() {
        assert_eq!(language_for_langid(0x0416), "pt"); // pt-BR
        assert_eq!(language_for_langid(0x0816), "pt"); // pt-PT
        assert_eq!(language_for_langid(0x0409), "en"); // en-US
        assert_eq!(language_for_langid(0x0C0A), "en"); // es-ES: sem tabela própria
        assert!(matches!(system_library_language(), "pt" | "en"));
    }
}
