//! The Accounts ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `AccountUsage`, `AccountUnits` and `AccountInstitutions` read from the Transactions, Budgets,
//! Units and Institutions stores through `lib-database`'s [`StoredAccountReads`] (#993, #1052,
//! #1053). The Ledger keeps no read copy of the Accounts (#1054). The write is committed and
//! announced by the input it was made in (`crate::input::send`), not here.
//!
//! The ports are one value shared several ways: the Accounts module's Presenters read and write
//! them as [`AccountsPorts`], and the same value is the `AccountsStore` the Units and Bills
//! reads and `lib-account-deletion`'s adapter hold.
//!
//! The usage is counted, and the Units and Institutions read, in the stores on each ask, whether they are the
//! adapters or `Stores::fake`'s in-memory ones, so nothing is recounted or copied from the Ledger.

use std::cell::RefCell;
use std::rc::Rc;

use lib_accounts::{
    Account, AccountInstitutions, AccountRepository, AccountUnits, AccountUsage, AccountsPorts,
};
use lib_core::AccountId;
use lib_database::FakeStores;
use lib_database::{AccountsStore, SharedAccountsStore, StoredAccountReads};

/// The Accounts ports the Accounts module and the Ledger share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// The shared Accounts store's Accounts, borrowed for each call: the store is `LedgerPorts`'
/// shared one (#1082), whose guard can't be lent out of it.
struct SharedAccounts {
    store: SharedAccountsStore,
}

impl AccountRepository for SharedAccounts {
    fn accounts(&self) -> Vec<Account> {
        self.store.borrow().accounts().accounts()
    }

    fn upsert_account(&mut self, account: Account) {
        self.store
            .borrow_mut()
            .accounts_mut()
            .upsert_account(account);
    }

    fn delete_account(&mut self, id: AccountId) {
        self.store.borrow_mut().accounts_mut().delete_account(id);
    }
}

/// `store`'s Accounts, with the Institutions an Account may name, the Units it may count in and the
/// Transactions and Budgets that name each read from `reads`.
pub struct LedgerBackedPorts {
    store: SharedAccountsStore,
    accounts: SharedAccounts,
    reads: StoredAccountReads,
}

impl LedgerBackedPorts {
    pub fn shared(store: SharedAccountsStore, reads: StoredAccountReads) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            accounts: SharedAccounts {
                store: store.clone(),
            },
            store,
            reads,
        }))
    }

    /// The sample Accounts in memory, reading the other domains through `reads`, for a test that
    /// needs no storage.
    pub fn in_memory(reads: StoredAccountReads) -> DesktopPorts {
        Self::shared(
            Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts {
                accounts: lib_accounts::fake::InMemoryAccounts {
                    accounts: lib_accounts::sample_accounts(),
                },
                ..Default::default()
            })),
            reads,
        )
    }
}

impl AccountsPorts for LedgerBackedPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        &self.accounts
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    fn usage(&self) -> &dyn AccountUsage {
        &self.reads
    }

    fn institutions(&self) -> &dyn AccountInstitutions {
        &self.reads
    }

    fn units(&self) -> &dyn AccountUnits {
        &self.reads
    }
}

/// The same ports as the store they wrap, for the Units and Bills reads
/// and the deletion adapter.
impl AccountsStore for LedgerBackedPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        &self.accounts
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.borrow_mut().reload()
    }
}

/// `sample`'s Transactions and Budgets in memory, as the usage of its Accounts reads them, and the
/// sample Units and Institutions, for a test that needs no storage.
pub fn in_memory_reads(sample: &FakeStores) -> StoredAccountReads {
    StoredAccountReads {
        transactions: Rc::new(RefCell::new(sample.transactions.clone())),
        budgets: crate::budgets::ports::in_memory(sample),
        units: crate::bills::ports::in_memory_units(chrono::Utc::now()),
        institutions: crate::institutions::ports::in_memory_store(),
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AccountId, UnitId};

    use super::*;

    #[test]
    fn the_transactions_and_budgets_are_counted_in_their_stores_on_each_ask() {
        let sample = FakeStores::sample(chrono::Local::now().date_naive());
        let reads = in_memory_reads(&sample);
        let ports = LedgerBackedPorts::in_memory(reads.clone());
        let everyday = AccountId::seed(2);
        let expected = lib_accounts::AccountUse {
            transactions: lib_database::seed::account_transactions(
                &sample.transactions.transactions,
                everyday,
            ),
            budgets: 2,
        };

        assert!(expected.transactions > 0);
        assert_eq!(
            AccountsPorts::usage(&*ports.borrow()).account_usage(everyday),
            expected
        );

        let mut budget = reads
            .budgets
            .borrow()
            .budgets()
            .budgets()
            .into_iter()
            .find(|budget| budget.account_ids.contains(&everyday))
            .expect("a sample Budget counts the Account");
        budget.account_ids.retain(|id| *id != everyday);
        reads
            .budgets
            .borrow_mut()
            .budgets_mut()
            .upsert_budget(&budget);
        assert_eq!(
            AccountsPorts::usage(&*ports.borrow())
                .account_usage(everyday)
                .budgets,
            1,
            "read from the store on each ask"
        );
        let mut booked = sample.transactions.transactions[0].clone();
        booked.id = lib_core::TransactionId::seed(9_999);
        booked.account_id = everyday;
        reads.transactions.borrow_mut().upsert_transaction(booked);
        assert_eq!(
            AccountsPorts::usage(&*ports.borrow())
                .account_usage(everyday)
                .transactions,
            expected.transactions + 1,
            "a Transaction written since is counted with nothing copied"
        );
    }

    #[test]
    fn the_ports_offer_the_stores_institutions_units_and_base_unit() {
        let reads = in_memory_reads(&FakeStores::sample(chrono::Local::now().date_naive()));
        let held = reads.units.borrow().units().units().len();
        let ports = LedgerBackedPorts::in_memory(reads);
        let ports = ports.borrow();

        assert_eq!(
            AccountsPorts::institutions(&*ports)
                .account_institutions()
                .len(),
            lib_institutions::sample_institutions().len()
        );
        let units = AccountsPorts::units(&*ports);
        assert_eq!(units.account_units().len(), held);
        assert_eq!(units.base_unit(), Some(UnitId::SEED_AUD));
    }

    #[test]
    fn a_write_through_the_ports_lands_in_the_store() {
        let ports = LedgerBackedPorts::in_memory(in_memory_reads(&FakeStores::sample(
            chrono::Local::now().date_naive(),
        )));
        let mut held = ports.borrow_mut();

        AccountsStore::accounts_mut(&mut *held).delete_account(AccountId::seed(3));

        assert!(
            AccountsStore::accounts(&*held)
                .account(AccountId::seed(3))
                .is_none()
        );
    }
}
