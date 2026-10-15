//! Pure Transactions-surface domain types and the stub dataset behind them
//! (`docs/ux/desktop-mockups/03-transactions/README.md`) -- `gpui`-free, the same "pure state, chrome renders
//! it" split `accounts/mod.rs` uses. All data is stubbed and in-memory (the Desktop Transactions
//! Surface map's Destination): nothing here reads `lib_database`.
//!
//! **The shape runs ahead of the persisted model on purpose.** The glossary makes every Transaction
//! one or more **Splits** (an amount and a Category, plus an optional Payee and any Tags), and the
//! desktop table is built around that; `lib_database`'s `Transactions` row still carries a single
//! `category_id` and `payee_id` with no splits table. Real wiring later has to add splits.
//!
//! **The seed** ([`default_transactions`]) is deterministic (fixed seed, injectable `today`):
//! per-account counts equal the Accounts stub's `transaction_count` for the accounts that take
//! Transactions (Cash, Bank and Credit Card; Loan and Investment accounts take Repayments and
//! Trades instead), about 8% of the random ones are multi-Split, and a few hand-authored cases
//! always exist so every rule the table and filters must handle has a row to exercise it.
//!
//! No transaction account in this seed is in a second Unit, so the mixed-Units rule (the running
//! total blanks) is exercised by the filter engine's unit tests rather than visible in the app
//! until an account in another Unit holds transactions; seeding one would mean changing the
//! Accounts and Settings stubs the earlier maps reconciled to their mockups.

pub(crate) mod chips;
pub(crate) mod filter_form;
pub(crate) mod query;
pub(crate) mod rows;

#[cfg(test)]
pub use lib_ledger::default_transactions;
pub use lib_transactions::{Transaction, takes_transactions};
