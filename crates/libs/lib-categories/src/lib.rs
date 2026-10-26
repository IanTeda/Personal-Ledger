//! Categories: the tree a Ledger's Splits and Bill Plans are filed under (`GLOSSARY.md`'s
//! Category). Only **leaf** Categories take Splits; a parent exists to roll up and to filter by.
//! Depth is capped at three levels, and a child is its parent's type.
//!
//! A VIPER module (`docs/development/viper-architecture.md`, #847, #859): the Entity with its tree
//! rules and stub seed (`entity`), one Interactor per intent (`interactor`), one Presenter per
//! View and Dialog (`presenter`), the ports and the `CategoriesRouter` (`protocols`), the page's
//! default keys (`keymap`) and its Messages (`msg`). `fake` holds the in-memory ports, the stub
//! seed and a recording Router; `contract` the checks both they and `lib-database`'s adapter keep
//! (#858). Delete re-points other domains' rows, so it goes through the reassign port
//! (`CategoryReassign`, Q6), which a Client answers through its Ledger until `lib-reassign`
//! (#813). No UI toolkit and no I/O; the Desktop draws the view models
//! (`bin-desktop/src/categories/`).

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod entity;
pub mod fake;
pub mod interactor;
pub mod keymap;
pub mod msg;
pub mod presenter;
pub mod protocols;

pub use entity::*;
pub use protocols::{
    CategoriesDialog, CategoriesOutcome, CategoriesRouter, CategoryBudget, CategoryChange,
    CategoryReassign, CategoryReassigned, CategoryRepository, CategoryUsage, CategoryUse,
    MonthlyBudget, MonthlyBudgetRefusal, ReassignRefusal, SharedCategories,
};
