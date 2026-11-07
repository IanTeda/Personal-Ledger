//! The Budgets module on the Desktop: the page and every Dialog, running on `lib-budgets`'
//! Presenters (`view`, `router`), over `lib-database`'s `StoredBudgetsPorts`, which `LedgerPorts`
//! hands them (#1083). The Budgets model lives in `lib_budgets`, re-exported here so callers keep reaching
//! it as `budgets::*`. The Budgets are kept in `lib-database`'s Budgets store (#952), which the
//! Desktop reads them from; the Ledger keeps no read copy (#1028).

pub mod router;
pub mod view;

pub use lib_budgets::*;
