//! # Personal Ledger Section Configuration
//!
//! Configuration for the `[Personal-Ledger]` section: settings applied across Personal
//! Ledger files and the client on a given system, but not intended to sync across computer
//! systems (see `docs/configuration.md`).

/// Default logging level for the `[Personal-Ledger]` section.
const DEFAULT_LOG: lib_tracing::Levels = lib_tracing::Levels::INFO;

/// The database filename appended to the resolved data directory when `file` isn't
/// explicitly configured.
const DEFAULT_FILE_NAME: &str = "My-Personal-Ledger.pldb";

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

/// Configuration for the `[Personal-Ledger]` section.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct PersonalLedgerConfig {
    /// Points to a configuration file not in the predefined search hierarchy. Optional --
    /// when unset, the normal hierarchy (see `LedgerConfig::parse`) decides which file (if
    /// any) is read.
    #[serde(default)]
    pub config: Option<std::path::PathBuf>,

    /// The default location for Personal Ledger database files. Defaults to the user's
    /// document folder.
    pub data: std::path::PathBuf,

    /// The Personal Ledger database file to open if none is given. Defaults to
    /// `<data-dir>/My-Personal-Ledger.pldb`.
    pub file: std::path::PathBuf,

    /// The tracing logging level for the application (see [`lib_tracing::Levels`]).
    pub log: lib_tracing::Levels,

    /// Optional path to a file that tracing output should also be written to, in addition
    /// to the console. Defaults to `None` (console output only) when the key is absent
    /// from every configuration source.
    #[serde(default)]
    pub log_file_path: Option<std::path::PathBuf>,

    /// The requested Locale as a BCP-47 tag (e.g. `en-GB`), from a configuration file,
    /// environment variable or `--locale`. Unset means "follow the operating system" -- use
    /// [`Self::resolved_locale`] rather than reading this directly.
    #[serde(default)]
    pub locale: Option<String>,

    /// The operating system's Locale, detected by `LedgerConfig::parse`. Never read from or
    /// written to a configuration source.
    #[serde(skip)]
    system_locale: Option<String>,
}

impl Default for PersonalLedgerConfig {
    fn default() -> Self {
        let data = Self::default_data_dir();
        let file = data.join(DEFAULT_FILE_NAME);

        Self {
            config: None,
            data,
            file,
            log: DEFAULT_LOG,
            log_file_path: None,
            locale: None,
            system_locale: None,
        }
    }
}

impl PersonalLedgerConfig {
    /// Returns a reference to the configuration file path, if one was configured.
    pub fn config(&self) -> Option<&std::path::Path> {
        self.config.as_deref()
    }

    /// Returns the configured data directory.
    pub fn data(&self) -> &std::path::Path {
        &self.data
    }

    /// Returns the configured Personal Ledger database file.
    pub fn file(&self) -> &std::path::Path {
        &self.file
    }

    /// Returns the configured logging level.
    pub fn log(&self) -> lib_tracing::Levels {
        self.log
    }

    /// Returns the configured log file path, if any.
    ///
    /// `None` means console output only -- [`lib_tracing::init`] doesn't also write to a
    /// file.
    pub fn log_file_path(&self) -> Option<&std::path::Path> {
        self.log_file_path.as_deref()
    }

    /// Returns the Locale set by configuration, if any (canonical casing once parsed).
    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
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
        match self.locale.as_deref() {
            Some(value) => self.locale = Some(parse_locale_tag(value)?),
            None => self.system_locale = detect().and_then(|value| usable_system_locale(&value)),
        }
        Ok(())
    }

    /// The default data directory: the user's document folder, falling back to the current
    /// working directory when it can't be determined (e.g. `$HOME` unset).
    fn default_data_dir() -> std::path::PathBuf {
        dirs::document_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
    }

    /// Get the default configuration values as key-value pairs, for seeding a layered
    /// configuration builder's defaults -- mirrors `SyncServerConfig::default_config_values()`.
    ///
    /// `config`, `log_file_path` and `locale` are excluded: none has a meaningful default (see
    /// [`Self::config`]/[`Self::log_file_path`]) -- all default to `None` when absent from
    /// every configuration source. `locale` in particular must stay unset so the operating
    /// system's Locale can outrank [`DEFAULT_LOCALE`] (see [`Self::resolved_locale`]).
    pub fn default_config_values() -> Vec<(&'static str, String)> {
        let default_config = Self::default();
        vec![
            (
                "personal_ledger.data",
                default_config.data().to_string_lossy().into_owned(),
            ),
            (
                "personal_ledger.file",
                default_config.file().to_string_lossy().into_owned(),
            ),
            ("personal_ledger.log", default_config.log().to_string()),
        ]
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
    use super::*;

    #[test]
    fn default_config_has_expected_log_level() {
        let config = PersonalLedgerConfig::default();
        assert_eq!(config.log(), lib_tracing::Levels::INFO);
    }

    #[test]
    fn default_config_has_no_explicit_config_path() {
        let config = PersonalLedgerConfig::default();
        assert_eq!(config.config(), None);
    }

    #[test]
    fn default_config_has_no_log_file_path() {
        let config = PersonalLedgerConfig::default();
        assert_eq!(config.log_file_path(), None);
    }

    #[test]
    fn log_file_path_returns_configured_path() {
        let config = PersonalLedgerConfig {
            log_file_path: Some(std::path::PathBuf::from("/var/log/personal-ledger.log")),
            ..PersonalLedgerConfig::default()
        };
        assert_eq!(
            config.log_file_path(),
            Some(std::path::Path::new("/var/log/personal-ledger.log"))
        );
    }

    #[test]
    fn default_file_is_named_correctly_inside_data_dir() {
        let config = PersonalLedgerConfig::default();
        assert_eq!(config.file(), config.data().join(DEFAULT_FILE_NAME));
    }

    #[test]
    fn default_config_values_returns_expected_pairs() {
        let defaults = PersonalLedgerConfig::default_config_values();
        assert_eq!(defaults.len(), 3);
        assert!(defaults.iter().any(|(k, _)| *k == "personal_ledger.data"));
        assert!(defaults.iter().any(|(k, _)| *k == "personal_ledger.file"));
        assert_eq!(
            defaults
                .iter()
                .find(|(k, _)| *k == "personal_ledger.log")
                .map(|(_, v)| v.as_str()),
            Some("info")
        );
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = PersonalLedgerConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: PersonalLedgerConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
