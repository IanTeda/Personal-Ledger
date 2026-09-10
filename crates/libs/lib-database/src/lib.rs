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
//! - **Configuration** ([`DatabaseConfig`]): Database connection settings and pool configuration
//! - **Connections** ([`DatabaseConnection`]): High-level connection pool management
//! - **Error Handling** ([`DatabaseError`], [`DatabaseResult`]): Domain-specific error types
//! - **Data Models**: Domain-specific types for financial entities ([`Categories`], [`SyncUser`], [`ChangeSet`])
//!
//! ## Key Features
//!
//! - **SQLite Integration**: Built on SQLx for robust, async SQLite operations
//! - **Connection Pooling**: Configurable connection pools with automatic lifecycle management
//! - **Type Safety**: Strong typing for database operations and error handling
//! - **Async-First**: Non-blocking operations for high-performance applications
//! - **Health Monitoring**: Built-in connection health checks and validation
//! - **Configuration-Driven**: Flexible configuration through environment variables and files

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

/// Single-entry Transactions against exactly one Account and one Category (CC-TUI-009).
pub mod transactions;
pub use transactions::{TransactionScope, Transactions};

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
/// ## Example
///
/// ```rust,no_run
/// use lib_database::{DatabaseConnection, DatabaseConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = DatabaseConfig::default();
/// let connection = DatabaseConnection::new(config).await?;
///
/// // Verify connection health
/// connection.health_check().await?;
/// # Ok(())
/// # }
/// ```
///
/// See [`connection`] module for detailed API documentation.
pub use connection::DatabaseConnection;
