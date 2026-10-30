//! The Transactions query engine lives in `lib_transactions` (`docs/ux/desktop-mockups/03-transactions/README.md`);
//! this module re-exports it, answers its read ports from the stores' rows ([`Ledger`]) and keeps
//! the one piece that needs the Desktop's Message layer, the Status filter's label.

pub use lib_transactions::query::*;

use lib_accounts::Account;
use lib_categories::{self as categories, Category};
use lib_core::{TransactionStatus, UnitId};
use lib_locale::Label;
use lib_payees::Payee;
use lib_tags::Tag;
use lib_transactions::{
    TransactionAccount, TransactionAccounts, TransactionCategories, TransactionCategory,
    TransactionPayee, TransactionPayees, TransactionTag, TransactionTags, TransactionUnits,
};
use lib_units::UnitRow;

/// The reference data the View reads, borrowed from the shared stores. It answers the engine's
/// read ports from those rows, and the rows and chips read the full rows from it directly.
pub struct Ledger<'a> {
    pub accounts: &'a [Account],
    pub categories: &'a [Category],
    pub payees: &'a [Payee],
    pub tags: &'a [Tag],
    /// Read to turn an Account's Unit id into the code the total names.
    pub units: &'a [UnitRow],
}

impl TransactionAccounts for Ledger<'_> {
    fn transaction_accounts(&self) -> Vec<TransactionAccount> {
        self.accounts
            .iter()
            .map(|account| TransactionAccount {
                id: account.id,
                name: account.name.clone(),
                unit: account.unit,
            })
            .collect()
    }
}

impl TransactionCategories for Ledger<'_> {
    fn transaction_categories(&self) -> Vec<TransactionCategory> {
        categories::paths_in_tree_order(self.categories)
            .into_iter()
            .filter_map(|(id, path)| {
                let category = categories::get(self.categories, id)?;
                Some(TransactionCategory {
                    id,
                    name: category.name.clone(),
                    parent: category.parent,
                    path,
                })
            })
            .collect()
    }
}

impl TransactionPayees for Ledger<'_> {
    fn transaction_payees(&self) -> Vec<TransactionPayee> {
        self.payees
            .iter()
            .map(|payee| TransactionPayee {
                id: payee.id,
                name: payee.name.clone(),
            })
            .collect()
    }
}

impl TransactionTags for Ledger<'_> {
    fn transaction_tags(&self) -> Vec<TransactionTag> {
        self.tags
            .iter()
            .map(|tag| TransactionTag {
                id: tag.id,
                name: tag.name.clone(),
                color: tag.color.clone(),
            })
            .collect()
    }
}

impl TransactionUnits for Ledger<'_> {
    fn transaction_unit_code(&self, id: UnitId) -> Option<String> {
        self.units
            .iter()
            .find(|row| row.id == id)
            .map(|row| row.code.clone())
    }
}

/// The word for a Status filter in the Locale in effect: the shared Status label, or "All".
pub fn status_filter_label(filter: StatusFilter) -> String {
    match filter {
        StatusFilter::All => crate::msg::desktop_transactions_status_all(),
        StatusFilter::Open => TransactionStatus::Open.label(),
        StatusFilter::Cleared => TransactionStatus::Cleared.label(),
        StatusFilter::Reconciled => TransactionStatus::Reconciled.label(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_filter_labels() {
        crate::locale::init_for_tests();
        assert_eq!(
            StatusFilter::ALL.map(status_filter_label),
            ["All", "Open", "Cleared", "Reconciled"]
        );
    }
}
