//! The Ledger Creation module (`docs/development/viper-architecture.md`, #912, ADR-0054), a
//! cross-domain module after `lib-unit-deletion`: creating a Ledger, the only way a Ledger file
//! comes to be, which writes what every new Ledger starts with, all or nothing. `gpui`-free and
//! I/O-free; the port does the I/O.
//!
//! It depends on `lib-units`, `lib-institutions`, `lib-budgets` and `lib-documents`, for their
//! starting rows and `lib-units`' Unit rules, and none depends on it; the guardrail test in
//! `lib-database` keeps it that way.
//!
//! - `starting`: [`NewLedger`], what every new Ledger starts with.
//! - `protocols`: the one atomic port, [`LedgerCreation`], over a path, which `lib-database`'s
//!   `LedgerCreationAdapter` implements.
//! - `interactor`: [`CreateLedger`]. `error`: its refusals and failures.
//! - `fake`: the in-memory port; `contract`: the checks both the fake and the adapter keep.
//!   `msg`: the module's Messages.
//!
//! [`CreateLedger`]: interactor::CreateLedger

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
mod starting;

pub use error::{Error, Result};
pub use protocols::{CreationFailure, LedgerCreation};
pub use starting::NewLedger;
