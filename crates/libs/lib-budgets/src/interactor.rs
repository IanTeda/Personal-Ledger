//! The Budgets intents, one Interactor each: it loads the Budget from the [`BudgetRepository`](crate::BudgetRepository),
//! applies the rule the entity or the set holds, and writes each Budget the rule changed whole
//! through `upsert_budget`. A refusal is an [`Error`](crate::Error) and writes nothing: a chain edit runs on a
//! copy of the Budget, kept only when the rule lands, since some rules touch a chain before they
//! refuse.
//!
//! The rules that read the other domains take the read port they need ([`BudgetCategories`](crate::BudgetCategories),
//! [`BudgetAccounts`](crate::BudgetAccounts) or [`BudgetReads`](crate::BudgetReads)); the repository is a separate argument, so a caller
//! whose store holds both can pass the reads it keeps beside it.

mod archive_budget;
mod clear_month_only;
mod create_budget;
mod cycle_rollover;
mod duplicate_budget;
mod edit_budget;
mod fill;
mod restore_budget;
mod save_cell;
mod set_amount;
mod set_default_budget;
mod set_monthly_limit;
mod stop_budgeting;
mod stop_new_parent;
mod support;
#[cfg(test)]
mod test_support;

pub use archive_budget::ArchiveBudget;
pub use clear_month_only::ClearMonthOnly;
pub use create_budget::CreateBudget;
pub use cycle_rollover::CycleRollover;
pub use duplicate_budget::DuplicateBudget;
pub use edit_budget::EditBudget;
pub use fill::Fill;
pub use restore_budget::RestoreBudget;
pub use save_cell::SaveCell;
pub use set_amount::SetAmount;
pub use set_default_budget::SetDefaultBudget;
pub use set_monthly_limit::SetMonthlyLimit;
pub use stop_budgeting::StopBudgeting;
pub use stop_new_parent::StopNewParent;
