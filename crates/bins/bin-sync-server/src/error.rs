//! Crate error type for the Sync Server binary.
//!
//! `SyncServerError` wraps the lower-level errors `main.rs`'s startup and bootstrap flow
//! surfaces while parsing configuration, initialising telemetry, connecting/migrating its
//! durable store, hashing the bootstrap account's password, and binding its listener, so that
//! flow returns a single `SyncServerResult<T>` alias rather than a mix of `io::Result`,
//! `lib_database::Result`, and ad hoc `Box<dyn Error>`.

#[derive(thiserror::Error, Debug)]
pub enum SyncServerError {
    /// Configuration loading/validation failures.
    #[error("Configuration error: {0}")]
    Config(#[from] lib_config::Error),

    /// Telemetry initialisation failures.
    #[error("Telemetry error: {0}")]
    Telemetry(#[from] lib_tracing::Error),

    /// Errors from the Sync Server's durable Change Set / `sync_users` store (connection,
    /// migration, query).
    #[error("Database error: {0}")]
    Database(#[from] lib_database::Error),

    /// Bind-address parsing failures.
    #[error("Invalid bind address: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    /// Network/listener I/O failures.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Argon2 password hashing failures (bootstrap account provisioning).
    #[error("Password hashing error: {0}")]
    PasswordHash(#[from] argon2::password_hash::Error),
}

/// Crate Result type alias used across the Sync Server binary.
///
/// Use `SyncServerResult<T>` for functions that return `T` or a `SyncServerError`.
pub type SyncServerResult<T> = std::result::Result<T, SyncServerError>;
