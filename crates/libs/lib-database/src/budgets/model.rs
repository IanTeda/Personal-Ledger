//! # Budgets Database Model
//!
//! Defines the `Budgets` struct, one row of the `budgets` table — a limit on the total
//! amount of Transactions in one Category over a recurring period (FR.22-27, CC-TUI-010),
//! line-item only (see `CONTEXT.md`'s Budget entry; envelope/reverse budgeting are deferred,
//! PRD §7). Restricted to Expense-type Categories, enforced by [`Self::insert`] — "spending"
//! against an Asset/Liability/Income/Equity Category has no coherent meaning yet.

/// Database row model representing one persisted Budget.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Budgets {
    /// Unique time-ordered identifier for the Budget.
    pub id: lib_core::RowID,

    /// The exactly-one (Expense-type) Category this Budget caps, fixed at creation — FR.26
    /// only allows changing limit amount, period, or active status.
    pub category_id: lib_core::RowID,

    /// The Unit this Budget's limit is denominated in, fixed at creation.
    pub unit_id: lib_core::RowID,

    /// The cap on Transaction totals in `category_id`/`unit_id` per recurring period.
    pub limit_amount: lib_core::Money,

    /// How often the limit recurs.
    pub period: lib_core::BudgetPeriod,

    /// Soft-delete flag — `false` excludes the Budget from progress tracking, but it remains
    /// a valid historical record.
    pub is_active: bool,

    /// UTC timestamp when the Budget was first created.
    pub created_on: chrono::DateTime<chrono::Utc>,

    /// UTC timestamp when the Budget was last modified.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

impl Budgets {
    /// Generates a mock `Budgets` instance with randomised test data. `category_id` and
    /// `unit_id` must be supplied by the caller (real, already-inserted rows — both foreign
    /// keys are enforced).
    #[cfg(test)]
    pub fn mock(category_id: lib_core::RowID, unit_id: lib_core::RowID) -> Self {
        use crate::budgets::BudgetsBuilder;

        let now = chrono::Utc::now();
        BudgetsBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_category_id(category_id)
            .with_unit_id(unit_id)
            .with_limit_amount(lib_core::Money::mock())
            .with_period(lib_core::BudgetPeriod::mock())
            .with_is_active_opt(Some(true))
            .with_created_on_opt(Some(now))
            .with_updated_on_opt(Some(now))
            .build()
            .expect("Mock Budgets should always build successfully")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_budget() {
        let budget = Budgets::mock(lib_core::RowID::new(), lib_core::RowID::new());
        assert!(budget.is_active);
    }

    #[test]
    fn budgets_struct_derives_work() {
        let a = Budgets::mock(lib_core::RowID::new(), lib_core::RowID::new());
        let b = a.clone();
        assert_eq!(a, b);

        let debug_str = format!("{:?}", a);
        assert!(debug_str.contains("Budgets"));

        let json = serde_json::to_string(&a).unwrap();
        let deserialized: Budgets = serde_json::from_str(&json).unwrap();
        assert_eq!(a, deserialized);
    }
}
