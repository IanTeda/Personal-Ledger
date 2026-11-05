//! Halfway shims between `lib-budgets` and the domains that still live on the Ledger:
//!
//! - **The stub's own Budgets seed's mappings.** [`budget_accounts`] and [`budget_categories`] map
//!   the stub's rows for its Budgets seed and the Category-tree callers. The Ledger no longer
//!   answers the Budgets read ports: the Budgets page, the Dashboard and the rail badge read
//!   `lib-database`'s `StoredBudgetReads` (#952, #993).

use lib_accounts::Account;
use lib_budgets::{BudgetAccount, BudgetCategory};
use lib_categories::Category;

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
    use crate::Ledger;

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
}
