//! The Tag Removal module (`docs/development/viper-architecture.md`, #1002), a cross-domain module
//! after `lib-unit-deletion`: Tag remove, which untags every Split carrying a Tag and deletes it,
//! and Tag merge, which retags the source's Splits with the target and deletes the source, each in
//! one commit, and the Remove tag and Merge tags Dialogs that confirm them. `gpui`-free and
//! I/O-free.
//!
//! It depends on `lib-tags` and `lib-transactions`, the feature crates it writes, and none depends
//! on it; the guardrail test in `lib-ledger` keeps it that way.
//!
//! - `protocols`: the one atomic port, [`TagRemoval`], which `lib-database` implements over its
//!   Tags and Transactions stores, and [`TagRemovalReads`], what the Dialogs show when they open.
//! - `interactor`: [`RemoveTag`] and [`MergeTags`]. `error`: their refusals.
//! - `presenter`: the Remove tag and Merge tags Dialogs' Presenters, which close through
//!   `lib-tags`' outcome so the Tags page hears of them as of its own Dialogs.
//! - `fake`: the in-memory port and reads, and the `world` the tests remove from; `contract`: the
//!   checks both the fake and the adapter keep. `msg`: the Dialogs' Messages.
//!
//! [`RemoveTag`]: interactor::RemoveTag
//! [`MergeTags`]: interactor::MergeTags

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
    SharedTagRemoval, TagCount, TagMerged, TagRemoval, TagRemovalReads, TagRemoved, TagRewrite,
};
