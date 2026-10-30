//! The contract every stored Transactions port keeps (ADR-0046), run against both the in-memory
//! port here and the `lib-database` adapter, so the fake the Clients' tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the port, reloads as a restart would, and expects the same rows back.
//!
//! The read ports' checks run on a [`ReadsFactory`] instead: the rows they read are other domains',
//! so a check hands the factory the rows as Transactions read them, the factory stores them as their
//! own domain would, and the check expects them answered the same, before and after a reload.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{
    AccountId, CategoryId, HexColor, Money, PayeeId, TagId, TransactionId, TransactionStatus,
    UnitId,
};

use crate::fake::{InMemoryTransactionReads, InMemoryTransactions};
use crate::{
    Split, Transaction, TransactionAccount, TransactionAccounts, TransactionCategories,
    TransactionCategory, TransactionPayee, TransactionPayees, TransactionRepository,
    TransactionTag, TransactionTags, TransactionUnit, TransactionUnits,
};

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

/// Builds the read port a read check runs on, over rows of type `Row`.
pub trait ReadsFactory<Row> {
    type Reads;

    /// A read port over `rows`, stored in the order given as their own domain stores them, and
    /// loaded. Each Category comes after its parent, with its path, and every Account names one
    /// of the stub's Units (aud, btc or vas).
    fn create(&mut self, rows: Vec<Row>) -> Self::Reads;

    /// Loads a fresh read port from wherever [`Self::create`] stored the rows, as a restart would.
    fn reload(&mut self, reads: Self::Reads) -> Self::Reads;
}

/// The in-memory reads are their own storage, so a reload hands them back as they are. They hold
/// the rows as answered, so the Categories are put in tree order as they are stored.
#[derive(Debug, Default)]
pub struct InMemoryReadsFactory;

/// `categories` depth-first from each top-level one, siblings in the order given.
fn tree_order(categories: &[TransactionCategory]) -> Vec<TransactionCategory> {
    fn walk(
        categories: &[TransactionCategory],
        parent: Option<CategoryId>,
        out: &mut Vec<TransactionCategory>,
    ) {
        for category in categories.iter().filter(|row| row.parent == parent) {
            out.push(category.clone());
            walk(categories, Some(category.id), out);
        }
    }
    let mut out = Vec::with_capacity(categories.len());
    walk(categories, None, &mut out);
    out
}

impl ReadsFactory<TransactionAccount> for InMemoryReadsFactory {
    type Reads = InMemoryTransactionReads;

    fn create(&mut self, rows: Vec<TransactionAccount>) -> InMemoryTransactionReads {
        InMemoryTransactionReads {
            accounts: rows,
            ..InMemoryTransactionReads::default()
        }
    }

    fn reload(&mut self, reads: InMemoryTransactionReads) -> InMemoryTransactionReads {
        reads
    }
}

impl ReadsFactory<TransactionCategory> for InMemoryReadsFactory {
    type Reads = InMemoryTransactionReads;

    fn create(&mut self, rows: Vec<TransactionCategory>) -> InMemoryTransactionReads {
        InMemoryTransactionReads {
            categories: tree_order(&rows),
            ..InMemoryTransactionReads::default()
        }
    }

    fn reload(&mut self, reads: InMemoryTransactionReads) -> InMemoryTransactionReads {
        reads
    }
}

impl ReadsFactory<TransactionPayee> for InMemoryReadsFactory {
    type Reads = InMemoryTransactionReads;

    fn create(&mut self, rows: Vec<TransactionPayee>) -> InMemoryTransactionReads {
        InMemoryTransactionReads {
            payees: rows,
            ..InMemoryTransactionReads::default()
        }
    }

    fn reload(&mut self, reads: InMemoryTransactionReads) -> InMemoryTransactionReads {
        reads
    }
}

impl ReadsFactory<TransactionTag> for InMemoryReadsFactory {
    type Reads = InMemoryTransactionReads;

    fn create(&mut self, rows: Vec<TransactionTag>) -> InMemoryTransactionReads {
        InMemoryTransactionReads {
            tags: rows,
            ..InMemoryTransactionReads::default()
        }
    }

    fn reload(&mut self, reads: InMemoryTransactionReads) -> InMemoryTransactionReads {
        reads
    }
}

impl ReadsFactory<TransactionUnit> for InMemoryReadsFactory {
    type Reads = InMemoryTransactionReads;

    fn create(&mut self, rows: Vec<TransactionUnit>) -> InMemoryTransactionReads {
        InMemoryTransactionReads {
            units: rows,
            ..InMemoryTransactionReads::default()
        }
    }

    fn reload(&mut self, reads: InMemoryTransactionReads) -> InMemoryTransactionReads {
        reads
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

/// Every Account is answered with its name and Unit, in the order stored, and again after a
/// reload.
pub fn accounts_read_back<F>(factory: &mut F)
where
    F: ReadsFactory<TransactionAccount>,
    F::Reads: TransactionAccounts,
{
    let account = |id: u32, name: &str, unit: UnitId| TransactionAccount {
        id: AccountId::seed(id),
        name: name.into(),
        unit,
    };
    let stored = vec![
        account(7, "Offset", UnitId::SEED_AUD),
        account(3, "Cold wallet", UnitId::SEED_BTC),
        account(5, "Amex", UnitId::SEED_AUD),
    ];

    let reads = factory.create(stored.clone());
    assert_eq!(reads.transaction_accounts(), stored);
    let reads = factory.reload(reads);
    assert_eq!(reads.transaction_accounts(), stored);
}

/// Every Category is answered with its parent and path in tree order, whatever order they were
/// added in, and again after a reload.
pub fn categories_read_back_in_tree_order<F>(factory: &mut F)
where
    F: ReadsFactory<TransactionCategory>,
    F::Reads: TransactionCategories,
{
    // The path separator is `lib-categories`' rule, which the answering side fills.
    let category = |id: u32, name: &str, parent: Option<u32>, path: &str| TransactionCategory {
        id: CategoryId::seed(id),
        name: name.into(),
        parent: parent.map(CategoryId::seed),
        path: path.into(),
    };
    let housing = category(1, "Housing", None, "Housing");
    let rent = category(2, "Rent", Some(1), "Housing \u{203a} Rent");
    let utilities = category(3, "Utilities", Some(1), "Housing \u{203a} Utilities");
    let power = category(
        4,
        "Power",
        Some(3),
        "Housing \u{203a} Utilities \u{203a} Power",
    );
    let food = category(5, "Food", None, "Food");
    let groceries = category(6, "Groceries", Some(5), "Food \u{203a} Groceries");
    let added = vec![
        housing.clone(),
        food.clone(),
        utilities.clone(),
        groceries.clone(),
        rent.clone(),
        power.clone(),
    ];
    let tree = vec![housing, utilities, power, rent, food, groceries];

    let reads = factory.create(added);
    assert_eq!(reads.transaction_categories(), tree);
    let reads = factory.reload(reads);
    assert_eq!(reads.transaction_categories(), tree);
}

/// Every Payee is answered with its name, in the order stored, and again after a reload.
pub fn payees_read_back<F>(factory: &mut F)
where
    F: ReadsFactory<TransactionPayee>,
    F::Reads: TransactionPayees,
{
    let payee = |id: u32, name: &str| TransactionPayee {
        id: PayeeId::seed(id),
        name: name.into(),
    };
    let stored = vec![payee(4, "Woolworths"), payee(2, "BP"), payee(9, "Origin")];

    let reads = factory.create(stored.clone());
    assert_eq!(reads.transaction_payees(), stored);
    let reads = factory.reload(reads);
    assert_eq!(reads.transaction_payees(), stored);
}

/// Every Tag is answered with its name and colour, or none, in the order stored, and again after a
/// reload.
pub fn tags_read_back<F>(factory: &mut F)
where
    F: ReadsFactory<TransactionTag>,
    F::Reads: TransactionTags,
{
    let tag = |id: u32, name: &str, color: Option<&str>| TransactionTag {
        id: TagId::seed(id),
        name: name.into(),
        color: color.map(|hex| HexColor::parse(hex).expect("a colour")),
    };
    let stored = vec![
        tag(6, "japan-trip", Some("#1f6feb")),
        tag(2, "work", None),
        tag(4, "tax", Some("#ae1800")),
    ];

    let reads = factory.create(stored.clone());
    assert_eq!(reads.transaction_tags(), stored);
    let reads = factory.reload(reads);
    assert_eq!(reads.transaction_tags(), stored);
}

/// Each Unit's code is answered by its id, and again after a reload; a Unit not stored has none.
pub fn unit_codes_read_back<F>(factory: &mut F)
where
    F: ReadsFactory<TransactionUnit>,
    F::Reads: TransactionUnits,
{
    let unit = |id: u32, code: &str| TransactionUnit {
        id: UnitId::seed(id),
        code: code.into(),
    };
    let stored = vec![unit(11, "aud"), unit(12, "usd"), unit(13, "btc")];
    let codes = |reads: &F::Reads| -> Vec<Option<String>> {
        stored
            .iter()
            .map(|unit| reads.transaction_unit_code(unit.id))
            .collect()
    };
    let expected: Vec<_> = stored.iter().map(|unit| Some(unit.code.clone())).collect();

    let reads = factory.create(stored.clone());
    assert_eq!(codes(&reads), expected);
    assert_eq!(reads.transaction_unit_code(UnitId::seed(99)), None);
    let reads = factory.reload(reads);
    assert_eq!(codes(&reads), expected);
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

    #[test]
    fn in_memory_accounts_read_back() {
        accounts_read_back(&mut InMemoryReadsFactory);
    }

    #[test]
    fn in_memory_categories_read_back_in_tree_order() {
        categories_read_back_in_tree_order(&mut InMemoryReadsFactory);
    }

    #[test]
    fn in_memory_payees_read_back() {
        payees_read_back(&mut InMemoryReadsFactory);
    }

    #[test]
    fn in_memory_tags_read_back() {
        tags_read_back(&mut InMemoryReadsFactory);
    }

    #[test]
    fn in_memory_unit_codes_read_back() {
        unit_codes_read_back(&mut InMemoryReadsFactory);
    }
}
