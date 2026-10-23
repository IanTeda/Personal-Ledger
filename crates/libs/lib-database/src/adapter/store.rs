//! What a Client holds each module's ports by: the SQLite adapter start-up opens
//! ([`crate::open_storage`]), or the in-memory ports a test builds the Client on. The bins'
//! `build_shell` take them bundled as [`Stores`], so either can stand behind the module. A store
//! only holds its rows: the bin commits every store's writes at once, through
//! [`Commit`](crate::Commit), once per input. [`Stores::fake`] builds the bundle on in-memory
//! stores with a [`CountingCommit`], so a test asserts that once.
//!
//! A cross-domain module writes two stores another module also holds, so the Transactions and
//! Bills stores can be shared ([`SharedTransactionsStore`], [`SharedBillsStore`]), as
//! [`BillPaymentsAdapter`](crate::BillPaymentsAdapter) holds them.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;

use lib_bills::fake::InMemoryBills;
use lib_bills::{BillPlanRepository, BillScheduleRepository};
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{Transaction, TransactionRepository};
use lib_units::fake::InMemoryPorts;
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRepository, UnitRepository, UnitUsage, UnitsPorts,
};

use crate::{
    BillsAdapter, Commit, InMemoryCommit, Result, Stores, TransactionsAdapter, UnitsAdapter,
};

/// The Units, Price Sources and base Unit ports, wherever they are kept. Usage and holdings are
/// other domains', so each Client answers them itself (its `units/ports.rs`).
pub trait UnitsStore {
    fn units(&self) -> &dyn UnitRepository;

    fn price_sources(&self) -> &dyn PriceSourceRepository;

    fn base_unit(&self) -> &dyn BaseUnitRepository;

    /// The three ports, with `usage`, for one Interactor call.
    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a>;

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl UnitsStore for UnitsAdapter {
    fn units(&self) -> &dyn UnitRepository {
        UnitsAdapter::units(self)
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        UnitsAdapter::price_sources(self)
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        UnitsAdapter::base_unit(self)
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        UnitsAdapter::split_mut(self, usage)
    }

    fn reload(&mut self) -> Result<()> {
        UnitsAdapter::reload(self)
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl UnitsStore for InMemoryPorts {
    fn units(&self) -> &dyn UnitRepository {
        UnitsPorts::units(self)
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        UnitsPorts::price_sources(self)
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        UnitsPorts::base_unit(self)
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage,
        }
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A Transactions store shared by the modules that write it. Single-threaded, as the ports are.
pub type SharedTransactionsStore = Rc<RefCell<dyn TransactionsStore>>;

/// A Bills store shared by the modules that write it. Single-threaded, as the ports are.
pub type SharedBillsStore = Rc<RefCell<dyn BillsStore>>;

/// The Transactions port, wherever it is kept.
pub trait TransactionsStore {
    fn transactions(&self) -> &dyn TransactionRepository;

    fn transactions_mut(&mut self) -> &mut dyn TransactionRepository;

    /// Every Transaction, so a caller needn't reach through the repository.
    fn all_transactions(&self) -> Vec<Transaction> {
        self.transactions().transactions()
    }

    /// Stores `transaction`, inserting or replacing it.
    fn upsert_transaction(&mut self, transaction: Transaction) {
        self.transactions_mut().upsert_transaction(transaction);
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl TransactionsStore for TransactionsAdapter {
    fn transactions(&self) -> &dyn TransactionRepository {
        self
    }

    fn transactions_mut(&mut self) -> &mut dyn TransactionRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        TransactionsAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedTransactionsStore`]).
impl<T: TransactionsStore + ?Sized> TransactionsStore for Box<T> {
    fn transactions(&self) -> &dyn TransactionRepository {
        (**self).transactions()
    }

    fn transactions_mut(&mut self) -> &mut dyn TransactionRepository {
        (**self).transactions_mut()
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl TransactionsStore for InMemoryTransactions {
    fn transactions(&self) -> &dyn TransactionRepository {
        self
    }

    fn transactions_mut(&mut self) -> &mut dyn TransactionRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// The Bill Plan and Bill Schedule ports, wherever they are kept.
pub trait BillsStore {
    fn plans(&self) -> &dyn BillPlanRepository;

    fn plans_mut(&mut self) -> &mut dyn BillPlanRepository;

    fn entries(&self) -> &dyn BillScheduleRepository;

    fn entries_mut(&mut self) -> &mut dyn BillScheduleRepository;

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl BillsStore for BillsAdapter {
    fn plans(&self) -> &dyn BillPlanRepository {
        self
    }

    fn plans_mut(&mut self) -> &mut dyn BillPlanRepository {
        self
    }

    fn entries(&self) -> &dyn BillScheduleRepository {
        self
    }

    fn entries_mut(&mut self) -> &mut dyn BillScheduleRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        BillsAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedBillsStore`]).
impl<T: BillsStore + ?Sized> BillsStore for Box<T> {
    fn plans(&self) -> &dyn BillPlanRepository {
        (**self).plans()
    }

    fn plans_mut(&mut self) -> &mut dyn BillPlanRepository {
        (**self).plans_mut()
    }

    fn entries(&self) -> &dyn BillScheduleRepository {
        (**self).entries()
    }

    fn entries_mut(&mut self) -> &mut dyn BillScheduleRepository {
        (**self).entries_mut()
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl BillsStore for InMemoryBills {
    fn plans(&self) -> &dyn BillPlanRepository {
        self
    }

    fn plans_mut(&mut self) -> &mut dyn BillPlanRepository {
        self
    }

    fn entries(&self) -> &dyn BillScheduleRepository {
        self
    }

    fn entries_mut(&mut self) -> &mut dyn BillScheduleRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A commit that counts its calls before handing each on, so a test asserts the Client commits
/// exactly once per input, whether or not the input wrote anything.
pub struct CountingCommit {
    inner: Rc<dyn Commit>,
    count: Rc<Cell<usize>>,
}

impl CountingCommit {
    /// Counts the commits made through `inner`, starting from none.
    pub fn new(inner: Rc<dyn Commit>) -> Self {
        Self {
            inner,
            count: Rc::default(),
        }
    }

    /// The count so far, shared so the test reads it after the commit has moved into the Client.
    pub fn count(&self) -> Rc<Cell<usize>> {
        self.count.clone()
    }
}

impl Commit for CountingCommit {
    fn commit(&self) {
        self.count.set(self.count.get() + 1);
        self.inner.commit();
    }
}

impl Stores {
    /// `units`, `transactions` and `bills` in memory, committed through a [`CountingCommit`] whose
    /// count is returned, on a failure channel nothing reports on, since there is no writer.
    pub fn fake(
        units: InMemoryPorts,
        transactions: InMemoryTransactions,
        bills: InMemoryBills,
    ) -> (Self, Rc<Cell<usize>>) {
        let commit = CountingCommit::new(Rc::new(InMemoryCommit));
        let count = commit.count();
        let stores = Self {
            units: Box::new(units),
            transactions: Box::new(transactions),
            bills: Box::new(bills),
            commit: Rc::new(commit),
            failures: mpsc::channel().1,
        };
        (stores, count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_counts_every_commit_including_one_with_nothing_written() {
        let (stores, count) = Stores::fake(
            InMemoryPorts::stub(chrono::Utc::now()),
            InMemoryTransactions::default(),
            InMemoryBills::default(),
        );

        stores.commit.commit();
        stores.commit.commit();

        assert_eq!(count.get(), 2);
    }
}
