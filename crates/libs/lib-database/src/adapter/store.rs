//! What a Client holds each module's ports by: the SQLite adapter start-up opens
//! ([`crate::open_storage`]), or the in-memory ports a test builds the Client on. The bins'
//! `build_shell` take them bundled as [`Stores`], so either can stand behind the module. A store
//! only holds its rows: the bin commits every store's writes at once, through
//! [`Commit`](crate::Commit), once per input. [`Stores::fake`] builds the bundle on in-memory
//! stores with a [`CountingCommit`], so a test asserts that once.
//!
//! The Accounts store is shared as well ([`SharedAccountsStore`]): the Desktop's Ledger forwards
//! the Accounts writers to it, and the Units and Bills read its rows (#853). So is the Categories
//! store ([`SharedCategoriesStore`]): the Desktop's Ledger forwards the Categories writers to it,
//! and the Bills and Payees read its rows (#858). So is the Tags store ([`SharedTagsStore`]): the
//! Desktop's Ledger forwards the Tags writers to it, and the Tags page and Dialogs read its
//! usage (#862). So is the Inventory store ([`SharedInventoryStore`]): the Desktop's Ledger
//! forwards the Inventory writers to it and keeps a read copy of it for Documents (#866).
//!
//! A cross-domain module writes two stores another module also holds, so the Transactions and
//! Bills stores can be shared ([`SharedTransactionsStore`], [`SharedBillsStore`]), as
//! [`BillPaymentsAdapter`](crate::BillPaymentsAdapter) holds them. The Payees store is shared too
//! ([`SharedPayeesStore`]): Bills reads its rows, the Desktop's Ledger forwards the Payees writers
//! not yet migrated to it, and [`TransactionImportAdapter`](crate::TransactionImportAdapter)
//! writes it beside the Transactions store. A cross-domain adapter holds only the shared stores,
//! so over [`Stores::fake`]'s in-memory stores it is its own in-memory counterpart.
//!
//! [`Stores::fake`] takes its in-memory stores as one [`FakeStores`], so a domain joining the
//! bundle adds a field rather than an argument (#866).
//!
//! [`Stores::with_units`] fills every store but the Units with an empty in-memory one, for a Client
//! test that drives only the Units: a domain joining the bundle then needs no edit there (#849).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;

use chrono::NaiveDate;
use lib_accounts::{AccountRepository, AccountUsage};
use lib_bills::fake::InMemoryBills;
use lib_bills::{
    BillAccount, BillCategory, BillPayee, BillPlanRepository, BillScheduleRepository, BillSplit,
    BillTransactions, SplitRef,
};
use lib_categories::CategoryRepository;
use lib_categories::fake::InMemoryCategories;
use lib_core::{AccountId, CategoryId, InstitutionId, LocationId, TagId, UnitId};
use lib_institutions::fake::InMemoryInstitutions;
use lib_institutions::{InstitutionRepository, InstitutionUse};
use lib_inventory::InventoryRepository;
use lib_inventory::fake::InMemoryInventory;
use lib_payees::{PayeeCategory, PayeeRepository, PayeeUsage};
use lib_tags::fake::InMemoryTagUsage;
use lib_tags::{TagRepository, TagUsage};
use lib_transaction_import::ImportAccount;
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{Transaction, TransactionRepository};
use lib_units::fake::InMemoryPorts;
use lib_units::{
    BaseUnitRepository, Holding, PortsMut, PriceSourceRepository, UnitRepository, UnitUsage,
    UnitsPorts,
};

use crate::adapter::payees::bill_payees;
use crate::adapter::{accounts, categories, transactions};
use crate::{
    AccountsAdapter, BillsAdapter, CategoriesAdapter, Commit, InMemoryCommit, InstitutionsAdapter,
    InventoryAdapter, PayeesAdapter, Result, Stores, TagsAdapter, TransactionsAdapter,
    UnitsAdapter,
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

/// A Units store shared by the Units module and the Unit Deletion adapter, which writes it with the
/// Institutions store in one delete (#902). Single-threaded, as the ports are.
pub type SharedUnitsStore = Rc<RefCell<dyn UnitsStore>>;

/// An Institutions store shared by the Institutions module and the Unit Deletion adapter (#902).
pub type SharedInstitutionsStore = Rc<RefCell<dyn InstitutionsStore>>;

/// A boxed store is the store, so a bin given one can share it ([`SharedUnitsStore`]).
impl<T: UnitsStore + ?Sized> UnitsStore for Box<T> {
    fn units(&self) -> &dyn UnitRepository {
        (**self).units()
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        (**self).price_sources()
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        (**self).base_unit()
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        (**self).split_mut(usage)
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedInstitutionsStore`]).
impl<T: InstitutionsStore + ?Sized> InstitutionsStore for Box<T> {
    fn institutions(&self) -> &dyn InstitutionRepository {
        (**self).institutions()
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        (**self).institutions_mut()
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// An Accounts store shared by the Ledger's forwarding shim and the Units and Bills reads.
/// Single-threaded, as the ports are.
pub type SharedAccountsStore = Rc<RefCell<dyn AccountsStore>>;

/// The Accounts repository and usage ports, wherever they are kept, and the rows as the Units and
/// Bills read them. The reads are the store's to answer, since the orphan rule stops
/// `lib-database` implementing `lib-units`' and `lib-bills`' ports for `lib-accounts`' fake.
pub trait AccountsStore {
    fn accounts(&self) -> &dyn AccountRepository;

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository;

    /// The Transactions booked to each Account. Its Budgets count is the holder's to add from the
    /// `Ledger` until Budgets persist (#811).
    fn usage(&self) -> &dyn AccountUsage;

    /// The usage a store holds in memory rather than counts, for its holder to recount from the
    /// Transactions it can see before each ask. `None` for the adapter, which counts in the file
    /// on each ask.
    fn usage_to_count(&mut self) -> Option<&mut lib_accounts::fake::InMemoryAccountUsage> {
        None
    }

    /// The Accounts whose Unit is `unit`, in order, as `lib-units`' `UnitHoldings` reads them.
    fn holdings(&self, unit: UnitId) -> Vec<Holding> {
        let usage = self.usage();
        accounts::holdings(&self.accounts().accounts(), unit, |id| {
            usage.account_usage(id).transactions
        })
    }

    /// How many Accounts count in `unit`: the Accounts half of `lib-units`' `UnitUsage`.
    fn unit_accounts(&self, unit: UnitId) -> usize {
        accounts::unit_accounts(&self.accounts().accounts(), unit)
    }

    /// Every Account as `lib-bills`' `BillAccounts` reads it.
    fn bill_accounts(&self) -> Vec<BillAccount> {
        accounts::bill_accounts(&self.accounts().accounts())
    }

    /// Account `id` as `lib-transaction-import`'s `ImportAccounts` reads it.
    fn import_account(&self, id: AccountId) -> Option<ImportAccount> {
        accounts::import_account(&self.accounts().accounts(), id)
    }

    /// The Accounts naming `institution`: `lib-institutions`' `InstitutionUsage`. The one place this is
    /// worked out: the adapter's `InstitutionUsage` impl, and every other store, answer through it.
    fn institution_usage(&self, institution: InstitutionId) -> InstitutionUse {
        InstitutionUse {
            accounts: accounts::institution_accounts(&self.accounts().accounts(), institution),
        }
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl AccountsStore for AccountsAdapter {
    fn accounts(&self) -> &dyn AccountRepository {
        AccountsAdapter::accounts(self)
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        AccountsAdapter::accounts_mut(self)
    }

    fn usage(&self) -> &dyn AccountUsage {
        AccountsAdapter::usage(self)
    }

    /// One count of the file for every Account, rather than one per Account.
    fn holdings(&self, unit: UnitId) -> Vec<Holding> {
        lib_units::UnitHoldings::holdings(self, unit)
    }

    fn unit_accounts(&self, unit: UnitId) -> usize {
        UnitUsage::unit_usage(self, unit).accounts
    }

    fn bill_accounts(&self) -> Vec<BillAccount> {
        lib_bills::BillAccounts::bill_accounts(self)
    }

    fn import_account(&self, id: AccountId) -> Option<ImportAccount> {
        lib_transaction_import::ImportAccounts::import_account(self, id)
    }

    fn reload(&mut self) -> Result<()> {
        AccountsAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedAccountsStore`]).
impl<T: AccountsStore + ?Sized> AccountsStore for Box<T> {
    fn accounts(&self) -> &dyn AccountRepository {
        (**self).accounts()
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        (**self).accounts_mut()
    }

    fn usage(&self) -> &dyn AccountUsage {
        (**self).usage()
    }

    fn usage_to_count(&mut self) -> Option<&mut lib_accounts::fake::InMemoryAccountUsage> {
        (**self).usage_to_count()
    }

    fn holdings(&self, unit: UnitId) -> Vec<Holding> {
        (**self).holdings(unit)
    }

    fn unit_accounts(&self, unit: UnitId) -> usize {
        (**self).unit_accounts(unit)
    }

    fn bill_accounts(&self) -> Vec<BillAccount> {
        (**self).bill_accounts()
    }

    fn import_account(&self, id: AccountId) -> Option<ImportAccount> {
        (**self).import_account(id)
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read, and the usage is whatever its holder
/// last counted.
impl AccountsStore for lib_accounts::fake::InMemoryPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        &self.accounts
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    fn usage(&self) -> &dyn AccountUsage {
        &self.usage
    }

    fn usage_to_count(&mut self) -> Option<&mut lib_accounts::fake::InMemoryAccountUsage> {
        Some(&mut self.usage)
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A Categories store shared by the Ledger's forwarding shim and the Bills and Payees reads.
/// Single-threaded, as the ports are.
pub type SharedCategoriesStore = Rc<RefCell<dyn CategoriesStore>>;

/// The Categories repository port, wherever it is kept, the delete Category reassign sends until
/// `lib-reassign` (#813), and the rows as the Bills and Payees read them. The reads are the
/// store's to answer, since the orphan rule stops `lib-database` implementing `lib-bills`' and
/// `lib-payees`' ports for `lib-categories`' fake.
pub trait CategoriesStore {
    fn categories(&self) -> &dyn CategoryRepository;

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository;

    /// Removes Category `id`, once nothing names it: Category reassign's delete, which the
    /// Desktop's forwarding shim sends. Not a port write, since the delete re-points other
    /// domains' rows (#813).
    fn delete_category(&mut self, id: CategoryId);

    /// Every Category as `lib-bills`' `BillCategories` reads it, in tree order.
    fn bill_categories(&self) -> Vec<BillCategory> {
        categories::bill_categories(&self.categories().categories())
    }

    /// The leaves as `lib-payees`' `PayeeCategories` reads them, in tree order.
    fn payee_categories(&self) -> Vec<PayeeCategory> {
        categories::payee_categories(&self.categories().categories())
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl CategoriesStore for CategoriesAdapter {
    fn categories(&self) -> &dyn CategoryRepository {
        self
    }

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository {
        self
    }

    fn delete_category(&mut self, id: CategoryId) {
        CategoriesAdapter::delete_category(self, id);
    }

    fn reload(&mut self) -> Result<()> {
        CategoriesAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedCategoriesStore`]).
impl<T: CategoriesStore + ?Sized> CategoriesStore for Box<T> {
    fn categories(&self) -> &dyn CategoryRepository {
        (**self).categories()
    }

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository {
        (**self).categories_mut()
    }

    fn delete_category(&mut self, id: CategoryId) {
        (**self).delete_category(id);
    }

    fn bill_categories(&self) -> Vec<BillCategory> {
        (**self).bill_categories()
    }

    fn payee_categories(&self) -> Vec<PayeeCategory> {
        (**self).payee_categories()
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl CategoriesStore for InMemoryCategories {
    fn categories(&self) -> &dyn CategoryRepository {
        self
    }

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository {
        self
    }

    fn delete_category(&mut self, id: CategoryId) {
        self.categories.retain(|category| category.id != id);
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A Tags store shared by the Ledger's forwarding shim and the Tags page. Single-threaded, as the
/// ports are.
pub type SharedTagsStore = Rc<RefCell<dyn TagsStore>>;

/// The Tags repository and usage ports, wherever they are kept, and the delete Tag delete and
/// merge send until `lib-reassign` (#813).
pub trait TagsStore {
    fn tags(&self) -> &dyn TagRepository;

    fn tags_mut(&mut self) -> &mut dyn TagRepository;

    fn usage(&self) -> &dyn TagUsage;

    /// Removes Tag `id`, once no Split carries it: Tag delete's and merge's delete, which the
    /// Desktop's forwarding shim sends. Not a port write, since the delete untags other
    /// domains' rows (#813).
    fn delete_tag(&mut self, id: TagId);

    /// The in-memory usage a test or the stub counts into, if the store keeps its usage in
    /// memory: `None` for an adapter, which counts in the file.
    fn usage_to_count(&mut self) -> Option<&mut InMemoryTagUsage> {
        None
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl TagsStore for TagsAdapter {
    fn tags(&self) -> &dyn TagRepository {
        TagsAdapter::tags(self)
    }

    fn tags_mut(&mut self) -> &mut dyn TagRepository {
        TagsAdapter::tags_mut(self)
    }

    fn usage(&self) -> &dyn TagUsage {
        TagsAdapter::usage(self)
    }

    fn delete_tag(&mut self, id: TagId) {
        TagsAdapter::delete_tag(self, id);
    }

    fn reload(&mut self) -> Result<()> {
        TagsAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedTagsStore`]).
impl<T: TagsStore + ?Sized> TagsStore for Box<T> {
    fn tags(&self) -> &dyn TagRepository {
        (**self).tags()
    }

    fn tags_mut(&mut self) -> &mut dyn TagRepository {
        (**self).tags_mut()
    }

    fn usage(&self) -> &dyn TagUsage {
        (**self).usage()
    }

    fn delete_tag(&mut self, id: TagId) {
        (**self).delete_tag(id);
    }

    fn usage_to_count(&mut self) -> Option<&mut InMemoryTagUsage> {
        (**self).usage_to_count()
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl TagsStore for lib_tags::fake::InMemoryPorts {
    fn tags(&self) -> &dyn TagRepository {
        &self.tags
    }

    fn tags_mut(&mut self) -> &mut dyn TagRepository {
        &mut self.tags
    }

    fn usage(&self) -> &dyn TagUsage {
        &self.usage
    }

    fn delete_tag(&mut self, id: TagId) {
        self.tags.tags.retain(|tag| tag.id != id);
    }

    fn usage_to_count(&mut self) -> Option<&mut InMemoryTagUsage> {
        Some(&mut self.usage)
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// An Inventory store shared by the Ledger's forwarding shim and read copy. Single-threaded, as
/// the ports are.
pub type SharedInventoryStore = Rc<RefCell<dyn InventoryStore>>;

/// The Inventory repository port, wherever it is kept, and the delete Remove Location sends until
/// `lib-reassign` (#813).
pub trait InventoryStore {
    fn inventory(&self) -> &dyn InventoryRepository;

    fn inventory_mut(&mut self) -> &mut dyn InventoryRepository;

    /// Removes Location `id` with its Areas and Items: Remove Location's delete, which the
    /// Desktop's forwarding shim sends. Not a port write, since the delete also drops the
    /// Document Links to its Items (Q18, #813).
    fn delete_location(&mut self, id: LocationId);

    /// How many Locations are priced in `unit`: the Locations half of `lib-units`' `UnitUsage`.
    /// The file refuses deleting a Unit one names, so Delete Unit must see them first (#885).
    fn unit_locations(&self, unit: UnitId) -> usize {
        self.inventory()
            .locations()
            .iter()
            .filter(|location| location.unit == unit)
            .count()
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl InventoryStore for InventoryAdapter {
    fn inventory(&self) -> &dyn InventoryRepository {
        InventoryAdapter::inventory(self)
    }

    fn inventory_mut(&mut self) -> &mut dyn InventoryRepository {
        InventoryAdapter::inventory_mut(self)
    }

    fn delete_location(&mut self, id: LocationId) {
        InventoryAdapter::delete_location(self, id);
    }

    fn reload(&mut self) -> Result<()> {
        InventoryAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedInventoryStore`]).
impl<T: InventoryStore + ?Sized> InventoryStore for Box<T> {
    fn inventory(&self) -> &dyn InventoryRepository {
        (**self).inventory()
    }

    fn inventory_mut(&mut self) -> &mut dyn InventoryRepository {
        (**self).inventory_mut()
    }

    fn delete_location(&mut self, id: LocationId) {
        (**self).delete_location(id);
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl InventoryStore for InMemoryInventory {
    fn inventory(&self) -> &dyn InventoryRepository {
        self
    }

    fn inventory_mut(&mut self) -> &mut dyn InventoryRepository {
        self
    }

    fn delete_location(&mut self, id: LocationId) {
        self.locations.retain(|location| location.id != id);
        self.items.retain(|item| item.location != id);
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

    /// The Splits dated `from` to `to`: `lib-bills`' `BillTransactions::splits_between`.
    fn bill_splits_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<BillSplit> {
        transactions::splits_between(&self.all_transactions(), from, to)
    }

    /// The Split `split` names: `lib-bills`' `BillTransactions::paid_split`.
    fn bill_paid_split(&self, split: SplitRef) -> Option<BillSplit> {
        transactions::paid_split(&self.all_transactions(), split)
    }

    /// The Splits filed under Category `id`: the Splits half of `lib-categories`' `CategoryUsage`.
    /// An in-memory store counts its own rows; the adapter counts in the file.
    fn category_splits(&self, id: CategoryId) -> usize {
        transactions::category_splits(&self.all_transactions(), id)
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

    fn bill_splits_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<BillSplit> {
        BillTransactions::splits_between(self, from, to)
    }

    fn bill_paid_split(&self, split: SplitRef) -> Option<BillSplit> {
        BillTransactions::paid_split(self, split)
    }

    fn category_splits(&self, id: CategoryId) -> usize {
        TransactionsAdapter::category_splits(self, id)
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

    fn bill_splits_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<BillSplit> {
        (**self).bill_splits_between(from, to)
    }

    fn bill_paid_split(&self, split: SplitRef) -> Option<BillSplit> {
        (**self).bill_paid_split(split)
    }

    fn category_splits(&self, id: CategoryId) -> usize {
        (**self).category_splits(id)
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read.
/// Reads its rows in place, so Bills' Schedule, which asks once per entry, copies none of them.
impl TransactionsStore for InMemoryTransactions {
    fn transactions(&self) -> &dyn TransactionRepository {
        self
    }

    fn transactions_mut(&mut self) -> &mut dyn TransactionRepository {
        self
    }

    fn bill_splits_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<BillSplit> {
        transactions::splits_between(&self.transactions, from, to)
    }

    fn bill_paid_split(&self, split: SplitRef) -> Option<BillSplit> {
        transactions::paid_split(&self.transactions, split)
    }

    fn category_splits(&self, id: CategoryId) -> usize {
        transactions::category_splits(&self.transactions, id)
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

/// The in-memory stores [`Stores::fake`] bundles, one per persisted domain.
pub struct FakeStores {
    pub units: InMemoryPorts,
    pub institutions: InMemoryInstitutions,
    pub accounts: lib_accounts::fake::InMemoryPorts,
    pub categories: InMemoryCategories,
    pub tags: lib_tags::fake::InMemoryPorts,
    pub payees: lib_payees::fake::InMemoryPorts,
    pub transactions: InMemoryTransactions,
    pub bills: InMemoryBills,
    pub inventory: InMemoryInventory,
}

impl FakeStores {
    /// Every domain's stub seed as of `today`, with the stub Ledger's Transactions and each
    /// Account's usage counted from it. For a Client that runs on the stub without a Ledger file.
    pub fn stub(today: NaiveDate) -> Self {
        let ledger = lib_ledger::Ledger::stub_on(today);
        let mut accounts = lib_accounts::fake::InMemoryPorts::stub();
        accounts.usage.uses = ledger
            .accounts()
            .iter()
            .map(|account| (account.id, ledger.account_use(account.id)))
            .collect();
        Self {
            units: InMemoryPorts::stub(chrono::Utc::now()),
            institutions: InMemoryInstitutions::stub(),
            accounts,
            categories: InMemoryCategories::stub(),
            tags: lib_tags::fake::InMemoryPorts::stub(),
            payees: lib_payees::fake::InMemoryPorts::stub(),
            transactions: InMemoryTransactions {
                transactions: ledger.transactions().to_vec(),
            },
            bills: lib_bills::fake::InMemoryPorts::stub(today).bills,
            inventory: InMemoryInventory::stub(today),
        }
    }
}

impl Stores {
    /// `fakes` in memory, committed through a [`CountingCommit`] whose count is returned, on a
    /// failure channel nothing reports on, since there is no writer.
    pub fn fake(fakes: FakeStores) -> (Self, Rc<Cell<usize>>) {
        let FakeStores {
            units,
            institutions,
            accounts,
            categories,
            tags,
            payees,
            transactions,
            bills,
            inventory,
        } = fakes;
        let commit = CountingCommit::new(Rc::new(InMemoryCommit));
        let count = commit.count();
        let stores = Self {
            units: Box::new(units),
            institutions: Box::new(institutions),
            accounts: Box::new(accounts),
            categories: Box::new(categories),
            tags: Box::new(tags),
            payees: Box::new(payees),
            transactions: Box::new(transactions),
            bills: Box::new(bills),
            inventory: Box::new(inventory),
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
        let (stores, count) = Stores::fake(FakeStores {
            units: InMemoryPorts::stub(chrono::Utc::now()),
            institutions: InMemoryInstitutions::stub(),
            accounts: lib_accounts::fake::InMemoryPorts::stub(),
            categories: InMemoryCategories::stub(),
            tags: lib_tags::fake::InMemoryPorts::stub(),
            payees: lib_payees::fake::InMemoryPorts::stub(),
            transactions: InMemoryTransactions::default(),
            bills: InMemoryBills::default(),
            inventory: InMemoryInventory::default(),
        });

        stores.commit.commit();
        stores.commit.commit();

        assert_eq!(count.get(), 2);
    }
}
