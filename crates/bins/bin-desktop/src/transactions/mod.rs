//! Pure Transactions-surface domain types (`docs/ux/desktop-mockups/03-transactions/README.md`) --
//! `gpui`-free, the same "pure state, chrome renders it" split `accounts/mod.rs` uses. The View
//! reads the Ledger's Transactions, a read copy of the Transactions port that `ports.rs` keeps in
//! line (#774); the stub seed is `lib_transactions::fake`'s, joined to the other stubs by
//! `lib_ledger::default_transactions`.

pub(crate) mod chips;
pub(crate) mod filter_form;
pub(crate) mod ports;
pub(crate) mod query;
pub(crate) mod router;
pub(crate) mod rows;

#[cfg(test)]
pub use lib_ledger::default_transactions;
pub use lib_transactions::{Transaction, takes_transactions};
