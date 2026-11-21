//! Crate error type for the TUI binary.
//!
//! `Error` wraps the lower-level errors the TUI surfaces while parsing configuration,
//! initialising telemetry, driving the terminal, and reading/writing its local Ledger store,
//! and holds the fixture Views' refusals as flat variants (ADR-0060), so every fallible path across the crate returns the same `Result<T>` alias rather than a
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

    // The fixture Views' refusals, until each View moves onto its domain crate's Presenters and
    // its variants leave with it, as the Units View's did. Today's toast shows their text.
    /// The Account is no longer in the store.
    #[error("account not found")]
    AccountNotFound,
    /// Deleting an Account that holds Transactions or Balance Checks needs a target to move
    /// them into.
    #[error(
        "{name} holds transactions or Balance Checks — choose a same-unit account to move them into"
    )]
    AccountRequiresTransferTarget { name: String },
    #[error("the transfer target must share {name}'s unit ({unit})")]
    TransferTargetUnitMismatch { name: String, unit: String },
    #[error("can't transfer an account's transactions to itself")]
    TransferTargetIsSource,
    #[error("the transfer target must be active")]
    TransferTargetInactive,

    /// The move would put a Category inside its own descendants.
    #[error("{name} can't move into its own descendant set")]
    CategoryWouldCycle { name: String },
    #[error("{name} can't cross into the other root while it or a descendant has transactions")]
    CategoryCrossRootWithTransactions { name: String },
    #[error("a category named \"{name}\" already exists under this parent")]
    CategoryDuplicateSibling { name: String },
    #[error("category not found")]
    CategoryNotFound,
    /// The income and expenses roots are fixed.
    #[error("the two roots (income/expenses) can't be renamed, moved or archived")]
    CategoryIsRoot,

    #[error("payee not found")]
    PayeeNotFound,
    #[error("alias not found")]
    PayeeAliasNotFound,
    /// Payee names are unique, ignoring case; `holder` is the Payee that already has it.
    #[error("a payee named \"{holder}\" already exists")]
    PayeeDuplicateName { holder: String },
    #[error("{transaction_count} txns · {alias_count} matches reference this payee")]
    PayeeReferencesExist {
        transaction_count: u32,
        alias_count: usize,
    },
    /// A match a rename left behind can only be removed, never edited.
    #[error("a rename match is protected — edit refuses it, only remove is allowed")]
    PayeeRenameProtectedAlias,
    #[error(
        "\"{pattern}\" also matches {other} — two payees matching one text resolves arbitrarily"
    )]
    PayeePatternCollision { pattern: String, other: String },

    #[error("tag not found")]
    TagNotFound,
    /// Tag names are unique across the Ledger, ignoring case (ADR-0015).
    #[error("a tag named \"{name}\" already exists")]
    TagDuplicateName { name: String },
}

/// This crate's fallible results.
pub type Result<T, E = Error> = std::result::Result<T, E>;
