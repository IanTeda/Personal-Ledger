//! Halfway shims between `lib-budgets` and the domains that still live on the Ledger:
//!
//! - **The stub's own Budgets seed's mappings.** [`budget_accounts`] and [`budget_categories`] map
//!   the stub's rows for its Budgets seed and the Category-tree callers. The Ledger no longer
//!   answers the Budgets read ports: the Budgets page, the Dashboard and the rail badge read
//!   `lib-database`'s `StoredBudgetReads` (#952, #993).
//! - **The Budgets read copy ([`Ledger::refresh_budgets`]).** The Budgets store is the truth.
//!   Category delete, the last command that dropped Limits on the copy, writes the Budgets store
//!   through `lib-category-deletion` (#1005), and the Dashboard and the rail badge read Budgets
//!   through `StoredBudgetReads` (#1024, #1026), so the copy goes with its last reader (#816).

use lib_accounts::Account;
use lib_budgets::{BudgetAccount, BudgetCategory};
use lib_categories::Category;

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
    use chrono::NaiveDate;
    use lib_budgets::fake::{stub_accounts, stub_categories};

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap_or_default()
    }

    /// `lib-budgets`' fake stub stands in for these mappings, so its tests run on what the
    /// Ledger's stub holds.
    #[test]
    fn the_budgets_fake_stub_is_the_ledgers() {
        let ledger = Ledger::stub_on(today());
        assert_eq!(budget_accounts(ledger.accounts()), stub_accounts());
        assert_eq!(budget_categories(ledger.categories()), stub_categories());
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
}
