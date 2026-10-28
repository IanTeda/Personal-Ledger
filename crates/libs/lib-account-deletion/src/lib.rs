//! The Account Deletion module (`docs/development/viper-architecture.md`, #854), a cross-domain
//! module after `lib-bill-payments`: Account delete, which removes an Account, the Transactions
//! booked to it and the Paid links they held in one commit, and the Delete Dialog that confirms
//! it. `gpui`-free and I/O-free.
//!
//! It depends on `lib-accounts`, `lib-transactions` and `lib-bills`, the feature crates it writes,
//! and none depends on it; the guardrail test in `lib-ledger` keeps it that way.
//!
//! - `protocols`: the one atomic port, [`AccountDeletion`], which `lib-database` implements over
//!   its Accounts, Transactions and Bills adapters, the [`DeletionReads`] the Dialog quotes, and
//!   the Router the Dialog closes through.
//! - `interactor`: [`DeleteAccount`]. `error`: its refusals.
//! - `presenter`: the Delete Dialog's Presenter.
//! - `fake`: the in-memory port and reads, a recording Router and the `world` the tests delete
//!   from; `contract`: the checks both the fake and the adapter keep. `msg`: the Dialog's
//!   Messages.
//!
//! [`DeleteAccount`]: interactor::DeleteAccount

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod error;
pub mod fake;
pub mod interactor;
pub mod msg;
pub mod presenter;
pub mod protocols;

pub use error::{Error, Result};
pub use protocols::{
    AccountDeleted, AccountDeletion, AccountDeletionRouter, AccountRemoval, DeletionReads,
    SharedDeletion,
};
