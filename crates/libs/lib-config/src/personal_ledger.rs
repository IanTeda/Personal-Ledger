//! # Personal Ledger Section Configuration
//!
//! Configuration for the `[Personal-Ledger]` section: settings applied across Personal
//! Ledger files and the client on a given system, but not intended to sync across computer
//! systems (see `docs/settings.md`). A device's settings are among them (ADR-0053): Toasts, the
//! Colour Theme and Appearance, the date style, Row density, Status glyphs, On start and Start
//! view, each spelled in the file as `:set` takes it.

use lib_colour_theme::ColourAppearance;
use lib_core::{DateStyle, OnStart, RowDensity, StartView, StatusGlyphs};

/// Default logging level for the `[Personal-Ledger]` section.
const DEFAULT_LOG: lib_tracing::Levels = lib_tracing::Levels::INFO;

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

    /// The folder `:open`'s file browser starts in. Defaults to the user's document folder.
    pub data: std::path::PathBuf,

    /// A Ledger file to open at start, overriding On start. Optional, with no
    /// default: unset, the Client opens the Ledger On start names, or none (ADR-0051).
    #[serde(default)]
    pub file: Option<std::path::PathBuf>,

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

    /// Draw with the terminal's own ANSI colours instead of the Colour Theme (ADR-0024).
    /// Read only by the TUI; Configuration, so it never syncs.
    #[serde(default)]
    pub terminal_colours: bool,

    /// Draw status glyphs as Unicode or plain ASCII: `unicode`, the default, or `ascii`. Both
    /// Clients read and write it.
    #[serde(default, with = "status_glyphs")]
    pub status_glyphs: StatusGlyphs,

    /// How tightly a table's rows are spaced: `compact`, `regular`, the default, or `roomy`. Read
    /// only by the Desktop, which has the tables it spaces.
    #[serde(default, with = "row_density")]
    pub row_density: RowDensity,

    /// Whether the TUI's footer shows the key hints. Read only by the TUI; Configuration.
    #[serde(default = "shown")]
    pub hint_bar: bool,

    /// Whether non-Error Toasts show (ADR-0027). Both Clients; on by default.
    #[serde(default = "shown")]
    pub toasts: bool,

    /// The chosen Colour Theme's id. Unset is the default Colour Theme; kept raw, so an id from a
    /// newer release survives a run of this one (ADR-0023).
    #[serde(default)]
    pub colour_theme: Option<String>,

    /// Which Colour Variant to draw: `light`, `dark` or `system`, the default.
    #[serde(default, with = "appearance")]
    pub colour_appearance: ColourAppearance,

    /// How dates show: `short`, `medium`, `long` or `iso`, or `default`, the Locale's (ADR-0021),
    /// which unset means too.
    #[serde(default, with = "date_style")]
    pub date_style: Option<DateStyle>,

    /// Which Ledger file a Client opens as it starts: `reopen`, the default, or `none`
    /// (ADR-0051). `file` outranks it.
    #[serde(default, with = "on_start")]
    pub on_start: OnStart,

    /// The View a Client lands on as it starts: `dashboard`, the default, `last`, `settings` or a
    /// noun, kept raw so a noun only the other Client has survives (ADR-0053).
    #[serde(default, with = "start_view")]
    pub start_view: StartView,
}

/// The value `date_style` and `:set date-style` spell the Locale's default with.
pub const DATE_STYLE_DEFAULT: &str = "default";

/// The value the file and `:set date-style` spell `style` with.
pub fn date_style_key(style: Option<DateStyle>) -> &'static str {
    style.map_or(DATE_STYLE_DEFAULT, |style| style.as_str())
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
fn by_key<'de, D, T>(deserializer: D, parse: impl Fn(&str) -> Option<T>) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let key = <String as serde::Deserialize>::deserialize(deserializer)?;
    parse(&key).ok_or_else(|| serde::de::Error::custom(format!("unknown value `{key}`")))
}

/// `colour_appearance` by [`ColourAppearance::key`].
mod appearance {
    use super::ColourAppearance;

    pub fn serialize<S: serde::Serializer>(
        value: &ColourAppearance,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.key())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ColourAppearance, D::Error> {
        super::by_key(deserializer, ColourAppearance::from_key)
    }
}

/// `date_style` by [`super::date_style_key`].
mod date_style {
    use super::DateStyle;

    pub fn serialize<S: serde::Serializer>(
        value: &Option<DateStyle>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(super::date_style_key(*value))
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<DateStyle>, D::Error> {
        super::by_key(deserializer, |key| {
            let key = key.trim();
            if key.eq_ignore_ascii_case(super::DATE_STYLE_DEFAULT) {
                Some(None)
            } else {
                key.parse().ok().map(Some)
            }
        })
    }
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

/// `status_glyphs` by [`StatusGlyphs::key`].
mod status_glyphs {
    use super::StatusGlyphs;

    pub fn serialize<S: serde::Serializer>(
        value: &StatusGlyphs,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.key())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<StatusGlyphs, D::Error> {
        super::by_key(deserializer, StatusGlyphs::from_key)
    }
}

/// `row_density` by [`RowDensity::key`].
mod row_density {
    use super::RowDensity;

    pub fn serialize<S: serde::Serializer>(
        value: &RowDensity,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.key())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<RowDensity, D::Error> {
        super::by_key(deserializer, RowDensity::from_key)
    }
}

/// The hint bar's default: shown.
fn shown() -> bool {
    true
}

impl Default for PersonalLedgerConfig {
    fn default() -> Self {
        Self {
            config: None,
            data: Self::default_data_dir(),
            file: None,
            log: DEFAULT_LOG,
            log_file_path: None,
            locale: None,
            system_locale: None,
            terminal_colours: false,
            status_glyphs: StatusGlyphs::Unicode,
            row_density: RowDensity::Regular,
            hint_bar: true,
            toasts: true,
            colour_theme: None,
            colour_appearance: ColourAppearance::System,
            date_style: None,
            on_start: OnStart::ReopenLastLedger,
            start_view: StartView::Dashboard,
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

    /// Returns the Ledger file explicitly configured to open at start, if any.
    pub fn file(&self) -> Option<&std::path::Path> {
        self.file.as_deref()
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

    /// Whether the TUI draws with the terminal's own colours rather than the Colour Theme.
    pub fn terminal_colours(&self) -> bool {
        self.terminal_colours
    }

    /// How status glyphs are drawn.
    pub fn status_glyphs(&self) -> StatusGlyphs {
        self.status_glyphs
    }

    /// How tightly the Desktop spaces a table's rows.
    pub fn row_density(&self) -> RowDensity {
        self.row_density
    }

    /// Whether the TUI's footer shows the key hints.
    pub fn hint_bar(&self) -> bool {
        self.hint_bar
    }

    /// Whether non-Error Toasts show.
    pub fn toasts(&self) -> bool {
        self.toasts
    }

    /// The chosen Colour Theme's id, or `None` for the default Colour Theme.
    pub fn colour_theme(&self) -> Option<&str> {
        self.colour_theme.as_deref()
    }

    /// Which Colour Variant to draw.
    pub fn colour_appearance(&self) -> ColourAppearance {
        self.colour_appearance
    }

    /// The date style, or `None` for the Locale's default.
    pub fn date_style(&self) -> Option<DateStyle> {
        self.date_style
    }

    /// Which Ledger file a Client opens as it starts, when `file` doesn't name one.
    pub fn on_start(&self) -> OnStart {
        self.on_start
    }

    /// The View a Client lands on as it starts.
    pub fn start_view(&self) -> &StartView {
        &self.start_view
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
    /// `config`, `file`, `log_file_path` and `locale` are excluded: none has a meaningful
    /// default (see [`Self::config`]/[`Self::file`]/[`Self::log_file_path`]) -- all default to
    /// `None` when absent from every configuration source. `locale` in particular must stay unset so the operating
    /// system's Locale can outrank [`DEFAULT_LOCALE`] (see [`Self::resolved_locale`]).
    pub fn default_config_values() -> Vec<(&'static str, String)> {
        let default_config = Self::default();
        vec![
            (
                "personal_ledger.data",
                default_config.data().to_string_lossy().into_owned(),
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
    use std::collections::HashMap;

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
    fn default_config_has_no_file() {
        let config = PersonalLedgerConfig::default();
        assert_eq!(config.file(), None);
    }

    #[test]
    fn default_config_values_returns_expected_pairs() {
        let defaults = PersonalLedgerConfig::default_config_values();
        assert_eq!(defaults.len(), 2);
        assert!(defaults.iter().any(|(k, _)| *k == "personal_ledger.data"));
        assert!(!defaults.iter().any(|(k, _)| *k == "personal_ledger.file"));
        assert_eq!(
            defaults
                .iter()
                .find(|(k, _)| *k == "personal_ledger.log")
                .map(|(_, v)| v.as_str()),
            Some("info")
        );
    }

    /// The `[Personal-Ledger]` section `text` and `env` give, with no system Locale.
    fn parsed(text: &str, env: &[(&str, &str)]) -> PersonalLedgerConfig {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        std::fs::write(&path, text).unwrap();
        let env = env
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        crate::Config::parse_with_detector(Some(&path), Some(env), || None)
            .unwrap()
            .personal_ledger_config()
            .clone()
    }

    #[test]
    fn the_device_settings_default_to_toasts_on_the_default_theme_system_locale_style_and_reopen() {
        let section = parsed("[Personal-Ledger]\n", &[]);
        assert!(section.toasts());
        assert_eq!(section.colour_theme(), None);
        assert_eq!(section.colour_appearance(), ColourAppearance::System);
        assert_eq!(section.date_style(), None);
        assert_eq!(section.on_start(), OnStart::ReopenLastLedger);
        assert_eq!(section.start_view(), &StartView::Dashboard);
    }

    #[test]
    fn start_view_reads_its_three_words_or_a_noun_and_an_environment_variable_outranks_it() {
        for (text, view) in [
            ("last", StartView::Last),
            ("Settings", StartView::Settings),
            ("dashboard", StartView::Dashboard),
            ("Units", StartView::Noun("units".into())),
        ] {
            let section = parsed(&format!("[Personal-Ledger]\nstart_view = {text}\n"), &[]);
            assert_eq!(section.start_view(), &view, "{text}");
        }
        let section = parsed(
            "[Personal-Ledger]\nstart_view = last\n",
            &[("PERSONAL_LEDGER_PERSONAL_LEDGER__START_VIEW", "tags")],
        );
        assert_eq!(section.start_view(), &StartView::Noun("tags".into()));
    }

    #[test]
    fn the_device_settings_read_from_the_file_as_set_spells_them() {
        let section = parsed(
            "[Personal-Ledger]\ntoasts = false\ncolour_theme = nord\ncolour_appearance = Light\n\
             date_style = ISO\non_start = none\n",
            &[],
        );
        assert!(!section.toasts());
        assert_eq!(section.colour_theme(), Some("nord"));
        assert_eq!(section.colour_appearance(), ColourAppearance::Light);
        assert_eq!(section.date_style(), Some(DateStyle::Iso));
        assert_eq!(section.on_start(), OnStart::NoLedger);

        let section = parsed(
            "[Personal-Ledger]\ndate_style = default\non_start = reopen\n",
            &[],
        );
        assert_eq!(section.date_style(), None);
        assert_eq!(section.on_start(), OnStart::ReopenLastLedger);
    }

    #[test]
    fn an_environment_variable_outranks_the_file_for_a_device_setting() {
        let section = parsed(
            "[Personal-Ledger]\non_start = none\ndate_style = long\n",
            &[
                ("PERSONAL_LEDGER_PERSONAL_LEDGER__ON_START", "reopen"),
                ("PERSONAL_LEDGER_PERSONAL_LEDGER__TOASTS", "false"),
                ("PERSONAL_LEDGER_PERSONAL_LEDGER__COLOUR_APPEARANCE", "dark"),
            ],
        );
        assert_eq!(section.on_start(), OnStart::ReopenLastLedger);
        assert!(!section.toasts());
        assert_eq!(section.colour_appearance(), ColourAppearance::Dark);
        assert_eq!(section.date_style(), Some(DateStyle::Long));
    }

    #[test]
    fn row_density_and_status_glyphs_default_to_regular_and_unicode() {
        let section = parsed("[Personal-Ledger]\n", &[]);
        assert_eq!(section.row_density(), RowDensity::Regular);
        assert_eq!(section.status_glyphs(), StatusGlyphs::Unicode);
    }

    #[test]
    fn row_density_and_status_glyphs_read_as_the_file_spells_them_in_any_case() {
        for (text, density, glyphs) in [
            ("compact", RowDensity::Compact, "ascii"),
            ("Roomy", RowDensity::Roomy, "ASCII"),
            ("regular", RowDensity::Regular, "Ascii"),
        ] {
            let section = parsed(
                &format!("[Personal-Ledger]\nrow_density = {text}\nstatus_glyphs = {glyphs}\n"),
                &[],
            );
            assert_eq!(section.row_density(), density, "{text}");
            assert_eq!(
                section.status_glyphs(),
                StatusGlyphs::AsciiFallback,
                "{glyphs}"
            );
        }
        let section = parsed("[Personal-Ledger]\nstatus_glyphs = unicode\n", &[]);
        assert_eq!(section.status_glyphs(), StatusGlyphs::Unicode);
    }

    #[test]
    fn an_environment_variable_outranks_the_file_for_row_density_and_status_glyphs() {
        let section = parsed(
            "[Personal-Ledger]\nrow_density = compact\nstatus_glyphs = ascii\n",
            &[
                ("PERSONAL_LEDGER_PERSONAL_LEDGER__ROW_DENSITY", "roomy"),
                ("PERSONAL_LEDGER_PERSONAL_LEDGER__STATUS_GLYPHS", "unicode"),
            ],
        );
        assert_eq!(section.row_density(), RowDensity::Roomy);
        assert_eq!(section.status_glyphs(), StatusGlyphs::Unicode);
    }

    #[test]
    fn a_device_setting_value_it_doesnt_take_is_a_configuration_error() {
        for text in [
            "[Personal-Ledger]\non_start = sometimes\n",
            "[Personal-Ledger]\ndate_style = fancy\n",
            "[Personal-Ledger]\ncolour_appearance = sepia\n",
            "[Personal-Ledger]\nrow_density = cosy\n",
            "[Personal-Ledger]\nstatus_glyphs = ascii fallback\n",
        ] {
            let dir = tempfile::TempDir::new().unwrap();
            let path = dir.path().join("personal-ledger.conf");
            std::fs::write(&path, text).unwrap();
            assert!(
                crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None)
                    .is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = PersonalLedgerConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: PersonalLedgerConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
