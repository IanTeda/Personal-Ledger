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
//! - **Data Models**: Domain-specific types for financial entities (planned)
//!
//! ## Key Features
//!
//! - **SQLite Integration**: Built on SQLx for robust, async SQLite operations
//! - **Connection Pooling**: Configurable connection pools with automatic lifecycle management
//! - **Type Safety**: Strong typing for database operations and error handling
//! - **Async-First**: Non-blocking operations for high-performance applications
//! - **Health Monitoring**: Built-in connection health checks and validation
//! - **Configuration-Driven**: Flexible configuration through environment variables and files
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use lib_database::{DatabaseConfig, DatabaseConnection};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure the database connection
//! let config = DatabaseConfig::default();
//!
//! // Create a connection pool
//! let connection = DatabaseConnection::new(config).await?;
//!
//! // Verify the connection
//! connection.health_check().await?;
//!
//! // Access the underlying pool for queries
//! let pool = connection.pool();
//! // ... perform database operations ...
//! # Ok(())
//! # }
//! ```
//!
//! ## Error Handling
//!
//! All database operations return `Result<T, DatabaseError>` for consistent error propagation.
//! The `DatabaseError` enum provides specific error variants for different failure modes:
//!
//! ```rust
//! use lib_database::DatabaseResult;
//!
//! fn perform_database_operation() -> DatabaseResult<()> {
//!     // Database operations that may fail...
//!     Ok(())
//! }
//! ```
//!
//! ## Configuration
//!
//! Database behaviour is controlled through `DatabaseConfig`, which supports:
//! - Connection pool sizing (min/max connections)
//! - Timeout configuration (acquire, idle, lifetime)
//! - Database URL specification
//!
//! Configuration can be loaded from environment variables, config files, or programmatic setup.
//!
//! ## Thread Safety
//!
//! All types in this crate are thread-safe and can be shared across async tasks.
//! The underlying SQLx connection pools handle concurrent access safely.

/// Database entity categories for organising financial records.
mod categories;
pub use categories::{Categories};

mod config;
/// Database configuration settings for connection pool management.
///
/// `DatabaseConfig` encapsulates all database-related settings including connection pool
/// parameters, timeouts, and database URL. It provides a flexible configuration system
/// that supports loading from environment variables, configuration files, or programmatic
/// setup with sensible defaults for SQLite databases.
///
/// ## Key Configuration Options
///
/// - **Database URL**: Connection string for the SQLite database
/// - **Connection Pool**: Maximum and minimum connection limits
/// - **Timeouts**: Acquire, idle, and lifetime timeout settings
/// - **Default Values**: Pre-configured defaults suitable for most applications
///
/// ## Usage
///
/// ```rust
/// use lib_database::DatabaseConfig;
///
/// // Use default configuration (recommended for most cases)
/// let config = DatabaseConfig::default();
///
/// // Access configuration values
/// let url = config.url();
/// let max_conn = config.max_connections();
///
/// // Use with DatabaseConnection
/// # // let connection = DatabaseConnection::new(config).await?;
/// ```
///
/// ## Configuration Sources
///
/// Configuration can be loaded from:
/// - Environment variables (with `PERSONAL_LEDGER_DATABASE__` prefix)
/// - Configuration files (INI/TOML format)
/// - Programmatic construction
///
/// See the [`config`] module for detailed configuration options and examples.
pub use config::DatabaseConfig;

mod error;
/// Core error type for all database operations.
///
/// This enum standardises error handling across the database layer, wrapping
/// connection failures, SQLx errors, migration issues, and validation problems.
/// All database functions should return `Result<T, DatabaseError>` for consistent
/// error propagation.
///
/// ## Error Variants
///
/// - `Connection`: Database connection failures (invalid config, unreachable server, etc.)
/// - `Sqlx`: Errors from the `sqlx` crate (query, pool, etc.)
/// - `Migration`: Errors from running migrations
/// - `Validation`: Domain validation errors (constraint violations, etc.)
/// - `NotFound`: Resource not found errors
/// - `Generic`: Catch-all for miscellaneous DB errors
///
/// ## Usage
///
/// ```rust
/// use lib_database::DatabaseResult;
///
/// fn database_operation() -> DatabaseResult<()> {
///     // Database operations that may fail...
///     Ok(())
/// }
/// ```
///
/// See [`error`] module for detailed documentation and examples.
pub use error::DatabaseError;

/// Result type alias for database operations.
///
/// Convenience type for functions that return database results.
/// Equivalent to `Result<T, DatabaseError>`.
///
/// # Examples
///
/// ```rust
/// use lib_database::DatabaseResult;
///
/// fn get_user(id: i32) -> DatabaseResult<()> {
///     // Database operation that may fail...
///     Ok(())
/// }
/// ```
pub use error::DatabaseResult;

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

// Future Development Notes:
//
// The following modules are planned for future implementation:
//
// - `pool`: Advanced connection pool wrapper with lifecycle management
// - `categories`: Financial category domain models and validation
// - Additional data models for transactions, accounts, and financial entities
//
// These will be implemented as the application requirements evolve.