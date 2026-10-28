//! The contract every stored deletion port keeps (ADR-0046), run against both the in-memory port
//! here and the `lib-database` adapter, so the fake the Clients' tests use can be trusted: a delete
//! lands the Unit's removal, the base's move and the Institutions' new Usual units together, a
//! delete whose write fails lands none of them, and a refused delete writes nothing.
//!
//! Each check is a function generic over a [`DeletionFactory`]: a consumer runs it from its own
//! test, since an adapter's tests need their own set-up (a fresh database per test).

use lib_core::{InstitutionId, UnitId};
use lib_institutions::Institution;
use lib_units::fake::InMemoryUnitUsage;

use crate::error::UnitDeletionError;
use crate::fake::InMemoryUnitDeletion;
use crate::fake::world::{AUD, BTC, COINSPOT, KRAKEN, VAS, WESTPAC, world};
use crate::interactor::DeleteUnit;
use crate::protocols::UnitDeletion;

/// What a store holds, in an order two stores compare in: the Units' ids, the base, and the
/// Institutions by id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub units: Vec<UnitId>,
    pub base_unit: UnitId,
    pub institutions: Vec<Institution>,
}

impl Stored {
    pub fn new(
        mut units: Vec<UnitId>,
        base_unit: UnitId,
        mut institutions: Vec<Institution>,
    ) -> Self {
        units.sort();
        institutions.sort_by_key(|institution| institution.id);
        Self {
            units,
            base_unit,
            institutions,
        }
    }

    fn of(rows: &InMemoryUnitDeletion) -> Self {
        Self::new(
            rows.units.units.iter().map(|unit| unit.id).collect(),
            rows.base_unit.0,
            rows.institutions.institutions.clone(),
        )
    }

    fn usual_unit(&self, id: InstitutionId) -> Option<UnitId> {
        self.institutions
            .iter()
            .find(|institution| institution.id == id)
            .and_then(|institution| institution.usual_unit)
    }
}

/// Builds the ports a contract check runs on.
pub trait DeletionFactory {
    type Ports: UnitDeletion;

    /// Ports holding `rows`' Units, base and Institutions, as if stored and loaded.
    fn create(&mut self, rows: InMemoryUnitDeletion) -> Self::Ports;

    /// Ends the writes made on `ports`, then reads back what is stored, as a restart would.
    fn stored(&mut self, ports: Self::Ports) -> Stored;

    /// Makes every Unit delete from now on fail, as a store that refuses it would.
    fn break_unit_deletes(&mut self, ports: &mut Self::Ports);

    /// Ends the writes made on `ports` and settles them, then reads back what is stored, with the
    /// number of failures reported.
    fn recover(&mut self, ports: Self::Ports) -> (Stored, usize);
}

/// The in-memory port is its own storage. It keeps a copy as of the last commit, which a failed
/// write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryUnitDeletion>,
    broken: bool,
    failures: usize,
}

impl InMemoryFactory {
    fn commit(&mut self, ports: &InMemoryUnitDeletion) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn read(&self) -> Stored {
        Stored::of(self.committed.as_ref().expect("create ran first"))
    }
}

impl DeletionFactory for InMemoryFactory {
    type Ports = InMemoryUnitDeletion;

    fn create(&mut self, rows: InMemoryUnitDeletion) -> InMemoryUnitDeletion {
        self.committed = Some(rows.clone());
        rows
    }

    fn stored(&mut self, ports: InMemoryUnitDeletion) -> Stored {
        self.commit(&ports);
        self.read()
    }

    fn break_unit_deletes(&mut self, _ports: &mut InMemoryUnitDeletion) {
        self.broken = true;
    }

    fn recover(&mut self, ports: InMemoryUnitDeletion) -> (Stored, usize) {
        self.commit(&ports);
        (self.read(), std::mem::take(&mut self.failures))
    }
}

fn delete(
    ports: &mut dyn UnitDeletion,
    id: UnitId,
    new_base: Option<UnitId>,
    replacements: Vec<(InstitutionId, UnitId)>,
) -> Result<crate::UnitDeleted, UnitDeletionError> {
    DeleteUnit {
        id,
        new_base,
        replacements,
    }
    // Nothing outside the Institutions names a world Unit.
    .run(ports, &InMemoryUnitUsage::default())
}

/// Deleting the base Unit Westpac names stores the Unit gone, the base moved and Westpac on the
/// Unit chosen for it; the exchanges, which name another Unit, stay.
pub fn a_delete_lands_the_unit_the_base_move_and_the_reassignment<F: DeletionFactory>(
    factory: &mut F,
) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    delete(&mut ports, AUD, Some(VAS), vec![(WESTPAC, BTC)]).expect("Westpac names aud");
    let stored = factory.stored(ports);

    assert_eq!(stored.units, [BTC, VAS]);
    assert_eq!(stored.base_unit, VAS);
    assert_eq!(stored.usual_unit(WESTPAC), Some(BTC));
    assert_eq!(stored.usual_unit(COINSPOT), before.usual_unit(COINSPOT));
}

/// Deleting a Unit with no Usual unit chosen for its Institutions stores each with none.
pub fn a_delete_left_on_none_lands_the_institutions_with_none<F: DeletionFactory>(factory: &mut F) {
    let mut ports = factory.create(world());

    delete(&mut ports, BTC, None, Vec::new()).expect("only Institutions name btc");
    let stored = factory.stored(ports);

    assert_eq!(stored.units, [AUD, VAS]);
    assert_eq!(stored.base_unit, AUD);
    assert_eq!(stored.usual_unit(COINSPOT), None);
    assert_eq!(stored.usual_unit(KRAKEN), None);
    assert_eq!(stored.usual_unit(WESTPAC), Some(AUD));
}

/// A delete whose Unit write fails stores none of it: the Unit, the base and every Usual unit
/// stand, and the failure is reported.
pub fn a_failing_delete_lands_nothing<F: DeletionFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    factory.break_unit_deletes(&mut ports);
    delete(&mut ports, AUD, Some(VAS), vec![(WESTPAC, BTC)]).expect("the port takes the write");
    let (stored, failures) = factory.recover(ports);

    assert_eq!(stored, before);
    assert!(failures > 0, "the failure is reported");
}

/// A Usual unit that isn't offered is refused by the port's own reads, before anything is written.
pub fn a_refused_delete_writes_nothing<F: DeletionFactory>(factory: &mut F) {
    let mut ports = factory.create(world());

    assert_eq!(
        delete(&mut ports, BTC, None, vec![(KRAKEN, BTC)]),
        Err(UnitDeletionError::ReplacementGone)
    );

    assert_eq!(factory.stored(ports), Stored::of(&world()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_lands_a_delete() {
        a_delete_lands_the_unit_the_base_move_and_the_reassignment(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_lands_institutions_left_on_none() {
        a_delete_left_on_none_lands_the_institutions_with_none(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_lands_no_part_of_a_failing_delete() {
        a_failing_delete_lands_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_writes_nothing_for_a_refused_delete() {
        a_refused_delete_writes_nothing(&mut InMemoryFactory::default());
    }
}
