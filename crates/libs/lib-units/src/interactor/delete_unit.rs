use lib_core::UnitId;

use crate::UnitRow;
use crate::protocols::{BaseUnitRepository, UnitRepository, UnitUsage, UnitUse};

/// Delete a Unit. The base Unit can only go together with a move of the base to `new_base`, so
/// the Ledger is never left without one; for any other Unit `new_base` is ignored.
///
/// A Unit in use is refused with its counts, and that refusal is the one place "in use, so
/// deactivate instead" is decided: no caller checks usage first to pick Delete or Deactivate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteUnit {
    pub id: UnitId,
    pub new_base: Option<UnitId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeleteUnitRefusal {
    #[error("unit not found")]
    UnitGone,
    /// The Ledger always has a base Unit, so its last Unit stays.
    #[error("the only unit can't be deleted")]
    OnlyUnit,
    /// An Account, a Bill Plan or a Budget names the Unit, so it can only be deactivated.
    #[error("a unit in use can only be deactivated")]
    UnitInUse(UnitUse),
    /// The Unit is the base Unit and no new base was given.
    #[error("the base unit needs a new base unit before it can be deleted")]
    BaseUnitNeeded,
    /// The new base Unit is the Unit being deleted, or isn't on the Ledger.
    #[error("new base unit not found")]
    NewBaseGone,
    /// The new base Unit is deactivated; the base Unit is always active.
    #[error("a deactivated unit can't be the base unit")]
    NewBaseInactive,
}

impl DeleteUnit {
    /// Returns the Unit removed.
    pub fn run(
        self,
        units: &mut dyn UnitRepository,
        base_unit: &mut dyn BaseUnitRepository,
        usage: &dyn UnitUsage,
    ) -> Result<UnitRow, DeleteUnitRefusal> {
        let all = units.units();
        let unit = all
            .iter()
            .find(|unit| unit.id == self.id)
            .cloned()
            .ok_or(DeleteUnitRefusal::UnitGone)?;
        if all.len() == 1 {
            return Err(DeleteUnitRefusal::OnlyUnit);
        }
        let in_use = usage.unit_usage(self.id);
        if in_use.is_used() {
            return Err(DeleteUnitRefusal::UnitInUse(in_use));
        }
        let moves_base = if self.id == base_unit.base_unit() {
            let new_base = self.new_base.ok_or(DeleteUnitRefusal::BaseUnitNeeded)?;
            let new_base = all
                .iter()
                .find(|unit| unit.id == new_base && unit.id != self.id)
                .ok_or(DeleteUnitRefusal::NewBaseGone)?;
            if !new_base.is_active {
                return Err(DeleteUnitRefusal::NewBaseInactive);
            }
            Some(new_base.id)
        } else {
            None
        };
        if let Some(new_base) = moves_base {
            base_unit.set_base_unit(new_base);
        }
        units.delete_unit(self.id);
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::fake::{InMemoryPorts, InMemoryUnits};

    fn delete(
        ports: &mut InMemoryPorts,
        id: UnitId,
        new_base: Option<UnitId>,
    ) -> Result<UnitRow, DeleteUnitRefusal> {
        DeleteUnit { id, new_base }.run(&mut ports.units, &mut ports.base_unit, &ports.usage)
    }

    /// Runs a delete that should be refused and checks it wrote nothing.
    fn refused(
        ports: &mut InMemoryPorts,
        id: UnitId,
        new_base: Option<UnitId>,
    ) -> DeleteUnitRefusal {
        let before = ports.clone();
        let refusal = delete(ports, id, new_base).expect_err("the delete should be refused");
        assert_eq!(*ports, before, "a refused delete writes nothing");
        refusal
    }

    #[test]
    fn deleting_a_unit_removes_it_and_returns_it() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        let removed = delete(&mut ports, UnitId::SEED_BTC, None).unwrap();

        assert_eq!(removed.code, "btc");
        assert_eq!(ports.units.unit(UnitId::SEED_BTC), None);
        assert_eq!(ports.units.units.len(), 2);
        assert_eq!(ports.base_unit.0, UnitId::SEED_AUD);
    }

    #[test]
    fn a_new_base_is_ignored_when_the_deleted_unit_is_not_the_base() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        delete(&mut ports, UnitId::SEED_BTC, Some(UnitId::SEED_VAS)).unwrap();

        assert_eq!(ports.base_unit.0, UnitId::SEED_AUD);
    }

    #[test]
    fn deleting_the_base_unit_moves_the_base_to_the_new_base() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        let removed = delete(&mut ports, UnitId::SEED_AUD, Some(UnitId::SEED_BTC)).unwrap();

        assert_eq!(removed.id, UnitId::SEED_AUD);
        assert_eq!(ports.base_unit.0, UnitId::SEED_BTC);
        assert_eq!(ports.units.unit(UnitId::SEED_AUD), None);
    }

    #[test]
    fn deleting_a_unit_that_is_gone_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        assert_eq!(
            refused(&mut ports, UnitId::new(), None),
            DeleteUnitRefusal::UnitGone
        );
    }

    #[test]
    fn deleting_the_only_unit_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        ports.units = InMemoryUnits {
            units: vec![ports.units.unit(UnitId::SEED_AUD).unwrap()],
        };
        for new_base in [None, Some(UnitId::SEED_AUD)] {
            assert_eq!(
                refused(&mut ports, UnitId::SEED_AUD, new_base),
                DeleteUnitRefusal::OnlyUnit
            );
        }
    }

    #[test]
    fn deleting_a_unit_in_use_is_refused_with_its_counts() {
        for uses in [
            UnitUse {
                accounts: 2,
                ..UnitUse::default()
            },
            UnitUse {
                bill_plans: 1,
                ..UnitUse::default()
            },
            UnitUse {
                budgets: 3,
                ..UnitUse::default()
            },
        ] {
            let mut ports = InMemoryPorts::stub(Utc::now());
            ports.usage.uses.insert(UnitId::SEED_BTC, uses);
            assert_eq!(
                refused(&mut ports, UnitId::SEED_BTC, None),
                DeleteUnitRefusal::UnitInUse(uses)
            );
        }
    }

    #[test]
    fn deleting_the_base_unit_without_a_new_base_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        assert_eq!(
            refused(&mut ports, UnitId::SEED_AUD, None),
            DeleteUnitRefusal::BaseUnitNeeded
        );
    }

    #[test]
    fn a_new_base_that_is_the_deleted_unit_or_unknown_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        for new_base in [UnitId::SEED_AUD, UnitId::new()] {
            assert_eq!(
                refused(&mut ports, UnitId::SEED_AUD, Some(new_base)),
                DeleteUnitRefusal::NewBaseGone
            );
        }
    }

    #[test]
    fn a_deactivated_new_base_is_refused() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        ports.units.units[1].is_active = false;
        assert_eq!(
            refused(&mut ports, UnitId::SEED_AUD, Some(UnitId::SEED_BTC)),
            DeleteUnitRefusal::NewBaseInactive
        );
    }
}
