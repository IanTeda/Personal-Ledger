//! The port the Ledger persists through (ADR-0039, ADR-0044): load the whole Ledger once, then hand
//! over each command's recorded changes. Synchronous, so the domain crates stay free of async; the
//! SQLite adapter blocks on its own runtime inside a writer thread instead.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::{Ledger, LedgerChange, LedgerParts};

/// Why a repository could not load or apply. An adapter's own error rides as the source, since
/// `lib-ledger` never sees the crate that raised it.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// A [`StubRepository`] was told to fail its next call.
    #[error("the stub repository was set to fail")]
    StubFailure,
    #[error("the ledger storage failed")]
    Storage(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Where the Ledger is kept. `apply` takes one command's changes in recorded order and writes them
/// together.
pub trait LedgerRepository {
    fn load(&self) -> Result<Ledger, RepositoryError>;
    fn apply(&self, changes: &[LedgerChange]) -> Result<(), RepositoryError>;
}

/// An in-memory repository for tests and the headless UI tests (ADR-0030). It records every batch
/// it is given. No domain persists through it any more (the Units moved to `lib-units`' ports,
/// #751), so a reload returns the Ledger it was built with. Clones share one state, so a test keeps a handle on a repository it has
/// given away.
#[derive(Debug, Clone)]
pub struct StubRepository {
    state: Arc<Mutex<StubState>>,
}

#[derive(Debug)]
struct StubState {
    ledger: Ledger,
    applied: Vec<Vec<LedgerChange>>,
    fail_next: bool,
}

impl StubRepository {
    /// A repository holding `Ledger::stub()`.
    pub fn new() -> Self {
        Self::from_ledger(Ledger::stub())
    }

    /// A repository holding a Ledger over `parts`.
    pub fn from_parts(parts: LedgerParts) -> Self {
        Self::from_ledger(Ledger::from_parts(parts))
    }

    fn from_ledger(ledger: Ledger) -> Self {
        Self {
            state: Arc::new(Mutex::new(StubState {
                ledger,
                applied: Vec::new(),
                fail_next: false,
            })),
        }
    }

    /// Every batch `apply` accepted, oldest first.
    pub fn applied(&self) -> Vec<Vec<LedgerChange>> {
        self.state().applied.clone()
    }

    /// Makes the next `load` or `apply` fail, as a broken database would; the one after succeeds.
    pub fn fail_next(&self) {
        self.state().fail_next = true;
    }

    /// A test that panicked mid-call leaves nothing half-written here, so a poisoned lock is safe
    /// to take over.
    fn state(&self) -> MutexGuard<'_, StubState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for StubRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl StubState {
    fn take_failure(&mut self) -> Result<(), RepositoryError> {
        if std::mem::take(&mut self.fail_next) {
            Err(RepositoryError::StubFailure)
        } else {
            Ok(())
        }
    }
}

impl LedgerRepository for StubRepository {
    fn load(&self) -> Result<Ledger, RepositoryError> {
        let mut state = self.state();
        state.take_failure()?;
        Ok(state.ledger.clone())
    }

    /// A refused batch is neither recorded nor replayed, as a rolled-back transaction leaves
    /// nothing behind.
    fn apply(&self, changes: &[LedgerChange]) -> Result<(), RepositoryError> {
        let mut state = self.state();
        state.take_failure()?;
        state.applied.push(changes.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_core::{TagId, UnitId};

    #[test]
    fn a_new_stub_loads_the_stub_ledger() {
        let loaded = StubRepository::new().load().unwrap();
        assert_eq!(loaded.units().len(), 3);
        assert_eq!(loaded.base_unit(), UnitId::SEED_AUD);
        assert!(!loaded.accounts().is_empty());
    }

    #[test]
    fn apply_records_each_batch_in_order() {
        let repository = StubRepository::new();
        let first = vec![LedgerChange::DeleteTag(TagId::seed(1))];
        let second = vec![LedgerChange::DeleteTag(TagId::seed(2))];
        repository.apply(&first).unwrap();
        repository.apply(&second).unwrap();
        assert_eq!(repository.applied(), vec![first, second]);
    }

    #[test]
    fn a_clone_sees_what_the_original_was_given() {
        let repository = StubRepository::new();
        let handle = repository.clone();
        repository.apply(&[]).unwrap();
        assert_eq!(handle.applied().len(), 1);
    }

    #[test]
    fn fail_next_fails_only_the_next_call() {
        let repository = StubRepository::new();
        repository.fail_next();
        assert!(matches!(
            repository.apply(&[LedgerChange::DeleteTag(TagId::seed(1))]),
            Err(RepositoryError::StubFailure)
        ));
        assert!(repository.applied().is_empty());
        assert_eq!(repository.load().unwrap().units().len(), 3);

        repository.fail_next();
        assert!(matches!(
            repository.load(),
            Err(RepositoryError::StubFailure)
        ));
        assert!(repository.load().is_ok());
    }

    #[test]
    fn from_parts_loads_the_given_parts() {
        let repository = StubRepository::from_parts(LedgerParts::default());
        assert!(repository.load().unwrap().units().is_empty());
    }

    /// No domain persists through it, so changes are recorded but leave a reload as it was.
    #[test]
    fn changes_are_recorded_but_not_replayed() {
        let repository = StubRepository::new();
        let before = repository.load().unwrap().accounts().len();
        let id = repository.load().unwrap().accounts()[0].id;
        repository
            .apply(&[LedgerChange::DeleteAccount(id)])
            .unwrap();
        assert_eq!(repository.load().unwrap().accounts().len(), before);
        assert_eq!(repository.applied().len(), 1);
    }
}
