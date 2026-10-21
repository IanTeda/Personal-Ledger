//! A halfway shim (#746): `lib-units`' `UnitUsage` read port answered from the stub's Accounts,
//! Bill Plans and Budgets, so the Units Interactors can refuse deleting a Unit in use before those
//! domains persist. Each count moves to `lib-database` as its domain migrates, and this goes when
//! the last one has.

use lib_core::UnitId;
use lib_units::{UnitUsage, UnitUse};

use crate::Ledger;

impl UnitUsage for Ledger {
    fn unit_usage(&self, id: UnitId) -> UnitUse {
        self.unit_use(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stubs_unit_usage_counts_its_accounts_bill_plans_and_budgets() {
        let ledger = Ledger::stub();
        for unit in ledger.units() {
            let usage = ledger.unit_usage(unit.id);
            assert_eq!(usage, ledger.unit_use(unit.id), "{}", unit.code);
            let accounts = ledger
                .accounts()
                .iter()
                .filter(|a| a.unit == unit.id)
                .count();
            let bill_plans = ledger
                .bill_plans()
                .iter()
                .filter(|p| p.unit == unit.id)
                .count();
            let budgets = ledger
                .budgets()
                .all()
                .iter()
                .filter(|b| b.unit == unit.id)
                .count();
            assert_eq!(
                (usage.accounts, usage.bill_plans, usage.budgets),
                (accounts, bill_plans, budgets),
                "{}",
                unit.code
            );
        }
        assert!(ledger.unit_usage(UnitId::SEED_AUD).is_used());
        assert_eq!(ledger.unit_usage(UnitId::new()), UnitUse::default());
    }
}
