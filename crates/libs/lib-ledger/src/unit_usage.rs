//! Halfway shims (#746) between `lib-units` and the domains that still live on the Ledger:
//!
//! - **`UnitUsage` and `UnitHoldings` for `Ledger`.** `lib-units`' read ports answered from the
//!   stub's Accounts, Bill Plans, Budgets and Inventory Locations, so the Units Interactors can refuse deleting a Unit
//!   in use and the Units page can list a Unit's Accounts before those domains persist. The
//!   Desktop reads the holdings and the Accounts count from `lib-database`'s Accounts adapter since
//!   Accounts persist (#853) and the Locations count from the Inventory store (#885), and only the
//!   Bill Plans and Budgets counts here. The TUI's Units ports read none of it, but count from
//!   the stores opened beside them (#887). They go with the Desktop's last read.
//! - **The Units read copy ([`Ledger::refresh_units`]).** The Units ports are the truth; the
//!   Ledger keeps a copy for Bills, Budgets, Transactions, the Dashboard and the Bill and Budget
//!   forms. It goes when the last of them reads Units through its own domain's read port.

use lib_core::UnitId;
use lib_units::{Holding, PriceSourceRecord, UnitHoldings, UnitRow, UnitUsage, UnitUse};

use crate::Ledger;

impl Ledger {
    /// How many Accounts, Bill Plans, Budgets, Locations and Institutions (as their Usual unit) name Unit `id`.
    pub fn unit_use(&self, id: UnitId) -> UnitUse {
        UnitUse {
            accounts: self.accounts.iter().filter(|a| a.unit == id).count(),
            bill_plans: self.bill_plans.iter().filter(|p| p.unit == id).count(),
            budgets: self.budgets.all().iter().filter(|b| b.unit == id).count(),
            locations: self
                .inventory
                .locations
                .iter()
                .filter(|l| l.unit == id)
                .count(),
            institutions: self
                .institutions
                .iter()
                .filter(|i| i.usual_unit == Some(id))
                .count(),
        }
    }

    /// Drops every Account, Bill Plan and Budget so no Unit is in use, for a UI test that deletes a
    /// seeded Unit.
    #[doc(hidden)]
    pub fn drop_unit_users_for_test(&mut self) {
        self.accounts.clear();
        self.bill_plans.clear();
        self.budgets = lib_budgets::Budgets::default();
    }

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

impl UnitUsage for Ledger {
    fn unit_usage(&self, id: UnitId) -> UnitUse {
        self.unit_use(id)
    }
}

impl UnitHoldings for Ledger {
    fn holdings(&self, id: UnitId) -> Vec<Holding> {
        self.accounts
            .iter()
            .filter(|account| account.unit == id)
            .map(|account| Holding {
                name: account.name.clone(),
                kind: account.account_type.clone(),
                balance: account.balance.clone(),
                transactions: u32::try_from(self.account_use(account.id).transactions)
                    .unwrap_or(u32::MAX),
            })
            .collect()
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
            let locations = ledger
                .inventory()
                .locations
                .iter()
                .filter(|l| l.unit == unit.id)
                .count();
            assert_eq!(
                (
                    usage.accounts,
                    usage.bill_plans,
                    usage.budgets,
                    usage.locations
                ),
                (accounts, bill_plans, budgets, locations),
                "{}",
                unit.code
            );
        }
        assert!(ledger.unit_usage(UnitId::SEED_AUD).is_used());
        assert_eq!(ledger.unit_usage(UnitId::new()), UnitUse::default());
    }

    #[test]
    fn an_institutions_usual_unit_counts_as_the_units_use() {
        let ledger = Ledger::stub();

        assert_eq!(ledger.unit_usage(UnitId::SEED_BTC).institutions, 1);
        assert_eq!(ledger.unit_usage(UnitId::SEED_AUD).institutions, 0);
        assert!(ledger.unit_usage(UnitId::SEED_BTC).is_used());
    }

    #[test]
    fn a_units_holdings_are_its_accounts_in_ledger_order() {
        let ledger = Ledger::stub();
        let names: Vec<_> = ledger
            .holdings(UnitId::SEED_AUD)
            .into_iter()
            .map(|holding| holding.name)
            .collect();
        let expected: Vec<_> = ledger
            .accounts()
            .iter()
            .filter(|account| account.unit == UnitId::SEED_AUD)
            .map(|account| account.name.clone())
            .collect();
        assert!(!names.is_empty());
        assert_eq!(names, expected);
        assert!(ledger.holdings(UnitId::new()).is_empty());
    }

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
