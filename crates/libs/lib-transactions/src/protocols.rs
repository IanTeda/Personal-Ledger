//! The port the Transactions are stored through. It is synchronous (ADR-0046): an adapter serves
//! reads from memory and queues each write, so a write never fails here; a failed save is the
//! adapter's to report. `fake` implements it in memory.
//!
//! Every writer of Transactions is still today's code, which reaches the repository through the
//! Desktop's forwarding shim (#774); the Transactions page reads it, with the read ports, as one
//! [`TransactionsPorts`] value.
//!
//! The read ports name the other domains' rows as Transactions-owned values with only the fields
//! the query engine, rows, chips and filter form read, so this crate depends on no other feature
//! crate. `lib-database`'s adapters answer them from the rows they hold.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{AccountId, CategoryId, HexColor, PayeeId, TagId, TransactionId, UnitId};

use crate::Transaction;

/// The Transactions on the Ledger, each with its Splits.
pub trait TransactionRepository {
    /// Every Transaction, in no particular order.
    fn transactions(&self) -> Vec<Transaction>;

    fn transaction(&self, id: TransactionId) -> Option<Transaction> {
        self.transactions()
            .into_iter()
            .find(|transaction| transaction.id == id)
    }

    /// Writes `transaction` with its Splits, replacing the row with its id or adding it.
    fn upsert_transaction(&mut self, transaction: Transaction);

    /// Removes the Transaction and its Splits. A Transaction not stored is no error.
    fn delete_transaction(&mut self, id: TransactionId);
}

/// An Account as the Transactions read it: the Account filter's option, a row's Account name, and
/// the Unit its Transactions' totals are named in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionAccount {
    pub id: AccountId,
    pub name: String,
    pub unit: UnitId,
}

/// A Category as the Transactions read it. `parent` lets a parent's filter match its
/// descendants, and `path` (ancestors then `name`) is the Categories' rule, filled by the side that
/// answers [`TransactionCategories`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionCategory {
    pub id: CategoryId,
    pub name: String,
    pub parent: Option<CategoryId>,
    pub path: String,
}

/// A Payee as the Transactions read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionPayee {
    pub id: PayeeId,
    pub name: String,
}

/// A Tag as the Transactions read it: a row's Tag chip takes its colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionTag {
    pub id: TagId,
    pub name: String,
    pub color: Option<HexColor>,
}

/// A Unit as the Transactions read it: the code a single-Unit total is named in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionUnit {
    pub id: UnitId,
    pub code: String,
}

/// The Accounts a Transaction may be posted against: every Account, in the order they were added.
pub trait TransactionAccounts {
    fn transaction_accounts(&self) -> Vec<TransactionAccount>;
}

/// The Categories a Split may be filed under: every Category, in tree order (each parent before
/// its children, siblings in the order they were added).
pub trait TransactionCategories {
    fn transaction_categories(&self) -> Vec<TransactionCategory>;
}

/// The Payees a Split may name: every Payee, inactive ones included, as a Split may name one.
pub trait TransactionPayees {
    fn transaction_payees(&self) -> Vec<TransactionPayee>;
}

/// The Tags a Split may carry: every Tag, in the order they were added.
pub trait TransactionTags {
    fn transaction_tags(&self) -> Vec<TransactionTag>;
}

/// The Units the Transactions' totals are named in. A read port, so `lib-transactions` needn't
/// depend on the Units crate to write `aud`.
pub trait TransactionUnits {
    /// Unit `id`'s code, or `None` when it is gone.
    fn transaction_unit_code(&self, id: UnitId) -> Option<String>;
}

/// Every port the Transactions page reads, as one value: the Transactions themselves and the other
/// domains' rows they name.
pub trait TransactionsPorts:
    TransactionAccounts + TransactionCategories + TransactionPayees + TransactionTags + TransactionUnits
{
    /// Every Transaction, in no particular order.
    fn all_transactions(&self) -> Vec<Transaction>;
}

/// The ports as the page holds them, shared with the Client that keeps them in line.
/// Single-threaded, as both Clients' UI threads are.
pub type SharedPorts = Rc<RefCell<dyn TransactionsPorts>>;
