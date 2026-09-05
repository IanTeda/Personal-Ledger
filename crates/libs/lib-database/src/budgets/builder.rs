//! # Budgets Builder
//!
//! Provides a fluent API for constructing [`Budgets`] records, mirroring
//! [`crate::accounts::AccountsBuilder`]'s shape.

use super::Budgets;
use crate::DatabaseError;

/// Fluent builder for [`Budgets`] rows.
#[derive(Debug, Default, Clone)]
pub struct BudgetsBuilder {
    id: Option<lib_core::RowID>,
    category_id: Option<lib_core::RowID>,
    unit_id: Option<lib_core::RowID>,
    limit_amount: Option<lib_core::Money>,
    period: Option<lib_core::BudgetPeriod>,
    is_active: Option<bool>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl BudgetsBuilder {
    /// Starts building a new Budget with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Budget.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the Category this Budget caps.
    #[must_use]
    pub fn with_category_id(mut self, category_id: lib_core::RowID) -> Self {
        self.category_id = Some(category_id);
        self
    }

    /// Set the Unit this Budget's limit is denominated in.
    #[must_use]
    pub fn with_unit_id(mut self, unit_id: lib_core::RowID) -> Self {
        self.unit_id = Some(unit_id);
        self
    }

    /// Set the Budget's limit amount.
    #[must_use]
    pub fn with_limit_amount(mut self, limit_amount: lib_core::Money) -> Self {
        self.limit_amount = Some(limit_amount);
        self
    }

    /// Set how often the limit recurs.
    #[must_use]
    pub fn with_period(mut self, period: lib_core::BudgetPeriod) -> Self {
        self.period = Some(period);
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

    /// Build the [`Budgets`], returning an error when required fields are missing.
    pub fn build(self) -> crate::DatabaseResult<Budgets> {
        let category_id = self.category_id.ok_or(DatabaseError::BudgetsBuilder(
            "category_id is required but was not set".to_string(),
        ))?;
        let unit_id = self.unit_id.ok_or(DatabaseError::BudgetsBuilder(
            "unit_id is required but was not set".to_string(),
        ))?;
        let limit_amount = self.limit_amount.ok_or(DatabaseError::BudgetsBuilder(
            "limit_amount is required but was not set".to_string(),
        ))?;

        Ok(Budgets {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run whenever no id was explicitly provided.
            #[allow(clippy::unwrap_or_default)]
            id: self.id.unwrap_or_else(lib_core::RowID::new),
            category_id,
            unit_id,
            limit_amount,
            period: self.period.unwrap_or_default(),
            is_active: self.is_active.unwrap_or(true),
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
