//! The Bill Payments module (`docs/development/viper-architecture.md`, #779), the first
//! cross-domain module: Pay it directly, which writes a Transaction and the Bill Schedule entry it
//! settles in one commit, and the Pay Dialog that offers it beside Match. `gpui`-free and I/O-free.
//!
//! It depends on `lib-bills` and `lib-transactions`, the feature crates it writes, and neither
//! depends on it; the guardrail test in `lib-ledger` keeps it that way.
//!
//! - `protocols`: the one atomic port, [`BillPayments`], which `lib-database` implements over its
//!   Transactions and Bills adapters, and the Router the Dialog closes through.
//! - `interactor`: [`PayBill`], Pay it directly.
//! - `presenter`: the Pay Dialog's Presenter, both panels. Its Match panel runs `lib-bills`'
//!   `MatchBill`, since a Match writes only the entry.
//! - `fake`: the in-memory port and a recording Router; `contract`: the checks both the fake and
//!   the adapter keep. `msg`: the Dialog's Messages.
//!
//! [`PayBill`]: interactor::PayBill

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
pub mod fake;
pub mod interactor;
pub mod msg;
pub mod presenter;
pub mod protocols;

pub use protocols::{BillPayments, BillPaymentsRouter, PayOutcome, SharedPayments};
