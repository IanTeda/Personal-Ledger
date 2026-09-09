//! # Preferences Builder
//!
//! Provides a fluent API for constructing [`Preferences`] instances, mirroring
//! [`crate::units::UnitsBuilder`]'s shape. Unlike other entities' builders, every field has
//! a sensible default (the whole point of a singleton default row), so [`PreferencesBuilder::build`]
//! never fails.

use super::Preferences;

/// Fluent builder for [`Preferences`] rows.
#[derive(Debug, Default, Clone)]
pub struct PreferencesBuilder {
    id: Option<lib_core::RowID>,
    default_unit_id: Option<lib_core::RowID>,
    colour_theme: Option<lib_core::HexColor>,
    date_format: Option<lib_core::DateFormat>,
    number_format: Option<lib_core::NumberFormat>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl PreferencesBuilder {
    /// Starts building a new Preferences row with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Preferences row.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the default Unit for new Accounts (or clear it, with `None`).
    #[must_use]
    pub fn with_default_unit_id(mut self, default_unit_id: Option<lib_core::RowID>) -> Self {
        self.default_unit_id = default_unit_id;
        self
    }

    /// Set the accent colour.
    #[must_use]
    pub fn with_colour_theme(mut self, colour_theme: lib_core::HexColor) -> Self {
        self.colour_theme = Some(colour_theme);
        self
    }

    /// Set the date display format.
    #[must_use]
    pub fn with_date_format(mut self, date_format: lib_core::DateFormat) -> Self {
        self.date_format = Some(date_format);
        self
    }

    /// Set the number display format.
    #[must_use]
    pub fn with_number_format(mut self, number_format: lib_core::NumberFormat) -> Self {
        self.number_format = Some(number_format);
        self
    }

    /// Provide an optional creation timestamp, defaulting to now when unset.
    #[must_use]
    pub fn with_created_on_opt(
        mut self,
        created_on: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.created_on = created_on;
        self
    }

    /// Provide an optional update timestamp, defaulting to now when unset.
    #[must_use]
    pub fn with_updated_on_opt(
        mut self,
        updated_on: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.updated_on = updated_on;
        self
    }

    /// Build the [`Preferences`] row. Every field has a sensible default, so this never
    /// fails -- unlike other entities' builders, there is nothing a caller is required to
    /// set.
    #[must_use]
    pub fn build(self) -> Preferences {
        Preferences {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default() is a nil
            // (version 0) UUID, not a usable row id -- RowID's Decode requires version 7.
            #[allow(clippy::unwrap_or_default)]
            id: self.id.unwrap_or_else(lib_core::RowID::new),
            default_unit_id: self.default_unit_id,
            colour_theme: self
                .colour_theme
                .unwrap_or_else(|| lib_core::HexColor::from_rgb(255, 0, 0)),
            date_format: self.date_format.unwrap_or_default(),
            number_format: self.number_format.unwrap_or_default(),
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        }
    }
}
