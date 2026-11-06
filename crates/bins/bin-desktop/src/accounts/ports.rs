//! The Accounts ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `AccountUsage`, `AccountUnits` and `AccountInstitutions` read from the Transactions, Budgets,
//! Units and Institutions stores through `lib-database`'s [`StoredAccountReads`] (#993, #1052,
//! #1053), and the announcement that tells the Views reading the Accounts of a write. The Ledger
//! keeps no read copy of the Accounts (#1054). The write is committed by the input it was made in
//! (`crate::input::send`), not here.
//!
//! The ports are one value shared several ways: the Accounts module's Presenters read and write
//! them as [`AccountsPorts`], and the same value is the `AccountsStore` the Units and Bills
//! reads and `lib-account-deletion`'s adapter hold.
//!
//! The usage is counted, and the Units and Institutions read, in the stores on each ask, whether they are the
//! adapters or `Stores::fake`'s in-memory ones, so nothing is recounted or copied from the Ledger.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{App, Entity};
use lib_accounts::{
    Account, AccountInstitutions, AccountRepository, AccountUnits, AccountUsage, AccountsPorts,
};
use lib_core::AccountId;
use lib_database::FakeStores;
use lib_database::{AccountsStore, StoredAccountReads};

use crate::announcer::{Announcer, Domain};
use crate::write_count::WriteCount;

/// The Accounts ports the Accounts module and the Ledger share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Accounts, with the Institutions an Account may name, the Units it may count in and the
/// Transactions and Budgets that name each read from `reads`.
pub struct LedgerBackedPorts {
    accounts: CountedAccounts,
    reads: StoredAccountReads,
    /// The write count the last announcement saw, so a write is announced once.
    announced: Cell<u64>,
}

/// The store's Accounts, each write counted so the ports can tell whether an input wrote.
struct CountedAccounts {
    store: Box<dyn AccountsStore>,
    writes: WriteCount,
}

impl AccountRepository for CountedAccounts {
    fn accounts(&self) -> Vec<Account> {
        self.store.accounts().accounts()
    }

    fn upsert_account(&mut self, account: Account) {
        self.writes.bump();
        self.store.accounts_mut().upsert_account(account);
    }

    fn delete_account(&mut self, id: AccountId) {
        self.writes.bump();
        self.store.accounts_mut().delete_account(id);
    }
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn AccountsStore>, reads: StoredAccountReads) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            accounts: CountedAccounts {
                store,
                writes: WriteCount::default(),
            },
            reads,
            announced: Cell::new(0),
        }))
    }

    /// Whether these ports were written since the last ask.
    fn take_written(&self) -> bool {
        let writes = self.accounts.writes.get();
        writes != self.announced.replace(writes)
    }

    /// The stub Accounts in memory, reading the other domains through `reads`, for a test that
    /// needs no storage.
    pub fn in_memory(reads: StoredAccountReads) -> DesktopPorts {
        Self::shared(
            Box::new(lib_accounts::fake::InMemoryPorts {
                accounts: lib_accounts::fake::InMemoryAccounts {
                    accounts: lib_accounts::default_accounts(),
                },
                ..Default::default()
            }),
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
        self.accounts.store.reload()
    }
}

/// Puts the Accounts back as stored, after a write that named them failed, and announces
/// `Domain::Accounts` so every reader redraws (ADR-0046).
///
/// # Errors
/// Returns the error reading them back; the ports are left as they were.
pub fn reload(
    ports: &DesktopPorts,
    announcer: &Entity<Announcer>,
    cx: &mut App,
) -> lib_database::Result<()> {
    AccountsStore::reload(&mut *ports.borrow_mut())?;
    announce(announcer, cx);
    Ok(())
}

/// Announces `Domain::Accounts`, so every reader redraws (#1054).
fn announce(announcer: &Entity<Announcer>, cx: &mut App) {
    announcer.update(cx, |announcer, cx| announcer.announce(Domain::Accounts, cx));
}

/// Announces `Domain::Accounts` when the ports were written since the last announcement, so the
/// other Views see a write at once. Run before and after every Accounts event and Dialog confirm,
/// and after Account delete.
pub fn refresh(ports: &DesktopPorts, announcer: &Entity<Announcer>, cx: &mut App) {
    if ports.borrow().take_written() {
        announce(announcer, cx);
    }
}

/// `stub`'s Transactions and Budgets in memory, as the usage of its Accounts reads them, and the
/// stub Units and Institutions, for a test that needs no storage.
pub fn in_memory_reads(stub: &FakeStores) -> StoredAccountReads {
    StoredAccountReads {
        transactions: Rc::new(RefCell::new(stub.transactions.clone())),
        budgets: crate::budgets::ports::in_memory(stub),
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
        let stub = FakeStores::sample(chrono::Local::now().date_naive());
        let reads = in_memory_reads(&stub);
        let ports = LedgerBackedPorts::in_memory(reads.clone());
        let everyday = AccountId::seed(2);
        let expected = lib_accounts::AccountUse {
            transactions: lib_database::seed::account_transactions(
                &stub.transactions.transactions,
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
        let mut booked = stub.transactions.transactions[0].clone();
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
            lib_institutions::default_institutions().len()
        );
        let units = AccountsPorts::units(&*ports);
        assert_eq!(units.account_units().len(), held);
        assert_eq!(units.base_unit(), Some(UnitId::SEED_AUD));
    }

    #[test]
    fn a_write_through_the_ports_is_noted_once() {
        let ports = LedgerBackedPorts::in_memory(in_memory_reads(&FakeStores::sample(
            chrono::Local::now().date_naive(),
        )));
        let mut held = ports.borrow_mut();
        assert!(!held.take_written());

        AccountsStore::accounts_mut(&mut *held).delete_account(AccountId::seed(3));

        assert!(
            AccountsStore::accounts(&*held)
                .account(AccountId::seed(3))
                .is_none()
        );
        assert!(held.take_written());
        assert!(!held.take_written());
    }
}
