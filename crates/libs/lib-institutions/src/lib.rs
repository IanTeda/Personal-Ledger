//! Institutions: the banks, brokers and funds that hold a Ledger's Accounts (`GLOSSARY.md`'s
//! Institution). A VIPER module being migrated (`docs/development/viper-architecture.md`, #847):
//! the Entity with its add rule and stub seed (`entity`), one Interactor per intent
//! (`interactor`) and the repository port (`protocols`). `fake` holds the in-memory port and the
//! stub seed; `contract` the checks both it and `lib-database`'s adapter keep (#849). The page's
//! Presenter, keymap and Messages arrive with the page (#850); until then the Desktop's Settings ›
//! Institutions page reads them from the `LedgerStore`'s read copy and writes them through a
//! `LedgerCommand`, which the store forwards to the port. No UI toolkit and no I/O.
//!
//! The rule: an Institution's name is required, stored trimmed, and case-insensitively unique.
//! Only add exists; edit and delete are a backlog ticket.

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
pub use protocols::InstitutionRepository;
