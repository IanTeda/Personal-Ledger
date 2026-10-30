//! The Transactions module's Desktop half (`docs/development/viper-architecture.md`): the View for
//! the Transactions page. Every rule, every piece of text and the keymap are `lib-transactions`';
//! this folder turns gpui keys and clicks into Presenter events and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Shell hosts as the
//!   Transactions destination; `view/header.rs`, `view/table.rs`, `view/footer.rs` and
//!   `view/filter_popover.rs` draw it.
//! - `router.rs` is the module's entry point for other modules' Routers. Halfway (#923): other
//!   modules still reach the page through `Reveal` and `Root::open_transactions_filtered`.
//! - `ports.rs` answers the page's ports from the stores, and holds the forwarding shim that saves
//!   today's writers' Transaction changes (#774).
//!
//! The stub seed is `lib_transactions::fake`'s, joined to the other stubs by
//! `lib_ledger::default_transactions`.

pub(crate) mod ports;
pub(crate) mod router;
pub(crate) mod view;

#[cfg(test)]
pub use lib_ledger::default_transactions;
pub use lib_transactions::{Transaction, takes_transactions};
pub use view::page::TransactionsPage;
