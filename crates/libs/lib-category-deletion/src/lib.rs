//! The Category Deletion module (`docs/development/viper-architecture.md`, #1004), a cross-domain
//! module after `lib-tag-removal`: Category delete, which moves a leaf Category's Splits and Bill
//! Plans to the Uncategorised Category of its type, clears it as any Payee's Default category,
//! drops its Limits from every Budget and deletes it, all in one commit, and the Delete category
//! Dialog that confirms it. `gpui`-free and I/O-free.
//!
//! It depends on `lib-categories`, `lib-transactions`, `lib-bills`, `lib-payees` and
//! `lib-budgets`, the feature crates it writes, and none depends on it; the guardrail test in
//! `lib-database` keeps it that way.
//!
//! - `protocols`: the one atomic port, [`CategoryDeletion`], which `lib-database` implements over
//!   its Categories, Transactions, Bills, Payees and Budgets stores, and
//!   [`CategoryDeletionReads`], what the Dialog shows when it opens.
//! - `interactor`: [`DeleteCategory`]. `error`: its refusals.
//! - `presenter`: the Delete category Dialog's Presenter, which closes through `lib-categories`'
//!   outcome so the Categories page hears of it as of its own Dialogs.
//! - `fake`: the in-memory port and reads, and the `world` the tests delete from; `contract`: the
//!   checks both the fake and the adapter keep. `msg`: the Dialog's Messages.
//!
//! [`DeleteCategory`]: interactor::DeleteCategory

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
    CategoryDeleted, CategoryDeletion, CategoryDeletionReads, CategoryInUse, CategoryRemoval,
    SharedCategoryDeletion,
};
