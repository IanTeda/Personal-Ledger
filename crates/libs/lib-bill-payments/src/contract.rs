//! The contract every stored payments port keeps (ADR-0046), run against both the in-memory port
//! here and the `lib-database` adapter, so the fake the Clients' tests use can be trusted: a
//! payment lands its Transaction, the Transaction's Split and the Paid link together, and a payment
//! whose write fails lands none of them.
//!
//! Each check is a function generic over a [`PaymentsFactory`]: a consumer runs it from its own
//! test, since an adapter's tests need their own set-up (a fresh database per test).

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_bills::fake::InMemoryBills;
use lib_bills::{
    AmountKind, BillPlan, BillScheduleEntry, EntryId, Recurrence, Resolution, SplitRef,
};
use lib_core::{
    AccountId, BillPlanId, BillScheduleEntryId, CategoryId, Money, PayeeId, TransactionId,
    TransactionStatus, UnitId,
};
use lib_transactions::{Split, Transaction};

use crate::fake::InMemoryPayments;
use crate::interactor::PayBill;
use crate::protocols::BillPayments;

/// The rows a store holds, in an order two stores compare in: Transactions by id, Bill Plans by
/// id, entries by Plan and due date.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub transactions: Vec<Transaction>,
    pub plans: Vec<BillPlan>,
    pub entries: Vec<BillScheduleEntry>,
}

impl Stored {
    pub fn new(
        mut transactions: Vec<Transaction>,
        mut plans: Vec<BillPlan>,
        mut entries: Vec<BillScheduleEntry>,
    ) -> Self {
        transactions.sort_by_key(|transaction| transaction.id);
        plans.sort_by_key(|plan| plan.id);
        entries.sort_by_key(BillScheduleEntry::id);
        Self {
            transactions,
            plans,
            entries,
        }
    }
}

/// Builds the ports a contract check runs on.
pub trait PaymentsFactory {
    type Ports: BillPayments;

    /// Ports holding `bills`' Bill Plans and entries beside `transactions`, as if stored and
    /// loaded. Every Plan's Unit is the stub's aud.
    fn create(&mut self, bills: InMemoryBills, transactions: Vec<Transaction>) -> Self::Ports;

    /// Ends the writes made on `ports`, then reads back what is stored, as a restart would.
    fn stored(&mut self, ports: Self::Ports) -> Stored;

    /// Makes every Bill Schedule write from now on fail, as a store that refuses it would.
    fn break_entry_writes(&mut self, ports: &mut Self::Ports);

    /// Ends the writes made on `ports` and settles them, then reads back what is stored, with the
    /// number of failures reported.
    fn recover(&mut self, ports: Self::Ports) -> (Stored, usize);
}

/// The in-memory port is its own storage. It keeps a copy as of the last commit, which a failed
/// write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryPayments>,
    broken: bool,
    failures: usize,
}

impl InMemoryFactory {
    fn commit(&mut self, ports: &InMemoryPayments) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn read(&self) -> Stored {
        let committed = self.committed.clone().expect("create ran first");
        Stored::new(
            committed.transactions.transactions,
            committed.bills.plans,
            committed.bills.entries,
        )
    }
}

impl PaymentsFactory for InMemoryFactory {
    type Ports = InMemoryPayments;

    fn create(&mut self, bills: InMemoryBills, transactions: Vec<Transaction>) -> InMemoryPayments {
        let ports = InMemoryPayments {
            transactions: lib_transactions::fake::InMemoryTransactions { transactions },
            bills,
        };
        self.committed = Some(ports.clone());
        ports
    }

    fn stored(&mut self, ports: InMemoryPayments) -> Stored {
        self.commit(&ports);
        self.read()
    }

    fn break_entry_writes(&mut self, _ports: &mut InMemoryPayments) {
        self.broken = true;
    }

    fn recover(&mut self, ports: InMemoryPayments) -> (Stored, usize) {
        self.commit(&ports);
        (self.read(), std::mem::take(&mut self.failures))
    }
}

fn cents(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, day).expect("a September day")
}

/// One Monthly Plan with an open entry due on the 22nd, and one Transaction already stored.
fn seed() -> (InMemoryBills, Vec<Transaction>) {
    let plan = BillPlan {
        id: BillPlanId::seed(1),
        name: "Internet".to_string(),
        category_id: CategoryId::seed(7),
        unit: UnitId::SEED_AUD,
        account_id: AccountId::seed(2),
        payee_id: Some(PayeeId::seed(3)),
        planned_amount: cents(8_999),
        amount_kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: date(22),
        ends_on: None,
        attention_lead: Some(5),
        is_active: true,
        generate_from: date(1),
    };
    let entry = BillScheduleEntry {
        row_id: BillScheduleEntryId::seed(1),
        plan_id: plan.id,
        due: date(22),
        resolution: Resolution::Unresolved,
        superseded: false,
    };
    let groceries = Transaction {
        id: TransactionId::seed(1),
        date: date(10),
        account_id: AccountId::seed(2),
        status: TransactionStatus::Cleared,
        is_flagged: false,
        description: Some("Groceries".to_string()),
        splits: vec![Split {
            amount: cents(-4_250),
            category_id: CategoryId::seed(4),
            payee_id: None,
            tag_ids: Vec::new(),
        }],
    };
    (
        InMemoryBills {
            plans: vec![plan],
            entries: vec![entry],
        },
        vec![groceries],
    )
}

fn pay() -> PayBill {
    PayBill {
        entry: EntryId {
            plan_id: BillPlanId::seed(1),
            due: date(22),
        },
        amount: cents(8_999),
        date: date(21),
    }
}

/// A payment stores its Transaction with its Split, and the entry Paid by that Split.
pub fn a_payment_lands_the_transaction_its_split_and_the_paid_link<F: PaymentsFactory>(
    factory: &mut F,
) {
    let (bills, transactions) = seed();
    let mut ports = factory.create(bills, transactions);

    let paid = pay().run(&mut ports).expect("the entry is open");
    let stored = factory.stored(ports);

    let transaction = stored
        .transactions
        .iter()
        .find(|transaction| transaction.id == paid)
        .expect("the Transaction is stored");
    assert_eq!(transaction.date, date(21));
    assert_eq!(transaction.status, TransactionStatus::Open);
    assert_eq!(
        transaction.splits,
        [Split {
            amount: cents(-8_999),
            category_id: CategoryId::seed(7),
            payee_id: Some(PayeeId::seed(3)),
            tag_ids: Vec::new(),
        }]
    );
    assert_eq!(stored.transactions.len(), 2, "the stored one is kept");
    assert_eq!(
        stored.entries[0].resolution,
        Resolution::Paid(SplitRef {
            transaction_id: paid,
            split_index: 0,
        })
    );
}

/// A payment whose entry write fails stores neither the entry nor the Transaction, and the failure
/// is reported.
pub fn a_failing_payment_lands_neither_row<F: PaymentsFactory>(factory: &mut F) {
    let (bills, transactions) = seed();
    let before = Stored::new(
        transactions.clone(),
        bills.plans.clone(),
        bills.entries.clone(),
    );
    let mut ports = factory.create(bills, transactions);

    factory.break_entry_writes(&mut ports);
    pay().run(&mut ports).expect("the port takes the write");
    let (stored, failures) = factory.recover(ports);

    assert_eq!(stored, before);
    assert!(failures > 0, "the failure is reported");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_lands_a_payment() {
        a_payment_lands_the_transaction_its_split_and_the_paid_link(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_lands_no_part_of_a_failing_payment() {
        a_failing_payment_lands_neither_row(&mut InMemoryFactory::default());
    }
}
