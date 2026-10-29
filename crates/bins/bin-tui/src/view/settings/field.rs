//! The fields the form pages edit ([`SettingField`]), by the name `:set` takes, the values that
//! hold for the session or that `Shell` keeps for the state file ([`SettingsValues`]), and the Configuration Display writes back to
//! `personal-ledger.conf` ([`Configuration`], #904).
//!
//! Opening and saving a field is one [`Action::SetSetting`] with the field and an optional value:
//! `enter` on a row sends it without one, which opens the field in place, and the open field's
//! `enter` sends it with the value chosen, which saves. `:set <field> [value]` (#898) sends the
//! same `Action`, so a key and a command can't drift.
//!
//! [`Action::SetSetting`]: crate::view::Action::SetSetting

use chrono::{Datelike, Months, NaiveDate};
use lib_config::{ConfigKey, ConfigWriter, PersonalLedgerConfig, Shadow, StatusGlyphs};
use lib_core::{DateStyle, OnStart};

use super::SettingsPage;
use crate::msg;

/// Every field on General, Ledger and Display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingField {
    OnStart,
    Toasts,
    Locale,
    LedgerFolder,
    LedgerName,
    Owner,
    FinancialYearStart,
    BaseUnit,
    Appearance,
    ColourTheme,
    DateStyle,
    TerminalColours,
    StatusGlyphs,
    HintBar,
}

/// Why a field can't be edited here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOnly {
    /// The Locale is Configuration read at start-up (ADR-0021).
    Locale,
    /// `data`, the ledger folder, is Configuration read at start-up.
    LedgerFolder,
}

impl SettingField {
    /// Every field, General's, Ledger's then Display's, each in its page's row order.
    pub const ALL: [SettingField; 14] = [
        Self::OnStart,
        Self::Toasts,
        Self::Locale,
        Self::LedgerFolder,
        Self::LedgerName,
        Self::Owner,
        Self::FinancialYearStart,
        Self::BaseUnit,
        Self::Appearance,
        Self::ColourTheme,
        Self::DateStyle,
        Self::TerminalColours,
        Self::StatusGlyphs,
        Self::HintBar,
    ];

    /// The name `:set <field>` takes: command syntax the user types, so stable English rather
    /// than a Message, as command names are.
    pub fn command_name(self) -> &'static str {
        match self {
            Self::OnStart => "on-start",
            Self::LedgerFolder => "ledger-folder",
            Self::LedgerName => "ledger-name",
            Self::Owner => "owner",
            Self::FinancialYearStart => "fy-start",
            Self::BaseUnit => "base",
            Self::Appearance => "appearance",
            Self::ColourTheme => "colour-theme",
            Self::DateStyle => "date-style",
            Self::Locale => "locale",
            Self::TerminalColours => "terminal-colours",
            Self::StatusGlyphs => "glyphs",
            Self::Toasts => "toasts",
            Self::HintBar => "hint-bar",
        }
    }

    /// The field `:set <field>` names, ignoring case.
    pub fn from_command_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|field| field.command_name().eq_ignore_ascii_case(name))
    }

    /// The page the field's row is on.
    pub fn page(self) -> SettingsPage {
        match self {
            Self::OnStart | Self::Toasts | Self::Locale | Self::LedgerFolder => {
                SettingsPage::General
            }
            Self::LedgerName | Self::Owner | Self::FinancialYearStart | Self::BaseUnit => {
                SettingsPage::Ledger
            }
            _ => SettingsPage::Display,
        }
    }

    /// Why the field can't be edited, or `None` when it can. A Configuration row can be too,
    /// but whether it is depends on the files and environment ([`Configuration::refusal`]).
    pub fn read_only(self) -> Option<ReadOnly> {
        match self {
            Self::Locale => Some(ReadOnly::Locale),
            Self::LedgerFolder => Some(ReadOnly::LedgerFolder),
            _ => None,
        }
    }

    /// The `personal-ledger.conf` key the field writes, for a Configuration row.
    pub fn config_key(self) -> Option<ConfigKey> {
        match self {
            Self::TerminalColours => Some(ConfigKey::TerminalColours),
            Self::StatusGlyphs => Some(ConfigKey::StatusGlyphs),
            Self::HintBar => Some(ConfigKey::HintBar),
            _ => None,
        }
    }

    /// The row's label.
    pub fn label(self) -> String {
        match self {
            Self::OnStart => msg::tui_settings_general_on_start(),
            Self::LedgerFolder => msg::tui_settings_general_ledger_folder(),
            Self::LedgerName => msg::tui_settings_ledger_ledger_name(),
            Self::Owner => msg::tui_settings_ledger_owner(),
            Self::FinancialYearStart => msg::tui_settings_ledger_financial_year(),
            Self::BaseUnit => msg::tui_settings_ledger_base_unit(),
            Self::Appearance => msg::tui_settings_setting_colour_appearance(),
            Self::ColourTheme => msg::tui_settings_setting_colour_theme(),
            Self::DateStyle => msg::tui_settings_setting_date_style(),
            Self::Locale => msg::tui_settings_setting_locale(),
            Self::TerminalColours => msg::tui_settings_setting_terminal_colours(),
            Self::StatusGlyphs => msg::tui_settings_setting_status_glyphs(),
            Self::Toasts => lib_locale::msg::toast_setting_label(),
            Self::HintBar => msg::tui_settings_setting_hint_bar(),
        }
    }

    /// What the status line says when the field is opened or set: why it can't be.
    pub fn refusal(self) -> Option<String> {
        self.read_only()
            .map(|_| msg::tui_settings_read_only_restart(self.command_name()))
    }
}

/// The financial year's start months the field offers, by the value `:set fy-start` takes:
/// Australia's, the calendar year's and the United Kingdom's.
pub const FINANCIAL_YEAR_STARTS: [(&str, u32); 3] = [("july", 7), ("january", 1), ("april", 4)];

/// A financial year start month's label.
pub fn month_label(month: u32) -> String {
    match month {
        1 => msg::tui_settings_month_january(),
        4 => msg::tui_settings_month_april(),
        _ => msg::tui_settings_month_july(),
    }
}

/// The On start choices, in the order the field shows them, by the value `:set on-start` takes.
pub const ON_STARTS: [(&str, OnStart); 2] = [
    ("reopen", OnStart::ReopenLastLedger),
    ("none", OnStart::NoLedger),
];

/// An On start choice's label.
pub fn on_start_label(on_start: OnStart) -> String {
    match on_start {
        OnStart::ReopenLastLedger => msg::tui_settings_on_start_reopen(),
        OnStart::NoLedger => msg::tui_settings_on_start_none(),
    }
}

/// The date style choices, in the order the field cycles through them, by the value
/// `:set date-style` takes. `None` is the Locale's default (ADR-0021).
pub const DATE_STYLES: [(&str, Option<DateStyle>); 5] = [
    ("default", None),
    ("short", Some(DateStyle::Short)),
    ("medium", Some(DateStyle::Medium)),
    ("long", Some(DateStyle::Long)),
    ("iso", Some(DateStyle::Iso)),
];

/// The values `:set terminal-colours` and `:set hint-bar` take, as `:set toasts` does.
pub fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// The Configuration Display's rows edit, as read at start-up and written since, with what
/// outranks each key's write. `Shell` writes the file; the rows read this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    /// The user's `personal-ledger.conf`, which a row writes; `None` leaves the rows read-only.
    pub path: Option<std::path::PathBuf>,
    pub terminal_colours: bool,
    pub status_glyphs: StatusGlyphs,
    pub hint_bar: bool,
    /// The source above the file that sets each key, so a write there changes nothing. Read
    /// once at start-up, as the values are.
    pub shadows: Vec<(ConfigKey, Shadow)>,
}

impl Default for Configuration {
    /// `lib-config`'s defaults, with nowhere to write.
    fn default() -> Self {
        Self::from_config(&PersonalLedgerConfig::default(), &ConfigWriter::default())
    }
}

impl Configuration {
    pub fn from_config(config: &PersonalLedgerConfig, writer: &ConfigWriter) -> Self {
        Self {
            path: writer.target().map(std::path::Path::to_path_buf),
            terminal_colours: config.terminal_colours(),
            status_glyphs: config.status_glyphs(),
            hint_bar: config.hint_bar(),
            shadows: [
                ConfigKey::TerminalColours,
                ConfigKey::StatusGlyphs,
                ConfigKey::HintBar,
            ]
            .into_iter()
            .filter_map(|key| writer.shadow(key).map(|shadow| (key, shadow)))
            .collect(),
        }
    }

    /// The source that outranks `key`'s write, named as the row and the status line name it.
    pub fn shadow(&self, key: ConfigKey) -> Option<String> {
        self.shadows
            .iter()
            .find(|(shadowed, _)| *shadowed == key)
            .map(|(_, shadow)| match shadow {
                Shadow::Env(var) => var.clone(),
                Shadow::File(path) => path.display().to_string(),
            })
    }

    /// Why `field`'s Configuration row can't be written: there's no file to write, or a source
    /// above it sets the key. `None` for any other field.
    pub fn refusal(&self, field: SettingField) -> Option<String> {
        let key = field.config_key()?;
        if self.path.is_none() {
            return Some(msg::tui_settings_read_only_config(field.command_name()));
        }
        self.shadow(key)
            .map(|source| msg::tui_settings_set_shadowed(field.command_name(), &source))
    }

    /// The value `field` holds as the file spells it, from the value `:set` takes.
    pub fn file_value(field: SettingField, value: &str) -> Option<&'static str> {
        match field {
            SettingField::TerminalColours | SettingField::HintBar => match value {
                "on" => Some("true"),
                "off" => Some("false"),
                _ => None,
            },
            SettingField::StatusGlyphs => StatusGlyphs::from_key(value).map(StatusGlyphs::key),
            _ => None,
        }
    }
}

/// The values the form pages show, shared by every Settings View `Shell` opens.
///
/// Ledger name, owner and the financial year's start have no Preference columns yet, so they
/// start on TUI-019's worked values and hold for the session. The date style and On start are
/// Client-scoped Preferences (ADR-0051): `Shell` reads them back for the state file as it quits,
/// as it does the Colour Theme and Appearance from its `Colours`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsValues {
    pub ledger_name: String,
    pub owner: String,
    /// The month the financial year starts on, 1–12.
    pub financial_year_start: u32,
    pub date_style: Option<DateStyle>,
    pub on_start: OnStart,
    /// `data`, the folder `:open` and `:new` start in, as read at start-up.
    pub ledger_folder: std::path::PathBuf,
    /// The Ledger file On start's Reopen last Ledger would reopen were the TUI to quit now: the
    /// Open Ledger's, which `Shell` keeps here. `None` with no Ledger open.
    pub last_ledger: Option<std::path::PathBuf>,
    /// Not the session's: `Shell` writes it to the file, then sets it here for the rows.
    pub configuration: Configuration,
}

impl Default for SettingsValues {
    fn default() -> Self {
        Self {
            // wireframe: TUI-019's worked values, until the Ledger has Preference columns for them.
            ledger_name: "Teda household".into(),
            owner: "Ian Teda".into(),
            financial_year_start: 7,
            date_style: None,
            on_start: OnStart::default(),
            ledger_folder: PersonalLedgerConfig::default().data().to_path_buf(),
            last_ledger: None,
            configuration: Configuration::default(),
        }
    }
}

impl SettingsValues {
    /// Saves `value` to `field`, if `field` is one this holds and `value` is one it takes;
    /// `false` otherwise, so the caller can refuse it.
    pub fn set(&mut self, field: SettingField, value: &str) -> bool {
        match field {
            SettingField::LedgerName | SettingField::Owner => {
                let value = value.trim();
                if value.is_empty() {
                    return false;
                }
                let slot = if field == SettingField::LedgerName {
                    &mut self.ledger_name
                } else {
                    &mut self.owner
                };
                *slot = value.to_string();
            }
            SettingField::FinancialYearStart => {
                let Some((_, month)) = FINANCIAL_YEAR_STARTS
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(value))
                else {
                    return false;
                };
                self.financial_year_start = *month;
            }
            SettingField::OnStart => {
                let Some((_, on_start)) = ON_STARTS
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(value))
                else {
                    return false;
                };
                self.on_start = *on_start;
            }
            SettingField::DateStyle => {
                let Some((_, style)) = DATE_STYLES
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(value))
                else {
                    return false;
                };
                self.date_style = *style;
            }
            SettingField::TerminalColours | SettingField::HintBar => {
                let on = match value {
                    "on" => true,
                    "off" => false,
                    _ => return false,
                };
                if field == SettingField::TerminalColours {
                    self.configuration.terminal_colours = on;
                } else {
                    self.configuration.hint_bar = on;
                }
            }
            SettingField::StatusGlyphs => {
                let Some(glyphs) = StatusGlyphs::from_key(value) else {
                    return false;
                };
                self.configuration.status_glyphs = glyphs;
            }
            _ => return false,
        }
        true
    }

    /// The value `field` holds, as `:set` would take it.
    pub fn get(&self, field: SettingField) -> Option<String> {
        Some(match field {
            SettingField::LedgerName => self.ledger_name.clone(),
            SettingField::Owner => self.owner.clone(),
            SettingField::FinancialYearStart => FINANCIAL_YEAR_STARTS
                .iter()
                .find(|(_, month)| *month == self.financial_year_start)
                .map(|(name, _)| (*name).to_string())?,
            SettingField::OnStart => ON_STARTS
                .iter()
                .find(|(_, on_start)| *on_start == self.on_start)
                .map(|(name, _)| (*name).to_string())?,
            SettingField::DateStyle => DATE_STYLES
                .iter()
                .find(|(_, style)| *style == self.date_style)
                .map(|(name, _)| (*name).to_string())?,
            SettingField::TerminalColours => on_off(self.configuration.terminal_colours).into(),
            SettingField::StatusGlyphs => self.configuration.status_glyphs.key().into(),
            SettingField::HintBar => on_off(self.configuration.hint_bar).into(),
            _ => return None,
        })
    }
}

/// The first and last day of the financial year holding `today`, when years start on the first
/// of `start` (1–12).
pub fn financial_year(today: NaiveDate, start: u32) -> (NaiveDate, NaiveDate) {
    let year = if today.month() >= start {
        today.year()
    } else {
        today.year() - 1
    };
    let first = NaiveDate::from_ymd_opt(year, start, 1).unwrap_or(today);
    let last = first
        .checked_add_months(Months::new(12))
        .and_then(|next| next.pred_opt())
        .unwrap_or(today);
    (first, last)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("the test dates are real")
    }

    #[test]
    fn every_field_has_a_distinct_command_name_that_round_trips() {
        for field in SettingField::ALL {
            assert_eq!(
                SettingField::from_command_name(field.command_name()),
                Some(field)
            );
        }
        assert_eq!(
            SettingField::from_command_name("FY-Start"),
            Some(SettingField::FinancialYearStart)
        );
        assert_eq!(SettingField::from_command_name("nope"), None);
    }

    #[test]
    fn the_financial_year_holding_a_day_runs_from_its_start_month() {
        assert_eq!(
            financial_year(date(2026, 10, 28), 7),
            (date(2026, 7, 1), date(2027, 6, 30))
        );
        assert_eq!(
            financial_year(date(2026, 3, 1), 7),
            (date(2025, 7, 1), date(2026, 6, 30))
        );
        assert_eq!(
            financial_year(date(2026, 10, 28), 1),
            (date(2026, 1, 1), date(2026, 12, 31))
        );
        assert_eq!(
            financial_year(date(2026, 4, 1), 4),
            (date(2026, 4, 1), date(2027, 3, 31))
        );
    }

    #[test]
    fn the_session_values_take_only_the_values_their_fields_offer() {
        let mut values = SettingsValues::default();
        assert!(values.set(SettingField::FinancialYearStart, "january"));
        assert_eq!(values.financial_year_start, 1);
        assert!(!values.set(SettingField::FinancialYearStart, "march"));
        assert!(values.set(SettingField::DateStyle, "iso"));
        assert_eq!(values.date_style, Some(DateStyle::Iso));
        assert!(values.set(SettingField::DateStyle, "default"));
        assert!(values.set(SettingField::OnStart, "none"));
        assert_eq!(values.on_start, OnStart::NoLedger);
        assert_eq!(values.get(SettingField::OnStart).as_deref(), Some("none"));
        assert!(!values.set(SettingField::OnStart, "sometimes"));
        assert_eq!(values.date_style, None);
        assert!(!values.set(SettingField::Owner, "  "));
        assert!(values.set(SettingField::Owner, "Alex"));
        assert_eq!(values.get(SettingField::Owner).as_deref(), Some("Alex"));
        assert!(!values.set(SettingField::BaseUnit, "btc"), "not held here");
        assert!(values.set(SettingField::StatusGlyphs, "ascii"));
        assert!(values.set(SettingField::HintBar, "off"));
        assert!(!values.set(SettingField::TerminalColours, "maybe"));
        assert_eq!(
            values.get(SettingField::StatusGlyphs).as_deref(),
            Some("ascii")
        );
        assert_eq!(values.get(SettingField::HintBar).as_deref(), Some("off"));
    }

    #[test]
    fn a_configuration_row_is_refused_with_no_file_or_under_a_higher_source() {
        crate::locale::init_for_tests();
        let mut configuration = Configuration::default();
        assert!(
            configuration
                .refusal(SettingField::HintBar)
                .is_some_and(|reason| reason.contains("no user config directory"))
        );
        configuration.path = Some("/home/someone/.config/personal-ledger.conf".into());
        assert_eq!(configuration.refusal(SettingField::HintBar), None);
        assert_eq!(configuration.refusal(SettingField::DateStyle), None);
        configuration.shadows = vec![(
            ConfigKey::HintBar,
            Shadow::Env("PERSONAL_LEDGER_PERSONAL_LEDGER__HINT_BAR".into()),
        )];
        assert!(
            configuration
                .refusal(SettingField::HintBar)
                .is_some_and(|reason| reason.contains("PERSONAL_LEDGER_PERSONAL_LEDGER__HINT_BAR"))
        );
        assert_eq!(
            Configuration::file_value(SettingField::TerminalColours, "on"),
            Some("true")
        );
        assert_eq!(
            Configuration::file_value(SettingField::StatusGlyphs, "ASCII"),
            Some("ascii")
        );
    }
}
