use lib_core::UnitId;

use crate::protocols::{BaseUnitRepository, UnitRepository};
use crate::{Error, Result, UnitRow};

/// Deactivate a Unit: it stays on whatever names it but is no longer offered to pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeactivateUnit(pub UnitId);

impl DeactivateUnit {
    /// Returns the Unit as written.
    pub fn run(
        self,
        units: &mut dyn UnitRepository,
        base_unit: &dyn BaseUnitRepository,
    ) -> Result<UnitRow> {
        let outcome = self.apply(units, base_unit);
        match &outcome {
            Ok(_) => tracing::info!(unit_id = %self.0, "Deactivated a Unit"),
            Err(refusal) => {
                tracing::info!(unit_id = %self.0, %refusal, "Refused to deactivate a Unit");
            }
        }
        outcome
    }

    fn apply(
        self,
        units: &mut dyn UnitRepository,
        base_unit: &dyn BaseUnitRepository,
    ) -> Result<UnitRow> {
        let mut unit = units.unit(self.0).ok_or(Error::UnitGone)?;
        if unit.id == base_unit.base_unit() {
            return Err(Error::DeactivateBase);
        }
        unit.is_active = false;
        units.upsert_unit(unit.clone());
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::fake::InMemoryPorts;

    #[test]
    fn deactivating_a_unit_writes_it_deactivated_and_leaves_the_rest_alone() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        let unit = DeactivateUnit(UnitId::SEED_BTC)
            .run(&mut ports.units, &ports.base_unit)
            .unwrap();

        assert!(!unit.is_active);
        assert_eq!(ports.units.unit(UnitId::SEED_BTC), Some(unit));
        let inactive: Vec<_> = ports.units.units.iter().filter(|u| !u.is_active).collect();
        assert_eq!(inactive.len(), 1);
    }

    #[test]
    fn deactivating_the_base_unit_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();

        assert_eq!(
            DeactivateUnit(UnitId::SEED_AUD).run(&mut ports.units, &ports.base_unit),
            Err(Error::DeactivateBase)
        );
        assert_eq!(ports, before);
    }

    #[test]
    fn deactivating_a_unit_that_is_gone_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();

        assert_eq!(
            DeactivateUnit(UnitId::new()).run(&mut ports.units, &ports.base_unit),
            Err(Error::UnitGone)
        );
        assert_eq!(ports, before);
    }
}
