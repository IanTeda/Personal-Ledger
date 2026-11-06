//! The in-memory creation port, for tests and UI-first work. It holds what it created in plain
//! fields so a test can set up or read state directly.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::protocols::{CreationFailure, LedgerCreation};
use crate::starting::NewLedger;

/// Ledger files in memory, by path.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryLedgerCreation {
    /// Every Ledger created, by its path.
    pub ledgers: BTreeMap<PathBuf, NewLedger>,
    /// Paths something other than a Ledger is at, as another app's file would be.
    pub occupied: BTreeSet<PathBuf>,
    /// Whether every create fails after claiming the name, as on a full disk.
    pub broken: bool,
    /// How many creates were asked for, refused or not.
    pub calls: usize,
}

impl LedgerCreation for InMemoryLedgerCreation {
    fn create_ledger(&mut self, path: &Path, ledger: &NewLedger) -> Result<(), CreationFailure> {
        self.calls += 1;
        if self.occupied.contains(path) || self.ledgers.contains_key(path) {
            return Err(CreationFailure::Exists);
        }
        if self.broken {
            return Err(CreationFailure::Unwritable);
        }
        self.ledgers.insert(path.to_path_buf(), ledger.clone());
        Ok(())
    }
}
