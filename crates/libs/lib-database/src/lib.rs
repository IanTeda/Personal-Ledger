//! # Personal Ledger Database Library
//!
//! This crate provides the core database functionality for the Personal Ledger application,
//! offering a high-level, type-safe interface for SQLite database operations. It encapsulates
//! connection management, error handling, and data access patterns specific to financial
//! record keeping.
//!
//! ## Architecture Overview
//!
//! The library is structured around several key components:
//!
//! - **Connections** ([`DatabaseConnection`]): High-level connection pool management, built
//!   from a database URI -- a Client's `[Personal-Ledger] file` or the Sync Server's
//!   `[Sync-Server] database_uri` (see `lib_config`); pool settings themselves are fixed
//!   (see `connection`'s `MAX_CONNECTIONS` etc)
//! - **Error Handling** ([`DatabaseError`], [`DatabaseResult`]): Domain-specific error types
//! - **Data Models**: Domain-specific types for financial entities ([`Categories`], [`SyncUser`], [`ChangeSet`])
//!
//! ## Key Features
//!
//! - **SQLite Integration**: Built on SQLx for robust, async SQLite operations
//! - **Connection Pooling**: Fixed-size connection pool with automatic lifecycle management
//! - **Type Safety**: Strong typing for database operations and error handling
//! - **Async-First**: Non-blocking operations for high-performance applications
//! - **Health Monitoring**: Built-in connection health checks and validation

/// Database entity categories for organising financial records.
mod categories;
pub use categories::Categories;

/// Currencies, cryptocurrencies, stocks, and other tradeable instruments Accounts and
/// Transactions are denominated in (CC-TUI-005).
pub mod units;
pub use units::Units;

/// The domain Account entity — Cash/Bank/Credit Card/Investment/Loan (CC-TUI-008). Not the
/// Sync Server's own auth credential; see `sync_users` for that.
pub mod accounts;
pub use accounts::Accounts;

/// What a per-entity totals report scopes its Transactions to: a Unit or an Account.
mod transaction_scope;
pub use transaction_scope::TransactionScope;

/// Who a Transaction's money moved to or from (CC-TUI-007), a first-class entity as of
/// ADR-0012.
pub mod payees;
pub use payees::Payees;

/// A Payee's former names, preserved by `Payees::rename` (ADR-0012).
pub mod payee_aliases;
pub use payee_aliases::PayeeAliases;

/// Point-in-time assertions of what an Account's Balance should be (FR.28-32, CC-TUI-013).
pub mod balance_checks;
pub use balance_checks::BalanceChecks;

/// A cap on Transaction totals in one Expense Category over a recurring period (FR.22-27,
/// CC-TUI-010).
pub mod budgets;
pub use budgets::{BudgetProgress, Budgets};

/// The Sync Server's own auth user store (ADR-0010).
pub mod sync_users;
pub use sync_users::SyncUser;

/// User-editable Ledger-scoped settings (default Unit, colour theme, date format,
/// decimal/thousands separator), synced across a user's Clients in future (ADR-0014).
/// Singleton by convention (mirroring `sync_users`), not a database constraint.
pub mod preferences;
pub use preferences::Preferences;

/// The Sync Server's durable Change Set log (ADR-0009).
pub mod change_sets;
pub use change_sets::ChangeSet;

/// The per-domain adapters behind the VIPER modules' ports (ADR-0046).
pub mod adapter;
pub use adapter::bill_payments::BillPaymentsAdapter;
pub use adapter::bills::BillsAdapter;
pub use adapter::institutions::InstitutionsAdapter;
pub use adapter::payees::PayeesAdapter;
pub use adapter::store::{
    BillsStore, CountingCommit, InstitutionsStore, PayeesStore, SharedBillsStore,
    SharedPayeesStore, SharedTransactionsStore, TransactionsStore, UnitsStore,
};
pub use adapter::transactions::TransactionsAdapter;
pub use adapter::units::UnitsAdapter;

/// A Client's Ledger file, opened at start-up with its adapters and the writer, and the
/// [`Stores`] its Shell is built on.
pub mod storage;
pub use storage::{Storage, Stores, open_storage};

/// The dev seed: each persisted domain's stub seed, written through its adapter.
pub mod seed;

/// The writes every adapter makes in one Interactor call, committed together (ADR-0046).
pub mod commit;
pub use commit::{Commit, InMemoryCommit, PendingWrites};

/// The one writer thread every adapter saves through (ADR-0046).
pub mod writer;
pub use writer::{Domain, WriteFailure, Writer};

/// Test-only rows for the readers that total Transactions.
#[cfg(test)]
mod test_support;

/// Crate error type for all database operations.
mod error;
pub use error::Error;

/// Crate Result type alias used across database modules.
///
/// Use `Result<T>` for functions that return `T` or a `Error`.
/// This keeps signatures concise and makes it clear the function is database-related.
/// TODO: Move this to crate namespace, need to add bin errors and results
pub type Result<T> = std::result::Result<T, Error>;

mod connection;
/// Database connection management and pool handling.
///
/// Provides high-level access to SQLite connection pools with automatic lifecycle
/// management, health monitoring, and resource safety. The `DatabaseConnection`
/// struct wraps SQLx's connection pool with domain-specific error handling and
/// configuration.
///
/// ## Key Features
///
/// - Connection pool creation with configurable settings
/// - Health check functionality for connection validation
/// - Safe access to underlying SQLx pools
/// - Ownership transfer capabilities
///
/// See [`connection`] module for detailed API documentation and unit tests.
pub use connection::DatabaseConnection;
