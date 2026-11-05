//! The Budgets module on the Desktop: the page and every Dialog, running on `lib-budgets`'
//! Presenters (`view`, `router`), and the Budgets ports the Presenters read and write through
//! (`ports`). The Budgets model lives in `lib_budgets`, re-exported here so callers keep reaching
//! it as `budgets::*`. The Budgets are kept in `lib-database`'s Budgets store (#952), which the
//! Desktop reads them from; the Ledger keeps no read copy (#1028).

pub mod ports;
pub mod router;
pub mod view;

pub use lib_budgets::*;
