//! Crate error type for the Desktop binary.
//!
//! `DesktopError` wraps the lower-level errors the Desktop app surfaces while parsing
//! configuration, initialising telemetry, and reading/writing its embedded Ledger store, so
//! every fallible path across the crate returns the same `DesktopResult<T>` alias rather than a
//! mix of `lib_database::Result` and ad hoc `Box<dyn Error>`.

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Configuration loading/validation failures.
    #[error("Configuration error: {0}")]
    Config(#[from] lib_config::Error),

    /// Telemetry initialisation failures.
    #[error("Telemetry error: {0}")]
    Telemetry(#[from] lib_tracing::Error),

    /// Errors from the Desktop app's embedded Ledger store (connection, migration, query).
    #[error("Database error: {0}")]
    Database(#[from] lib_database::Error),
}
