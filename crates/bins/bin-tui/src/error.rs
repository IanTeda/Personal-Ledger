//! Crate error type for the TUI binary.
//!
//! `TuiError` wraps the lower-level errors the TUI surfaces while parsing configuration,
//! initialising telemetry, driving the terminal, and reading/writing its local Ledger store,
//! so every fallible path across the crate returns the same `TuiResult<T>` alias rather than a
//! mix of `io::Result`, `lib_database::Result`, and ad hoc `Box<dyn Error>`.

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Terminal I/O failures — entering/leaving the alternate screen, raw mode, drawing a
    /// frame.
    #[error("Terminal I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration loading/validation failures.
    #[error("Configuration error: {0}")]
    Config(#[from] lib_config::Error),

    /// Telemetry initialisation failures.
    #[error("Telemetry error: {0}")]
    Telemetry(#[from] lib_tracing::Error),

    /// Errors from the Client's local Ledger store (connection, migration, query).
    #[error("Database error: {0}")]
    Database(#[from] lib_database::Error),
}
