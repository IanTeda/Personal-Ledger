//! The contract every stored import port keeps (ADR-0046), run against both the in-memory port
//! here and the `lib-database` adapter, so the fake the Clients' tests use can be trusted: a commit
//! lands its new Payees, their aliases and its Transactions together, and a refused plan lands none
//! of them.
//!
//! Each check is a function generic over an [`ImportFactory`]: a consumer runs it from its own
//! test, since an adapter's tests need their own set-up (a fresh database per test).

use chrono::NaiveDate;
use lib_core::{CategoryId, PayeeId, TransactionId, TransactionStatus};
use lib_payees::Payee;
use lib_payees::fake::InMemoryPayees;
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{Split, Transaction};

use crate::cents;
use crate::entity::{ImportRow, match_line};
use crate::error::Error;
use crate::fake::InMemoryImport;
use crate::plan::{ImportCommitted, ImportPlan};
use crate::protocols::{StatementSource, TransactionImport};
use crate::statement::SeededStatement;

/// The rows a store holds, in an order two stores compare in: Payees and Transactions by id.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub payees: Vec<Payee>,
    pub transactions: Vec<Transaction>,
}

impl Stored {
    pub fn new(mut payees: Vec<Payee>, mut transactions: Vec<Transaction>) -> Self {
        payees.sort_by_key(|payee| payee.id);
        transactions.sort_by_key(|transaction| transaction.id);
        Self {
            payees,
            transactions,
        }
    }
}

/// Builds the ports a contract check runs on.
pub trait ImportFactory {
    type Ports: TransactionImport;

    /// Ports holding `payees` with their aliases beside `transactions`, as if stored and loaded.
    /// Every row names the stub Ledger's Accounts and Categories.
    fn create(&mut self, payees: Vec<Payee>, transactions: Vec<Transaction>) -> Self::Ports;

    /// Ends the writes made on `ports`, then reads back what is stored, as a restart would.
    fn stored(&mut self, ports: Self::Ports) -> Stored;
}

/// The in-memory port is its own storage.
#[derive(Debug, Default)]
pub struct InMemoryFactory;

impl ImportFactory for InMemoryFactory {
    type Ports = InMemoryImport;

    fn create(&mut self, payees: Vec<Payee>, transactions: Vec<Transaction>) -> InMemoryImport {
        InMemoryImport {
            payees: InMemoryPayees { payees },
            transactions: InMemoryTransactions { transactions },
        }
    }

    fn stored(&mut self, ports: InMemoryImport) -> Stored {
        Stored::new(ports.payees.payees, ports.transactions.transactions)
    }
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 28).expect("a September day")
}

/// The Category every unresolved row is filed under: the stub's Household leaf.
const HOUSEHOLD: CategoryId = CategoryId::seed(10);

/// The stub Payees and one Transaction already stored.
fn seed() -> (Vec<Payee>, Vec<Transaction>) {
    let groceries = Transaction {
        id: TransactionId::seed(1),
        date: today(),
        account_id: SeededStatement::STUB_ACCOUNT,
        status: TransactionStatus::Cleared,
        is_flagged: false,
        description: Some("Groceries".to_string()),
        splits: vec![Split {
            amount: cents(-4_250),
            category_id: CategoryId::seed(4),
            payee_id: Some(PayeeId::seed(1)),
            tag_ids: Vec::new(),
        }],
    };
    (lib_payees::default_payees(), vec![groceries])
}

/// The seeded statement matched against `payees`, every row resolved, as one plan.
fn plan(payees: &[Payee]) -> ImportPlan {
    let rows: Vec<ImportRow> = SeededStatement::stub(today())
        .statement()
        .lines
        .iter()
        .map(|line| {
            let row = match_line(payees, line);
            ImportRow {
                category: row.category.or(Some(HOUSEHOLD)),
                ..row
            }
        })
        .collect();
    ImportPlan::resolve(SeededStatement::STUB_ACCOUNT, &rows, true, payees)
        .expect("every row is resolved")
}

/// A commit stores its new Payees with their aliases and its Transactions with their Splits,
/// beside the rows already stored.
pub fn a_commit_lands_the_payees_their_aliases_and_the_transactions<F: ImportFactory>(
    factory: &mut F,
) {
    let (payees, transactions) = seed();
    let plan = plan(&payees);
    let expected = Stored::new(
        payees.iter().chain(&plan.payees).cloned().collect(),
        transactions
            .iter()
            .chain(&plan.transactions)
            .cloned()
            .collect(),
    );
    let mut ports = factory.create(payees, transactions);

    let committed = ports.commit(plan).expect("the plan is free to write");
    let stored = factory.stored(ports);

    assert_eq!(
        committed,
        ImportCommitted {
            transactions: 18,
            new_payees: 3
        }
    );
    assert_eq!(stored, expected);
    let candle = stored
        .payees
        .iter()
        .find(|payee| payee.name == "Aussie Candle Co")
        .expect("the new Payee is stored");
    assert_eq!(candle.aliases, ["AUSSIE CANDLE CO"]);
    let booked = stored
        .transactions
        .iter()
        .filter(|transaction| transaction.splits[0].payee_id == Some(candle.id))
        .count();
    assert_eq!(booked, 1, "its row's Transaction names it");
}

/// A plan the port refuses, a new Payee whose name is taken, stores none of its Payees or
/// Transactions, though the rest of it was free to write.
pub fn a_refused_plan_writes_nothing<F: ImportFactory>(factory: &mut F) {
    let (payees, transactions) = seed();
    let before = Stored::new(payees.clone(), transactions.clone());
    let mut plan = plan(&payees);
    let taken = payees[0].name.clone();
    plan.payees.push(Payee {
        id: PayeeId::new(),
        name: taken.clone(),
        aliases: Vec::new(),
        default_category: None,
        is_active: true,
    });
    let mut ports = factory.create(payees, transactions);

    let refused = ports.commit(plan);
    let stored = factory.stored(ports);

    assert_eq!(refused, Err(Error::PayeeNameTaken(taken)));
    assert_eq!(stored, before);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_lands_a_commit() {
        a_commit_lands_the_payees_their_aliases_and_the_transactions(&mut InMemoryFactory);
    }

    #[test]
    fn the_fake_lands_no_part_of_a_refused_plan() {
        a_refused_plan_writes_nothing(&mut InMemoryFactory);
    }
}
