//! # Preferences Database Model
//!
//! Defines the `Preferences` struct, the one row of the `preferences` table -- Ledger-scoped
//! settings a user edits from inside a running Client (ADR-0014): the default Unit for new
//! Accounts, colour theme, date format, and decimal/thousands separator. Singleton by
//! convention (mirroring `sync_users` -- see [`crate::Preferences::find_only`] and
//! [`crate::Preferences::get_or_create_default`]), not a database constraint.

/// Database row model representing the singleton Preferences row.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Preferences {
    /// Unique identifier for the Preferences row. Ordinary `RowID`, not a fixed/special
    /// value -- singleton-ness is enforced by convention, not a primary key trick.
    pub id: lib_core::RowID,

    /// The default Unit new Accounts are created with. Nullable: cleared (`ON DELETE SET
    /// NULL`) if the referenced Unit is later hard-deleted.
    pub default_unit_id: Option<lib_core::RowID>,

    /// The TUI/Desktop's one accent colour (reserved for negatives, over-budget, variance,
    /// and Liabilities, per `docs/ux/tui/README.md`'s style table).
    pub colour_theme: lib_core::HexColor,

    /// How dates are displayed.
    pub date_format: lib_core::DateFormat,

    /// How amounts' thousands/decimal separators are displayed.
    pub number_format: lib_core::NumberFormat,

    /// UTC timestamp when the Preferences row was first created.
    pub created_on: chrono::DateTime<chrono::Utc>,

    /// UTC timestamp when the Preferences row was last modified.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

impl Preferences {
    /// Generates a mock `Preferences` instance with randomised test data.
    #[cfg(test)]
    pub fn mock() -> Self {
        use crate::preferences::PreferencesBuilder;

        let now = chrono::Utc::now();
        PreferencesBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_default_unit_id(Some(lib_core::RowID::mock()))
            .with_colour_theme(lib_core::HexColor::mock())
            .with_date_format(lib_core::DateFormat::mock())
            .with_number_format(lib_core::NumberFormat::mock())
            .with_created_on_opt(Some(now))
            .with_updated_on_opt(Some(now))
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_preferences() {
        let preferences = Preferences::mock();
        assert!(preferences.default_unit_id.is_some());
    }

    #[test]
    fn preferences_struct_derives_work() {
        let a = Preferences::mock();
        let b = a.clone();
        assert_eq!(a, b);

        let debug_str = format!("{:?}", a);
        assert!(debug_str.contains("Preferences"));

        let json = serde_json::to_string(&a).unwrap();
        let deserialized: Preferences = serde_json::from_str(&json).unwrap();
        assert_eq!(a, deserialized);
    }
}
