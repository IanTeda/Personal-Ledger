//! # Preferences Database Model
//!
//! Defines the `Preferences` struct, the one row of the `preferences` table -- Ledger-scoped
//! settings a user edits from inside a running Client (ADR-0014): the default Unit for new
//! Accounts, a nullable date style (ADR-0021), and a nullable Colour Theme and Colour
//! Appearance (ADR-0023). Singleton by
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

    /// The chosen Colour Theme's id, such as `modernist`. Nullable: `None` means the default
    /// Colour Theme. Kept as the raw id rather than validated here, so an id from a newer
    /// release survives a round trip through this Client (ADR-0023).
    pub colour_theme: Option<String>,

    /// The chosen Colour Appearance key (`light`, `dark` or `system`). Nullable: `None`
    /// means System. Clients parse it with `lib_colour_theme::ColourAppearance::from_key`.
    pub colour_appearance: Option<String>,

    /// How dates are displayed. Nullable: `None` means the Locale's default (ADR-0021).
    pub date_style: Option<lib_core::DateStyle>,

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
            .with_colour_theme(Some("gruvbox".to_string()))
            .with_colour_appearance(Some("dark".to_string()))
            .with_date_style(Some(lib_core::DateStyle::mock()))
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
