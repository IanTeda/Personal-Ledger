
//! # Database Configuration
//!
//! This module provides configuration structures for the database connection pool in the Personal Ledger application.
//!
//! The database configuration allows users to customise connection pool settings such as
//! connection limits, timeouts, and database URL through configuration files, environment variables,
//! or programmatic settings.
//!
//! ## Configuration Structure
//!
//! The `DatabaseConfig` struct encapsulates all database-related settings:
//! - **URL**: Database connection string
//! - **Connection Pool Settings**: Max/min connections, timeouts
//! - **Default Behaviors**: Provides sensible defaults for SQLite use
//!
//! ## Usage
//!
//! ```rust
//! use lib_database::DatabaseConfig;
//!
//! // Create default configuration
//! let config = DatabaseConfig::default();
//!
//! // Access configuration values
//! let url = config.url();
//! let max_conn = config.max_connections();
//!
//! // Use with connection pool
//! # // Note: This would normally require DatabaseConnection
//! # // let connection = DatabaseConnection::new(config).await?;
//! ```
//!
//! ## Configuration File Example
//!
//! ```ini
//! [database]
//! url = "sqlite:./personal-ledger.sqlite"
//! max_connections = 10
//! min_connections = 1
//! acquire_timeout_seconds = 30
//! idle_timeout_seconds = 600
//! max_lifetime_seconds = 1800
//! ```
//!
//! ## Environment Variables
//!
//! Database settings can be overridden using environment variables with the `PERSONAL_LEDGER_DATABASE__` prefix:
//!
//! ```bash
//! # Database connection
//! PERSONAL_LEDGER_DATABASE__URL=sqlite:/tmp/test.db
//!
//! # Connection pool settings
//! PERSONAL_LEDGER_DATABASE__MAX_CONNECTIONS=20
//! PERSONAL_LEDGER_DATABASE__MIN_CONNECTIONS=5
//!
//! # Timeout settings (in seconds)
//! PERSONAL_LEDGER_DATABASE__ACQUIRE_TIMEOUT_SECONDS=60
//! PERSONAL_LEDGER_DATABASE__IDLE_TIMEOUT_SECONDS=300
//! PERSONAL_LEDGER_DATABASE__MAX_LIFETIME_SECONDS=3600
//! ```

use chrono::Duration;
use crate::{DatabaseResult, DatabaseError};

/// Default database URL for SQLite database.
///
/// This constant defines the default SQLite database file location.
/// Uses a relative path that works well for development and simple deployments.
const DEFAULT_URL: &str = "sqlite:./personal-ledger.sqlite";

/// Default maximum number of connections in the pool.
///
/// This provides a reasonable upper limit for connection pool size.
/// Higher values may be needed for high-traffic applications.
const DEFAULT_MAX_CONNECTIONS: u32 = 10;

/// Default minimum number of connections in the pool.
///
/// Keeping a minimum number of connections ready can improve performance
/// by avoiding connection creation overhead.
const DEFAULT_MIN_CONNECTIONS: u32 = 1;

/// Default timeout for acquiring a connection from the pool (in seconds).
///
/// If no connection becomes available within this time, an error is returned.
/// This prevents indefinite waiting in high-load scenarios.
const DEFAULT_ACQUIRE_TIMEOUT_SECONDS: i64 = 30;

/// Default idle timeout for connections (in seconds).
///
/// Connections that remain idle beyond this time may be closed to free resources.
/// This helps manage resource usage in low-activity periods.
const DEFAULT_IDLE_TIMEOUT_SECONDS: i64 = 600;

/// Default maximum lifetime for connections (in seconds).
///
/// Connections older than this will be closed and replaced.
/// This helps prevent issues with stale connections or database server limits.
const DEFAULT_MAX_LIFETIME_SECONDS: i64 = 1800;

/// Configuration structure for database connection pool settings.
///
/// This struct encapsulates all configurable aspects of the database connection pool,
/// providing a clean interface for applications to customise database behaviors.
/// The configuration is designed to be serializable, allowing it to be loaded
/// from various sources like configuration files, environment variables, or
/// programmatically constructed.
///
/// # Serialization
///
/// The struct supports both JSON and TOML serialization formats, making it
/// compatible with common configuration file formats.
///
/// # Default Values
///
/// When created with `Default::default()`, the configuration uses SQLite-optimised
/// defaults that balance performance with resource usage.
///
/// # Examples
///
/// ```rust
/// use lib_database::DatabaseConfig;
///
/// // Default configuration
/// let config = DatabaseConfig::default();
///
/// // Custom configuration
/// let custom_config = DatabaseConfig {
///     url: "sqlite:/tmp/custom.db".to_string(),
///     max_connections: 20,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct DatabaseConfig {
    /// The database connection URL.
    ///
    /// This should be a valid database URL string. For SQLite, use the format:
    /// - `sqlite:./relative/path.db` for relative paths
    /// - `sqlite:/absolute/path.db` for absolute paths
    /// - `sqlite::memory:` for in-memory databases
    ///
    /// The URL will be validated when creating the connection pool.
    pub url: String,

    /// Maximum number of connections in the pool.
    ///
    /// This limits the total number of simultaneous connections to the database.
    /// Higher values allow more concurrent operations but use more resources.
    /// Must be greater than or equal to `min_connections`.
    pub max_connections: u32,

    /// Minimum number of connections to maintain in the pool.
    ///
    /// The pool will try to keep at least this many connections ready.
    /// This can improve performance by reducing connection creation overhead.
    /// Must be less than or equal to `max_connections`.
    pub min_connections: u32,

    /// Timeout for acquiring a connection from the pool (in seconds).
    ///
    /// If no connection becomes available within this time, an error is returned.
    /// This prevents the application from hanging during high load.
    /// Must be positive.
    pub acquire_timeout_seconds: i64,

    /// Idle timeout for connections (in seconds).
    ///
    /// Connections that remain unused beyond this time may be closed.
    /// This helps manage resource usage when the application is idle.
    /// Use 0 to disable idle timeouts.
    pub idle_timeout_seconds: i64,

    /// Maximum lifetime for connections (in seconds).
    ///
    /// Connections older than this will be closed and replaced.
    /// This helps prevent issues with database server connection limits.
    /// Use 0 to disable lifetime limits.
    pub max_lifetime_seconds: i64,
}

impl Default for DatabaseConfig {
    /// Creates a default database configuration.
    ///
    /// The default configuration provides sensible settings for SQLite databases
    /// in development and small-scale production environments. The settings balance
    /// performance with resource usage.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    ///
    /// let config = DatabaseConfig::default();
    /// assert_eq!(config.url(), "sqlite:./personal-ledger.sqlite");
    /// assert_eq!(config.max_connections(), 10);
    /// ```
    fn default() -> Self {
        Self {
            url: DEFAULT_URL.to_string(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            min_connections: DEFAULT_MIN_CONNECTIONS,
            acquire_timeout_seconds: DEFAULT_ACQUIRE_TIMEOUT_SECONDS,
            idle_timeout_seconds: DEFAULT_IDLE_TIMEOUT_SECONDS,
            max_lifetime_seconds: DEFAULT_MAX_LIFETIME_SECONDS,
        }
    }
}

impl DatabaseConfig {
    /// Get the database URL.
    ///
    /// Returns the configured database connection URL as a string slice.
    ///
    /// # Returns
    ///
    /// The database URL that can be passed to connection pool creation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    ///
    /// let config = DatabaseConfig::default();
    /// let url = config.url();
    /// assert_eq!(url, "sqlite:./personal-ledger.sqlite");
    /// ```
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the maximum number of connections.
    ///
    /// Returns the configured maximum pool size.
    ///
    /// # Returns
    ///
    /// The maximum number of connections allowed in the pool.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    ///
    /// let config = DatabaseConfig::default();
    /// assert_eq!(config.max_connections(), 10);
    /// ```
    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }

    /// Get the minimum number of connections.
    ///
    /// Returns the configured minimum pool size.
    ///
    /// # Returns
    ///
    /// The minimum number of connections to maintain in the pool.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    ///
    /// let config = DatabaseConfig::default();
    /// assert_eq!(config.min_connections(), 1);
    /// ```
    pub fn min_connections(&self) -> u32 {
        self.min_connections
    }

    /// Get the acquire timeout as a Duration.
    ///
    /// Returns the timeout for acquiring connections from the pool.
    ///
    /// # Returns
    ///
    /// A `chrono::Duration` representing the acquire timeout.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    /// use chrono::Duration;
    ///
    /// let config = DatabaseConfig::default();
    /// let timeout = config.acquire_timeout();
    /// assert_eq!(timeout, Duration::seconds(30));
    /// ```
    pub fn acquire_timeout(&self) -> Duration {
        Duration::seconds(self.acquire_timeout_seconds)
    }

    /// Get the idle timeout as a Duration.
    ///
    /// Returns the timeout for idle connections. If idle timeout is disabled
    /// (set to 0), this method returns `None`.
    ///
    /// # Returns
    ///
    /// A `chrono::Duration` representing the idle timeout, or `None` if disabled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    /// use chrono::Duration;
    ///
    /// let config = DatabaseConfig::default();
    /// let idle_timeout = config.idle_timeout();
    /// assert_eq!(idle_timeout, Some(Duration::seconds(600)));
    ///
    /// // Disable idle timeout
    /// let mut config = DatabaseConfig::default();
    /// config.idle_timeout_seconds = 0;
    /// assert_eq!(config.idle_timeout(), None);
    /// ```
    pub fn idle_timeout(&self) -> Option<Duration> {
        if self.idle_timeout_seconds > 0 {
            Some(Duration::seconds(self.idle_timeout_seconds))
        } else {
            None
        }
    }

    /// Get the maximum lifetime as a Duration.
    ///
    /// Returns the maximum lifetime for connections. If lifetime limits are disabled
    /// (set to 0), this method returns `None`.
    ///
    /// # Returns
    ///
    /// A `chrono::Duration` representing the maximum lifetime, or `None` if disabled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    /// use chrono::Duration;
    ///
    /// let config = DatabaseConfig::default();
    /// let max_lifetime = config.max_lifetime();
    /// assert_eq!(max_lifetime, Some(Duration::seconds(1800)));
    ///
    /// // Disable lifetime limits
    /// let mut config = DatabaseConfig::default();
    /// config.max_lifetime_seconds = 0;
    /// assert_eq!(config.max_lifetime(), None);
    /// ```
    pub fn max_lifetime(&self) -> Option<Duration> {
        if self.max_lifetime_seconds > 0 {
            Some(Duration::seconds(self.max_lifetime_seconds))
        } else {
            None
        }
    }

    /// Validate the configuration.
    ///
    /// Checks that all configuration values are valid and consistent.
    /// This should be called before using the configuration to create a connection pool.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the configuration is valid, or a `DatabaseError::Validation` describing the problem.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    ///
    /// let config = DatabaseConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// let invalid_config = DatabaseConfig {
    ///     max_connections: 5,
    ///     min_connections: 10, // min > max
    ///     ..Default::default()
    /// };
    /// assert!(invalid_config.validate().is_err());
    /// ```
    pub fn validate(&self) -> DatabaseResult<()> {
        if self.max_connections < self.min_connections {
            return Err(DatabaseError::Validation(format!(
                "max_connections ({}) must be >= min_connections ({})",
                self.max_connections, self.min_connections
            )));
        }

        if self.acquire_timeout_seconds <= 0 {
            return Err(DatabaseError::Validation(
                "acquire_timeout_seconds must be positive".to_string()
            ));
        }

        if self.idle_timeout_seconds < 0 {
            return Err(DatabaseError::Validation(
                "idle_timeout_seconds must be non-negative".to_string()
            ));
        }

        if self.max_lifetime_seconds < 0 {
            return Err(DatabaseError::Validation(
                "max_lifetime_seconds must be non-negative".to_string()
            ));
        }

        // Basic URL validation for SQLite
        if !self.url.starts_with("sqlite:") {
            return Err(DatabaseError::Validation(
                "URL must start with 'sqlite:'".to_string()
            ));
        }

        Ok(())
    }

    /// Get the default configuration values as key-value pairs.
    ///
    /// This method provides the default database configuration values in a format
    /// suitable for use with layered configuration systems. Each key-value pair
    /// represents a configuration setting that can be overridden by environment
    /// variables, config files, or programmatic settings.
    ///
    /// The returned keys use the "database." prefix to namespace the settings
    /// within the broader application configuration.
    ///
    /// # Returns
    ///
    /// A vector of `(key, value)` tuples where:
    /// - `key` is a configuration key string (e.g., "database.url")
    /// - `value` is the string representation of the default value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_database::DatabaseConfig;
    ///
    /// let defaults = DatabaseConfig::default_config_values();
    /// assert!(!defaults.is_empty());
    ///
    /// // Find a specific default value
    /// let url_default = defaults.iter()
    ///     .find(|(key, _)| *key == "database.url")
    ///     .map(|(_, value)| value)
    ///     .unwrap();
    /// assert_eq!(url_default, "sqlite:./personal-ledger.sqlite");
    /// ```
    pub fn default_config_values() -> Vec<(&'static str, String)> {
        let default_config = Self::default();
        vec![
            ("database.url", default_config.url().to_string()),
            ("database.max_connections", default_config.max_connections().to_string()),
            ("database.min_connections", default_config.min_connections().to_string()),
            ("database.acquire_timeout_seconds", default_config.acquire_timeout_seconds.to_string()),
            ("database.idle_timeout_seconds", default_config.idle_timeout_seconds.to_string()),
            ("database.max_lifetime_seconds", default_config.max_lifetime_seconds.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let config = DatabaseConfig::default();
        assert_eq!(config.url(), DEFAULT_URL);
        assert_eq!(config.max_connections(), DEFAULT_MAX_CONNECTIONS);
        assert_eq!(config.min_connections(), DEFAULT_MIN_CONNECTIONS);
        assert_eq!(config.acquire_timeout(), Duration::seconds(DEFAULT_ACQUIRE_TIMEOUT_SECONDS));
        assert_eq!(config.idle_timeout(), Some(Duration::seconds(DEFAULT_IDLE_TIMEOUT_SECONDS)));
        assert_eq!(config.max_lifetime(), Some(Duration::seconds(DEFAULT_MAX_LIFETIME_SECONDS)));
    }

    #[test]
    fn config_getters_return_correct_values() {
        let config = DatabaseConfig {
            url: "sqlite:test.db".to_string(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_seconds: 60,
            idle_timeout_seconds: 300,
            max_lifetime_seconds: 900,
        };

        assert_eq!(config.url(), "sqlite:test.db");
        assert_eq!(config.max_connections(), 20);
        assert_eq!(config.min_connections(), 5);
        assert_eq!(config.acquire_timeout(), Duration::seconds(60));
        assert_eq!(config.idle_timeout(), Some(Duration::seconds(300)));
        assert_eq!(config.max_lifetime(), Some(Duration::seconds(900)));
    }

    #[test]
    fn idle_timeout_returns_none_when_zero() {
        let config = DatabaseConfig {
            idle_timeout_seconds: 0,
            ..Default::default()
        };
        assert_eq!(config.idle_timeout(), None);
    }

    #[test]
    fn max_lifetime_returns_none_when_zero() {
        let config = DatabaseConfig {
            max_lifetime_seconds: 0,
            ..Default::default()
        };
        assert_eq!(config.max_lifetime(), None);
    }

    #[test]
    fn validate_succeeds_with_default_config() {
        let config = DatabaseConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_fails_with_min_greater_than_max_connections() {
        let config = DatabaseConfig {
            max_connections: 5,
            min_connections: 10,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(DatabaseError::Validation(_))));
    }

    #[test]
    fn validate_fails_with_non_positive_acquire_timeout() {
        let config = DatabaseConfig {
            acquire_timeout_seconds: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(DatabaseError::Validation(_))));
    }

    #[test]
    fn validate_fails_with_negative_idle_timeout() {
        let config = DatabaseConfig {
            idle_timeout_seconds: -1,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(DatabaseError::Validation(_))));
    }

    #[test]
    fn validate_fails_with_negative_max_lifetime() {
        let config = DatabaseConfig {
            max_lifetime_seconds: -1,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(DatabaseError::Validation(_))));
    }

    #[test]
    fn validate_fails_with_invalid_url() {
        let config = DatabaseConfig {
            url: "postgres://invalid".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(DatabaseError::Validation(_))));
    }

    #[test]
    fn config_is_cloneable() {
        let config = DatabaseConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn config_is_debuggable() {
        let config = DatabaseConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("DatabaseConfig"));
        assert!(debug_str.contains(DEFAULT_URL));
    }

    #[test]
    fn config_partial_eq_works() {
        let config1 = DatabaseConfig::default();
        let config2 = DatabaseConfig::default();
        assert_eq!(config1, config2);

        let config3 = DatabaseConfig {
            max_connections: 20,
            ..Default::default()
        };
        assert_ne!(config1, config3);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = DatabaseConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: DatabaseConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn default_config_values_returns_expected_pairs() {
        let defaults = DatabaseConfig::default_config_values();
        assert_eq!(defaults.len(), 6);

        let expected_keys = [
            "database.url",
            "database.max_connections",
            "database.min_connections",
            "database.acquire_timeout_seconds",
            "database.idle_timeout_seconds",
            "database.max_lifetime_seconds",
        ];

        for key in expected_keys {
            assert!(defaults.iter().any(|(k, _)| *k == key), "Missing key: {}", key);
        }

        // Check specific values
        let url_value = defaults.iter().find(|(k, _)| *k == "database.url").unwrap().1.clone();
        assert_eq!(url_value, DEFAULT_URL);

        let max_conn_value = defaults.iter().find(|(k, _)| *k == "database.max_connections").unwrap().1.clone();
        assert_eq!(max_conn_value, DEFAULT_MAX_CONNECTIONS.to_string());
    }
}