//! Tags: free-form labels a Split carries (`GLOSSARY.md`'s Tag), each with an optional colour. A
//! VIPER module (`docs/development/viper-architecture.md`, #847), built ticket by ticket: the
//! Entity with its rules and stub seed (`entity`), one Interactor per intent (`interactor`) and the
//! ports (`protocols`). `fake` holds the in-memory ports and the stub seed; `contract` the checks
//! both they and a stored adapter keep. Presenters, keys and Messages join with the Desktop's move
//! (#862). Remove and merge rewrite Splits, so they stay `lib-ledger`'s (Q6). No UI toolkit and no
//! I/O.

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
pub use protocols::{TagRepository, TagUsage, TagUse};
