//! The Configuration key catalogue (#1153): every `(section, key)` `lib-config` parses ([`KEYS`]),
//! and of those the keys Settings writes ([`ConfigKey`]), each with its section, its environment
//! variable and, as a [`ConfigValue`], how its value is written in the file and read back.
//!
//! A key Settings writes is added here, to [`ConfigKey`] and [`ConfigValue`], beside its struct
//! field in `general.rs`, `display.rs` or `logs.rs` and its row in [`KEYS`]; the tests below fail
//! until the four agree. Neither Client spells a value: each hands [`crate::ConfigWriter::set`] a
//! typed [`ConfigValue`].
//!
//! [`KEYS`]' section names are the normalised spelling (lower case, `_` for `-`). `[Keybindings]`'
//! command bindings are an open-ended map and are not listed: only `super_key` is a fixed key.

use std::path::PathBuf;

use lib_colour_theme::ColourAppearance;
use lib_core::{DateStyle, OnStart, RowDensity, StartView, StatusGlyphs};

/// The keys of each section, beside the structs that read them: a struct field added without a
/// row here fails `table_matches_the_structs`.
pub(crate) const KEYS: &[(&str, &[&str])] = &[
    (
        "general",
        &["on_start", "start_view", "toasts", "locale", "data", "file"],
    ),
    (
        "display",
        &[
            "colour_theme",
            "colour_appearance",
            "date_style",
            "row_density",
            "status_glyphs",
            "hint_bar",
            "terminal_colours",
        ],
    ),
    ("logs", &["log", "log_file_path"]),
    ("keybindings", &["super_key"]),
    ("sync_server", &["bind_address", "database_uri"]),
    ("theme", THEME_ROLES),
    ("theme.light", THEME_ROLES),
    ("theme.dark", THEME_ROLES),
];

/// The keys of the sections Settings writes that it never writes: `file` is set only by hand or
/// `--file`. Every other key of those sections is a [`ConfigKey`].
const NOT_WRITTEN: &[(&str, &str)] = &[("general", "file")];

/// The seven stored Colour Roles (`lib_colour_theme::ColourRole::ALL`, checked by a test).
const THEME_ROLES: &[&str] = &[
    "foreground",
    "background",
    "accent",
    "cursor",
    "muted",
    "positive",
    "negative",
];

/// The sections Settings writes, one per THIS DEVICE page with keys (ADR-0057). `[Theme*]`,
/// `[Keybindings]` and `[Sync-Server]` are edited only by hand, so none is here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// `[General]`: Settings › General.
    General,
    /// `[Display]`: Settings › Display.
    Display,
    /// `[Logs]`: Settings › Logs.
    Logs,
}

impl Section {
    /// The section's header as a write adds it to a file that lacks it.
    pub fn header(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Display => "Display",
            Self::Logs => "Logs",
        }
    }

    /// The section's name as `normalise_ini` reads its header, as [`KEYS`] spells it, and as its
    /// environment variables spell it, upper-cased.
    pub(crate) fn normalised(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Display => "display",
            Self::Logs => "logs",
        }
    }
}

/// The keys Settings writes, each in its page's [`Section`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKey {
    /// `on_start`: `reopen` or `none`.
    OnStart,
    /// `start_view`: `dashboard`, `last`, `settings` or a noun.
    StartView,
    /// `toasts`: whether non-Error Toasts show.
    Toasts,
    /// `locale`: the Locale's BCP-47 tag; removed for System, the operating system's.
    Locale,
    /// `data`: the folder `:open` and `:new` start in.
    Data,
    /// `colour_theme`: the Colour Theme's id.
    ColourTheme,
    /// `colour_appearance`: `light`, `dark` or `system`.
    ColourAppearance,
    /// `date_style`: `default`, `short`, `medium`, `long` or `iso`.
    DateStyle,
    /// `row_density`: `compact`, `regular` or `roomy`.
    RowDensity,
    /// `status_glyphs`: `unicode` or `ascii`.
    StatusGlyphs,
    /// `hint_bar`: whether the footer shows the key hints.
    HintBar,
    /// `terminal_colours`: draw with the terminal's own colours (ADR-0024).
    TerminalColours,
    /// `log`: the tracing level.
    Log,
    /// `log_file_path`: a file the logs are also written to.
    LogFilePath,
}

impl ConfigKey {
    /// Every key Settings writes, in the order the file's sections and keys are documented.
    pub const ALL: [ConfigKey; 14] = [
        Self::OnStart,
        Self::StartView,
        Self::Toasts,
        Self::Locale,
        Self::Data,
        Self::ColourTheme,
        Self::ColourAppearance,
        Self::DateStyle,
        Self::RowDensity,
        Self::StatusGlyphs,
        Self::HintBar,
        Self::TerminalColours,
        Self::Log,
        Self::LogFilePath,
    ];

    /// The key as it is written in the file.
    pub fn name(self) -> &'static str {
        match self {
            Self::OnStart => "on_start",
            Self::StartView => "start_view",
            Self::Toasts => "toasts",
            Self::Locale => "locale",
            Self::Data => "data",
            Self::ColourTheme => "colour_theme",
            Self::ColourAppearance => "colour_appearance",
            Self::DateStyle => "date_style",
            Self::RowDensity => "row_density",
            Self::StatusGlyphs => "status_glyphs",
            Self::HintBar => "hint_bar",
            Self::TerminalColours => "terminal_colours",
            Self::Log => "log",
            Self::LogFilePath => "log_file_path",
        }
    }

    /// The section the key lives in, its Settings page's.
    pub fn section(self) -> Section {
        match self {
            Self::OnStart | Self::StartView | Self::Toasts | Self::Locale | Self::Data => {
                Section::General
            }
            Self::ColourTheme
            | Self::ColourAppearance
            | Self::DateStyle
            | Self::RowDensity
            | Self::StatusGlyphs
            | Self::HintBar
            | Self::TerminalColours => Section::Display,
            Self::Log | Self::LogFilePath => Section::Logs,
        }
    }

    /// The environment variable that sets the key, by `add_explicit_and_env`'s naming:
    /// `PERSONAL_LEDGER_<SECTION>__<KEY>`.
    pub fn env_var(self) -> String {
        format!(
            "{}_{}__{}",
            crate::ledger::LedgerConfig::env_prefix(),
            self.section().normalised().to_uppercase(),
            self.name().to_uppercase()
        )
    }
}

/// A value Settings writes, typed by its key, so a value the parser would refuse can't be spelt:
/// the variant names the [`ConfigKey`] and holds the type the parser reads it into. The Locale and
/// the Colour Theme are the open-ended two: a Locale tag is checked as it is written, and a Colour
/// Theme id is kept raw, as the parser keeps it (ADR-0023).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    OnStart(OnStart),
    StartView(StartView),
    Toasts(bool),
    /// A BCP-47 tag. **System** is no value: [`crate::ConfigWriter::remove`] takes the key out.
    Locale(String),
    Data(PathBuf),
    ColourTheme(String),
    ColourAppearance(ColourAppearance),
    /// `None` is the Locale's default, written `default`.
    DateStyle(Option<DateStyle>),
    RowDensity(RowDensity),
    StatusGlyphs(StatusGlyphs),
    HintBar(bool),
    TerminalColours(bool),
    Log(lib_tracing::Levels),
    LogFilePath(PathBuf),
}

impl ConfigValue {
    /// The key the value is written under.
    pub fn key(&self) -> ConfigKey {
        match self {
            Self::OnStart(_) => ConfigKey::OnStart,
            Self::StartView(_) => ConfigKey::StartView,
            Self::Toasts(_) => ConfigKey::Toasts,
            Self::Locale(_) => ConfigKey::Locale,
            Self::Data(_) => ConfigKey::Data,
            Self::ColourTheme(_) => ConfigKey::ColourTheme,
            Self::ColourAppearance(_) => ConfigKey::ColourAppearance,
            Self::DateStyle(_) => ConfigKey::DateStyle,
            Self::RowDensity(_) => ConfigKey::RowDensity,
            Self::StatusGlyphs(_) => ConfigKey::StatusGlyphs,
            Self::HintBar(_) => ConfigKey::HintBar,
            Self::TerminalColours(_) => ConfigKey::TerminalColours,
            Self::Log(_) => ConfigKey::Log,
            Self::LogFilePath(_) => ConfigKey::LogFilePath,
        }
    }

    /// The value as the file spells it, by the same functions the section structs' serde helpers
    /// read it back with, before the writer escapes it.
    pub(crate) fn spelt(&self) -> String {
        match self {
            Self::OnStart(on_start) => crate::on_start_key(*on_start).to_string(),
            Self::StartView(start_view) => start_view.key().to_string(),
            Self::Toasts(on) | Self::HintBar(on) | Self::TerminalColours(on) => on.to_string(),
            Self::Locale(tag) | Self::ColourTheme(tag) => tag.clone(),
            Self::Data(path) | Self::LogFilePath(path) => path.display().to_string(),
            Self::ColourAppearance(appearance) => appearance.key().to_string(),
            Self::DateStyle(style) => crate::date_style_key(*style).to_string(),
            Self::RowDensity(density) => density.key().to_string(),
            Self::StatusGlyphs(glyphs) => glyphs.key().to_string(),
            Self::Log(level) => level.to_string(),
        }
    }

    /// The value `config` holds for `key`, as the parser read it: `None` only for the keys with no
    /// default, unset (the Locale, the Colour Theme and the log file).
    pub fn read(key: ConfigKey, config: &crate::Config) -> Option<Self> {
        let general = config.general_config();
        let display = config.display_config();
        let logs = config.logs_config();
        Some(match key {
            ConfigKey::OnStart => Self::OnStart(general.on_start()),
            ConfigKey::StartView => Self::StartView(general.start_view().clone()),
            ConfigKey::Toasts => Self::Toasts(general.toasts()),
            ConfigKey::Locale => Self::Locale(general.locale()?.to_string()),
            ConfigKey::Data => Self::Data(general.data().to_path_buf()),
            ConfigKey::ColourTheme => Self::ColourTheme(display.colour_theme()?.to_string()),
            ConfigKey::ColourAppearance => Self::ColourAppearance(display.colour_appearance()),
            ConfigKey::DateStyle => Self::DateStyle(display.date_style()),
            ConfigKey::RowDensity => Self::RowDensity(display.row_density()),
            ConfigKey::StatusGlyphs => Self::StatusGlyphs(display.status_glyphs()),
            ConfigKey::HintBar => Self::HintBar(display.hint_bar()),
            ConfigKey::TerminalColours => Self::TerminalColours(display.terminal_colours()),
            ConfigKey::Log => Self::Log(logs.log()),
            ConfigKey::LogFilePath => Self::LogFilePath(logs.log_file_path()?.to_path_buf()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keys [`KEYS`] lists for `section`.
    fn listed(section: Section) -> &'static [&'static str] {
        KEYS.iter()
            .find(|(name, _)| *name == section.normalised())
            .map(|(_, keys)| *keys)
            .unwrap_or_else(|| panic!("[{}] has no row in KEYS", section.header()))
    }

    #[test]
    fn every_key_names_its_section_and_its_environment_variable() {
        assert_eq!(
            ConfigKey::ALL.map(|key| (key.section(), key.name())),
            [
                (Section::General, "on_start"),
                (Section::General, "start_view"),
                (Section::General, "toasts"),
                (Section::General, "locale"),
                (Section::General, "data"),
                (Section::Display, "colour_theme"),
                (Section::Display, "colour_appearance"),
                (Section::Display, "date_style"),
                (Section::Display, "row_density"),
                (Section::Display, "status_glyphs"),
                (Section::Display, "hint_bar"),
                (Section::Display, "terminal_colours"),
                (Section::Logs, "log"),
                (Section::Logs, "log_file_path"),
            ]
        );
        assert_eq!(
            ConfigKey::Toasts.env_var(),
            "PERSONAL_LEDGER_GENERAL__TOASTS"
        );
        assert_eq!(
            ConfigKey::RowDensity.env_var(),
            "PERSONAL_LEDGER_DISPLAY__ROW_DENSITY"
        );
        assert_eq!(ConfigKey::Log.env_var(), "PERSONAL_LEDGER_LOGS__LOG");
    }

    #[test]
    fn every_written_key_is_one_the_table_lists_in_its_section() {
        for key in ConfigKey::ALL {
            assert!(
                listed(key.section()).contains(&key.name()),
                "{key:?} writes `{}` in [{}], which KEYS doesn't list there",
                key.name(),
                key.section().header()
            );
        }
    }

    #[test]
    fn every_key_the_table_lists_in_a_written_section_is_written_or_named_as_not() {
        for section in [Section::General, Section::Display, Section::Logs] {
            for name in listed(section) {
                let written = ConfigKey::ALL
                    .iter()
                    .any(|key| key.section() == section && key.name() == *name);
                let excused = NOT_WRITTEN.contains(&(section.normalised(), *name));
                assert!(
                    written != excused,
                    "[{}] {name}: a key the table lists is either a ConfigKey or in NOT_WRITTEN, \
                     never both or neither",
                    section.header()
                );
            }
        }
    }

    #[test]
    fn every_value_names_the_key_it_reads_back_under() {
        let config = crate::Config::default();
        for key in ConfigKey::ALL {
            if let Some(value) = ConfigValue::read(key, &config) {
                assert_eq!(value.key(), key);
            }
        }
    }
}
