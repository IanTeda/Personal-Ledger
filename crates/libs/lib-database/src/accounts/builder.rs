//! # Accounts Builder
//!
//! Provides a fluent API for constructing [`Accounts`] records, mirroring
//! [`crate::units::UnitsBuilder`]'s shape.

use super::Accounts;
use crate::DatabaseError;

/// Fluent builder for [`Accounts`] rows.
#[derive(Debug, Default, Clone)]
pub struct AccountsBuilder {
    id: Option<lib_core::RowID>,
    name: Option<String>,
    account_type: Option<lib_core::AccountType>,
    unit_id: Option<lib_core::RowID>,
    starting_balance: Option<lib_core::Money>,
    is_active: Option<bool>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl AccountsBuilder {
    /// Starts building a new Account with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Account.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the Account's display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the Account's type.
    #[must_use]
    pub fn with_account_type(mut self, account_type: lib_core::AccountType) -> Self {
        self.account_type = Some(account_type);
        self
    }

    /// Set the Unit this Account is denominated in.
    #[must_use]
    pub fn with_unit_id(mut self, unit_id: lib_core::RowID) -> Self {
        self.unit_id = Some(unit_id);
        self
    }

    /// Set the Account's starting balance.
    #[must_use]
    pub fn with_starting_balance(mut self, starting_balance: lib_core::Money) -> Self {
        self.starting_balance = Some(starting_balance);
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

    /// Build the [`Accounts`], returning an error when required fields are missing.
    pub fn build(self) -> crate::DatabaseResult<Accounts> {
        let name = self.name.ok_or(DatabaseError::AccountsBuilder(
            "name is required but was not set".to_string(),
        ))?;
        let unit_id = self.unit_id.ok_or(DatabaseError::AccountsBuilder(
            "unit_id is required but was not set".to_string(),
        ))?;
        let starting_balance = self.starting_balance.ok_or(DatabaseError::AccountsBuilder(
            "starting_balance is required but was not set".to_string(),
        ))?;

        Ok(Accounts {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run whenever no id was explicitly provided.
            #[allow(clippy::unwrap_or_default)]
            id: self.id.unwrap_or_else(lib_core::RowID::new),
            name,
            account_type: self.account_type.unwrap_or_default(),
            unit_id,
            starting_balance,
            is_active: self.is_active.unwrap_or(true),
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
