//! The contract every stored Transactions port keeps (ADR-0046), run against both the in-memory
//! port here and the `lib-database` adapter, so the fake the Clients' tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the port, reloads as a restart would, and expects the same rows back.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{AccountId, CategoryId, Money, PayeeId, TagId, TransactionId, TransactionStatus};

use crate::fake::InMemoryTransactions;
use crate::{Split, Transaction, TransactionRepository};

/// Builds the port a contract check runs on.
pub trait PortsFactory {
    type Ports: TransactionRepository;

    /// A port holding `seed`'s Transactions, as if stored and loaded.
    fn create(&mut self, seed: InMemoryTransactions) -> Self::Ports;

    /// Ends the writes made on `ports`, then loads a fresh port from wherever they were stored, as
    /// a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one call's writes on `ports`, sending them to storage as an adapter's owner does.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Transactions domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
}

/// The in-memory port is its own storage, so a reload hands it back as it is. It keeps a copy as
/// of the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryTransactions>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryTransactions;

    fn create(&mut self, seed: InMemoryTransactions) -> InMemoryTransactions {
        self.committed = Some(seed.clone());
        seed
    }

    fn reload(&mut self, mut ports: InMemoryTransactions) -> InMemoryTransactions {
        self.commit(&mut ports);
        ports
    }

    fn commit(&mut self, ports: &mut InMemoryTransactions) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryTransactions) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryTransactions) -> (InMemoryTransactions, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

/// The port's Transactions in id order, so two ports compare whatever order they list in.
pub fn snapshot(ports: &impl TransactionRepository) -> Vec<Transaction> {
    let mut transactions = ports.transactions();
    transactions.sort_by_key(|transaction| transaction.id);
    transactions
}

fn cents(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

fn split(amount: i64, category: u32, payee: Option<u32>, tags: &[u32]) -> Split {
    Split {
        amount: cents(amount),
        category_id: CategoryId::seed(category),
        payee_id: payee.map(PayeeId::seed),
        tag_ids: tags.iter().copied().map(TagId::seed).collect(),
    }
}

fn transaction(id: u32, day: u32, splits: Vec<Split>) -> Transaction {
    Transaction {
        id: TransactionId::seed(id),
        date: NaiveDate::from_ymd_opt(2026, 9, day).expect("a September day"),
        account_id: AccountId::seed(1),
        status: TransactionStatus::Open,
        is_flagged: false,
        description: None,
        splits,
    }
}

/// Two stored Transactions: a plain one and a two-Split receipt with a Tag on one Split.
fn seed() -> InMemoryTransactions {
    InMemoryTransactions {
        transactions: vec![
            transaction(1, 1, vec![split(-1_850, 7, Some(3), &[])]),
            transaction(
                2,
                2,
                vec![
                    split(-18_000, 7, Some(3), &[1]),
                    split(-3_430, 10, Some(3), &[]),
                ],
            ),
        ],
    }
}

/// Every field round-trips: Splits in their order, each with or without a Payee and with any
/// number of Tags in their order, the status, the flag and the memo. Editing a receipt down to one
/// Split leaves no stale Split or Tag behind.
pub fn splits_and_tags_round_trip<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(seed());

    ports.upsert_transaction(Transaction {
        status: TransactionStatus::Reconciled,
        is_flagged: true,
        description: Some("Tokyo dinner and souvenirs".into()),
        account_id: AccountId::seed(2),
        ..transaction(
            3,
            6,
            vec![
                split(-6_400, 8, Some(4), &[5, 2]),
                split(-3_000, 10, None, &[]),
                split(421_000, 12, Some(9), &[2, 5, 6]),
            ],
        )
    });
    ports.upsert_transaction(transaction(2, 3, vec![split(-21_430, 7, None, &[])]));

    let before = snapshot(&ports);
    let ports = factory.reload(ports);
    assert_eq!(snapshot(&ports), before);
    assert_eq!(before.len(), 3);
    let receipt = ports
        .transaction(TransactionId::seed(2))
        .expect("the edited receipt stays");
    assert_eq!(receipt.splits, vec![split(-21_430, 7, None, &[])]);
}

/// A deleted Transaction stays gone after a reload, with its Splits, and the rest stay.
pub fn a_deleted_transaction_stays_deleted<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(seed());

    ports.delete_transaction(TransactionId::seed(2));
    ports.delete_transaction(TransactionId::seed(99));

    let ports = factory.reload(ports);
    let ids: Vec<_> = snapshot(&ports)
        .into_iter()
        .map(|transaction| transaction.id)
        .collect();
    assert_eq!(ids, [TransactionId::seed(1)]);
}

/// A write that fails is reported once, naming the Transactions domain, and the port goes back to
/// the last committed rows rather than keeping the change the store refused.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(seed());
    let stored = snapshot(&ports);

    factory.break_writes(&mut ports);
    ports.upsert_transaction(transaction(3, 4, vec![split(-500, 7, None, &[])]));
    factory.commit(&mut ports);

    let (ports, failures) = factory.recover(ports);
    assert_eq!(failures, 1, "one failure naming the Transactions domain");
    assert_eq!(snapshot(&ports), stored, "the refused write is gone");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_splits_and_tags_round_trip() {
        splits_and_tags_round_trip(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_deleted_transaction_stays_deleted() {
        a_deleted_transaction_stays_deleted(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
