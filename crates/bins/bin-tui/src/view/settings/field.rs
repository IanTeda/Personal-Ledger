//! The fields the form pages edit ([`SettingField`]), by the name `:set` takes, and the values
//! that hold for the session only ([`SettingsValues`]).
//!
//! Opening and saving a field is one [`Action::SetSetting`] with the field and an optional value:
//! `enter` on a row sends it without one, which opens the field in place, and the open field's
//! `enter` sends it with the value chosen, which saves. `:set <field> [value]` (#898) sends the
//! same `Action`, so a key and a command can't drift.
//!
//! [`Action::SetSetting`]: crate::view::Action::SetSetting

use chrono::{Datelike, Months, NaiveDate};
use lib_core::DateStyle;

use super::SettingsPage;
use crate::msg;

/// Every field on General and Display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingField {
    LedgerName,
    Owner,
    FinancialYearStart,
    BaseUnit,
    Appearance,
    ColourTheme,
    DateStyle,
    Locale,
    TerminalColours,
    StatusGlyphs,
    Toasts,
    HintBar,
}

/// Why a field can't be edited here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOnly {
    /// The Locale is Configuration read at start-up (ADR-0021).
    Locale,
    /// Configuration in `personal-ledger.conf`, which `lib-config` can't write yet.
    ConfigFile,
}

impl SettingField {
    /// Every field, General's then Display's, each in its page's row order.
    pub const ALL: [SettingField; 12] = [
        Self::LedgerName,
        Self::Owner,
        Self::FinancialYearStart,
        Self::BaseUnit,
        Self::Appearance,
        Self::ColourTheme,
        Self::DateStyle,
        Self::Locale,
        Self::TerminalColours,
        Self::StatusGlyphs,
        Self::Toasts,
        Self::HintBar,
    ];

    /// The name `:set <field>` takes: command syntax the user types, so stable English rather
    /// than a Message, as command names are.
    pub fn command_name(self) -> &'static str {
        match self {
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
            Self::LedgerName | Self::Owner | Self::FinancialYearStart | Self::BaseUnit => {
                SettingsPage::General
            }
            _ => SettingsPage::Display,
        }
    }

    /// Why the field can't be edited, or `None` when it can.
    pub fn read_only(self) -> Option<ReadOnly> {
        match self {
            Self::Locale => Some(ReadOnly::Locale),
            Self::TerminalColours | Self::StatusGlyphs | Self::HintBar => {
                Some(ReadOnly::ConfigFile)
            }
            _ => None,
        }
    }

    /// The row's label.
    pub fn label(self) -> String {
        match self {
            Self::LedgerName => msg::tui_settings_general_ledger_name(),
            Self::Owner => msg::tui_settings_general_owner(),
            Self::FinancialYearStart => msg::tui_settings_general_financial_year(),
            Self::BaseUnit => msg::tui_settings_general_base_unit(),
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
        self.read_only().map(|reason| match reason {
            ReadOnly::Locale => msg::tui_settings_read_only_locale(self.command_name()),
            ReadOnly::ConfigFile => msg::tui_settings_read_only_config(self.command_name()),
        })
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

/// The date style choices, in the order the field cycles through them, by the value
/// `:set date-style` takes. `None` is the Locale's default (ADR-0021).
pub const DATE_STYLES: [(&str, Option<DateStyle>); 5] = [
    ("default", None),
    ("short", Some(DateStyle::Short)),
    ("medium", Some(DateStyle::Medium)),
    ("long", Some(DateStyle::Long)),
    ("iso", Some(DateStyle::Iso)),
];

/// The values that hold for this session only, shared by every Settings View `Shell` opens.
///
/// Ledger name, owner and the financial year's start have no Preference columns yet, so they
/// start on TUI-019's worked values. The date style is a Preference, but neither Client saves
/// Preferences to the database yet (`lib_database::Preferences` has no adapter), so it lives here
/// for the session, as the Colour Theme and Appearance do in `Shell`'s `Colours`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsValues {
    pub ledger_name: String,
    pub owner: String,
    /// The month the financial year starts on, 1–12.
    pub financial_year_start: u32,
    pub date_style: Option<DateStyle>,
}

impl Default for SettingsValues {
    fn default() -> Self {
        Self {
            // wireframe: TUI-019's worked values, until the Ledger has Preference columns for them.
            ledger_name: "Teda household".into(),
            owner: "Ian Teda".into(),
            financial_year_start: 7,
            date_style: None,
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
            SettingField::DateStyle => {
                let Some((_, style)) = DATE_STYLES
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(value))
                else {
                    return false;
                };
                self.date_style = *style;
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
            SettingField::DateStyle => DATE_STYLES
                .iter()
                .find(|(_, style)| *style == self.date_style)
                .map(|(name, _)| (*name).to_string())?,
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
        assert_eq!(values.date_style, None);
        assert!(!values.set(SettingField::Owner, "  "));
        assert!(values.set(SettingField::Owner, "Alex"));
        assert_eq!(values.get(SettingField::Owner).as_deref(), Some("Alex"));
        assert!(!values.set(SettingField::BaseUnit, "btc"), "not held here");
    }
}
