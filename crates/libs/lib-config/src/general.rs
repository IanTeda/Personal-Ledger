//! # The `[General]` section
//!
//! The keys Settings › General shows (ADR-0057): On start, Start view, Toasts, the Locale, the
//! Data folder and the Ledger `file` to open at start. Each is spelled in the file as `:set` takes
//! it. `file` is set only by hand or `--file`.

use lib_core::{OnStart, StartView};

/// The Locale used when neither the configuration nor the operating system names a usable
/// one. `lib-config` doesn't know which Locales are supported -- `lib-locale` negotiates
/// that -- so this is only the last-resort request.
pub const DEFAULT_LOCALE: &str = "en-US";

/// Where the requested Locale came from, lowest to highest precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocaleSource {
    /// Neither the configuration nor the operating system supplied a usable Locale.
    Default,
    /// Detected from the operating system.
    System,
    /// Set by a configuration file, environment variable or the `--locale` flag.
    Config,
}

/// Configuration for the `[General]` section.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct GeneralConfig {
    /// Which Ledger file a Client opens as it starts: `reopen`, the default, or `none`
    /// (ADR-0051). `file` outranks it.
    #[serde(with = "on_start")]
    pub on_start: OnStart,

    /// The View a Client lands on as it starts: `dashboard`, the default, `last`, `settings` or a
    /// noun, kept raw so a noun only the other Client has survives (ADR-0053).
    #[serde(with = "start_view")]
    pub start_view: StartView,

    /// Whether non-Error Toasts show (ADR-0027). Both Clients; on by default.
    pub toasts: bool,

    /// The requested Locale as a BCP-47 tag (e.g. `en-GB`), from a configuration file,
    /// environment variable or `--locale`. Unset means "follow the operating system" -- use
    /// [`Self::resolved_locale`] rather than reading this directly.
    pub locale: Option<String>,

    /// The operating system's Locale, detected by `LedgerConfig::parse`. Never read from or
    /// written to a configuration source.
    #[serde(skip)]
    system_locale: Option<String>,

    /// The folder `:open`'s file browser starts in. Defaults to the user's document folder.
    pub data: std::path::PathBuf,

    /// A Ledger file to open at start, overriding On start. Optional, with no
    /// default: unset, the Client opens the Ledger On start names, or none (ADR-0051).
    pub file: Option<std::path::PathBuf>,
}

/// The value the file and `:set on-start` spell `on_start` with.
pub fn on_start_key(on_start: OnStart) -> &'static str {
    match on_start {
        OnStart::ReopenLastLedger => "reopen",
        OnStart::NoLedger => "none",
    }
}

/// The On start `key` spells, ignoring case.
pub fn on_start_from_key(key: &str) -> Option<OnStart> {
    [OnStart::ReopenLastLedger, OnStart::NoLedger]
        .into_iter()
        .find(|on_start| on_start_key(*on_start).eq_ignore_ascii_case(key.trim()))
}

/// Reads a value by its key: `parse` names the one the file spells, or refuses it.
pub(crate) fn by_key<'de, D, T>(
    deserializer: D,
    parse: impl Fn(&str) -> Option<T>,
) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let key = <String as serde::Deserialize>::deserialize(deserializer)?;
    parse(&key).ok_or_else(|| serde::de::Error::custom(format!("unknown value `{key}`")))
}

/// `start_view` by [`StartView::key`]. Every word is one: a noun no Client has lands on the
/// Dashboard rather than stopping the app.
mod start_view {
    use super::StartView;

    pub fn serialize<S: serde::Serializer>(
        value: &StartView,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.key())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<StartView, D::Error> {
        super::by_key(deserializer, |key| Some(StartView::from_key(key)))
    }
}

/// `on_start` by [`super::on_start_key`].
mod on_start {
    use super::OnStart;

    pub fn serialize<S: serde::Serializer>(
        value: &OnStart,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(super::on_start_key(*value))
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<OnStart, D::Error> {
        super::by_key(deserializer, super::on_start_from_key)
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            on_start: OnStart::ReopenLastLedger,
            start_view: StartView::Dashboard,
            toasts: true,
            locale: None,
            system_locale: None,
            data: Self::default_data_dir(),
            file: None,
        }
    }
}

impl GeneralConfig {
    /// Which Ledger file a Client opens as it starts, when `file` doesn't name one.
    pub fn on_start(&self) -> OnStart {
        self.on_start
    }

    /// The View a Client lands on as it starts.
    pub fn start_view(&self) -> &StartView {
        &self.start_view
    }

    /// Whether non-Error Toasts show.
    pub fn toasts(&self) -> bool {
        self.toasts
    }

    /// Returns the configured data directory.
    pub fn data(&self) -> &std::path::Path {
        &self.data
    }

    /// Returns the Ledger file explicitly configured to open at start, if any.
    pub fn file(&self) -> Option<&std::path::Path> {
        self.file.as_deref()
    }

    /// Returns the Locale set by configuration, if any (canonical casing once parsed).
    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
    }

    /// The operating system's Locale as a canonical tag, whatever configuration sets; `None` when
    /// it names no usable one. What Settings › General's **System** resolves to.
    pub fn system_locale(&self) -> Option<&str> {
        self.system_locale.as_deref()
    }

    /// Resolves the requested Locale tag and where it came from: the configured value, else
    /// the operating system's, else [`DEFAULT_LOCALE`]. The tag is not checked against the
    /// Locales the application supports -- `lib-locale` negotiates that.
    pub fn resolved_locale(&self) -> (&str, LocaleSource) {
        if let Some(locale) = self.locale.as_deref() {
            (locale, LocaleSource::Config)
        } else if let Some(locale) = self.system_locale.as_deref() {
            (locale, LocaleSource::System)
        } else {
            (DEFAULT_LOCALE, LocaleSource::Default)
        }
    }

    /// Validates the configured Locale (canonicalising its casing) and records the detected
    /// system Locale, so [`Self::resolved_locale`] can answer. Called once after the layered
    /// sources are merged; `detect` reports the operating system's Locale.
    ///
    /// # Errors
    /// Returns [`crate::Error::InvalidLocale`] when the configured value isn't a well-formed
    /// BCP-47 tag.
    pub(crate) fn resolve_locale(
        &mut self,
        detect: impl FnOnce() -> Option<String>,
    ) -> crate::Result<()> {
        if let Some(value) = self.locale.as_deref() {
            self.locale = Some(parse_locale_tag(value)?);
        }
        // Detected even when configuration sets the Locale, so Settings › General can say what
        // **System** would read.
        self.system_locale = detect().and_then(|value| usable_system_locale(&value));
        Ok(())
    }

    /// The default data directory: the user's document folder, falling back to the current
    /// working directory when it can't be determined (e.g. `$HOME` unset).
    fn default_data_dir() -> std::path::PathBuf {
        dirs::document_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
    }

    /// The defaults seeded into a layered configuration builder, mirroring
    /// `SyncServerConfig::default_config_values()`. Only `data` is seeded: `file` and `locale`
    /// have no meaningful default, and `locale` in particular must stay unset so the operating
    /// system's Locale can outrank [`DEFAULT_LOCALE`] (see [`Self::resolved_locale`]). The rest
    /// take their defaults from serde.
    pub fn default_config_values() -> Vec<(&'static str, String)> {
        vec![(
            "general.data",
            Self::default_data_dir().to_string_lossy().into_owned(),
        )]
    }
}

/// Validates a BCP-47 Locale tag and returns it in canonical casing (`en-gb` -> `en-GB`).
///
/// # Errors
/// Returns [`crate::Error::InvalidLocale`] when `value` isn't a well-formed tag.
pub(crate) fn parse_locale_tag(value: &str) -> crate::Result<String> {
    value
        .parse::<unic_langid::LanguageIdentifier>()
        .map(|langid| langid.to_string())
        .map_err(|_| crate::Error::InvalidLocale {
            value: value.to_string(),
        })
}

/// The canonical tag for an operating-system Locale, or `None` when it isn't a usable tag.
/// The POSIX placeholders `C` and `POSIX` mean "no locale", not a language, so they are
/// ignored quietly.
fn usable_system_locale(value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("posix") {
        return None;
    }
    parse_locale_tag(value).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// The `[General]` section `text` and `env` give, with no system Locale.
    fn parsed(text: &str, env: &[(&str, &str)]) -> GeneralConfig {
        crate::ledger::parsed_for_test(text, env)
            .general_config()
            .clone()
    }

    #[test]
    fn default_config_has_no_file() {
        assert_eq!(GeneralConfig::default().file(), None);
    }

    #[test]
    fn default_config_values_seed_only_data() {
        let defaults = GeneralConfig::default_config_values();
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].0, "general.data");
    }

    #[test]
    fn the_general_settings_default_to_toasts_on_reopen_and_the_dashboard() {
        let section = parsed("[General]\n", &[]);
        assert!(section.toasts());
        assert_eq!(section.on_start(), OnStart::ReopenLastLedger);
        assert_eq!(section.start_view(), &StartView::Dashboard);
        assert_eq!(section.locale(), None);
    }

    #[test]
    fn start_view_reads_its_three_words_or_a_noun_and_an_environment_variable_outranks_it() {
        for (text, view) in [
            ("last", StartView::Last),
            ("Settings", StartView::Settings),
            ("dashboard", StartView::Dashboard),
            ("Units", StartView::Noun("units".into())),
        ] {
            let section = parsed(&format!("[General]\nstart_view = {text}\n"), &[]);
            assert_eq!(section.start_view(), &view, "{text}");
        }
        let section = parsed(
            "[General]\nstart_view = last\n",
            &[("PERSONAL_LEDGER_GENERAL__START_VIEW", "tags")],
        );
        assert_eq!(section.start_view(), &StartView::Noun("tags".into()));
    }

    #[test]
    fn the_general_settings_read_from_the_file_as_set_spells_them() {
        let section = parsed(
            "[general]\ntoasts = false\non_start = none\nlocale = en-gb\n\
             data = /tmp/ledgers\nfile = /tmp/ledgers/mine.pldb\n",
            &[],
        );
        assert!(!section.toasts());
        assert_eq!(section.on_start(), OnStart::NoLedger);
        assert_eq!(section.locale(), Some("en-GB"));
        assert_eq!(section.data(), std::path::Path::new("/tmp/ledgers"));
        assert_eq!(
            section.file(),
            Some(std::path::Path::new("/tmp/ledgers/mine.pldb"))
        );

        let section = parsed("[General]\non_start = reopen\n", &[]);
        assert_eq!(section.on_start(), OnStart::ReopenLastLedger);
    }

    #[test]
    fn an_environment_variable_outranks_the_file_for_a_general_setting() {
        let section = parsed(
            "[General]\non_start = none\ntoasts = true\n",
            &[
                ("PERSONAL_LEDGER_GENERAL__ON_START", "reopen"),
                ("PERSONAL_LEDGER_GENERAL__TOASTS", "false"),
            ],
        );
        assert_eq!(section.on_start(), OnStart::ReopenLastLedger);
        assert!(!section.toasts());
    }

    #[test]
    fn a_general_setting_value_it_doesnt_take_is_a_configuration_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        std::fs::write(&path, "[General]\non_start = sometimes\n").unwrap();
        assert!(
            crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None).is_err()
        );
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = GeneralConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: GeneralConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
