//! The Budgets surface's dialogs and forms. The Budgets model (Budgets, Category Limit chains,
//! the Progress, Plan and History figures, the stub seed) lives in `lib_budgets` so the TUI can
//! share it, and is re-exported here so callers keep reaching it as `budgets::*`. The Desktop
//! reads it through the `BudgetsStore` Entity (ADR-0032).

pub(crate) mod form;
pub(crate) mod limit_form;

mod surface;

pub use lib_budgets::*;
pub use surface::*;

#[cfg(test)]
mod tests;
