//! # Client-scoped Preferences
//!
//! The Preferences that belong to the device rather than the Ledger, so they apply with no
//! Ledger open and never sync (ADR-0027, ADR-0051). Each Client keeps one in its own state file;
//! this is only the value, with no I/O.

use crate::{DateStyle, OnStart};

/// Every Client-scoped Preference, as a Client keeps them between runs. Every field defaults on
/// its own, so a state file written before a Preference existed still loads.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ClientPreferences {
    /// Whether non-Error Toasts show (ADR-0027).
    pub toasts_on: bool,
    /// The chosen Colour Theme's id. `None` means the default Colour Theme; kept raw, so an id
    /// from a newer release survives a run of this one (ADR-0023).
    pub colour_theme: Option<String>,
    /// The chosen Colour Appearance key (`light`, `dark` or `system`). `None` means System.
    pub colour_appearance: Option<String>,
    /// `None` means the Locale's default (ADR-0021).
    pub date_style: Option<DateStyle>,
    pub on_start: OnStart,
}

impl Default for ClientPreferences {
    fn default() -> Self {
        Self {
            toasts_on: true,
            colour_theme: None,
            colour_appearance: None,
            date_style: None,
            on_start: OnStart::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_preference_loads_as_its_default() {
        let preferences: ClientPreferences =
            serde_json::from_str(r#"{ "date_style": "Iso" }"#).expect("partial JSON should load");
        assert_eq!(
            preferences,
            ClientPreferences {
                date_style: Some(DateStyle::Iso),
                ..ClientPreferences::default()
            }
        );
        assert!(preferences.toasts_on, "Toasts default on");
        assert_eq!(preferences.on_start, OnStart::ReopenLastLedger);
    }

    #[test]
    fn on_start_is_stored_by_its_snake_case_name() {
        let preferences = ClientPreferences {
            on_start: OnStart::NoLedger,
            ..ClientPreferences::default()
        };
        let json = serde_json::to_string(&preferences).expect("serialises");
        assert!(json.contains(r#""on_start":"no_ledger""#), "{json}");
    }
}
