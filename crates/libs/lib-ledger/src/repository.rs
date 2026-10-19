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
/// together, so a `SetBaseUnit` lands before the `DeleteUnit` that needed it.
pub trait LedgerRepository {
    fn load(&self) -> Result<Ledger, RepositoryError>;
    fn apply(&self, changes: &[LedgerChange]) -> Result<(), RepositoryError>;
}

/// An in-memory repository for tests and the headless UI tests (ADR-0030). It records every batch
/// it is given and replays the persisted domains onto its Ledger, so a reload matches what an
/// adapter would return. Clones share one state, so a test keeps a handle on a repository it has
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
        for change in changes {
            state.ledger.replay(change);
        }
        state.applied.push(changes.to_vec());
        Ok(())
    }
}

impl Ledger {
    /// Writes one recorded change back onto the Ledger, as loading it from storage would. Only the
    /// Units are persisted so far (ADR-0044's halfway state): every other change is ignored, as the
    /// SQLite adapter ignores it, and a reload takes those domains from the stub.
    fn replay(&mut self, change: &LedgerChange) {
        match change {
            LedgerChange::UpsertUnit(unit) => {
                match self.units.iter_mut().find(|row| row.id == unit.id) {
                    Some(row) => *row = unit.clone(),
                    None => self.units.push(unit.clone()),
                }
            }
            LedgerChange::DeleteUnit(id) => self.units.retain(|row| row.id != *id),
            LedgerChange::SetBaseUnit(id) => self.base_unit = *id,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_core::UnitId;

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
        let first = vec![LedgerChange::SetBaseUnit(UnitId::SEED_AUD)];
        let second = vec![LedgerChange::DeleteUnit(UnitId::SEED_AUD)];
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
            repository.apply(&[LedgerChange::DeleteUnit(UnitId::SEED_AUD)]),
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

    /// Accounts aren't persisted yet, so their changes are recorded but leave a reload as it was.
    #[test]
    fn changes_to_an_unpersisted_domain_are_not_replayed() {
        let repository = StubRepository::new();
        let before = repository.load().unwrap().accounts().len();
        let id = repository.load().unwrap().accounts()[0].id;
        repository
            .apply(&[LedgerChange::DeleteAccount(id)])
            .unwrap();
        assert_eq!(repository.load().unwrap().accounts().len(), before);
        assert_eq!(repository.applied().len(), 1);
    }

    #[test]
    fn the_stub_meets_the_units_contract() {
        crate::contract::units(StubRepository::new);
    }

    /// Accepts every batch and keeps none of it, as `LedgerStore` does before #725.
    struct ForgetfulRepository(Ledger);

    impl LedgerRepository for ForgetfulRepository {
        fn load(&self) -> Result<Ledger, RepositoryError> {
            Ok(self.0.clone())
        }

        fn apply(&self, _: &[LedgerChange]) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    #[test]
    #[should_panic(expected = "add a Unit")]
    fn a_repository_that_drops_its_changes_fails_the_contract() {
        crate::contract::units(|| ForgetfulRepository(Ledger::stub()));
    }
}
