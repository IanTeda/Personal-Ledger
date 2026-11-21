//! Crate error type for the Desktop binary.
//!
//! `Error` holds the Desktop's fatal failures (configuration, telemetry, the Ledger store at
//! start-up) and the typed failures its own functions return (ADR-0060): why a Ledger file
//! wouldn't open, the state file not saving, and an Undo or Redo step the Shell words. Typed only:
//! the Toast or status line that says a failure picks its Message by matching the variant, and the
//! `Display` text is for logs.

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Configuration loading/validation failures.
    #[error("Configuration error: {0}")]
    Config(#[from] lib_config::Error),

    /// Telemetry initialisation failures.
    #[error("Telemetry error: {0}")]
    Telemetry(#[from] lib_tracing::Error),

    /// Errors from the Desktop app's embedded Ledger store (connection, migration, query), and
    /// its refusals of a file that isn't a Ledger it can open.
    #[error("Database error: {0}")]
    Database(#[from] lib_database::Error),

    /// The Ledger file named to open isn't there.
    #[error("the Ledger file is missing")]
    LedgerMissing,

    /// The Ledger file named to open is a folder or another thing that isn't a file.
    #[error("the Ledger file is not a file")]
    LedgerNotAFile,

    /// The Ledger file named to open is empty, which SQLite would take for a new database.
    #[error("the Ledger file is empty")]
    LedgerEmpty,

    /// The Ledger file named to open couldn't be read (permissions, or another I/O failure).
    #[error("the Ledger file couldn't be read: {0}")]
    LedgerUnreadable(std::io::Error),

    /// Reading or writing a file of the Desktop's own, such as the state file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An Undo or Redo step that didn't apply, as `lib-shell` words it.
    #[error("Shell error: {0}")]
    Shell(#[from] lib_shell::Error),
}

/// This crate's fallible results.
pub type Result<T, E = Error> = std::result::Result<T, E>;
