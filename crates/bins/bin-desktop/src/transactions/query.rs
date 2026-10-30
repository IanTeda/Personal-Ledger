//! The Transactions query engine lives in `lib_transactions` (`docs/ux/desktop-mockups/03-transactions/README.md`);
//! this module re-exports it and answers its read ports from the stores' rows ([`Ledger`]).

pub use lib_transactions::query::*;

use lib_accounts::Account;
use lib_categories::{self as categories, Category};
use lib_core::UnitId;
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
