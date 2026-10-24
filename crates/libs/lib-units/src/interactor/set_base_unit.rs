use lib_core::UnitId;

use crate::protocols::{BaseUnitRepository, UnitRepository};
use crate::{Error, Result};

/// Make a Unit the base Unit, which new Accounts default to and the net worth is figured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetBaseUnit(pub UnitId);

impl SetBaseUnit {
    pub fn run(
        self,
        units: &dyn UnitRepository,
        base_unit: &mut dyn BaseUnitRepository,
    ) -> Result<()> {
        let outcome = self.apply(units, base_unit);
        match &outcome {
            Ok(()) => tracing::info!(unit_id = %self.0, "Set the base Unit"),
            Err(refusal) => {
                tracing::info!(unit_id = %self.0, %refusal, "Refused to set the base Unit");
            }
        }
        outcome
    }

    fn apply(
        self,
        units: &dyn UnitRepository,
        base_unit: &mut dyn BaseUnitRepository,
    ) -> Result<()> {
        let unit = units.unit(self.0).ok_or(Error::UnitGone)?;
        if !unit.is_active {
            return Err(Error::InactiveBase);
        }
        base_unit.set_base_unit(unit.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::fake::InMemoryPorts;

    #[test]
    fn setting_the_base_unit_moves_it_and_leaves_the_units_alone() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let units = ports.units.clone();

        SetBaseUnit(UnitId::SEED_BTC)
            .run(&ports.units, &mut ports.base_unit)
            .unwrap();

        assert_eq!(ports.base_unit.0, UnitId::SEED_BTC);
        assert_eq!(ports.units, units);
    }

    #[test]
    fn setting_a_unit_that_is_gone_as_base_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        assert_eq!(
            SetBaseUnit(UnitId::new()).run(&ports.units, &mut ports.base_unit),
            Err(Error::UnitGone)
        );
        assert_eq!(ports.base_unit.0, UnitId::SEED_AUD);
    }

    #[test]
    fn setting_a_deactivated_unit_as_base_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        ports.units.units[1].is_active = false;

        assert_eq!(
            SetBaseUnit(UnitId::SEED_BTC).run(&ports.units, &mut ports.base_unit),
            Err(Error::InactiveBase)
        );
        assert_eq!(ports.base_unit.0, UnitId::SEED_AUD);
    }
}
