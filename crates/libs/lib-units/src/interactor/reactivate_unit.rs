use lib_core::UnitId;

use crate::protocols::UnitRepository;
use crate::{Error, Result, UnitRow};

/// Reactivate a deactivated Unit, so it is offered to pick again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactivateUnit(pub UnitId);

impl ReactivateUnit {
    /// Returns the Unit as written.
    pub fn run(self, units: &mut dyn UnitRepository) -> Result<UnitRow> {
        let outcome = self.apply(units);
        match &outcome {
            Ok(_) => tracing::info!(unit_id = %self.0, "Reactivated a Unit"),
            Err(refusal) => {
                tracing::info!(unit_id = %self.0, %refusal, "Refused to reactivate a Unit");
            }
        }
        outcome
    }

    fn apply(self, units: &mut dyn UnitRepository) -> Result<UnitRow> {
        let mut unit = units.unit(self.0).ok_or(Error::UnitGone)?;
        unit.is_active = true;
        units.upsert_unit(unit.clone());
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::interactor::DeactivateUnit;

    #[test]
    fn reactivating_a_deactivated_unit_puts_it_back_as_it_was() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();
        DeactivateUnit(UnitId::SEED_BTC)
            .run(&mut ports.units, &ports.base_unit)
            .unwrap();

        let unit = ReactivateUnit(UnitId::SEED_BTC)
            .run(&mut ports.units)
            .unwrap();

        assert!(unit.is_active);
        assert_eq!(ports, before);
    }

    #[test]
    fn reactivating_a_unit_that_is_gone_is_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();

        assert_eq!(
            ReactivateUnit(UnitId::new()).run(&mut ports.units),
            Err(Error::UnitGone)
        );
        assert_eq!(ports, before);
    }
}
