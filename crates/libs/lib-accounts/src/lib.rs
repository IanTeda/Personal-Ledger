//! Accounts: where a Ledger's money and holdings sit, each at an Institution and in one Unit
//! (`GLOSSARY.md`'s Account). A VIPER module being migrated (`docs/development/viper-architecture.md`,
//! #847): the Entity with its rules and stub seed (`entity`), one Interactor per intent
//! (`interactor`) and the ports (`protocols`): the repository and [`AccountUsage`], the Transactions
//! and Budgets naming an Account. `fake` holds the in-memory ports and the stub seed; `contract`
//! the checks both they and `lib-database`'s adapter keep (#853). The page's Presenters, keymap
//! and Messages arrive with the page (#855); until then the Desktop's Settings › Accounts page
//! reads them from the `LedgerStore`'s read copy and writes them through `LedgerCommand`s, which
//! the Desktop forwards to the adapter. No UI toolkit and no I/O.

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
