//! Categories: the tree a Ledger's Splits and Bill Plans are filed under (`GLOSSARY.md`'s
//! Category). Only **leaf** Categories take Splits; a parent exists to roll up and to filter by.
//! Depth is capped at three levels, and a child is its parent's type.
//!
//! A VIPER module being migrated (`docs/development/viper-architecture.md`, #847): the Entity with
//! its tree rules and stub seed (`entity`), one Interactor per intent (`interactor`) and the
//! repository port (`protocols`). `fake` holds the in-memory port and the stub seed; `contract`
//! the checks both it and, once Categories persist (#858), `lib-database`'s adapter keep. The
//! page's Presenters, keymap and Messages arrive with the page (#859); until then the Desktop
//! writes Categories through `LedgerCommand`s and nothing persists them. Delete (reassign) stays
//! in `lib-ledger`, since it re-points other domains' rows (Q6). No UI toolkit and no I/O.

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
pub use protocols::CategoryRepository;
