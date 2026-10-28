//! Institutions: the banks, brokers and funds that hold a Ledger's Accounts (`GLOSSARY.md`'s
//! Institution). A VIPER module (`docs/development/viper-architecture.md`, #847): the Entity with
//! its add rule and stub seed (`entity`), one Interactor per intent (`interactor`), one Presenter
//! per View and Dialog (`presenter`), the ports and the `InstitutionsRouter` (`protocols`), the
//! page's default keys (`keymap`) and its Messages (`msg`). `fake` holds the in-memory ports, the
//! stub seed and a recording Router; `contract` the checks both they and `lib-database`'s adapter
//! keep (#849). No UI toolkit and no I/O; the Desktop draws the view models
//! (`bin-desktop/src/institutions/`). Accounts still read Institutions from the `Ledger` read copy
//! until the Accounts page moves.
//!
//! The rule: an Institution's name is required, stored trimmed, and case-insensitively unique. An
//! Institution an Account names can't be deleted.

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
    InstitutionRepository, InstitutionUsage, InstitutionUse, InstitutionsDialog,
    InstitutionsOutcome, InstitutionsPorts, InstitutionsRouter, SharedPorts,
};
