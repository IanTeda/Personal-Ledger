//! The Configuration key catalogue's file side (#1153, ADR-0058): every `(section, key)`
//! `lib-config` parses ([`keys`]), and for each key Settings writes, `lib_core::configuration`'s
//! [`ConfigKey`], its environment variable and, for its [`ConfigValue`], how the value is spelt in
//! the file and read back.
//!
//! A key Settings writes is a row of `lib_core::configuration`'s `config_keys!` table, which
//! names its [`ConfigKey`], section and [`ConfigValue`]; [`keys`] lists it from there. Here, beside
//! its struct field in `general.rs`, `display.rs` or `logs.rs`, it adds only its spelling in
//! [`spelt`] and its read in [`read_value`], whose matches won't compile without them. Neither
//! Client spells a value: each hands [`crate::ConfigWriter::set`] a typed [`ConfigValue`].
//!
//! [`keys`]' section names are the normalised spelling (lower case, `_` for `-`). `[Keybindings]`'
//! command bindings are an open-ended map and are not listed: only `super_key` is a fixed key.

use lib_core::configuration::{ConfigKey, ConfigValue, Section};

/// The keys of each section, beside the structs that read them: a struct field added without a
/// row here fails `ledger.rs`' `key_table_matches_the_structs`. The sections Settings writes are
/// derived, their [`ConfigKey`]s and then their [`NOT_WRITTEN`] keys; the rest are [`HAND_WRITTEN`].
pub(crate) fn keys() -> Vec<(&'static str, Vec<&'static str>)> {
    Section::ALL
        .into_iter()
        .map(|section| {
            (
                normalised(section),
                in_section(section).map(|(_, name)| name).collect(),
            )
        })
        .chain(
            HAND_WRITTEN
                .iter()
                .map(|(section, keys)| (*section, keys.to_vec())),
        )
        .collect()
}

/// The keys of the sections Settings never writes, edited only by hand.
const HAND_WRITTEN: &[(&str, &[&str])] = &[
    ("keybindings", &["super_key"]),
    ("secrets", &["api_key_secret"]),
    ("sync_server", &["bind_address", "database_uri"]),
    ("theme", THEME_ROLES),
    ("theme.light", THEME_ROLES),
    ("theme.dark", THEME_ROLES),
];

/// The keys of the sections Settings writes that it never writes: `file` is set only by hand or
/// `--file`. Every other key of those sections is a [`ConfigKey`].
const NOT_WRITTEN: &[KeyId] = &[FILE];

/// A key of a section Settings writes, as `(section, name)`: a [`ConfigKey`]'s, or [`FILE`]. What
/// the overrides report ([`crate::Overrides`]) is kept by, so a flag on a key Settings never writes
/// still reports itself.
pub(crate) type KeyId = (Section, &'static str);

/// `[General]`'s `file`, which only a hand or `--file` sets.
pub(crate) const FILE: KeyId = (Section::General, "file");

/// The keys of `section`: its [`ConfigKey`]s in [`ConfigKey::ALL`]'s order, then its
/// [`NOT_WRITTEN`] keys.
fn in_section(section: Section) -> impl Iterator<Item = KeyId> {
    ConfigKey::ALL
        .into_iter()
        .map(id)
        .chain(NOT_WRITTEN.iter().copied())
        .filter(move |(of, _)| *of == section)
}

/// Every key of the sections Settings writes: the keys the overrides report covers.
pub(crate) fn reported() -> impl Iterator<Item = KeyId> {
    Section::ALL.into_iter().flat_map(in_section)
}

/// `id`'s dotted path, as the parser keys a value: `general.toasts`.
pub(crate) fn path((section, name): KeyId) -> String {
    format!("{}.{name}", normalised(section))
}

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

/// `section`'s name as `normalise_ini` reads its header, as [`keys`] spells it, and as its
/// environment variables spell it, upper-cased.
pub(crate) fn normalised(section: Section) -> &'static str {
    match section {
        Section::General => "general",
        Section::Display => "display",
        Section::Logs => "logs",
    }
}

/// `key` as the overrides report keeps it.
pub(crate) fn id(key: ConfigKey) -> KeyId {
    (key.section(), key.name())
}

/// The environment variable that sets `key`, spelt by the parse's own `LedgerConfig::env_var`:
/// `PERSONAL_LEDGER_<SECTION>__<KEY>`.
pub fn env_var(key: ConfigKey) -> String {
    crate::ledger::LedgerConfig::env_var(id(key))
}

/// `value` as the file spells it, by the same functions the section structs' serde helpers read it
/// back with, before the writer escapes it.
pub(crate) fn spelt(value: &ConfigValue) -> String {
    match value {
        ConfigValue::OnStart(on_start) => crate::on_start_key(*on_start).to_string(),
        ConfigValue::StartView(start_view) => start_view.key().to_string(),
        ConfigValue::Toasts(on)
        | ConfigValue::StartSidebarMinimised(on)
        | ConfigValue::HintBar(on)
        | ConfigValue::TerminalColours(on) => on.to_string(),
        ConfigValue::Locale(tag) | ConfigValue::ColourTheme(tag) => tag.clone(),
        ConfigValue::Data(path) | ConfigValue::LogFilePath(path) => path.display().to_string(),
        ConfigValue::ColourAppearance(appearance) => appearance.key().to_string(),
        ConfigValue::DateStyle(style) => crate::date_style_key(*style).to_string(),
        ConfigValue::RowDensity(density) => density.key().to_string(),
        ConfigValue::StatusGlyphs(glyphs) => glyphs.key().to_string(),
        ConfigValue::Log(level) => level.to_string(),
    }
}

/// The value `config` holds for `key`, as the parser read it: `None` only for the keys with no
/// default, unset (the Locale, the Colour Theme and the log file).
pub fn read_value(key: ConfigKey, config: &crate::Config) -> Option<ConfigValue> {
    let general = config.general_config();
    let display = config.display_config();
    let logs = config.logs_config();
    Some(match key {
        ConfigKey::OnStart => ConfigValue::OnStart(general.on_start()),
        ConfigKey::StartView => ConfigValue::StartView(general.start_view().clone()),
        ConfigKey::Toasts => ConfigValue::Toasts(general.toasts()),
        ConfigKey::Locale => ConfigValue::Locale(general.locale()?.to_string()),
        ConfigKey::Data => ConfigValue::Data(general.data().to_path_buf()),
        ConfigKey::ColourTheme => ConfigValue::ColourTheme(display.colour_theme()?.to_string()),
        ConfigKey::ColourAppearance => ConfigValue::ColourAppearance(display.colour_appearance()),
        ConfigKey::DateStyle => ConfigValue::DateStyle(display.date_style()),
        ConfigKey::RowDensity => ConfigValue::RowDensity(display.row_density()),
        ConfigKey::StatusGlyphs => ConfigValue::StatusGlyphs(display.status_glyphs()),
        ConfigKey::StartSidebarMinimised => {
            ConfigValue::StartSidebarMinimised(display.start_sidebar_minimised())
        }
        ConfigKey::HintBar => ConfigValue::HintBar(display.hint_bar()),
        ConfigKey::TerminalColours => ConfigValue::TerminalColours(display.terminal_colours()),
        ConfigKey::Log => ConfigValue::Log(logs.log()),
        ConfigKey::LogFilePath => ConfigValue::LogFilePath(logs.log_file_path()?.to_path_buf()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
                (Section::Display, "start_sidebar_minimised"),
                (Section::Display, "hint_bar"),
                (Section::Display, "terminal_colours"),
                (Section::Logs, "log"),
                (Section::Logs, "log_file_path"),
            ]
        );
        assert_eq!(
            env_var(ConfigKey::Toasts),
            "PERSONAL_LEDGER_GENERAL__TOASTS"
        );
        assert_eq!(
            env_var(ConfigKey::RowDensity),
            "PERSONAL_LEDGER_DISPLAY__ROW_DENSITY"
        );
        assert_eq!(env_var(ConfigKey::Log), "PERSONAL_LEDGER_LOGS__LOG");
    }

    #[test]
    fn every_reported_key_s_variable_is_read_by_the_parse_as_that_key() {
        for id in reported() {
            let var = crate::ledger::LedgerConfig::env_var(id);
            assert_eq!(
                crate::ledger::LedgerConfig::env_key(&var, "x"),
                Some(path(id)),
                "{var}"
            );
        }
        assert!(reported().any(|id| id == FILE));
        for key in ConfigKey::ALL {
            assert!(reported().any(|id| id == super::id(key)), "{key:?}");
        }
    }

    #[test]
    fn no_key_is_both_written_and_not_written() {
        for key in ConfigKey::ALL {
            assert!(
                !NOT_WRITTEN.contains(&id(key)),
                "{key:?} is a ConfigKey and in NOT_WRITTEN, so the table would list it twice"
            );
        }
    }

    #[test]
    fn every_value_names_the_key_it_reads_back_under() {
        let config = crate::Config::default();
        for key in ConfigKey::ALL {
            if let Some(value) = read_value(key, &config) {
                assert_eq!(value.key(), key);
            }
        }
    }
}
