//! The Accounts ports as the Desktop holds them: the store `shell::router::build` is given, with
//! the Institutions copied from the Ledger read copy (a halfway shim until they answer the
//! Accounts' select through their own port, #1053), and `AccountUsage` and `AccountUnits` read from
//! the Transactions, Budgets and Units stores through `lib-database`'s [`StoredAccountReads`]
//! (#993, #1052), and the step that keeps the Ledger's read copy of the Accounts in line after an
//! Accounts write. The write is committed by the input it was made in (`crate::input::send`), not
//! here.
//!
//! The ports are one value shared several ways: the Accounts module's Presenters read and write
//! them as [`AccountsPorts`], and the same value is the `AccountsStore` the Ledger's read copy,
//! the Units and Bills reads and `lib-account-deletion`'s adapter hold.
//!
//! The usage is counted, and the Units read, in the stores on each ask, whether they are the
//! adapters or `Stores::fake`'s in-memory ones, so nothing is recounted or copied from the Ledger.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_accounts::fake::InMemoryAccountInstitutions;
use lib_accounts::{
    AccountInstitution, AccountInstitutions, AccountRepository, AccountUnits, AccountUsage,
    AccountsPorts,
};
use lib_database::{AccountsStore, StoredAccountReads};
use lib_ledger::Ledger;

use crate::ledger_store::LedgerStore;

/// The Accounts ports the Accounts module and the Ledger share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Accounts, with the Institutions an Account may name copied from the Ledger, and the
/// Units it may count in and the Transactions and Budgets that name each read from `reads`.
pub struct LedgerBackedPorts {
    store: Box<dyn AccountsStore>,
    institutions: InMemoryAccountInstitutions,
    reads: StoredAccountReads,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn AccountsStore>, reads: StoredAccountReads) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            institutions: InMemoryAccountInstitutions::default(),
            reads,
        }))
    }

    /// `ledger`'s own Accounts in memory, reading the other domains through `reads`, for a test
    /// that needs no storage.
    pub fn in_memory(ledger: &Ledger, reads: StoredAccountReads) -> DesktopPorts {
        let ports = Self::shared(
            Box::new(lib_accounts::fake::InMemoryPorts {
                accounts: lib_accounts::fake::InMemoryAccounts {
                    accounts: ledger.accounts().to_vec(),
                },
                ..Default::default()
            }),
            reads,
        );
        ports.borrow_mut().copy_from_ledger(ledger);
        ports
    }

    fn copy_from_ledger(&mut self, ledger: &Ledger) {
        self.institutions.institutions = ledger
            .institutions()
            .iter()
            .map(|institution| AccountInstitution {
                id: institution.id,
                name: institution.name.clone(),
                usual_unit: institution.usual_unit,
            })
            .collect();
    }
}

impl AccountsPorts for LedgerBackedPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        self.store.accounts()
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        self.store.accounts_mut()
    }

    fn usage(&self) -> &dyn AccountUsage {
        &self.reads
    }

    fn institutions(&self) -> &dyn AccountInstitutions {
        &self.institutions
    }

    fn units(&self) -> &dyn AccountUnits {
        &self.reads
    }
}

/// The same ports as the store they wrap, for the Ledger's read copy, the Units and Bills reads
/// and the deletion adapter.
impl AccountsStore for LedgerBackedPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        self.store.accounts()
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        self.store.accounts_mut()
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

/// Copies the Institutions from `ledger` into `ports`, for the next Accounts event and a view model
/// drawn outside one.
pub fn copy_from_ledger(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_from_ledger(ledger);
}

/// Brings the Ledger read copy of the Accounts in line with the ports, announcing
/// `Domain::Accounts` when it changed, and copies the Ledger's reads back for the next Accounts
/// event. Run before and after every Accounts event and Dialog confirm, so the other Views see a
/// write at once.
pub fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_accounts(cx));
    copy_from_ledger(ports, ledger.read(cx).ledger());
}

/// `ledger`'s own Transactions and Budgets in memory, as the usage of its Accounts reads them, and
/// the stub Units, for a test that needs no storage.
pub fn in_memory_reads(ledger: &Ledger) -> StoredAccountReads {
    StoredAccountReads {
        transactions: Rc::new(RefCell::new(lib_transactions::fake::InMemoryTransactions {
            transactions: ledger.transactions().to_vec(),
        })),
        budgets: crate::budgets::ports::in_memory(ledger),
        units: crate::bills::ports::in_memory_units(chrono::Utc::now()),
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AccountId, UnitId};

    use super::*;

    #[test]
    fn the_transactions_and_budgets_are_counted_in_their_stores_on_each_ask() {
        let ledger = Ledger::stub();
        let reads = in_memory_reads(&ledger);
        let ports = LedgerBackedPorts::in_memory(&ledger, reads.clone());
        let everyday = AccountId::seed(2);
        let expected = lib_accounts::AccountUse {
            transactions: ledger.account_transactions(everyday),
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
            .expect("a stub Budget counts the Account");
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
        let mut booked = ledger.transactions()[0].clone();
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
    fn the_ports_offer_the_ledgers_institutions_and_the_stores_units_and_base_unit() {
        let ledger = Ledger::stub();
        let reads = in_memory_reads(&ledger);
        let held = reads.units.borrow().units().units().len();
        let ports = LedgerBackedPorts::in_memory(&ledger, reads);
        let ports = ports.borrow();

        assert_eq!(
            AccountsPorts::institutions(&*ports)
                .account_institutions()
                .len(),
            ledger.institutions().len()
        );
        let units = AccountsPorts::units(&*ports);
        assert_eq!(units.account_units().len(), held);
        assert_eq!(units.base_unit(), Some(UnitId::SEED_AUD));
    }
}
