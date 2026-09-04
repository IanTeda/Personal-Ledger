//! # Units Builder
//!
//! Provides a fluent API for constructing [`Units`] records, mirroring
//! [`crate::categories::CategoriesBuilder`]'s shape.

use super::Units;
use crate::DatabaseError;

/// Fluent builder for [`Units`] rows.
#[derive(Debug, Default, Clone)]
pub struct UnitsBuilder {
    id: Option<lib_core::RowID>,
    code: Option<String>,
    name: Option<String>,
    unit_kind: Option<lib_core::UnitKind>,
    decimal_places: Option<i64>,
    is_active: Option<bool>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl UnitsBuilder {
    /// Starts building a new Unit with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Unit.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the Unit's code (e.g. "AUD", "BTC").
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set the Unit's display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the Unit's descriptive kind.
    #[must_use]
    pub fn with_unit_kind(mut self, unit_kind: lib_core::UnitKind) -> Self {
        self.unit_kind = Some(unit_kind);
        self
    }

    /// Set the number of decimal places this Unit displays with.
    #[must_use]
    pub fn with_decimal_places(mut self, decimal_places: i64) -> Self {
        self.decimal_places = Some(decimal_places);
        self
    }

    /// Provide an optional active flag, defaulting to `true` when unset.
    #[must_use]
    pub fn with_is_active_opt(mut self, is_active: Option<bool>) -> Self {
        self.is_active = is_active;
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

    /// Build the [`Units`], returning an error when required fields are missing.
    pub fn build(self) -> crate::DatabaseResult<Units> {
        let code = self.code.ok_or(DatabaseError::UnitsBuilder(
            "code is required but was not set".to_string(),
        ))?;
        let name = self.name.ok_or(DatabaseError::UnitsBuilder(
            "name is required but was not set".to_string(),
        ))?;

        Ok(Units {
            id: self.id.unwrap_or_default(),
            code,
            name,
            unit_kind: self.unit_kind.unwrap_or_default(),
            decimal_places: self.decimal_places.unwrap_or(2),
            is_active: self.is_active.unwrap_or(true),
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
