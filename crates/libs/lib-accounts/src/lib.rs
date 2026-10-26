//! Accounts: where a Ledger's money and holdings sit, each at an Institution and in one Unit
//! (`GLOSSARY.md`'s Account). A VIPER module (`docs/development/viper-architecture.md`, #847): the
//! Entity with its rules and stub seed (`entity`), one Interactor per intent (`interactor`), one
//! Presenter per View and Dialog (`presenter`), the ports and the `AccountsRouter` (`protocols`),
//! the page's default keys (`keymap`) and its Messages (`msg`). `fake` holds the in-memory ports,
//! the stub seed and a recording Router; `contract` the checks both they and `lib-database`'s
//! adapter keep (#853). Delete is `lib-account-deletion`'s, a cross-domain write the page's Router
//! opens. No UI toolkit and no I/O; the Desktop draws the view models (`bin-desktop/src/accounts/`).

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
    AccountInstitution, AccountInstitutions, AccountRepository, AccountUnit, AccountUnits,
    AccountUsage, AccountUse, AccountsDialog, AccountsOutcome, AccountsPorts, AccountsRouter,
    SharedPorts,
};
