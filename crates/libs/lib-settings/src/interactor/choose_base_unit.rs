use lib_core::UnitId;

use crate::error::Result;
use crate::protocols::LedgerPreferences;

/// Choose the Ledger's base Unit. The rule lives in `lib-units`, which the adapter runs, so a Unit
/// removed or deactivated under the select comes back as [`crate::Error::BaseUnitRefused`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseBaseUnit {
    pub id: UnitId,
}

impl ChooseBaseUnit {
    #[tracing::instrument(skip_all, fields(unit = ?self.id))]
    pub fn run(self, preferences: &dyn LedgerPreferences) -> Result<()> {
        let result = preferences.set_base_unit(self.id);
        if result.is_ok() {
            tracing::info!("Chose the Ledger's base Unit");
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::fake::InMemoryLedgerUnits;
    use crate::protocols::LedgerReads;

    #[test]
    fn a_saved_unit_reads_back() {
        let ledger = InMemoryLedgerUnits::sample();

        assert_eq!(
            ChooseBaseUnit {
                id: UnitId::SEED_BTC
            }
            .run(&ledger),
            Ok(())
        );

        assert_eq!(ledger.ledger_base_unit(), Some(UnitId::SEED_BTC));
    }

    #[test]
    fn an_unknown_unit_is_refused() {
        let ledger = InMemoryLedgerUnits::sample();

        assert_eq!(
            ChooseBaseUnit { id: UnitId::new() }.run(&ledger),
            Err(Error::BaseUnitRefused)
        );
    }
}
