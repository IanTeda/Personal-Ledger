//! The Unit Deletion module (`docs/development/viper-architecture.md`, #902), a cross-domain
//! module after `lib-account-deletion`: Unit delete, which removes a Unit, moves the base when it
//! was the base, and gives every Institution that named it as its Usual unit the Unit the person
//! chose instead, or none, in one commit, and the Delete unit Dialog that asks for them.
//! `gpui`-free and I/O-free.
//!
//! It depends on `lib-units` and `lib-institutions`, the feature crates it writes, and none
//! depends on it; the guardrail test in `lib-ledger` keeps it that way, and keeps `lib-units` off
//! `lib-institutions`. `lib-units`' own delete still refuses a Unit an Institution names, since it
//! can't reach them: the TUI, which has no Institutions screen, keeps that Dialog.
//!
//! - `protocols`: the one atomic port, [`UnitDeletion`], which `lib-database` implements over its
//!   Units and Institutions adapters.
//! - `interactor`: [`DeleteUnit`]. `error`: its refusals.
//! - `presenter`: the Delete unit Dialog's Presenter.
//! - `fake`: the in-memory port and the `world` the tests delete from; `contract`: the checks both
//!   the fake and the adapter keep. `msg`: the Dialog's Messages beside `lib-units`' own.
//!
//! [`DeleteUnit`]: interactor::DeleteUnit

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
pub mod error;
pub mod fake;
pub mod interactor;
pub mod msg;
pub mod presenter;
pub mod protocols;

pub use error::UnitDeletionError;
pub use protocols::{SharedUnitDeletion, UnitDeleted, UnitDeletion, UnitRemoval};
