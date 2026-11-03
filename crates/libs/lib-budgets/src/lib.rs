//! Pure Budgets domain types and the stub dataset behind them (`docs/ux/desktop-mockups/14-budgets-v2/`,
//! ADR-0028 and ADR-0029) -- `gpui`-free and in-memory, the same "pure state, chrome renders it"
//! split `bills/` uses. Nothing here reads `lib_database`.
//!
//! The rules are the Desktop Budgets Surface map's settled decisions (#383–#387, #399, #400):
//!
//! - A [`Budget`] is a named container: a [`Method`], one Unit locked at creation, the on-budget
//!   Accounts, a default flag and an archived date. It owns a chain of [`LimitRecord`]s per leaf
//!   Expense Category and never owns Transactions, so Budgets may overlap.
//! - A chain holds at most one record per month: **Onward** (from its month until the next record),
//!   **Month-only** (its month alone, the next month falls back to the Onward value carried
//!   forward) and **Stop** (no amount from its month). No amount is Unbudgeted; 0.00 is budgeted.
//! - **Rollover** is held on each record. A closed month's carry is its effective budget (amount
//!   plus carry in) minus Spent, compounds uncapped, and resets on a Stop or an unbudgeted month.
//!   The current month never carries, and carries are computed on read, never stored.
//! - **Spent** is signed (a refund nets off), positive when money went out, and counts every
//!   Transaction Status on the Budget's on-budget Accounts. **Known Costs** are the open Bill
//!   Schedule entries on those Accounts: the current month's plus carried-in Overdue, none for a
//!   past month, and a future month's own.
//! - Every figure on every surface comes from [`period_figures`], [`history`] and their
//!   helpers, all pure over an injectable `today`, so nothing is counted twice.
//!
//! Ids are the Ledger's `lib-core` newtypes (ADR-0040). The other domains are read through the
//! read ports in [`protocols`], as `lib-budgets`' own rows, so the crate depends on no other
//! feature crate.
//!
//! A VIPER module (`docs/development/viper-architecture.md`): the Entity (`entity`: the Budget and
//! its chain, the chain's edits, the Budgets set and the stub seed), one Interactor per intent
//! (`interactor`), one Presenter per View and Dialog (`presenter`), the ports and the Router
//! (`protocols`), the default keys (`keymap`) and the Messages (`msg`). Each surface's figures sit
//! in their own calculation module (`figures`, `plan`, `history`, `detail`), re-exported here. The Budgets are kept behind [`BudgetRepository`], which
//! `lib-database` implements; `fake` holds the in-memory ports and `contract` the checks both they

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod detail;
mod entity;
mod error;
pub mod fake;
mod figures;
mod history;
mod interactor;
pub mod keymap;
pub mod msg;
mod plan;
pub mod presenter;
pub mod protocols;
#[cfg(test)]
mod test_support;
mod tree;

pub use detail::*;
pub use entity::*;
pub use error::{Error, Result};
pub use figures::*;
pub use history::*;
pub use interactor::*;
pub use plan::*;
pub use protocols::*;
