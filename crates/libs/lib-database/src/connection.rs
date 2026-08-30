//! # Database Connection Management
//!
//! This module provides the `DatabaseConnection` struct, which manages SQLite database connections
//! through a connection pool. It serves as a high-level wrapper around SQLx's `SqlitePool`,
//! providing a clean API for database operations with proper error handling and configuration.
//!
//! ## Connection Pool Management
//!
//! The `DatabaseConnection` handles the lifecycle of database connections:
//! - **Pool Creation**: Configures and establishes connection pools based on `DatabaseConfig`
//! - **Pool Access**: Provides safe access to the underlying SQLx pool for queries
//! - **Health Monitoring**: Includes health check functionality for connection validation
//! - **Resource Management**: Proper cleanup and ownership transfer of pool resources
//!
//! ## Usage
//!
//! ### Basic Connection Setup
//!
//! ```rust,no_run
//! use lib_database::{DatabaseConnection, DatabaseConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create configuration
//! let config = DatabaseConfig::default();
//!
//! // Establish database connection
//! let connection = DatabaseConnection::new(config).await?;
//!
//! // Use the connection for queries
//! let pool = connection.pool();
//! let result = sqlx::query("SELECT 1").fetch_one(pool).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Health Checking
//!
//! ```rust,no_run
//! use lib_database::{DatabaseConnection, DatabaseConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let connection = DatabaseConnection::new(DatabaseConfig::default()).await?;
//!
//! // Verify connection health
//! connection.health_check().await?;
//! println!("Database connection is healthy!");
//! # Ok(())
//! # }
//! ```
//!
//! ### Pool Ownership Transfer
//!
//! ```rust,no_run
//! use lib_database::{DatabaseConnection, DatabaseConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let connection = DatabaseConnection::new(DatabaseConfig::default()).await?;
//!
//! // Transfer ownership of the pool
//! let pool = connection.into_pool();
//!
//! // Now you own the pool directly
//! let result = sqlx::query("SELECT 1").fetch_one(&pool).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture Notes
//!
//! - Built on top of SQLx's `SqlitePool` for robust connection pooling
//! - Configurable pool settings through `DatabaseConfig`
//! - Error handling mapped to domain-specific `DatabaseError` types
//! - Async-first design for non-blocking database operations
//! - Thread-safe pool access for concurrent operations

#![allow(unused_imports)]

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

use crate::{DatabaseError, DatabaseResult, DatabaseConfig};

/// Database connection wrapper providing high-level access to SQLite connection pools.
///
/// `DatabaseConnection` manages the lifecycle of database connections and provides
/// a safe, ergonomic interface for database operations. It wraps SQLx's `SqlitePool`
/// with proper error handling and configuration management.
///
/// ## Key Features
///
/// - **Connection Pool Management**: Automatic pool creation with configurable settings
/// - **Health Monitoring**: Built-in connection health checks
/// - **Resource Safety**: Proper ownership and cleanup of database resources
/// - **Error Handling**: Domain-specific error types for consistent error propagation
/// - **Async Support**: Non-blocking operations for high-performance applications
///
/// ## Thread Safety
///
/// The underlying `SqlitePool` is thread-safe and can be shared across async tasks.
/// Multiple concurrent operations are supported through the connection pool.
///
/// ## Example
///
/// ```rust,no_run
/// use lib_database::{DatabaseConnection, DatabaseConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Configure the connection
/// let config = DatabaseConfig::default();
///
/// // Create the connection
/// let connection = DatabaseConnection::new(config).await?;
///
/// // Use for database operations
/// let pool = connection.pool();
/// // ... perform queries ...
/// # Ok(())
/// # }
/// ```
pub struct DatabaseConnection {
    /// The underlying SQLx SQLite connection pool.
    ///
    /// This pool manages multiple database connections with automatic lifecycle
    /// management, connection reuse, and performance optimizations. The pool
    /// is configured based on the `DatabaseConfig` provided during construction.
    pool: SqlitePool,
}

impl DatabaseConnection {
    /// Create a new database connection with the specified configuration.
    ///
    /// This method establishes a connection pool to the SQLite database using the provided
    /// configuration settings. The pool is configured with connection limits, timeouts,
    /// and other settings as specified in the `DatabaseConfig`.
    ///
    /// # Parameters
    ///
    /// * `config` - The database configuration containing connection settings, pool limits,
    ///   and timeout values.
    ///
    /// # Returns
    ///
    /// Returns a `DatabaseResult<Self>` containing the new `DatabaseConnection` if successful,
    /// or a `DatabaseError::Connection` if the connection pool cannot be established.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The database URL is invalid or unreachable
    /// - The database file cannot be created or accessed
    /// - Pool configuration parameters are invalid
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_database::{DatabaseConnection, DatabaseConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = DatabaseConfig::default();
    /// let connection = DatabaseConnection::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: DatabaseConfig) -> DatabaseResult<Self> {
        let pool_options = SqlitePoolOptions::new()
            .max_connections(config.max_connections())
            .min_connections(config.min_connections())
            .acquire_timeout(std::time::Duration::from_secs(config.acquire_timeout().num_seconds() as u64))
            .idle_timeout(std::time::Duration::from_secs(config.idle_timeout_seconds as u64))
            .max_lifetime(std::time::Duration::from_secs(config.max_lifetime_seconds as u64));

        let pool = pool_options.connect(config.url()).await
            .map_err(|e| DatabaseError::Connection(format!("Failed to connect to database pool: {}", e)))?;
        
        Ok(Self { pool })
    }

    /// Get a reference to the underlying database pool.
    ///
    /// This allows direct access to the SQLx pool for executing queries.
    ///
    /// # Returns
    ///
    /// A reference to the `SqlitePool`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_database::{DatabaseConnection, DatabaseConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = DatabaseConfig::default();
    /// let connection = DatabaseConnection::new(config).await?;
    /// let pool = connection.pool();
    ///
    /// // Use the pool for queries
    /// let result = sqlx::query("SELECT 1").fetch_one(pool).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Consume this connection and return the underlying pool.
    ///
    /// This transfers ownership of the pool to the caller.
    ///
    /// # Returns
    ///
    /// The owned `SqlitePool`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_database::{DatabaseConnection, DatabaseConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = DatabaseConfig::default();
    /// let connection = DatabaseConnection::new(config).await?;
    /// let pool = connection.into_pool();
    ///
    /// // Now you own the pool
    /// let result = sqlx::query("SELECT 1").fetch_one(&pool).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_pool(self) -> SqlitePool {
        self.pool
    }

    /// Check if the database connection is healthy.
    ///
    /// Performs a simple query to verify the connection is working.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the connection is healthy, or a `DatabaseError` if not.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lib_database::{DatabaseConnection, DatabaseConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = DatabaseConfig::default();
    /// let connection = DatabaseConnection::new(config).await?;
    /// connection.health_check().await?;
    /// println!("Database connection is healthy!");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check(&self) -> DatabaseResult<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::Connection(format!("Health check failed: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connection_new_with_default_config() {
        // Use in-memory database for testing since file-based might not work in test environment
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            ..DatabaseConfig::default()
        };
        
        // This should succeed with default config
        let result = DatabaseConnection::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_connection_new_with_custom_config() {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            min_connections: 1,
            acquire_timeout_seconds: 10,
            idle_timeout_seconds: 60,
            max_lifetime_seconds: 300,
        };
        
        // This should succeed with custom config
        let result = DatabaseConnection::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_connection_pool_access() {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            ..DatabaseConfig::default()
        };

        let connection = DatabaseConnection::new(config).await.unwrap();

        // Test pool() method
        let pool_ref = connection.pool();
        assert!(!pool_ref.is_closed());
    }

    #[tokio::test]
    async fn test_database_connection_into_pool() {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            ..DatabaseConfig::default()
        };

        let connection = DatabaseConnection::new(config).await.unwrap();

        // Test into_pool() method
        let pool = connection.into_pool();
        assert!(!pool.is_closed());
    }

    #[tokio::test]
    async fn test_database_connection_health_check() {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            ..DatabaseConfig::default()
        };
        
        let connection = DatabaseConnection::new(config).await.unwrap();
        
        // Test health_check() method
        let result = connection.health_check().await;
        assert!(result.is_ok());
    }
}