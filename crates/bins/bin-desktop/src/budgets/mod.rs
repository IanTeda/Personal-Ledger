//! The Budgets module on the Desktop: the page and its Switcher and Manage budgets, running on
//! `lib-budgets`' Presenters (`view`, `router`), the Budgets ports the Presenters read and write
//! through (`ports`), and the Dialogs still on the legacy host (#955: `legacy`, `form`,
//! `limit_form`, `apply`, `surface`). The Budgets model lives in `lib_budgets`, re-exported here
//! so callers keep reaching it as `budgets::*`. The Budgets are kept in `lib-database`'s Budgets
//! store (#952); the Desktop reads them through `LedgerStore`'s read copy (ADR-0038).

pub(crate) mod apply;
pub(crate) mod form;
pub(crate) mod legacy;
pub(crate) mod limit_form;
pub mod ports;
pub mod router;
pub mod view;

mod surface;

pub use lib_budgets::*;
pub use surface::*;
