//! The fields the form pages edit ([`SettingField`]), by the name `:set` takes, the values that
//! hold for the session ([`SettingsValues`]), and the Configuration General and Display write back
//! to `personal-ledger.conf` ([`Configuration`], #904, ADR-0053).
//!
//! Opening and saving a field is one [`Action::SetSetting`] with the field and an optional value:
//! `enter` on a row sends it without one, which opens the field in place, and the open field's
//! `enter` sends it with the value chosen, which saves. `:set <field> [value]` (#898) sends the
//! same `Action`, so a key and a command can't drift.
//!
//! [`Action::SetSetting`]: crate::view::Action::SetSetting

use chrono::{Datelike, Months, NaiveDate};
use lib_colour_theme::ColourAppearance;
use lib_config::{ConfigKey, ConfigValue, ConfigWriter, Overrides, StatusGlyphs};
use lib_core::{DateStyle, OnStart, StartView};
use lib_locale::Locale;

use super::SettingsPage;
use crate::msg;

/// Every field on General, Ledger and Display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingField {
    OnStart,
    StartView,
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

impl SettingField {
    /// Every field, General's, Ledger's then Display's, each in its page's row order.
    pub const ALL: [SettingField; 15] = [
        Self::OnStart,
        Self::StartView,
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
            Self::StartView => "start-view",
            Self::LedgerFolder => "data",
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
            Self::OnStart | Self::StartView | Self::Toasts | Self::Locale | Self::LedgerFolder => {
                SettingsPage::General
            }
            Self::LedgerName | Self::Owner | Self::FinancialYearStart | Self::BaseUnit => {
                SettingsPage::Ledger
            }
            _ => SettingsPage::Display,
        }
    }

    /// The `personal-ledger.conf` key the field writes, for a Configuration row.
    pub fn config_key(self) -> Option<ConfigKey> {
        match self {
            Self::TerminalColours => Some(ConfigKey::TerminalColours),
            Self::StatusGlyphs => Some(ConfigKey::StatusGlyphs),
            Self::HintBar => Some(ConfigKey::HintBar),
            Self::Toasts => Some(ConfigKey::Toasts),
            Self::ColourTheme => Some(ConfigKey::ColourTheme),
            Self::Appearance => Some(ConfigKey::ColourAppearance),
            Self::DateStyle => Some(ConfigKey::DateStyle),
            Self::OnStart => Some(ConfigKey::OnStart),
            Self::StartView => Some(ConfigKey::StartView),
            Self::Locale => Some(ConfigKey::Locale),
            Self::LedgerFolder => Some(ConfigKey::Data),
            _ => None,
        }
    }

    /// The row's label.
    pub fn label(self) -> String {
        match self {
            Self::OnStart => msg::tui_settings_general_on_start(),
            Self::StartView => msg::tui_settings_general_start_view(),
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

/// An On start choice's label.
pub fn on_start_label(on_start: OnStart) -> String {
    match on_start {
        OnStart::ReopenLastLedger => msg::tui_settings_on_start_reopen(),
        OnStart::NoLedger => msg::tui_settings_on_start_none(),
    }
}

/// The Start view choice `value` names, ignoring case, as the file spells it.
pub fn start_view_choice(value: &str) -> Option<&'static str> {
    crate::start_view::CHOICES
        .into_iter()
        .find(|choice| choice.eq_ignore_ascii_case(value.trim()))
}

/// The values `:set terminal-colours` and `:set hint-bar` take, as `:set toasts` does.
pub fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// Whether `value`, as [`on_off`] spells it, is on; `None` for any other word.
pub fn from_on_off(value: &str) -> Option<bool> {
    [true, false].into_iter().find(|on| on_off(*on) == value)
}

/// The Configuration General's and Display's rows edit, as read at start-up and written since,
/// with what outranks each key's write. `Shell` writes the file; the rows read this. Toasts and
/// the colour keys are held where they apply, by `Shell`'s `Toasts` and `Colours`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    /// The user's `personal-ledger.conf`, which a row writes; `None` leaves the rows read-only.
    pub path: Option<std::path::PathBuf>,
    pub terminal_colours: bool,
    pub status_glyphs: StatusGlyphs,
    pub hint_bar: bool,
    pub date_style: Option<DateStyle>,
    pub on_start: OnStart,
    pub start_view: StartView,
    /// The Locale the user's file sets, `None` for **system**; read from the file itself, since a
    /// write goes there whatever outranks it (TUI-063).
    pub locale: Option<Locale>,
    /// The supported Locale the operating system's resolves to: what **system** reads.
    pub system_locale: Locale,
    /// The Locale this run started in, which a saved one doesn't change until the next start.
    pub effective_locale: Locale,
    /// The Data folder the user's file sets, `None` with no `data` key; read from the file itself,
    /// since a write goes there whatever outranks it (TUI-063).
    pub data: Option<std::path::PathBuf>,
    /// The source above the file that sets each key, so a write there changes nothing: the
    /// overrides report start-up's parse recorded, as the values are.
    pub overrides: Overrides,
}

impl Default for Configuration {
    /// `lib-config`'s defaults, with nowhere to write.
    fn default() -> Self {
        Self::from_config(&lib_config::Config::default(), &ConfigWriter::default())
    }
}

impl Configuration {
    pub fn from_config(config: &lib_config::Config, writer: &ConfigWriter) -> Self {
        Self {
            path: writer.target().map(std::path::Path::to_path_buf),
            terminal_colours: config.display.terminal_colours(),
            status_glyphs: config.display.status_glyphs(),
            hint_bar: config.display.hint_bar(),
            date_style: config.display.date_style(),
            on_start: config.general.on_start(),
            start_view: config.general.start_view().clone(),
            locale: writer
                .value(ConfigKey::Locale)
                .and_then(|tag| Locale::choice(&tag)),
            system_locale: Locale::negotiate(
                config
                    .general
                    .system_locale()
                    .unwrap_or(lib_config::DEFAULT_LOCALE),
            ),
            effective_locale: crate::locale::info().effective,
            data: writer.value(ConfigKey::Data).map(std::path::PathBuf::from),
            overrides: writer.overrides().clone(),
        }
    }

    /// The source that outranks `key`'s write, named as the row and the status line name it.
    pub fn shadow(&self, key: ConfigKey) -> Option<String> {
        self.overrides.get(key).map(lib_config::Shadow::source)
    }

    /// A Configuration row's note when a source above the file sets its key, else `None`.
    pub fn shadowed_note(&self, field: SettingField) -> Option<String> {
        if matches!(field, SettingField::Locale | SettingField::LedgerFolder) {
            return None;
        }
        field
            .config_key()
            .and_then(|key| self.shadow(key))
            .map(|source| msg::tui_settings_config_shadowed_note(&source))
    }

    /// Why `field`'s Configuration row can't be written: there's no file to write, or a source
    /// above it sets the key. `None` for any other field.
    pub fn refusal(&self, field: SettingField) -> Option<String> {
        let key = field.config_key()?;
        if self.path.is_none() {
            return Some(msg::tui_settings_read_only_config(field.command_name()));
        }
        // The Locale and the Data folder write under anything above them and say so on their rows
        // instead.
        if matches!(field, SettingField::Locale | SettingField::LedgerFolder) {
            return None;
        }
        self.shadow(key)
            .map(|source| msg::tui_settings_set_shadowed(field.command_name(), &source))
    }

    /// The typed value `field` writes for the value `:set` takes, which `lib-config` spells in the
    /// file; `None` for a value the field doesn't offer, or a field that isn't Configuration.
    /// **System** is no value: the Locale's key is removed instead.
    pub fn config_value(field: SettingField, value: &str) -> Option<ConfigValue> {
        Some(match field {
            SettingField::TerminalColours => ConfigValue::TerminalColours(from_on_off(value)?),
            SettingField::HintBar => ConfigValue::HintBar(from_on_off(value)?),
            SettingField::Toasts => ConfigValue::Toasts(from_on_off(value)?),
            SettingField::StatusGlyphs => ConfigValue::StatusGlyphs(StatusGlyphs::from_key(value)?),
            SettingField::ColourTheme => ConfigValue::ColourTheme(
                lib_colour_theme::ColourTheme::built_in()
                    .iter()
                    .map(|theme| theme.id)
                    .find(|id| id.eq_ignore_ascii_case(value))?
                    .to_string(),
            ),
            SettingField::Appearance => {
                ConfigValue::ColourAppearance(ColourAppearance::from_key(value)?)
            }
            SettingField::DateStyle => {
                ConfigValue::DateStyle(lib_config::date_style_from_key(value)?)
            }
            SettingField::OnStart => ConfigValue::OnStart(lib_config::on_start_from_key(value)?),
            SettingField::StartView => {
                ConfigValue::StartView(StartView::from_key(start_view_choice(value)?))
            }
            SettingField::Locale => {
                ConfigValue::Locale(Self::locale_choice(value)??.tag().to_string())
            }
            _ => return None,
        })
    }

    /// What `:set locale` takes: `system` (`Some(None)`) or a supported Locale's tag, never the
    /// pseudo-Locale. `None` for anything else.
    pub fn locale_choice(value: &str) -> Option<Option<Locale>> {
        if value.eq_ignore_ascii_case(LOCALE_SYSTEM) {
            return Some(None);
        }
        Locale::choice(value).map(Some)
    }

    /// The Locale **system** reads, or the file's, whichever the row's choice is.
    pub fn chosen_locale(&self) -> Locale {
        self.locale.unwrap_or(self.system_locale)
    }
}

/// The Locale choice that takes the key out of the file.
pub const LOCALE_SYSTEM: &str = "system";

/// The values the form pages show, shared by every Settings View `Shell` opens.
///
/// Ledger name, owner and the financial year's start have no Preference columns yet, so they
/// start on TUI-019's worked values and hold for the session. The date style and On start are
/// Configuration (ADR-0053), in [`Configuration`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsValues {
    pub ledger_name: String,
    pub owner: String,
    /// The month the financial year starts on, 1–12.
    pub financial_year_start: u32,
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
            ledger_folder: lib_config::GeneralConfig::default().data().to_path_buf(),
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
                let Some(on_start) = lib_config::on_start_from_key(value) else {
                    return false;
                };
                self.configuration.on_start = on_start;
            }
            SettingField::StartView => {
                let Some(choice) = start_view_choice(value) else {
                    return false;
                };
                self.configuration.start_view = StartView::from_key(choice);
            }
            SettingField::DateStyle => {
                let Some(style) = lib_config::date_style_from_key(value) else {
                    return false;
                };
                self.configuration.date_style = style;
            }
            SettingField::TerminalColours | SettingField::HintBar => {
                let Some(on) = from_on_off(value) else {
                    return false;
                };
                if field == SettingField::TerminalColours {
                    self.configuration.terminal_colours = on;
                } else {
                    self.configuration.hint_bar = on;
                }
            }
            SettingField::Locale => {
                let Some(choice) = Configuration::locale_choice(value) else {
                    return false;
                };
                self.configuration.locale = choice;
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
            SettingField::OnStart => lib_config::on_start_key(self.configuration.on_start).into(),
            SettingField::StartView => self.configuration.start_view.key().to_string(),
            SettingField::DateStyle => {
                lib_config::date_style_key(self.configuration.date_style).into()
            }
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
        assert_eq!(values.configuration.date_style, Some(DateStyle::Iso));
        assert!(values.set(SettingField::DateStyle, "default"));
        assert!(values.set(SettingField::OnStart, "none"));
        assert_eq!(values.configuration.on_start, OnStart::NoLedger);
        assert_eq!(values.get(SettingField::OnStart).as_deref(), Some("none"));
        assert!(!values.set(SettingField::OnStart, "sometimes"));
        assert!(values.set(SettingField::StartView, "Units"));
        assert_eq!(
            values.configuration.start_view,
            StartView::Noun("units".into())
        );
        assert_eq!(
            values.get(SettingField::StartView).as_deref(),
            Some("units")
        );
        assert!(values.set(SettingField::StartView, "last"));
        assert_eq!(values.configuration.start_view, StartView::Last);
        assert!(
            !values.set(SettingField::StartView, "bills"),
            "a noun the TUI doesn't have"
        );
        assert_eq!(values.configuration.date_style, None);
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
        configuration.overrides = Overrides::default().with(
            ConfigKey::HintBar,
            lib_config::Shadow::Env("PERSONAL_LEDGER_DISPLAY__HINT_BAR".into()),
        );
        assert!(
            configuration
                .refusal(SettingField::HintBar)
                .is_some_and(|reason| reason.contains("PERSONAL_LEDGER_DISPLAY__HINT_BAR"))
        );
        assert_eq!(
            Configuration::config_value(SettingField::TerminalColours, "on"),
            Some(ConfigValue::TerminalColours(true))
        );
        assert_eq!(
            Configuration::config_value(SettingField::StatusGlyphs, "ASCII"),
            Some(ConfigValue::StatusGlyphs(StatusGlyphs::AsciiFallback))
        );
    }

    #[test]
    fn a_device_setting_is_written_as_the_value_its_key_takes() {
        for (field, value, written) in [
            (
                SettingField::Toasts,
                "off",
                Some(ConfigValue::Toasts(false)),
            ),
            (SettingField::Toasts, "false", None),
            (
                SettingField::ColourTheme,
                "NORD",
                Some(ConfigValue::ColourTheme("nord".into())),
            ),
            (SettingField::ColourTheme, "no-such-theme", None),
            (
                SettingField::Appearance,
                "Light",
                Some(ConfigValue::ColourAppearance(ColourAppearance::Light)),
            ),
            (
                SettingField::DateStyle,
                "default",
                Some(ConfigValue::DateStyle(None)),
            ),
            (
                SettingField::DateStyle,
                "ISO",
                Some(ConfigValue::DateStyle(Some(DateStyle::Iso))),
            ),
            (
                SettingField::OnStart,
                "none",
                Some(ConfigValue::OnStart(OnStart::NoLedger)),
            ),
            (SettingField::OnStart, "sometimes", None),
            (
                SettingField::StartView,
                "Settings",
                Some(ConfigValue::StartView(StartView::Settings)),
            ),
            (
                SettingField::StartView,
                "tags",
                Some(ConfigValue::StartView(StartView::Noun("tags".into()))),
            ),
            (SettingField::StartView, "bills", None),
            (
                SettingField::Locale,
                "en-gb",
                Some(ConfigValue::Locale("en-GB".into())),
            ),
            (SettingField::Locale, LOCALE_SYSTEM, None),
            (SettingField::Owner, "Alex", None),
        ] {
            assert_eq!(
                Configuration::config_value(field, value),
                written,
                "{field:?} {value}"
            );
        }
    }

    #[test]
    fn a_field_s_value_is_written_under_the_field_s_own_key() {
        let offered = |field: SettingField| -> Vec<String> {
            match field {
                SettingField::TerminalColours | SettingField::HintBar | SettingField::Toasts => {
                    vec![on_off(true).into(), on_off(false).into()]
                }
                SettingField::StatusGlyphs => {
                    StatusGlyphs::ALL.map(|glyphs| glyphs.key().into()).into()
                }
                SettingField::ColourTheme => lib_colour_theme::ColourTheme::built_in()
                    .iter()
                    .map(|theme| theme.id.into())
                    .collect(),
                SettingField::Appearance => vec!["light".into(), "dark".into(), "system".into()],
                SettingField::DateStyle => lib_config::DATE_STYLES
                    .map(|style| lib_config::date_style_key(style).into())
                    .into(),
                SettingField::OnStart => lib_config::ON_STARTS
                    .map(|on_start| lib_config::on_start_key(on_start).into())
                    .into(),
                SettingField::StartView => crate::start_view::CHOICES
                    .iter()
                    .map(|choice| (*choice).to_string())
                    .collect(),
                SettingField::Locale => vec!["en-GB".into()],
                _ => Vec::new(),
            }
        };
        for field in SettingField::ALL {
            for value in offered(field) {
                let written = Configuration::config_value(field, &value)
                    .unwrap_or_else(|| panic!("{field:?} refuses its own {value}"));
                assert_eq!(Some(written.key()), field.config_key(), "{field:?} {value}");
            }
        }
        // `:set` and the file spell every choice alike, so a value read back is one `:set` takes.
        for name in crate::start_view::CHOICES {
            assert_eq!(StartView::from_key(name).key(), name);
        }
    }
}
