//! The Transaction Import module's Desktop half (`docs/development/viper-architecture.md`, #947):
//! the Import "match payees" step's View, shown in place of the Transactions list on `:import`, the
//! Noun staying Transactions. Every rule, every piece of text, the keymap and the Presenter are
//! `lib-transaction-import`'s; this folder draws the view model.
//!
//! - `view/page.rs` is the step Presenter's `PageView`, which the generic `ModulePage` hosts, open
//!   or closed; `view/mod.rs` draws the open step.
//! - `router.rs` is `impl ImportRouter`: a commit lands on the Transactions list's first row, and
//!   Notices go onto the bin's queue.
//!
//! A commit writes through the ports `LedgerPorts::transaction_import` builds, over the Payees and
//! Transactions stores, so its new Payees and Transactions join the one pending batch and land in
//! the commit of the input that continued.

pub(crate) mod router;
pub(crate) mod view;

pub use router::import_page;
pub use view::page::ImportPage;
