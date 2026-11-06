//! The port creating a Ledger writes through.
//!
//! [`LedgerCreation`] is the module's one atomic port, and unlike the other cross-domain ports it
//! works over a path rather than over stores: no store exists before the file does. It claims the
//! name without overwriting, migrates the file and writes every starting row in one transaction,
//! and removes the file again if any of that fails, so a Ledger file is either complete or absent
//! (ADR-0054). It is synchronous and does its own I/O, so, unlike a store's write, its failure is
//! the caller's to report, typed as [`CreationFailure`]. `fake` implements it in memory.

use std::path::Path;

use crate::NewLedger;

/// Why a create wrote no Ledger file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CreationFailure {
    /// Something is already at the path, perhaps made after the explorer checked the name. It is
    /// left as it was.
    #[error("a file is already at the path")]
    Exists,
    /// The file couldn't be made, migrated or written, as in a folder that can't be written or a
    /// full disk. Whatever was made of it has been removed.
    #[error("the ledger file couldn't be written")]
    Unwritable,
}

/// Creates a Ledger file.
pub trait LedgerCreation {
    /// Creates the Ledger file at `path` holding `ledger`'s rows and nothing else, all or nothing.
    ///
    /// # Errors
    /// `Exists` when anything is at `path` already, which is never touched, and `Unwritable` when
    /// the file can't be made or written, which leaves nothing at `path`.
    fn create_ledger(&mut self, path: &Path, ledger: &NewLedger) -> Result<(), CreationFailure>;
}
