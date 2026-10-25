//! What a Client holds each module's ports by: the SQLite adapter start-up opens
//! ([`crate::open_storage`]), or the in-memory ports a test builds the Client on. The bins'
//! `build_shell` take them bundled as [`Stores`], so either can stand behind the module. A store
//! only holds its rows: the bin commits every store's writes at once, through
//! [`Commit`](crate::Commit), once per input. [`Stores::fake`] builds the bundle on in-memory
//! stores with a [`CountingCommit`], so a test asserts that once.
//!
//! A cross-domain module writes two stores another module also holds, so the Transactions and
//! Bills stores can be shared ([`SharedTransactionsStore`], [`SharedBillsStore`]), as
//! [`BillPaymentsAdapter`](crate::BillPaymentsAdapter) holds them. The Payees store is shared too
//! ([`SharedPayeesStore`]): Bills reads its rows, and the Desktop's Ledger forwards the Payees
//! writers not yet migrated to it.
//!
//! [`Stores::with_units`] fills every store but the Units with an empty in-memory one, for a Client
//! test that drives only the Units: a domain joining the bundle then needs no edit there (#849).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;

use lib_bills::fake::InMemoryBills;
use lib_bills::{BillPayee, BillPlanRepository, BillScheduleRepository};
use lib_institutions::InstitutionRepository;
use lib_institutions::fake::InMemoryInstitutions;
use lib_payees::{PayeeRepository, PayeeUsage};
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{Transaction, TransactionRepository};
use lib_units::fake::InMemoryPorts;
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRepository, UnitRepository, UnitUsage, UnitsPorts,
};

use crate::adapter::payees::bill_payees;
use crate::{
    BillsAdapter, Commit, InMemoryCommit, InstitutionsAdapter, PayeesAdapter, Result, Stores,
    TransactionsAdapter, UnitsAdapter, WriteFailure,
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

/// The Institutions repository port, wherever it is kept.
pub trait InstitutionsStore {
    fn institutions(&self) -> &dyn InstitutionRepository;

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository;

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl InstitutionsStore for InstitutionsAdapter {
    fn institutions(&self) -> &dyn InstitutionRepository {
        self
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        InstitutionsAdapter::reload(self)
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl InstitutionsStore for InMemoryInstitutions {
    fn institutions(&self) -> &dyn InstitutionRepository {
        self
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        self
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

/// A Payees store shared by the Ledger's forwarding shim and the Bills reads. Single-threaded, as
/// the ports are.
pub type SharedPayeesStore = Rc<RefCell<dyn PayeesStore>>;

/// The Payees repository and usage ports, wherever they are kept, and the rows as Bills reads
/// them.
pub trait PayeesStore {
    fn payees(&self) -> &dyn PayeeRepository;

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository;

    fn usage(&self) -> &dyn PayeeUsage;

    /// The repository and usage, for one write.
    fn split_mut(&mut self) -> lib_payees::PortsMut<'_>;

    /// The usage a store holds in memory rather than counts, for its holder to recount from the
    /// Splits and Bill Plans it can see before each ask (#841). `None` for the adapter, which
    /// counts in the file on each ask.
    fn usage_to_count(&mut self) -> Option<&mut lib_payees::fake::InMemoryPayeeUsage> {
        None
    }

    /// Every Payee as `lib-bills`' `BillPayees` reads it, inactive ones included.
    fn bill_payees(&self) -> Vec<BillPayee> {
        bill_payees(&self.payees().payees())
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl PayeesStore for PayeesAdapter {
    fn payees(&self) -> &dyn PayeeRepository {
        PayeesAdapter::payees(self)
    }

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        PayeesAdapter::payees_mut(self)
    }

    fn usage(&self) -> &dyn PayeeUsage {
        PayeesAdapter::usage(self)
    }

    fn split_mut(&mut self) -> lib_payees::PortsMut<'_> {
        PayeesAdapter::split_mut(self)
    }

    fn bill_payees(&self) -> Vec<BillPayee> {
        lib_bills::BillPayees::bill_payees(self)
    }

    fn reload(&mut self) -> Result<()> {
        PayeesAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedPayeesStore`]).
impl<T: PayeesStore + ?Sized> PayeesStore for Box<T> {
    fn payees(&self) -> &dyn PayeeRepository {
        (**self).payees()
    }

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        (**self).payees_mut()
    }

    fn usage(&self) -> &dyn PayeeUsage {
        (**self).usage()
    }

    fn split_mut(&mut self) -> lib_payees::PortsMut<'_> {
        (**self).split_mut()
    }

    fn usage_to_count(&mut self) -> Option<&mut lib_payees::fake::InMemoryPayeeUsage> {
        (**self).usage_to_count()
    }

    fn bill_payees(&self) -> Vec<BillPayee> {
        (**self).bill_payees()
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read, and the usage is whatever its holder
/// last counted.
impl PayeesStore for lib_payees::fake::InMemoryPorts {
    fn payees(&self) -> &dyn PayeeRepository {
        &self.payees
    }

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        &mut self.payees
    }

    fn usage(&self) -> &dyn PayeeUsage {
        &self.usage
    }

    fn split_mut(&mut self) -> lib_payees::PortsMut<'_> {
        lib_payees::fake::InMemoryPorts::split_mut(self)
    }

    fn usage_to_count(&mut self) -> Option<&mut lib_payees::fake::InMemoryPayeeUsage> {
        Some(&mut self.usage)
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
    /// `units`, `institutions`, `payees`, `transactions` and `bills` in memory, committed through
    /// a [`CountingCommit`] whose count is returned, on a failure channel nothing reports on, since
    /// there is no writer.
    pub fn fake(
        units: InMemoryPorts,
        institutions: InMemoryInstitutions,
        payees: lib_payees::fake::InMemoryPorts,
        transactions: InMemoryTransactions,
        bills: InMemoryBills,
    ) -> (Self, Rc<Cell<usize>>) {
        let commit = CountingCommit::new(Rc::new(InMemoryCommit));
        let count = commit.count();
        let stores = Self {
            units: Box::new(units),
            institutions: Box::new(institutions),
            payees: Box::new(payees),
            transactions: Box::new(transactions),
            bills: Box::new(bills),
            commit: Rc::new(commit),
            failures: mpsc::channel().1,
        };
        (stores, count)
    }

    /// `units` committed through `commit`, failures reported on `failures`, and every other store
    /// empty in memory. For a Client test that drives only the Units, which then needs no edit as
    /// a domain joins the bundle (the TUI's, #849).
    pub fn with_units(
        units: Box<dyn UnitsStore>,
        commit: Rc<dyn Commit>,
        failures: mpsc::Receiver<WriteFailure>,
    ) -> Self {
        Self {
            units,
            institutions: Box::new(InMemoryInstitutions::default()),
            payees: Box::new(lib_payees::fake::InMemoryPorts::default()),
            transactions: Box::new(InMemoryTransactions::default()),
            bills: Box::new(InMemoryBills::default()),
            commit,
            failures,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_counts_every_commit_including_one_with_nothing_written() {
        let (stores, count) = Stores::fake(
            InMemoryPorts::stub(chrono::Utc::now()),
            InMemoryInstitutions::stub(),
            lib_payees::fake::InMemoryPorts::stub(),
            InMemoryTransactions::default(),
            InMemoryBills::default(),
        );

        stores.commit.commit();
        stores.commit.commit();

        assert_eq!(count.get(), 2);
    }
}
