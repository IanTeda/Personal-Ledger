//! Accounts: where a Ledger's money and holdings sit, each at an Institution and in one Unit
//! (`GLOSSARY.md`'s Account). A VIPER module being migrated (`docs/development/viper-architecture.md`,
//! #847): the Entity with its rules and stub seed (`entity`), one Interactor per intent
//! (`interactor`) and the ports (`protocols`): the repository and [`AccountUsage`], the Transactions
//! and Budgets naming an Account. `fake` holds the in-memory ports and the stub seed; `contract`
//! the checks both they and, once Accounts persist (#853), `lib-database`'s adapter keep. The
//! page's Presenters, keymap and Messages arrive with the page (#855); until then the Desktop's
//! Settings › Accounts page reads them from the `LedgerStore` and writes them through
//! `LedgerCommand`s, and `lib-ledger` answers [`AccountUsage`] from the `Ledger`. No UI toolkit
//! and no I/O.

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod entity;
pub mod fake;
pub mod interactor;
pub mod protocols;

pub use entity::*;
pub use protocols::{AccountRepository, AccountUsage, AccountUse};
