//! A halfway shim between `lib-units` and the domains that still live on the Ledger:
//!
//! - **The Units read copy ([`Ledger::refresh_units`]).** The Units ports are the truth; the
//!   Ledger keeps a copy for Bills, Budgets, Transactions, the Dashboard and the Bill and Budget
//!   forms. It goes when the last of them reads Units through its own domain's read port.
//!
//! `UnitUsage` and `UnitHoldings` are no longer answered here: the Desktop reads them from the
//! stores through `lib-database`'s `StoredUnitReads` (#993).

use lib_core::UnitId;
use lib_units::{PriceSourceRecord, UnitRow};

use crate::Ledger;

impl Ledger {
    /// Replaces the read copy of the Units, the base Unit and the stored Price Sources with what
    /// the Units ports hold. Returns whether anything changed, so a Client announces only a real
    /// change.
    pub fn refresh_units(
        &mut self,
        units: Vec<UnitRow>,
        base_unit: UnitId,
        price_sources: Vec<PriceSourceRecord>,
    ) -> bool {
        if self.units == units && self.base_unit == base_unit && self.price_sources == price_sources
        {
            return false;
        }
        self.units = units;
        self.base_unit = base_unit;
        self.price_sources = price_sources;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refreshing_the_units_read_copy_reports_only_a_real_change() {
        let mut ledger = Ledger::stub();
        let units = ledger.units().to_vec();
        assert!(!ledger.refresh_units(units.clone(), UnitId::SEED_AUD, Vec::new()));

        assert!(ledger.refresh_units(units[1..].to_vec(), UnitId::SEED_BTC, Vec::new()));
        assert_eq!(ledger.units().len(), 2);
        assert_eq!(ledger.base_unit(), UnitId::SEED_BTC);
    }
}
