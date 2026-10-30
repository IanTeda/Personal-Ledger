//! The Transactions ports as the Desktop holds them (#774, #923).
//!
//! - [`DesktopPorts`]: what the page's Presenter reads, answered from the stores `shell::router::build`
//!   is given (`LedgerSetup`) rather than from the Ledger read copy: the Transactions store, and the
//!   Accounts, Categories, Payees, Tags and Units each read port names.
//! - [`forward`]: the forwarding shim that saves today's writers' Transaction changes through the
//!   Transactions store. Every writer of Transactions is still today's code: each runs as a
//!   `LedgerCommand` on the Ledger and records a `LedgerChange`, which [`forward`] sends to the store
//!   in the same commit as the command. The Ledger's Transactions stay a read copy refreshed from the
//!   store for the readers not yet on the port. Each writer leaves the shim when it migrates; the
//!   read copy goes when the last reader reads through the port.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::UnitId;
use lib_database::{
    SharedAccountsStore, SharedCategoriesStore, SharedPayeesStore, SharedTagsStore,
    SharedTransactionsStore, SharedUnitsStore, TransactionsStore,
};
use lib_ledger::LedgerChange;
use lib_transactions::{
    SharedPorts, TransactionAccount, TransactionAccounts, TransactionCategories,
    TransactionCategory, TransactionPayee, TransactionPayees, TransactionTag, TransactionTags,
    TransactionUnits, TransactionsPorts,
};

/// The stores the page reads, each borrowed for one read, since a store is shared behind a
/// `RefCell` whose guard can't be lent out of it.
pub struct DesktopPorts {
    pub transactions: SharedTransactionsStore,
    pub accounts: SharedAccountsStore,
    pub categories: SharedCategoriesStore,
    pub payees: SharedPayeesStore,
    pub tags: SharedTagsStore,
    pub units: SharedUnitsStore,
}

impl DesktopPorts {
    pub fn shared(
        transactions: SharedTransactionsStore,
        accounts: SharedAccountsStore,
        categories: SharedCategoriesStore,
        payees: SharedPayeesStore,
        tags: SharedTagsStore,
        units: SharedUnitsStore,
    ) -> SharedPorts {
        Rc::new(RefCell::new(Self {
            transactions,
            accounts,
            categories,
            payees,
            tags,
            units,
        }))
    }
}

impl TransactionsPorts for DesktopPorts {
    fn all_transactions(&self) -> Vec<lib_transactions::Transaction> {
        self.transactions.borrow().all_transactions()
    }
}

impl TransactionAccounts for DesktopPorts {
    fn transaction_accounts(&self) -> Vec<TransactionAccount> {
        self.accounts
            .borrow()
            .accounts()
            .accounts()
            .into_iter()
            .map(|account| TransactionAccount {
                id: account.id,
                name: account.name,
                unit: account.unit,
            })
            .collect()
    }
}

/// In tree order, each with its path, as the Categories' rule writes it.
impl TransactionCategories for DesktopPorts {
    fn transaction_categories(&self) -> Vec<TransactionCategory> {
        let categories = self.categories.borrow().categories().categories();
        lib_categories::paths_in_tree_order(&categories)
            .into_iter()
            .filter_map(|(id, path)| {
                let category = lib_categories::get(&categories, id)?;
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

impl TransactionPayees for DesktopPorts {
    fn transaction_payees(&self) -> Vec<TransactionPayee> {
        self.payees
            .borrow()
            .payees()
            .payees()
            .into_iter()
            .map(|payee| TransactionPayee {
                id: payee.id,
                name: payee.name,
            })
            .collect()
    }
}

impl TransactionTags for DesktopPorts {
    fn transaction_tags(&self) -> Vec<TransactionTag> {
        self.tags
            .borrow()
            .tags()
            .tags()
            .into_iter()
            .map(|tag| TransactionTag {
                id: tag.id,
                name: tag.name,
                color: tag.color,
            })
            .collect()
    }
}

impl TransactionUnits for DesktopPorts {
    fn transaction_unit_code(&self, id: UnitId) -> Option<String> {
        self.units.borrow().units().unit(id).map(|unit| unit.code)
    }
}

/// Sends each Transaction row `changes` upserts to `store`, in the order recorded. The writers it
/// carries: Import commit, Tag delete and merge, and Category reassign; none deletes a
/// Transaction, since Account delete writes through `lib-account-deletion`'s port (#854). Every
/// other change is another domain's, which nothing persists yet.
pub fn forward(store: &mut dyn TransactionsStore, changes: &[LedgerChange]) {
    let port = store.transactions_mut();
    for change in changes {
        if let LedgerChange::UpsertTransaction(transaction) = change {
            port.upsert_transaction(transaction.clone());
        }
    }
}
