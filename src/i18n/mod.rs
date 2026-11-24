//! Internationalization: locale, regional formatting, text direction and translatable catalogs.
//!
//! Public strings are UTF-8, losslessly converted to UTF-16 (see [`crate::utilities::strings`]).

mod library;

pub use library::{
    apply_library_translations, library_format, library_text, set_library_language, set_library_text,
    system_library_language, use_system_library_language, LibraryText,
};

use crate::ffi::kernel32::*;
use crate::ffi::SYSTEMTIME;
use crate::utilities::{from_wide, to_wide};
use std::collections::HashMap;

/// Calendar date/time without timezone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    /// Year, e.g. 2026.
    pub year: u16,
    /// Month (1 to 12).
    pub month: u16,
    /// Day (1 to 31).
    pub day: u16,
    /// Hour (0 to 23).
    pub hour: u16,
    /// Minute (0 to 59).
    pub minute: u16,
    /// Second (0 to 59).
    pub second: u16,
}

impl DateTime {
    /// Creates a date at midnight.
    pub const fn date(year: u16, month: u16, day: u16) -> DateTime {
        DateTime { year, month, day, hour: 0, minute: 0, second: 0 }
    }

    /// Creates a date/time.
    pub const fn new(year: u16, month: u16, day: u16, hour: u16, minute: u16, second: u16) -> DateTime {
        DateTime { year, month, day, hour, minute, second }
    }

    /// Current local date/time.
    pub fn now() -> DateTime {
        let mut st = SYSTEMTIME::default();
        // SAFETY: ponteiro de saída válido.
        unsafe { GetLocalTime(&mut st) };
        DateTime::from_native(&st)
    }

    pub(crate) fn from_native(st: &SYSTEMTIME) -> DateTime {
        DateTime {
            year: st.wYear,
            month: st.wMonth,
            day: st.wDay,
            hour: st.wHour,
            minute: st.wMinute,
            second: st.wSecond,
        }
    }

    pub(crate) fn to_native(self) -> SYSTEMTIME {
        SYSTEMTIME {
            wYear: self.year,
            wMonth: self.month,
            wDayOfWeek: 0,
            wDay: self.day,
            wHour: self.hour,
            wMinute: self.minute,
            wSecond: self.second,
            wMilliseconds: 0,
        }
    }
}

/// Formatting locale (language/region).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale {
    name: String,
}

impl Locale {
    /// Windows user locale.
    pub fn user_default() -> Locale {
        let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH];
        // SAFETY: buffer do tamanho informado.
        let len = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
        if len <= 0 {
            return Locale { name: "en-US".into() };
        }
        Locale { name: from_wide(&buffer) }
    }

    /// Explicit locale, e.g. `"pt-BR"`, `"en-US"`, `"ja-JP"`.
    pub fn named(name: &str) -> Locale {
        Locale { name: name.to_string() }
    }

    /// BCP-47 locale name, e.g. `"pt-BR"`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Language code without region, e.g. `"pt"`.
    pub fn language(&self) -> &str {
        self.name.split('-').next().unwrap_or(&self.name)
    }

    fn info(&self, kind: u32) -> Option<String> {
        let name = to_wide(&self.name);
        let mut buffer = [0u16; 256];
        // SAFETY: nome terminado em NUL e buffer do tamanho informado.
        let len = unsafe { GetLocaleInfoEx(name.as_ptr(), kind, buffer.as_mut_ptr(), buffer.len() as i32) };
        (len > 0).then(|| from_wide(&buffer))
    }

    /// Locale display name in Windows UI language.
    pub fn display_name(&self) -> String {
        self.info(LOCALE_SLOCALIZEDDISPLAYNAME).unwrap_or_else(|| self.name.clone())
    }

    /// Whether written right-to-left, e.g. Arabic/Hebrew.
    pub fn is_right_to_left(&self) -> bool {
        self.info(LOCALE_IREADINGLAYOUT).map(|v| v.trim() == "1").unwrap_or(false)
    }

    /// Decimal separator, e.g. `","` in pt-BR.
    pub fn decimal_separator(&self) -> String {
        self.info(LOCALE_SDECIMAL).unwrap_or_else(|| ".".into())
    }

    /// Thousands separator, e.g. `"."` in pt-BR.
    pub fn thousands_separator(&self) -> String {
        self.info(LOCALE_STHOUSAND).unwrap_or_else(|| ",".into())
    }

    fn format_with(&self, call: impl Fn(*const u16, *mut u16, i32) -> i32) -> Option<String> {
        let name = to_wide(&self.name);
        let mut buffer = [0u16; 256];
        let len = call(name.as_ptr(), buffer.as_mut_ptr(), buffer.len() as i32);
        (len > 0).then(|| from_wide(&buffer))
    }

    /// Short date, e.g. `12/11/2026`.
    pub fn format_date(&self, date: &DateTime) -> String {
        self.date_with(date, DATE_SHORTDATE)
    }

    /// Long date, e.g. `quinta-feira, 12 de novembro de 2026` in pt-BR.
    pub fn format_long_date(&self, date: &DateTime) -> String {
        self.date_with(date, DATE_LONGDATE)
    }

    fn date_with(&self, date: &DateTime, flags: u32) -> String {
        let st = date.to_native();
        // SAFETY: ponteiros válidos durante a chamada; formato e calendário nulos usam o padrão.
        self.format_with(|locale, out, size| unsafe {
            GetDateFormatEx(locale, flags, &st, std::ptr::null(), out, size, std::ptr::null())
        })
        .unwrap_or_else(|| format!("{:04}-{:02}-{:02}", date.year, date.month, date.day))
    }

    /// Locale time; `with_seconds` controls seconds.
    pub fn format_time(&self, time: &DateTime, with_seconds: bool) -> String {
        let st = time.to_native();
        let flags = if with_seconds { 0 } else { TIME_NOSECONDS };
        // SAFETY: ponteiros válidos durante a chamada.
        self.format_with(|locale, out, size| unsafe {
            GetTimeFormatEx(locale, flags, &st, std::ptr::null(), out, size)
        })
        .unwrap_or_else(|| format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second))
    }

    /// Number with regional separators/two decimals, e.g. `1.234,50` in pt-BR.
    pub fn format_number(&self, value: f64) -> String {
        let text = to_wide(&format!("{value:.2}"));
        // SAFETY: ponteiros válidos durante a chamada; formato nulo usa o padrão da localidade.
        self.format_with(|locale, out, size| unsafe {
            GetNumberFormatEx(locale, 0, text.as_ptr(), std::ptr::null(), out, size)
        })
        .unwrap_or_else(|| format!("{value:.2}"))
    }

    /// Locale currency, e.g. `R$ 1.234,50`.
    pub fn format_currency(&self, value: f64) -> String {
        let text = to_wide(&format!("{value:.2}"));
        // SAFETY: ponteiros válidos durante a chamada; formato nulo usa o padrão da localidade.
        self.format_with(|locale, out, size| unsafe {
            GetCurrencyFormatEx(locale, 0, text.as_ptr(), std::ptr::null(), out, size)
        })
        .unwrap_or_else(|| format!("{value:.2}"))
    }
}

/// UI text catalog by language.
///
/// Register text in code or load `key = value`, one entry per line; `#` starts comments and `\n` becomes a newline.
///
/// ```
/// use arqrustui::i18n::Translations;
///
/// let mut texts = Translations::new("en");
/// texts.load("en", "greeting = Hello, {name}!\nquit = Quit");
/// texts.load("pt", "greeting = Olá, {name}!");
/// texts.set_language("pt-BR");
/// assert_eq!(texts.format("greeting", &[("name", "Ana")]), "Olá, Ana!");
/// assert_eq!(texts.get("quit"), "Quit"); // falls back to the default language
/// assert_eq!(texts.get("missing"), "missing"); // missing key: returns the key itself
/// ```
#[derive(Clone, Debug, Default)]
pub struct Translations {
    fallback: String,
    current: String,
    catalogs: HashMap<String, HashMap<String, String>>,
}

impl Translations {
    /// Creates a catalog with `fallback_language` default.
    pub fn new(fallback_language: &str) -> Translations {
        Translations {
            fallback: fallback_language.to_lowercase(),
            current: fallback_language.to_lowercase(),
            catalogs: HashMap::new(),
        }
    }

    /// Registers text.
    pub fn add(&mut self, language: &str, key: &str, text: &str) {
        self.catalogs.entry(language.to_lowercase()).or_default().insert(key.to_string(), text.to_string());
    }

    /// Loads `key = value` entries, returning count read.
    pub fn load(&mut self, language: &str, source: &str) -> usize {
        let mut count = 0;
        for line in source.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                self.add(language, key.trim(), &value.trim().replace("\\n", "\n"));
                count += 1;
            }
        }
        count
    }

    /// Selects active language. Region names (`"pt-BR"`) fall back to language (`"pt"`) if no exact catalog exists.
    pub fn set_language(&mut self, language: &str) {
        self.current = language.to_lowercase();
    }

    /// Selects Windows user language.
    pub fn use_system_language(&mut self) {
        self.set_language(Locale::user_default().name());
    }

    /// Active language.
    pub fn language(&self) -> &str {
        &self.current
    }

    fn lookup(&self, language: &str, key: &str) -> Option<&String> {
        if let Some(text) = self.catalogs.get(language).and_then(|c| c.get(key)) {
            return Some(text);
        }
        let base = language.split('-').next().unwrap_or(language);
        self.catalogs.get(base).and_then(|c| c.get(key))
    }

    /// Active-language text, falling back to default language then key itself.
    pub fn get(&self, key: &str) -> String {
        self.lookup(&self.current, key)
            .or_else(|| self.lookup(&self.fallback, key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Like [`get`](Self::get), replacing `{name}` placeholders.
    pub fn format(&self, key: &str, arguments: &[(&str, &str)]) -> String {
        let mut text = self.get(key);
        for (name, value) in arguments {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_formatting_follows_region() {
        let br = Locale::named("pt-BR");
        let us = Locale::named("en-US");
        let date = DateTime::new(2026, 11, 12, 14, 5, 9);
        assert_eq!(br.format_date(&date), "12/11/2026");
        assert_eq!(us.format_date(&date), "11/12/2026");
        assert_eq!(br.language(), "pt");
        assert!(!br.is_right_to_left());
        assert!(br.format_time(&date, false).contains("14:05"));
        assert!(!Locale::user_default().name().is_empty());
    }

    #[test]
    fn number_formatting_uses_regional_separators() {
        let br = Locale::named("pt-BR");
        assert_eq!(br.decimal_separator(), ",");
        assert_eq!(br.format_number(1234.5), "1.234,50");
        assert_eq!(Locale::named("en-US").format_number(1234.5), "1,234.50");
        assert!(br.format_currency(10.0).contains("10,00"));
    }

    #[test]
    fn translations_fallback_chain() {
        let mut t = Translations::new("en");
        assert_eq!(t.load("en", "# comentário\nhello = Hello\nmulti = a\\nb\n\nsem_igual"), 2);
        t.add("pt", "hello", "Olá");
        t.set_language("pt-BR");
        assert_eq!(t.get("hello"), "Olá");
        assert_eq!(t.get("multi"), "a\nb");
        assert_eq!(t.get("nada"), "nada");
        t.set_language("ja-JP");
        assert_eq!(t.get("hello"), "Hello");
        assert_eq!(t.language(), "ja-jp");
    }

    #[test]
    fn date_time_native_round_trip() {
        let value = DateTime::new(2024, 2, 29, 23, 59, 58);
        assert_eq!(DateTime::from_native(&value.to_native()), value);
        assert!(DateTime::now().year >= 2024);
        assert!(DateTime::date(2020, 1, 1) < value);
    }
}
