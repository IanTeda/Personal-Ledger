//! The Budgets surface's dialogs and forms, and the Budgets store as the Desktop holds it
//! (`ports`). The Budgets model (Budgets, Category Limit chains, the Progress, Plan and History
//! figures, the stub seed) lives in `lib_budgets` so the TUI can share it, and is re-exported here
//! so callers keep reaching it as `budgets::*`. The Budgets are kept in `lib-database`'s Budgets
//! store (#952); the Desktop reads them through `LedgerStore`'s read copy (ADR-0038).

pub(crate) mod apply;
pub(crate) mod form;
pub(crate) mod limit_form;
pub mod ports;

mod surface;

pub use lib_budgets::*;
pub use surface::*;

#[cfg(test)]
mod tests;
