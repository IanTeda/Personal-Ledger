//! Payees: who a Ledger pays and is paid by (`GLOSSARY.md`'s Payee), each with the Payee Aliases
//! that match it on import. A VIPER module (`docs/development/viper-architecture.md`, #832): the
//! Entity (`entity`, with the stub seed from the Payees handoff's sample list,
//! `docs/ux/desktop-mockups/20-payees/`), one Interactor per intent (`interactor`), one Presenter
//! per View and Dialog (`presenter`), the ports and the `PayeesRouter` (`protocols`), the page's
//! default keys (`keymap`) and its Messages (`msg`). `fake` holds the in-memory ports, the stub
//! seed and a recording Router; `contract` the checks both they and `lib-database`'s adapter keep.
//! No UI toolkit and no I/O; the Desktop draws the view models (`bin-desktop/src/payees/`).
//!
//! The rules are ADR-0012's and its two amendments (#282, #283):
//!
//! - A Payee's name is required and case-insensitively unique. Renaming one keeps the old name as
//!   a Payee Alias; renaming it to one of its own aliases drops that alias.
//! - A Payee Alias (the UI's "match rule") is stored trimmed and upper-cased, deduplicated, empty
//!   input ignored, and is unique across Payees. It matches raw text as a case-insensitive
//!   *contains*; the longest matching alias wins, then the Payee name alphabetically. Only active
//!   Payees match.
//! - A Payee no Split or Bill Plan names may be hard-deleted with its aliases; a named one can only
//!   be deactivated (and reactivated). Inactive Payees drop out of the Payee counts. What names a
//!   Payee is other domains' rows, read through the [`PayeeUsage`] port.
//! - A default Category is an optional leaf that only pre-fills new Splits.

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
    PayeeCategories, PayeeCategory, PayeeChange, PayeeRepository, PayeeUsage, PayeeUse,
    PayeesDialog, PayeesOutcome, PayeesPorts, PayeesRouter, PortsMut, SharedPorts,
};
