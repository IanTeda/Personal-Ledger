//! # Payees Builder
//!
//! Provides a fluent API for constructing [`Payees`] records, mirroring
//! [`crate::accounts::AccountsBuilder`]'s shape.

use super::Payees;
use crate::Error;

/// Fluent builder for [`Payees`] rows.
#[derive(Debug, Default, Clone)]
pub struct PayeesBuilder {
    id: Option<lib_core::RowID>,
    name: Option<String>,
    is_active: Option<bool>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl PayeesBuilder {
    /// Starts building a new Payee with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Payee.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the Payee's display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
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

    /// Build the [`Payees`], returning an error when required fields are missing.
    pub fn build(self) -> crate::Result<Payees> {
        let name = self.name.ok_or(Error::PayeesBuilder(
            "name is required but was not set".to_string(),
        ))?;

        Ok(Payees {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run whenever no id was explicitly provided.
            #[allow(clippy::unwrap_or_default)]
            id: self.id.unwrap_or_else(lib_core::RowID::new),
            name,
            is_active: self.is_active.unwrap_or(true),
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
