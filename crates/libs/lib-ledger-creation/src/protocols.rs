//! The port creating a Ledger writes through.
//!
//! [`LedgerCreation`] is the module's one atomic port, and unlike the other cross-domain ports it
//! works over a path rather than over stores: no store exists before the file does. It claims the
//! name without overwriting, migrates the file and writes every starting row in one transaction,
//! and removes the file again if any of that fails, so a Ledger file is either complete or absent
//! (ADR-0054). It is synchronous and does its own I/O, so, unlike a store's write, its failure is
//! the caller's to report, as [`Error::Exists`] or [`Error::Unwritable`]. `fake` implements it in memory.

use std::path::Path;

use crate::{NewLedger, Result};

#[cfg(doc)]
use crate::Error;

/// Creates a Ledger file.
pub trait LedgerCreation {
    /// Creates the Ledger file at `path` holding `ledger`'s rows and nothing else, all or nothing.
    ///
    /// # Errors
    /// [`Error::Exists`] when anything is at `path` already, which is never touched, and
    /// [`Error::Unwritable`] when the file can't be made or written, which leaves nothing at
    /// `path`.
    fn create_ledger(&mut self, path: &Path, ledger: &NewLedger) -> Result<()>;
}
