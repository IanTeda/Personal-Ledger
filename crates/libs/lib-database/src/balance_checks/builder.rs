//! # Balance Checks Builder
//!
//! Provides a fluent API for constructing [`BalanceChecks`] records, mirroring
//! [`crate::accounts::AccountsBuilder`]'s shape.

use super::BalanceChecks;
use crate::Error;

/// Fluent builder for [`BalanceChecks`] rows.
#[derive(Debug, Default, Clone)]
pub struct BalanceChecksBuilder {
    id: Option<lib_core::RowID>,
    account_id: Option<lib_core::RowID>,
    date: Option<chrono::NaiveDate>,
    asserted_balance: Option<lib_core::Money>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl BalanceChecksBuilder {
    /// Starts building a new Balance Check with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Balance Check.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the Account this Balance Check asserts a Balance for.
    #[must_use]
    pub fn with_account_id(mut self, account_id: lib_core::RowID) -> Self {
        self.account_id = Some(account_id);
        self
    }

    /// Set the point in time this assertion is made as of.
    #[must_use]
    pub fn with_date(mut self, date: chrono::NaiveDate) -> Self {
        self.date = Some(date);
        self
    }

    /// Set the asserted Balance.
    #[must_use]
    pub fn with_asserted_balance(mut self, asserted_balance: lib_core::Money) -> Self {
        self.asserted_balance = Some(asserted_balance);
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

    /// Build the [`BalanceChecks`], returning an error when required fields are missing.
    pub fn build(self) -> crate::Result<BalanceChecks> {
        let account_id = self.account_id.ok_or(Error::BalanceChecksBuilder(
            "account_id is required but was not set".to_string(),
        ))?;
        let date = self.date.ok_or(Error::BalanceChecksBuilder(
            "date is required but was not set".to_string(),
        ))?;
        let asserted_balance = self.asserted_balance.ok_or(Error::BalanceChecksBuilder(
            "asserted_balance is required but was not set".to_string(),
        ))?;

        Ok(BalanceChecks {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run whenever no id was explicitly provided.
            #[allow(clippy::unwrap_or_default)]
            id: self.id.unwrap_or_else(lib_core::RowID::new),
            account_id,
            date,
            asserted_balance,
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
