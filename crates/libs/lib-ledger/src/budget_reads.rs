//! Halfway shims between `lib-budgets` and the domains that still live on the Ledger:
//!
//! - **The read ports for `Ledger` (#951).** The Ledger answers every Budgets read port from its
//!   own read copies, each row's flags filled by its owning domain's rule
//!   (`docs/development/viper-architecture.md`, Rules > Dependencies), for the Budgets
//!   `LedgerCommand`s, the Dashboard and the rail badge. The Budgets View reads `lib-database`'s
//!   stores instead (#952). They go when the Budgets Interactors (#953) and the Dashboard (#814)
//!   read the stores.
//! - **The Budgets read copy ([`Ledger::refresh_budgets`]).** The Budgets store is the truth; the
//!   Ledger keeps a copy for its Budgets and Category reassign commands, the Dashboard and the rail
//!   badge. It goes when the Dashboard (#814) and `lib-reassign` (#813) leave it.

use chrono::NaiveDate;
use lib_accounts::Account;
use lib_budgets::{
    BudgetAccount, BudgetAccounts, BudgetBill, BudgetBillPlan, BudgetBills, BudgetCategories,
    BudgetCategory, BudgetPayees, BudgetSplit, BudgetSplits, BudgetUnits,
};
use lib_categories::Category;
use lib_core::{PayeeId, UnitId};

use crate::Ledger;

impl Ledger {
    /// Replaces the read copy of the Budgets with what the Budgets store holds, in its order.
    /// Returns whether anything changed, so a Client announces only a real change.
    pub fn refresh_budgets(&mut self, budgets: Vec<lib_budgets::Budget>) -> bool {
        if self.budgets.all() == budgets.as_slice() {
            return false;
        }
        self.budgets = lib_budgets::Budgets::from_stored(budgets);
        true
    }
}

impl BudgetCategories for Ledger {
    fn budget_categories(&self) -> Vec<BudgetCategory> {
        budget_categories(&self.categories)
    }
}

impl BudgetAccounts for Ledger {
    fn budget_accounts(&self) -> Vec<BudgetAccount> {
        budget_accounts(&self.accounts)
    }
}

impl BudgetSplits for Ledger {
    fn budget_splits(&self, from: NaiveDate, to: NaiveDate) -> Vec<BudgetSplit> {
        self.transactions
            .iter()
            .filter(|transaction| (from..=to).contains(&transaction.date))
            .flat_map(|transaction| {
                transaction.splits.iter().map(|split| BudgetSplit {
                    date: transaction.date,
                    account_id: transaction.account_id,
                    category_id: split.category_id,
                    amount: split.amount.clone(),
                    payee_id: split.payee_id,
                })
            })
            .collect()
    }
}

impl BudgetBills for Ledger {
    fn open_bills(&self) -> Vec<BudgetBill> {
        self.bill_entries
            .iter()
            .filter(|entry| entry.is_open())
            .filter_map(|entry| {
                let plan = lib_bills::get(&self.bill_plans, entry.plan_id)?;
                Some(BudgetBill {
                    due: entry.due,
                    category_id: plan.category_id,
                    account_id: plan.account_id,
                    amount: plan.planned_amount.clone(),
                })
            })
            .collect()
    }

    fn active_bill_plans(&self) -> Vec<BudgetBillPlan> {
        self.bill_plans
            .iter()
            .filter(|plan| plan.is_active)
            .map(|plan| BudgetBillPlan {
                category_id: plan.category_id,
                account_id: plan.account_id,
            })
            .collect()
    }
}

impl BudgetUnits for Ledger {
    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.units
            .iter()
            .find(|unit| unit.id == id)
            .map(|unit| unit.code.clone())
    }
}

impl BudgetPayees for Ledger {
    fn payee_name(&self, id: PayeeId) -> Option<String> {
        self.payees
            .iter()
            .find(|payee| payee.id == id)
            .map(|payee| payee.name.clone())
    }
}

/// `accounts` as Budgets reads them, for the stub's own Budgets seed and the Desktop's tests.
pub fn budget_accounts(accounts: &[Account]) -> Vec<BudgetAccount> {
    accounts
        .iter()
        .map(|account| BudgetAccount {
            id: account.id,
            name: account.name.clone(),
            unit: account.unit,
            takes_transactions: lib_transactions::takes_transactions(&account.account_type),
        })
        .collect()
}

/// `categories` as Budgets reads them, in tree order: the stub's own Budgets seed's, and a
/// Desktop caller's holding a Category tree rather than the Ledger.
pub fn budget_categories(categories: &[Category]) -> Vec<BudgetCategory> {
    lib_categories::paths_in_tree_order(categories)
        .into_iter()
        .filter_map(|(id, path)| {
            let category = lib_categories::get(categories, id)?;
            Some(BudgetCategory {
                id,
                name: category.name.clone(),
                path,
                parent: category.parent,
                category_type: category.category_type.clone(),
                is_leaf: lib_categories::is_leaf(categories, id),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use lib_budgets::fake::{stub_accounts, stub_categories};
    use lib_core::Period;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap_or_default()
    }

    /// `lib-budgets`' fake stub stands in for these mappings, so its tests run on what the
    /// Ledger's stub holds.
    #[test]
    fn the_budgets_fake_stub_is_the_ledgers() {
        let ledger = Ledger::stub_on(today());
        assert_eq!(ledger.budget_accounts(), stub_accounts());
        assert_eq!(ledger.budget_categories(), stub_categories());
    }

    #[test]
    fn the_budgets_read_copy_takes_the_stores_rows_and_says_whether_they_changed() {
        let mut ledger = Ledger::stub_on(today());
        let mut budgets = ledger.budgets().all().to_vec();
        assert!(!ledger.refresh_budgets(budgets.clone()), "the same rows");

        budgets[1].name = "Shared house".into();
        assert!(ledger.refresh_budgets(budgets.clone()));
        assert_eq!(ledger.budgets().all(), budgets.as_slice());
    }

    #[test]
    fn the_splits_are_flattened_within_the_dates() {
        let ledger = Ledger::stub_on(today());
        let month = Period::of(today());
        let splits = ledger.budget_splits(month.first_day(), month.last_day());
        let expected: usize = ledger
            .transactions()
            .iter()
            .filter(|transaction| month.contains(transaction.date))
            .map(|transaction| transaction.splits.len())
            .sum();
        assert!(expected > 0);
        assert_eq!(splits.len(), expected);
        assert!(splits.iter().all(|split| month.contains(split.date)));
    }

    #[test]
    fn only_open_bill_entries_are_read_with_their_plans_figures() {
        let ledger = Ledger::stub_on(today());
        let open: Vec<_> = ledger
            .bill_entries()
            .iter()
            .filter(|entry| entry.is_open())
            .collect();
        assert!(open.len() < ledger.bill_entries().len());
        let bills = ledger.open_bills();
        assert_eq!(bills.len(), open.len());
        for (bill, entry) in bills.iter().zip(open) {
            let plan = lib_bills::get(ledger.bill_plans(), entry.plan_id);
            assert_eq!(bill.due, entry.due);
            assert_eq!(plan.map(|p| p.category_id), Some(bill.category_id));
            assert_eq!(plan.map(|p| &p.planned_amount), Some(&bill.amount));
        }
    }

    #[test]
    fn units_and_payees_are_named() {
        let ledger = Ledger::stub_on(today());
        assert_eq!(
            BudgetUnits::unit_code(&ledger, UnitId::SEED_AUD).as_deref(),
            Some("aud")
        );
        let payee = &ledger.payees()[0];
        assert_eq!(
            BudgetPayees::payee_name(&ledger, payee.id),
            Some(payee.name.clone())
        );
    }
}
