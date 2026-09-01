//! # Server Configuration Types and Defaults
//!
//! This module defines [`ServerConfig`] and related constants for configuring the
//! Personal Ledger backend server. All values here represent program defaults and are
//! typically overridden by a config file under `config/` or by environment variables
//! prefixed with `LEDGER_BACKEND_`.
//!
//! The configuration is designed for SQLite-backed deployments and supports
//! settings for bind address, port, logging, TLS, data directory, and database path.
//! 
//! TODO: Add config dir and update Dockerfile

use crate::{telemetry};
use super::{ConfigResult, ConfigError};
use std::path::PathBuf;

/// Default name for the configuration file (without extension).
///
/// This constant is combined with a `config/` directory and a `.conf`
/// extension to produce the default config file path (for example
/// `config/ledger-backend.conf`).
pub const CONFIG_FILE_NAME: &str = "ledger-backend";

/// Default server bind address used when no value is provided by file or env.
///
/// Defaults to `"127.0.0.1"` (localhost) for security, ensuring the server only
/// accepts local connections unless explicitly configured otherwise.
pub const DEFAULT_SERVER_ADDRESS: &str = "127.0.0.1";

/// Default server port used when no value is provided by file or env.
///
/// Defaults to `50059`, a port commonly used for gRPC services. This port is
/// within the user port range (49152-65535) to avoid conflicts with well-known services.
pub const DEFAULT_SERVER_PORT: u16 = 50059;

/// Default log level if none is specified.
///
/// Defaults to `WARN` to balance visibility of important issues while minimizing
/// log noise in production environments. Can be overridden via configuration or
/// environment variables for more verbose logging during development or debugging.
pub const DEFAULT_LOG_LEVEL: telemetry::LogLevel = telemetry::LogLevel::WARN;

/// Default for whether TLS is enabled.
///
/// Defaults to `false` for ease of local development and testing. Production
/// deployments should enable TLS by setting this to `true` and providing valid
/// certificate and key paths.
pub const DEFAULT_TLS_ENABLED: bool = false;

/// Default TLS certificate path.
///
/// Defaults to `None`, requiring explicit configuration when TLS is enabled.
/// The certificate file must be in PEM format and readable by the server process.
pub const DEFAULT_TLS_CERT_PATH: Option<&str> = None;

/// Default TLS private key path.
///
/// Defaults to `None`, requiring explicit configuration when TLS is enabled.
/// The private key file must be in PEM format, match the certificate, and be
/// readable by the server process with appropriate file permissions.
pub const DEFAULT_TLS_KEY_PATH: Option<&str> = None;

/// Default data directory for storing application files.
///
/// Defaults to `"data/"` in the current working directory. This directory is used
/// for storing application state, logs, or other persistent files that are not
/// part of the database.
pub const DEFAULT_DATA_DIR: Option<&str> = Some("data/");

/// Default SQLite database path.
///
/// Defaults to `"personal_ledger.db"` in the current working directory. This can
/// be overridden to use a different filename or absolute path. For in-memory
/// databases, set this to `":memory:"` in the configuration.
pub const DEFAULT_DATABASE_PATH: Option<&str> = Some("personal_ledger.db");


#[derive(Debug, Clone, serde::Deserialize)]
/// Server-specific configuration values for the Personal Ledger backend.
///
/// `ServerConfig` holds the host/port the gRPC server will bind to, the SQLite database path,
/// TLS settings, logging, and data directory. Fields are intentionally simple to allow easy
/// deserialization from config files or environment sources.
///
/// # Fields
/// - `address`: The IP address to bind the server to (default: 127.0.0.1)
/// - `port`: The port number to bind the server on (default: 50059)
/// - `data_dir`: Optional data directory for application files
/// - `log_level`: Logging level for the application
/// - `tls_enabled`: Whether TLS is enabled
/// - `tls_cert_path`: Path to the TLS certificate file (PEM format)
/// - `tls_key_path`: Path to the TLS private key file (PEM format)
/// - `database_path`: Path to the SQLite database file (default: personal_ledger.db)
///
/// # Example
///
/// ```rust
/// use personal_ledger_backend::config::ServerConfig;
/// let config = ServerConfig::default();
/// assert_eq!(config.address, "127.0.0.1");
/// assert_eq!(config.port, 50059);
/// ```
pub struct ServerConfig {
    /// The IP address to bind the server to.
    /// Defaults to "127.0.0.1".
    pub address: String,

    /// The port number to bind the server on.
    /// Must be a valid port number (1023-65535).
    pub port: u16,

    /// Optional data directory path used by the application to store state
    /// or other files. When `None` the application will use a sensible
    /// platform-specific default (or the current working directory) as
    /// configured at runtime.
    pub data_dir: Option<PathBuf>,

    /// The logging level for the application.
    ///
    /// If not set the configured default will be used. Values are provided
    /// via the `telemetry::LogLevel` enum (case-insensitive when deserialised).
    pub log_level: Option<telemetry::LogLevel>,

    /// Whether TLS encryption should be enabled for secure connections.
    /// When enabled, both `tls_cert_path` and `tls_key_path` must be provided.
    pub tls_enabled: bool,

    /// Path to the TLS certificate file in PEM format. Required when
    /// `tls_enabled` is true.
    pub tls_cert_path: Option<PathBuf>,

    /// Path to the TLS private key file in PEM format. Required when
    /// `tls_enabled` is true.
    pub tls_key_path: Option<PathBuf>,

    /// Path to the SQLite database file. If None, defaults to "personal_ledger.db" 
    /// in the current directory.
    pub database_path: Option<PathBuf>,
}

impl Default for ServerConfig {
    /// Creates a default server configuration suitable for local development.
    ///
    /// Returns a `ServerConfig` with sensible defaults:
    /// - Binds to `127.0.0.1:50059` (localhost only)
    /// - SQLite database at `personal_ledger.db`
    /// - Data directory at `data/`
    /// - Log level set to `WARN`
    /// - TLS disabled
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use personal_ledger_backend::config::ServerConfig;
    /// let config = ServerConfig::default();
    /// assert_eq!(config.address, "127.0.0.1");
    /// assert_eq!(config.port, 50059);
    /// assert!(!config.tls_enabled);
    /// ```
    fn default() -> Self {
        Self {
            address: DEFAULT_SERVER_ADDRESS.to_string(),
            port: DEFAULT_SERVER_PORT,
            data_dir: DEFAULT_DATA_DIR.map(PathBuf::from),
            log_level: Some(DEFAULT_LOG_LEVEL),
            tls_enabled: DEFAULT_TLS_ENABLED,
            tls_cert_path: DEFAULT_TLS_CERT_PATH.map(PathBuf::from),
            tls_key_path: DEFAULT_TLS_KEY_PATH.map(PathBuf::from),
            database_path: DEFAULT_DATABASE_PATH.map(PathBuf::from),
        }
    }
}

impl ServerConfig {
    /// Build and return the bind `SocketAddr` for the server.
    ///
    /// Combines the configured `address` and `port` fields into a valid socket address
    /// that can be used to bind the gRPC server. This validates that the address and
    /// port form a valid network endpoint.
    ///
    /// # Returns
    ///
    /// Returns `Ok(SocketAddr)` if the address:port combination is valid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The address is not a valid IP address or hostname
    /// - The port is invalid (though `u16` type ensures this at compile time)
    /// - The combined address:port string cannot be parsed as a `SocketAddr`
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use personal_ledger_backend::config::ServerConfig;
    /// let config = ServerConfig::default();
    /// let addr = config.address().expect("default config should parse");
    /// assert_eq!(addr.port(), 50059);
    /// ```
    pub fn address(&self) -> ConfigResult<core::net::SocketAddr> {
        let address = format!("{}:{}", self.address, self.port);
        let address = address.parse()?;
        Ok(address)
    }

    /// Return the configured log level or the default.
    ///
    /// Returns the log level set in the configuration, or falls back to
    /// [`DEFAULT_LOG_LEVEL`] if none is specified.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use personal_ledger_backend::config::ServerConfig;
    /// # use personal_ledger_backend::telemetry::LogLevel;
    /// let config = ServerConfig::default();
    /// assert_eq!(config.log_level(), LogLevel::WARN);
    /// ```
    pub fn log_level(&self) -> telemetry::LogLevel {
        self.log_level.unwrap_or(DEFAULT_LOG_LEVEL)
    }

    /// Constructs a SQLite connection URL from the configured database path.
    ///
    /// Builds a SQLx-compatible SQLite connection string using the `database_path`
    /// field. If no path is configured, defaults to `"personal_ledger.db"`.
    ///
    /// # Returns
    ///
    /// Returns `Ok(String)` containing a connection URL in the format `"sqlite://path/to/database.db"`.
    /// Returns `Err(ConfigError::Validation)` when the configured database path is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use personal_ledger_backend::config::ServerConfig;
    /// # use std::path::PathBuf;
    /// let mut config = ServerConfig::default();
    /// assert_eq!(config.database_url().unwrap(), "sqlite://personal_ledger.db");
    ///
    /// config.database_path = Some(PathBuf::from("custom.db"));
    /// assert_eq!(config.database_url().unwrap(), "sqlite://custom.db");
    ///
    /// // In-memory database
    /// config.database_path = Some(PathBuf::from(":memory:"));
    /// assert_eq!(config.database_url().unwrap(), "sqlite://:memory:");
    /// ```
    pub fn database_url(&self) -> ConfigResult<String> {
        let db_path = self
            .database_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "personal_ledger.db".to_string());

        if db_path.trim().is_empty() {
            return Err(ConfigError::Validation("database path cannot be empty".to_string()));
        }

        Ok(format!("sqlite://{}", db_path))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use config::Config as ConfigLib;
    use std::net::SocketAddr;

    #[test]
    fn default_server_config_values() {
        let s = ServerConfig::default();
        assert_eq!(s.address, DEFAULT_SERVER_ADDRESS);
        assert_eq!(s.port, DEFAULT_SERVER_PORT);
        assert_eq!(s.tls_enabled, DEFAULT_TLS_ENABLED);
        assert!(s.tls_cert_path.is_none());
        assert!(s.tls_key_path.is_none());
        assert_eq!(s.database_path, Some(std::path::PathBuf::from("personal_ledger.db")));
    }

    #[test]
    fn server_address_parses_successfully() {
        use fake::faker::internet::en::IPv4;
        use fake::faker::number::en::NumberWithFormat;
        use fake::Fake;
        let ip: String = IPv4().fake();
        let port: u16 = NumberWithFormat("#####").fake::<String>().parse().unwrap_or(50051);
        let cfg = ServerConfig {
            address: ip.clone(),
            port,
            log_level: None,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            data_dir: None,
            database_path: Some(std::path::PathBuf::from("test.db")),
        };

        let addr = cfg.address().expect("address should parse");
        let expected: SocketAddr = format!("{}:{}", ip, port).parse().unwrap();
        assert_eq!(addr, expected);
    }

    #[test]
    fn server_address_parsing_fails_for_invalid_address() {
        use fake::faker::lorem::en::Word;
        use fake::Fake;
        let bad_ip: String = Word().fake();
        let cfg = ServerConfig {
            address: bad_ip,
            port: 80,
            log_level: None,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            data_dir: None,
            database_path: None,
        };

        assert!(cfg.address().is_err(), "invalid address should return an error");
    }

    #[test]
    fn deserialize_from_configlib_with_defaults() {
        let cfglib = ConfigLib::builder()
            .set_default("address", DEFAULT_SERVER_ADDRESS)
            .unwrap()
            .set_default("port", DEFAULT_SERVER_PORT)
            .unwrap()
            .set_default("tls_enabled", DEFAULT_TLS_ENABLED)
            .unwrap()
            .set_default("database_path", "personal_ledger.db")
            .unwrap()
            .build()
            .unwrap();

        let server_cfg: ServerConfig = cfglib.try_deserialize().expect("should deserialize");
        assert_eq!(server_cfg.address, DEFAULT_SERVER_ADDRESS);
        assert_eq!(server_cfg.port, DEFAULT_SERVER_PORT);
        assert_eq!(server_cfg.tls_enabled, DEFAULT_TLS_ENABLED);
        assert_eq!(server_cfg.database_path, Some(std::path::PathBuf::from("personal_ledger.db")));
    }

    #[test]
    fn server_config_default_is_valid_socket() {
        let s = ServerConfig::default();
        let combined = format!("{}:{}", s.address, s.port);
        assert!(
            combined.parse::<SocketAddr>().is_ok(),
            "default server address must parse to SocketAddr"
        );
    }

    #[test]
    fn database_url_uses_database_path() {
        let s = ServerConfig {
            address: DEFAULT_SERVER_ADDRESS.to_string(),
            port: DEFAULT_SERVER_PORT,
            data_dir: None,
            log_level: Some(DEFAULT_LOG_LEVEL),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            database_path: Some(PathBuf::from("custom.db")),
        };
        assert_eq!(s.database_url().unwrap(), "sqlite://custom.db");
    }

    #[test]
    fn database_url_uses_default_when_none() {
        let s = ServerConfig {
            address: DEFAULT_SERVER_ADDRESS.to_string(),
            port: DEFAULT_SERVER_PORT,
            data_dir: None,
            log_level: Some(DEFAULT_LOG_LEVEL),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            database_path: None,
        };
        assert_eq!(s.database_url().unwrap(), "sqlite://personal_ledger.db");
    }

    #[test]
    fn database_url_returns_error_for_empty_path() {
        let s = ServerConfig {
            address: DEFAULT_SERVER_ADDRESS.to_string(),
            port: DEFAULT_SERVER_PORT,
            data_dir: None,
            log_level: Some(DEFAULT_LOG_LEVEL),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            database_path: Some(PathBuf::from("")),
        };
        let result = s.database_url();
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert_eq!(msg, "database path cannot be empty");
        } else {
            panic!("Expected Validation error");
        }
    }
}