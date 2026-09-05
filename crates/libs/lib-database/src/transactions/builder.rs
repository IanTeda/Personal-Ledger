//! # Transactions Builder
//!
//! Provides a fluent API for constructing [`Transactions`] records, mirroring
//! [`crate::accounts::AccountsBuilder`]'s shape.

use super::Transactions;
use crate::DatabaseError;

/// Fluent builder for [`Transactions`] rows.
#[derive(Debug, Default, Clone)]
pub struct TransactionsBuilder {
    id: Option<lib_core::RowID>,
    date: Option<chrono::NaiveDate>,
    amount: Option<lib_core::Money>,
    category_id: Option<lib_core::RowID>,
    account_id: Option<lib_core::RowID>,
    payee_id: Option<Option<lib_core::RowID>>,
    description: Option<Option<String>>,
    status: Option<lib_core::TransactionStatus>,
    is_flagged: Option<bool>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl TransactionsBuilder {
    /// Starts building a new Transaction with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Transaction.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the Transaction's effective/value date.
    #[must_use]
    pub fn with_date(mut self, date: chrono::NaiveDate) -> Self {
        self.date = Some(date);
        self
    }

    /// Set the Transaction's amount.
    #[must_use]
    pub fn with_amount(mut self, amount: lib_core::Money) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Set the Category this Transaction is classified under.
    #[must_use]
    pub fn with_category_id(mut self, category_id: lib_core::RowID) -> Self {
        self.category_id = Some(category_id);
        self
    }

    /// Set the Account this Transaction is posted against.
    #[must_use]
    pub fn with_account_id(mut self, account_id: lib_core::RowID) -> Self {
        self.account_id = Some(account_id);
        self
    }

    /// Provide an optional Payee.
    #[must_use]
    pub fn with_payee_id_opt(mut self, payee_id: Option<lib_core::RowID>) -> Self {
        self.payee_id = Some(payee_id);
        self
    }

    /// Provide an optional description.
    #[must_use]
    pub fn with_description_opt(mut self, description: Option<String>) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the Transaction's reconciliation-workflow status.
    #[must_use]
    pub fn with_status(mut self, status: lib_core::TransactionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Provide an optional Flagged marker, defaulting to `false` when unset.
    #[must_use]
    pub fn with_is_flagged_opt(mut self, is_flagged: Option<bool>) -> Self {
        self.is_flagged = is_flagged;
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

    /// Build the [`Transactions`], returning an error when required fields are missing.
    pub fn build(self) -> crate::DatabaseResult<Transactions> {
        let date = self.date.ok_or(DatabaseError::TransactionsBuilder(
            "date is required but was not set".to_string(),
        ))?;
        let amount = self.amount.ok_or(DatabaseError::TransactionsBuilder(
            "amount is required but was not set".to_string(),
        ))?;
        let category_id = self.category_id.ok_or(DatabaseError::TransactionsBuilder(
            "category_id is required but was not set".to_string(),
        ))?;
        let account_id = self.account_id.ok_or(DatabaseError::TransactionsBuilder(
            "account_id is required but was not set".to_string(),
        ))?;

        Ok(Transactions {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run whenever no id was explicitly provided.
            #[allow(clippy::unwrap_or_default)]
            id: self.id.unwrap_or_else(lib_core::RowID::new),
            date,
            amount,
            category_id,
            account_id,
            payee_id: self.payee_id.unwrap_or(None),
            description: self.description.unwrap_or(None),
            status: self.status.unwrap_or_default(),
            is_flagged: self.is_flagged.unwrap_or(false),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
