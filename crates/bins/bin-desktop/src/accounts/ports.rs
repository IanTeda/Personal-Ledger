//! The Accounts ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `AccountUsage`, `AccountUnits` and `AccountInstitutions` read from the Transactions, Budgets,
//! Units and Institutions stores through `lib-database`'s [`StoredAccountReads`] (#993, #1052,
//! #1053), and the step that keeps the Ledger's read copy of the Accounts in line after an
//! Accounts write. The write is committed by the input it was made in (`crate::input::send`), not
//! here.
//!
//! The ports are one value shared several ways: the Accounts module's Presenters read and write
//! them as [`AccountsPorts`], and the same value is the `AccountsStore` the Ledger's read copy,
//! the Units and Bills reads and `lib-account-deletion`'s adapter hold.
//!
//! The usage is counted, and the Units and Institutions read, in the stores on each ask, whether they are the
//! adapters or `Stores::fake`'s in-memory ones, so nothing is recounted or copied from the Ledger.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_accounts::{
    AccountInstitutions, AccountRepository, AccountUnits, AccountUsage, AccountsPorts,
};
use lib_database::{AccountsStore, StoredAccountReads};
use lib_ledger::Ledger;

use crate::ledger_store::LedgerStore;

/// The Accounts ports the Accounts module and the Ledger share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Accounts, with the Institutions an Account may name, the Units it may count in and the
/// Transactions and Budgets that name each read from `reads`.
pub struct LedgerBackedPorts {
    store: Box<dyn AccountsStore>,
    reads: StoredAccountReads,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn AccountsStore>, reads: StoredAccountReads) -> DesktopPorts {
        Rc::new(RefCell::new(Self { store, reads }))
    }

    /// `ledger`'s own Accounts in memory, reading the other domains through `reads`, for a test
    /// that needs no storage.
    pub fn in_memory(ledger: &Ledger, reads: StoredAccountReads) -> DesktopPorts {
        Self::shared(
            Box::new(lib_accounts::fake::InMemoryPorts {
                accounts: lib_accounts::fake::InMemoryAccounts {
                    accounts: ledger.accounts().to_vec(),
                },
                ..Default::default()
            }),
            reads,
        )
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
        &self.reads
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

/// Brings the Ledger read copy of the Accounts in line with the ports, announcing
/// `Domain::Accounts` when it changed. Run before and after every Accounts event and Dialog confirm, so the other Views see a
/// write at once.
pub fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_accounts(cx));
}

/// `ledger`'s own Transactions and Budgets in memory, as the usage of its Accounts reads them, and
/// the stub Units and Institutions, for a test that needs no storage.
pub fn in_memory_reads(ledger: &Ledger) -> StoredAccountReads {
    StoredAccountReads {
        transactions: Rc::new(RefCell::new(lib_transactions::fake::InMemoryTransactions {
            transactions: ledger.transactions().to_vec(),
        })),
        budgets: crate::budgets::ports::in_memory(ledger),
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
    fn the_ports_offer_the_stores_institutions_units_and_base_unit() {
        let ledger = Ledger::stub();
        let reads = in_memory_reads(&ledger);
        let held = reads.units.borrow().units().units().len();
        let ports = LedgerBackedPorts::in_memory(&ledger, reads);
        let ports = ports.borrow();

        assert_eq!(
            AccountsPorts::institutions(&*ports)
                .account_institutions()
                .len(),
            lib_institutions::default_institutions().len()
        );
        let units = AccountsPorts::units(&*ports);
        assert_eq!(units.account_units().len(), held);
        assert_eq!(units.base_unit(), Some(UnitId::SEED_AUD));
    }
}
