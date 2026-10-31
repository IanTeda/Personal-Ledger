//! The Transaction Import module (`docs/development/viper-architecture.md`, #943), a cross-domain
//! module: an Import matches a statement's lines to Payees by their Payee Aliases and commits the
//! new Payees, their aliases and one Transaction per line in one write. `gpui`-free and I/O-free.
//!
//! It depends on `lib-payees` and `lib-transactions`, the feature crates it writes, and neither
//! depends on it; the guardrail test in `lib-ledger` keeps it that way.
//!
//! - `entity`: a statement line, the cleaning and Payee Alias matching rules, the Import row, its
//!   Row status, a row's Payee choices and the summary, moved from the Desktop's `import.rs`.
//! - `plan`: the resolved [`ImportPlan`] the port takes, and its check; `error`: the one
//!   [`Error`] every refusal is a variant of.
//! - `protocols`: the one atomic port, [`TransactionImport`], which `lib-database` implements over
//!   its Payees and Transactions adapters, the [`StatementSource`] and the reads.
//! - `statement`: the seeded statement, the one source while the Import is stubbed.
//! - `fake`: the in-memory port and reads; `contract`: the checks both the fake and the adapter
//!   keep.
//!
//! - `interactor`: [`MatchStatement`] and [`CommitImport`], one file each; `presenter`: the step's Presenter and
//!   its snapshot; `keymap`: its default keys; `msg`: its Messages, which a Client composes into
//!   its Locale as `LAYER`.
//!
//! The Desktop's Import runs on it (#947); the TUI's is still a wireframe placeholder.

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod entity;
mod error;
pub mod fake;
pub mod interactor;
pub mod keymap;
pub mod msg;
mod plan;
pub mod presenter;
pub mod protocols;
mod statement;

use bigdecimal::BigDecimal;
use lib_core::Money;

pub use entity::*;
pub use error::{Error, Result};
pub use interactor::{CommitImport, MatchStatement, Matched};
pub use plan::{ImportCommitted, ImportPlan};
pub use protocols::{
    ImportAccount, ImportAccounts, ImportCategories, ImportPayees, ImportPorts, ImportRouter,
    SharedImport, StatementSource, TransactionImport,
};
pub use statement::{STATEMENT_FILE, SeededStatement};

/// `cents` hundredths as Money, as the seeded statement writes its amounts.
fn cents(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}
