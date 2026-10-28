//! The contract every stored deletion port keeps (ADR-0046), run against both the in-memory port
//! here and the `lib-database` adapter, so the fake the Clients' tests use can be trusted: a
//! delete lands the Account's removal, its Transactions' and the Unmatch of the entries they paid
//! together, a delete whose write fails lands none of them, and a refused delete writes nothing.
//!
//! Each check is a function generic over a [`DeletionFactory`]: a consumer runs it from its own
//! test, since an adapter's tests need their own set-up (a fresh database per test). What the port
//! doesn't hold, the Balance Checks that cascade with an Account, the adapter tests itself.

use lib_accounts::Account;
use lib_bills::{BillPlan, BillScheduleEntry, Resolution, entry};
use lib_transactions::Transaction;

use crate::error::Error;
use crate::fake::InMemoryDeletion;
use crate::fake::world::{
    EVERYDAY, EVERYDAY_PAYMENT, OFFSET, WALLET, paid_by_everyday, paid_by_wallet, world,
};
use crate::interactor::DeleteAccount;
use crate::protocols::AccountDeletion;

/// The rows a store holds, in an order two stores compare in: Accounts, Transactions and Bill
/// Plans by id, entries by Plan and due date.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub accounts: Vec<Account>,
    pub transactions: Vec<Transaction>,
    pub plans: Vec<BillPlan>,
    pub entries: Vec<BillScheduleEntry>,
}

impl Stored {
    pub fn new(
        mut accounts: Vec<Account>,
        mut transactions: Vec<Transaction>,
        mut plans: Vec<BillPlan>,
        mut entries: Vec<BillScheduleEntry>,
    ) -> Self {
        accounts.sort_by_key(|account| account.id);
        transactions.sort_by_key(|transaction| transaction.id);
        plans.sort_by_key(|plan| plan.id);
        entries.sort_by_key(BillScheduleEntry::id);
        Self {
            accounts,
            transactions,
            plans,
            entries,
        }
    }

    fn of(rows: &InMemoryDeletion) -> Self {
        Self::new(
            rows.accounts.accounts.clone(),
            rows.transactions.transactions.clone(),
            rows.bills.plans.clone(),
            rows.bills.entries.clone(),
        )
    }
}

/// Builds the ports a contract check runs on.
pub trait DeletionFactory {
    type Ports: AccountDeletion;

    /// Ports holding `rows`' Accounts, Transactions, Bill Plans and entries, as if stored and
    /// loaded. Every Account and Plan's Unit is the stub's aud, and no Split names a Payee.
    fn create(&mut self, rows: InMemoryDeletion) -> Self::Ports;

    /// Ends the writes made on `ports`, then reads back what is stored, as a restart would.
    fn stored(&mut self, ports: Self::Ports) -> Stored;

    /// Makes every Account delete from now on fail, as a store that refuses it would.
    fn break_account_deletes(&mut self, ports: &mut Self::Ports);

    /// Ends the writes made on `ports` and settles them, then reads back what is stored, with the
    /// number of failures reported.
    fn recover(&mut self, ports: Self::Ports) -> (Stored, usize);
}

/// The in-memory port is its own storage. It keeps a copy as of the last commit, which a failed
/// write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryDeletion>,
    broken: bool,
    failures: usize,
}

impl InMemoryFactory {
    fn commit(&mut self, ports: &InMemoryDeletion) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn read(&self) -> Stored {
        Stored::of(self.committed.as_ref().expect("create ran first"))
    }
}

impl DeletionFactory for InMemoryFactory {
    type Ports = InMemoryDeletion;

    fn create(&mut self, rows: InMemoryDeletion) -> InMemoryDeletion {
        self.committed = Some(rows.clone());
        rows
    }

    fn stored(&mut self, ports: InMemoryDeletion) -> Stored {
        self.commit(&ports);
        self.read()
    }

    fn break_account_deletes(&mut self, _ports: &mut InMemoryDeletion) {
        self.broken = true;
    }

    fn recover(&mut self, ports: InMemoryDeletion) -> (Stored, usize) {
        self.commit(&ports);
        (self.read(), std::mem::take(&mut self.failures))
    }
}

fn resolution(stored: &Stored, id: lib_bills::EntryId) -> Option<Resolution> {
    entry(&stored.entries, id).map(|entry| entry.resolution.clone())
}

/// A delete stores the Account's removal with its Transactions', and the entry they paid
/// Unmatched; the other Accounts, their Transactions, the Plan and the entries they paid stay.
pub fn a_delete_lands_the_account_its_transactions_and_their_unmatch<F: DeletionFactory>(
    factory: &mut F,
) {
    let mut ports = factory.create(world());

    DeleteAccount { id: WALLET }
        .run(&mut ports)
        .expect("nothing pays from the Wallet");
    let stored = factory.stored(ports);

    let ids: Vec<_> = stored.accounts.iter().map(|account| account.id).collect();
    assert_eq!(ids, [EVERYDAY, OFFSET]);
    let booked: Vec<_> = stored.transactions.iter().map(|t| t.id).collect();
    assert_eq!(
        booked,
        [EVERYDAY_PAYMENT],
        "only Everyday's Transaction is left"
    );
    assert_eq!(
        resolution(&stored, paid_by_wallet()),
        Some(Resolution::Unresolved)
    );
    assert_eq!(
        resolution(&stored, paid_by_everyday()),
        resolution(&Stored::of(&world()), paid_by_everyday()),
        "Everyday's payment still pays its entry"
    );
    assert_eq!(stored.plans, world().bills.plans);
}

/// A delete whose Account write fails stores none of it: the Account, its Transactions and the
/// Paid links stand, and the failure is reported.
pub fn a_failing_delete_lands_nothing<F: DeletionFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    factory.break_account_deletes(&mut ports);
    DeleteAccount { id: WALLET }
        .run(&mut ports)
        .expect("the port takes the write");
    let (stored, failures) = factory.recover(ports);

    assert_eq!(stored, before);
    assert!(failures > 0, "the failure is reported");
}

/// An Account a Bill Plan pays from is refused by the port's own read, before anything is written.
pub fn an_account_a_bill_plan_pays_from_is_refused<F: DeletionFactory>(factory: &mut F) {
    let mut ports = factory.create(world());

    assert_eq!(
        DeleteAccount { id: EVERYDAY }.run(&mut ports),
        Err(Error::PaidFromByBillPlans { plans: 1 })
    );

    assert_eq!(factory.stored(ports), Stored::of(&world()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_lands_a_delete() {
        a_delete_lands_the_account_its_transactions_and_their_unmatch(
            &mut InMemoryFactory::default(),
        );
    }

    #[test]
    fn the_fake_lands_no_part_of_a_failing_delete() {
        a_failing_delete_lands_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_refuses_an_account_a_bill_plan_pays_from() {
        an_account_a_bill_plan_pays_from_is_refused(&mut InMemoryFactory::default());
    }
}
