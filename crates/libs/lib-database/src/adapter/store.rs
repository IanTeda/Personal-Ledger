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
//! The Documents store is shared too ([`SharedDocumentsStore`]): the Desktop's Documents module
//! writes it through `lib-documents`' Interactors and reads the other domains through
//! [`StoredDocumentReads`] (#963, #968).
//!
//! A module reads another domain's rows through one stored reads struct ([`StoredPayeeReads`],
//! [`StoredDocumentReads`], [`StoredBudgetReads`], [`StoredDashboardReads`]): it holds the shared stores of the domains it
//! reads and answers the module's read ports from them, so the same struct stands over the
//! adapters and over [`Stores::fake`]'s in-memory stores. Payees' is the pattern (#992): each
//! store answers about its own rows in its own terms (`payee_splits`, `payee_bill_plans`), and
//! the reads struct maps them into the reading module's. A stored reads struct composes its
//! domains' store methods; it never runs SQL across two domains' tables in one job. An atomic
//! write across domains is a cross-domain adapter's job (`BillPaymentsAdapter` and the like), not
//! a read's.
//!
//! [`Stores::fake`] takes its in-memory stores as one [`FakeStores`], so a domain joining the
//! bundle adds a field rather than an argument (#866).
//!
//! [`Stores::with_units`] fills every store but the Units with an empty in-memory one, for a Client
//! test that drives only the Units: a domain joining the bundle then needs no edit there (#849).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;

use chrono::NaiveDate;
use lib_account_deletion::DeletionReads;
use lib_accounts::{
    AccountInstitution, AccountInstitutions, AccountRepository, AccountUnit, AccountUnits,
    AccountUsage, AccountUse,
};
use lib_bills::fake::InMemoryBills;
use lib_bills::{
    BillAccount, BillAccounts, BillCategories, BillCategory, BillPayee, BillPayees,
    BillPlanRepository, BillScheduleRepository, BillSplit, BillTransactions, BillUnits, SplitRef,
};
use lib_budgets::fake::InMemoryBudgets;
use lib_budgets::{
    BudgetAccount, BudgetAccounts, BudgetBill, BudgetBillPlan, BudgetBills, BudgetCategories,
    BudgetCategory, BudgetPayees, BudgetRepository, BudgetSplit, BudgetSplits, BudgetUnits,
    Budgets,
};
use lib_categories::CategoryRepository;
use lib_categories::fake::InMemoryCategories;
use lib_category_deletion::{CategoryDeletionReads, CategoryInUse};
use lib_core::{
    AccountId, CategoryId, InstitutionId, LocationId, PayeeId, TagId, Total, TransactionId, UnitId,
    UnitKind,
};
use lib_dashboard::{
    DashboardBill, DashboardBills, DashboardBudgetBar, DashboardBudgetMonth, DashboardBudgets,
    DashboardElapsed, DashboardEntryId,
};
use lib_documents::fake::InMemoryDocuments;
use lib_documents::{
    DocumentAccount, DocumentAccounts, DocumentBillPlan, DocumentBillPlans, DocumentItem,
    DocumentItems, DocumentPayee, DocumentPayees, DocumentRepository, DocumentTransaction,
    DocumentTransactions,
};
use lib_institutions::fake::InMemoryInstitutions;
use lib_institutions::{
    InstitutionRepository, InstitutionUnit, InstitutionUnits, InstitutionUsage, InstitutionUse,
};
use lib_inventory::fake::InMemoryInventory;
use lib_inventory::{InventoryRepository, InventoryUnits, LocationUsage, LocationUse, UnitChoice};
use lib_payees::{PayeeCategories, PayeeCategory, PayeeRepository, PayeeUsage, PayeeUse};
use lib_shell::{RailCounts, ShellRailCounts};
use lib_tag_removal::{TagCount, TagRemovalReads};
use lib_tags::{TagRepository, TagUsage, TagUse};
use lib_transaction_import::{ImportAccount, ImportAccounts, ImportCategories};
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{
    Transaction, TransactionAccount, TransactionAccounts, TransactionCategories,
    TransactionCategory, TransactionPayee, TransactionPayees, TransactionRepository,
    TransactionTag, TransactionTags, TransactionUnits,
};
use lib_units::fake::InMemoryPorts;
use lib_units::{
    BaseUnitRepository, Holding, PortsMut, PriceSourceRepository, UnitHoldings, UnitRepository,
    UnitUsage, UnitUse, UnitsPorts,
};

use crate::adapter::payees::bill_payees;
use crate::adapter::transactions::TaggedSplit;
use crate::adapter::{
    accounts, bills, categories, institutions, inventory, payees, tags, transactions, units,
};
use crate::{
    AccountsAdapter, BillsAdapter, BudgetsAdapter, CategoriesAdapter, Commit, DocumentsAdapter,
    Domain, InMemoryCommit, InstitutionsAdapter, InventoryAdapter, PayeesAdapter, Result, Stores,
    TagsAdapter, TransactionsAdapter, UnitsAdapter,
};

/// The Units, Price Sources and base Unit ports, wherever they are kept. Usage and holdings are
/// other domains', so each Client answers them itself (its `units/ports.rs`).
pub trait UnitsStore {
    fn units(&self) -> &dyn UnitRepository;

    fn price_sources(&self) -> &dyn PriceSourceRepository;

    fn base_unit(&self) -> &dyn BaseUnitRepository;

    /// The three ports, with `usage`, for one Interactor call.
    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a>;

    /// Unit `id`'s code, as `lib-budgets`' `BudgetUnits` reads it.
    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.units()
            .units()
            .into_iter()
            .find(|unit| unit.id == id)
            .map(|unit| unit.code)
    }

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

/// An Accounts store shared by the Accounts module, the cross-domain adapters that write it and the
/// Units and Bills reads. Single-threaded, as the ports are.
pub type SharedAccountsStore = Rc<RefCell<dyn AccountsStore>>;

/// The Accounts repository port, wherever it is kept. What names an Account is other domains'
/// rows, which [`StoredAccountReads`] counts in their stores (#993); the reading modules map the
/// Accounts' own rows themselves (#994).
pub trait AccountsStore {
    fn accounts(&self) -> &dyn AccountRepository;

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository;

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

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read. Only the Accounts are the store's: its
/// usage is `lib-accounts`' own test fake, which a Client reads through [`StoredAccountReads`]
/// instead.
impl AccountsStore for lib_accounts::fake::InMemoryPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        &self.accounts
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A Categories store shared by the Categories module, the Category Deletion adapter and the Bills
/// and Payees reads. Single-threaded, as the ports are.
pub type SharedCategoriesStore = Rc<RefCell<dyn CategoriesStore>>;

/// The Categories repository port, wherever it is kept, and the delete Category delete sends. The
/// reading modules map the Categories' own rows themselves (#994).
pub trait CategoriesStore {
    fn categories(&self) -> &dyn CategoryRepository;

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository;

    /// Removes Category `id`, once nothing names it: Category delete's delete, which the
    /// [`CategoryDeletionAdapter`](crate::CategoryDeletionAdapter) sends after the rows it
    /// re-points. Not a `lib-categories` port write, since the delete re-points other domains'
    /// rows (#1004).
    fn delete_category(&mut self, id: CategoryId);

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

/// A Tags store shared by the Tags page and the Tag Removal adapter. Single-threaded, as the ports
/// are.
pub type SharedTagsStore = Rc<RefCell<dyn TagsStore>>;

/// The Tags repository port, wherever it is kept, and the delete Tag remove and merge send. What
/// carries a Tag is other domains' rows, which [`StoredTagReads`] counts in their stores.
pub trait TagsStore {
    fn tags(&self) -> &dyn TagRepository;

    fn tags_mut(&mut self) -> &mut dyn TagRepository;

    /// Removes Tag `id`, once no Split carries it: Tag remove's and merge's delete, which the
    /// [`TagRemovalAdapter`](crate::TagRemovalAdapter) sends after the Splits. Not a `lib-tags`
    /// port write, since the delete untags other domains' rows.
    fn delete_tag(&mut self, id: TagId);

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

    fn delete_tag(&mut self, id: TagId) {
        (**self).delete_tag(id);
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

    fn delete_tag(&mut self, id: TagId) {
        self.tags.tags.retain(|tag| tag.id != id);
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// An Inventory store shared by the Inventory page and the Documents reads. Single-threaded, as the
/// ports are.
pub type SharedInventoryStore = Rc<RefCell<dyn InventoryStore>>;

/// The Inventory repository port, wherever it is kept.
pub trait InventoryStore {
    fn inventory(&self) -> &dyn InventoryRepository;

    fn inventory_mut(&mut self) -> &mut dyn InventoryRepository;

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

    /// The Splits filed under Category `id`: the Splits half of `lib-categories`' `CategoryUsage`.
    /// An in-memory store counts its own rows; the adapter counts in the file.
    fn category_splits(&self, id: CategoryId) -> usize {
        transactions::category_splits(&self.all_transactions(), id)
    }

    /// The Transactions booked to each of `accounts` that has any: the Transactions half of
    /// [`StoredAccountReads`]' `AccountUsage` and of [`StoredUnitReads`]' holdings. An in-memory
    /// store counts its own rows; the adapter counts in the file in one query, and answers none
    /// for a count it can't read.
    fn account_transaction_counts(&self, accounts: &[AccountId]) -> HashMap<AccountId, usize> {
        transactions::account_transaction_counts(&self.all_transactions(), accounts)
    }

    /// The Splits naming Payee `id`, `None` when they can't be counted: the Splits half of
    /// [`StoredPayeeReads`]' `PayeeUsage`. An in-memory store counts its own rows; the adapter
    /// counts in the file.
    fn payee_splits(&self, id: PayeeId) -> Option<usize> {
        Some(transactions::payee_splits(&self.all_transactions(), id))
    }

    /// The Splits carrying Tag `id`, one each however often the Split lists it, in the
    /// Transactions' order: the Transactions half of [`StoredTagReads`]' `TagUsage`. An in-memory
    /// store reads its own rows; the adapter reads the file, and answers none for a read it can't
    /// make.
    fn tagged_splits(&self, id: TagId) -> Vec<TaggedSplit> {
        transactions::tagged_splits(&self.all_transactions(), id)
    }

    /// Distinct Transactions per Tag, for every Tag a Split carries; a Tag nothing carries has no
    /// entry. The batch half of [`StoredTagReads`]' `TagUsage`.
    fn tag_transaction_counts(&self) -> HashMap<TagId, usize> {
        transactions::tag_transaction_counts(&self.all_transactions())
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

    fn category_splits(&self, id: CategoryId) -> usize {
        TransactionsAdapter::category_splits(self, id)
    }

    fn payee_splits(&self, id: PayeeId) -> Option<usize> {
        TransactionsAdapter::payee_splits(self, id)
    }

    fn account_transaction_counts(&self, accounts: &[AccountId]) -> HashMap<AccountId, usize> {
        TransactionsAdapter::account_transaction_counts(self, accounts)
    }

    fn tagged_splits(&self, id: TagId) -> Vec<TaggedSplit> {
        TransactionsAdapter::tagged_splits(self, id)
    }

    fn tag_transaction_counts(&self) -> HashMap<TagId, usize> {
        TransactionsAdapter::tag_transaction_counts(self)
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

    fn category_splits(&self, id: CategoryId) -> usize {
        (**self).category_splits(id)
    }

    fn payee_splits(&self, id: PayeeId) -> Option<usize> {
        (**self).payee_splits(id)
    }

    fn account_transaction_counts(&self, accounts: &[AccountId]) -> HashMap<AccountId, usize> {
        (**self).account_transaction_counts(accounts)
    }

    fn tagged_splits(&self, id: TagId) -> Vec<TaggedSplit> {
        (**self).tagged_splits(id)
    }

    fn tag_transaction_counts(&self) -> HashMap<TagId, usize> {
        (**self).tag_transaction_counts()
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

    fn category_splits(&self, id: CategoryId) -> usize {
        transactions::category_splits(&self.transactions, id)
    }

    fn payee_splits(&self, id: PayeeId) -> Option<usize> {
        Some(transactions::payee_splits(&self.transactions, id))
    }

    fn account_transaction_counts(&self, accounts: &[AccountId]) -> HashMap<AccountId, usize> {
        transactions::account_transaction_counts(&self.transactions, accounts)
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

    /// The Bill Plans naming Payee `id`, `None` when they can't be counted: the Bill Plans half of
    /// [`StoredPayeeReads`]' `PayeeUsage`. An in-memory store counts its own rows; the adapter
    /// counts in the file.
    fn payee_bill_plans(&self, id: PayeeId) -> Option<usize> {
        Some(bills::payee_bill_plans(&self.plans().bill_plans(), id))
    }

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

    fn payee_bill_plans(&self, id: PayeeId) -> Option<usize> {
        BillsAdapter::payee_bill_plans(self, id)
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

    fn payee_bill_plans(&self, id: PayeeId) -> Option<usize> {
        (**self).payee_bill_plans(id)
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

/// A Payees store shared by the Payees module, the cross-domain adapters that write it and the
/// Bills reads. Single-threaded, as the ports are.
pub type SharedPayeesStore = Rc<RefCell<dyn PayeesStore>>;

/// The Payees repository port, wherever it is kept. What names a Payee is other domains' rows,
/// which [`StoredPayeeReads`] counts in their stores (#992).
pub trait PayeesStore {
    fn payees(&self) -> &dyn PayeeRepository;

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository;

    /// Payee `id`'s name, as `lib-budgets`' `BudgetPayees` reads it.
    fn payee_name(&self, id: PayeeId) -> Option<String> {
        self.payees().payee(id).map(|payee| payee.name)
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

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read. Only the Payees are the store's: its
/// usage and Categories are `lib-payees`' own test fakes, which a Client reads through
/// [`StoredPayeeReads`] instead.
impl PayeesStore for lib_payees::fake::InMemoryPorts {
    fn payees(&self) -> &dyn PayeeRepository {
        &self.payees
    }

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        &mut self.payees
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A Budgets store shared by the Budgets module, the Category Deletion adapter and the Accounts,
/// Units and Categories usage counts. Single-threaded, as the ports are.
pub type SharedBudgetsStore = Rc<RefCell<dyn BudgetsStore>>;

/// The Budgets repository port, wherever it is kept, and the counts of the Budgets naming another
/// domain's row, which the Accounts, Units and Categories usage take their Budgets half from.
pub trait BudgetsStore {
    fn budgets(&self) -> &dyn BudgetRepository;

    fn budgets_mut(&mut self) -> &mut dyn BudgetRepository;

    /// How many Budgets count Account `id`: the Budgets half of `lib-accounts`' `AccountUsage`.
    fn account_budgets(&self, id: AccountId) -> usize {
        self.budgets()
            .budgets()
            .iter()
            .filter(|budget| budget.account_ids.contains(&id))
            .count()
    }

    /// How many Budgets count in Unit `unit`: the Budgets half of `lib-units`' `UnitUsage`.
    fn unit_budgets(&self, unit: UnitId) -> usize {
        self.budgets()
            .budgets()
            .iter()
            .filter(|budget| budget.unit == unit)
            .count()
    }

    /// How many Budgets hold a chain for Category `id`: the Budgets half of `lib-categories`'
    /// `CategoryUsage`.
    fn category_budgets(&self, id: CategoryId) -> usize {
        Budgets::from_stored(self.budgets().budgets()).limit_count(id)
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl BudgetsStore for BudgetsAdapter {
    fn budgets(&self) -> &dyn BudgetRepository {
        self
    }

    fn budgets_mut(&mut self) -> &mut dyn BudgetRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        BudgetsAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedBudgetsStore`]).
impl<T: BudgetsStore + ?Sized> BudgetsStore for Box<T> {
    fn budgets(&self) -> &dyn BudgetRepository {
        (**self).budgets()
    }

    fn budgets_mut(&mut self) -> &mut dyn BudgetRepository {
        (**self).budgets_mut()
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl BudgetsStore for InMemoryBudgets {
    fn budgets(&self) -> &dyn BudgetRepository {
        self
    }

    fn budgets_mut(&mut self) -> &mut dyn BudgetRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// `lib-budgets`' `BudgetCategories` answered by the Categories store alone, borrowed for the one
/// read (#994): the Categories Monthly budget's save reads the tree before the six-port
/// [`StoredBudgetReads`] is needed.
#[derive(Clone)]
pub struct StoredBudgetCategories {
    pub categories: SharedCategoriesStore,
}

impl BudgetCategories for StoredBudgetCategories {
    fn budget_categories(&self) -> Vec<BudgetCategory> {
        categories::budget_categories(&self.categories.borrow().categories().categories())
    }
}

/// `lib-tags`' [`TagUsage`] answered from the shared stores of the domains that carry a Tag: the
/// Splits from the Transactions, and the Unit each Split's Account holds from the Accounts and
/// Units, each borrowed for the one read (#994). A figure only the page shows, so a Split whose
/// Account or Unit can't be found is left out rather than refused.
#[derive(Clone)]
pub struct StoredTagReads {
    pub transactions: SharedTransactionsStore,
    pub accounts: SharedAccountsStore,
    pub units: SharedUnitsStore,
}

impl TagUsage for StoredTagReads {
    fn tag_usage(&self, id: TagId) -> TagUse {
        let splits = self.transactions.borrow().tagged_splits(id);
        let units = self.units.borrow();
        let account_units: HashMap<AccountId, UnitId> = self
            .accounts
            .borrow()
            .accounts()
            .accounts()
            .into_iter()
            .map(|account| (account.id, account.unit))
            .collect();

        let mut transactions: std::collections::HashSet<TransactionId> = Default::default();
        let mut last_used: Option<NaiveDate> = None;
        let mut unit: Option<(UnitId, String)> = None;
        let mut mixed = false;
        let mut sum = bigdecimal::BigDecimal::new(0.into(), 2);
        for split in splits {
            let Some(&unit_id) = account_units.get(&split.account) else {
                continue;
            };
            let Some(code) = units.unit_code(unit_id) else {
                continue;
            };
            transactions.insert(split.transaction);
            last_used = last_used.max(Some(split.date));
            match &unit {
                None => unit = Some((unit_id, code)),
                Some((seen, _)) if *seen != unit_id => mixed = true,
                Some(_) => {}
            }
            sum += split.amount.0;
        }
        let total = match unit {
            None => Total::Empty,
            Some(_) if mixed => Total::Mixed,
            Some((_, unit)) => Total::Single {
                unit,
                amount: lib_core::Money(sum),
            },
        };
        TagUse {
            transactions: transactions.len(),
            total,
            last_used,
        }
    }

    fn transactions_by_tag(&self, ids: &[TagId]) -> HashMap<TagId, usize> {
        let counted = self.transactions.borrow().tag_transaction_counts();
        ids.iter()
            .map(|id| (*id, counted.get(id).copied().unwrap_or(0)))
            .collect()
    }
}

/// `lib-tag-removal`'s [`TagRemovalReads`] answered from the shared stores (#994): the Tags from
/// the Tags store and the distinct Transactions carrying each from the Transactions store, each
/// borrowed for the one read when a Remove tag or Merge tags Dialog opens.
#[derive(Clone)]
pub struct StoredTagRemovalReads {
    pub tags: SharedTagsStore,
    pub transactions: SharedTransactionsStore,
}

impl TagRemovalReads for StoredTagRemovalReads {
    fn tags_by_usage(&self) -> Vec<TagCount> {
        let counts = self.transactions.borrow().tag_transaction_counts();
        lib_tag_removal::protocols::by_usage(&self.tags.borrow().tags().tags(), &counts)
    }

    fn tag_count(&self, id: TagId) -> Option<TagCount> {
        let tag = self
            .tags
            .borrow()
            .tags()
            .tags()
            .into_iter()
            .find(|tag| tag.id == id)?;
        // The Splits carrying the Tag, not every Tag counted over every Transaction: the adapter
        // filters them in the file.
        let transactions = self
            .transactions
            .borrow()
            .tagged_splits(id)
            .into_iter()
            .map(|split| split.transaction)
            .collect::<std::collections::HashSet<_>>()
            .len();
        Some(TagCount { tag, transactions })
    }
}

/// `lib-category-deletion`'s [`CategoryDeletionReads`] answered from the shared stores (#994): the
/// Category from the Categories store, its Splits from the Transactions store and the Budgets
/// limiting it from the Budgets store, each borrowed for the one read when the Delete category
/// Dialog opens.
#[derive(Clone)]
pub struct StoredCategoryDeletionReads {
    pub categories: SharedCategoriesStore,
    pub transactions: SharedTransactionsStore,
    pub budgets: SharedBudgetsStore,
}

impl CategoryDeletionReads for StoredCategoryDeletionReads {
    fn category_in_use(&self, id: CategoryId) -> Option<CategoryInUse> {
        Some(CategoryInUse {
            category: self.categories.borrow().categories().category(id)?,
            splits: self.transactions.borrow().category_splits(id),
            budgets: self.budgets.borrow().category_budgets(id),
        })
    }
}

/// `lib-budgets`' six read ports answered by the shared stores of the domains Budgets reads, each
/// borrowed for the one read (#952). The Desktop's Budgets View reads through it.
#[derive(Clone)]
pub struct StoredBudgetReads {
    pub categories: SharedCategoriesStore,
    pub accounts: SharedAccountsStore,
    pub transactions: SharedTransactionsStore,
    pub bills: SharedBillsStore,
    pub units: SharedUnitsStore,
    pub payees: SharedPayeesStore,
}

impl BudgetCategories for StoredBudgetReads {
    fn budget_categories(&self) -> Vec<BudgetCategory> {
        categories::budget_categories(&self.categories.borrow().categories().categories())
    }
}

impl BudgetAccounts for StoredBudgetReads {
    fn budget_accounts(&self) -> Vec<BudgetAccount> {
        accounts::budget_accounts(&self.accounts.borrow().accounts().accounts())
    }
}

impl BudgetSplits for StoredBudgetReads {
    fn budget_splits(&self, from: NaiveDate, to: NaiveDate) -> Vec<BudgetSplit> {
        transactions::budget_splits(&self.transactions.borrow().all_transactions(), from, to)
    }
}

impl BudgetBills for StoredBudgetReads {
    fn open_bills(&self) -> Vec<BudgetBill> {
        let bills = self.bills.borrow();
        bills::budget_open_bills(&bills.plans().bill_plans(), &bills.entries().entries())
    }

    fn active_bill_plans(&self) -> Vec<BudgetBillPlan> {
        bills::budget_bill_plans(&self.bills.borrow().plans().bill_plans())
    }
}

impl BudgetUnits for StoredBudgetReads {
    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.units.borrow().unit_code(id)
    }
}

impl BudgetPayees for StoredBudgetReads {
    fn payee_name(&self, id: PayeeId) -> Option<String> {
        self.payees.borrow().payee_name(id)
    }
}

/// The Bills store's Needs Attention entries as of `today`, by `lib-bills`' one rule, with the Plans
/// and entries they were read from: the Dashboard's Bill rows and, counted, the Bills rail badge, so
/// [`StoredDashboardReads`] and [`StoredShellReads`] read the store the same way.
struct BillsAttention {
    plans: Vec<lib_bills::BillPlan>,
    entries: Vec<lib_bills::BillScheduleEntry>,
    ids: Vec<lib_bills::EntryId>,
}

fn bills_attention(bills: &SharedBillsStore, today: NaiveDate) -> BillsAttention {
    let bills = bills.borrow();
    let (plans, entries) = (bills.plans().bill_plans(), bills.entries().entries());
    let ids = lib_bills::attention_entries(&plans, &entries, today);
    BillsAttention {
        plans,
        entries,
        ids,
    }
}

/// The default Budget's figures for `today`'s month, or `None` with no default Budget: the
/// Dashboard's bars and, counted, the Budgets rail badge, so both read the one period query.
fn default_budget_figures(
    budgets: &SharedBudgetsStore,
    reads: &StoredBudgetReads,
    today: NaiveDate,
) -> Option<lib_budgets::PeriodFigures> {
    let stored = Budgets::from_stored(budgets.borrow().budgets().budgets());
    let budget = stored.default_budget()?;
    Some(lib_budgets::period_figures(
        budget,
        reads,
        lib_core::Period::of(today),
        today,
    ))
}

/// `lib-dashboard`'s two read ports answered by the shared stores of the domains the Dashboard
/// reads, each borrowed for the one read (#1023), as [`StoredBudgetReads`] answers Budgets'. It
/// calls `lib-bills`' Needs Attention rule and `lib-budgets`' period figures and bars and maps the
/// answers into `lib-dashboard`'s rows; it re-implements no rule and copies nothing from the
/// Ledger. The same struct stands over the adapters and [`Stores::fake`]'s in-memory stores.
#[derive(Clone)]
pub struct StoredDashboardReads {
    pub budgets: SharedBudgetsStore,
    /// The other domains' stores as the Budget figures read them. The Needs Attention rows read
    /// their Bills and Unit codes and the bars their Category names from them too, so a store the
    /// figures come to read is added in one place.
    pub budget_reads: StoredBudgetReads,
}

impl DashboardBills for StoredDashboardReads {
    fn attention_bills(&self, today: NaiveDate) -> Vec<DashboardBill> {
        let BillsAttention {
            plans,
            entries,
            ids,
        } = bills_attention(&self.budget_reads.bills, today);
        let units = self.budget_reads.units.borrow();
        ids.into_iter()
            .filter_map(|id| {
                let entry = lib_bills::entry(&entries, id)?;
                let plan = lib_bills::get(&plans, id.plan_id)?;
                // An entry needing attention is open, so its amount is the Plan's and reads no Split.
                let amount = lib_bills::amount(entry, &plans, &[])?;
                Some(DashboardBill {
                    id: DashboardEntryId {
                        plan_id: id.plan_id,
                        due: id.due,
                    },
                    plan: plan.name.clone(),
                    overdue: lib_bills::status(entry, today) == lib_bills::BillStatus::Overdue,
                    amount,
                    unit_code: units.unit_code(plan.unit).unwrap_or_default(),
                })
            })
            .collect()
    }

    fn base_unit_code(&self) -> Option<String> {
        let units = self.budget_reads.units.borrow();
        units.unit_code(units.base_unit().base_unit())
    }
}

impl DashboardBudgets for StoredDashboardReads {
    fn default_budget_month(&self, today: NaiveDate) -> Option<DashboardBudgetMonth> {
        let reads = &self.budget_reads;
        let figures = default_budget_figures(&self.budgets, reads, today)?;
        let categories = reads.categories.borrow().categories().categories();
        let bars = lib_budgets::dashboard_bars(&figures, reads)
            .into_iter()
            .map(|bar| DashboardBudgetBar {
                category: lib_categories::get(&categories, bar.category_id)
                    .map(|category| category.name.clone())
                    .unwrap_or_default(),
                spent: bar.spent,
                budget: bar.budget,
                over: bar.over,
            })
            .collect();
        Some(DashboardBudgetMonth {
            month: figures.month,
            elapsed: DashboardElapsed {
                day: figures.elapsed.day,
                days_in_month: figures.elapsed.days_in_month,
                percent: figures.elapsed.percent,
            },
            bars,
        })
    }
}

/// `lib-shell`'s rail counts answered by the shared stores of the domains whose rules they are, each
/// borrowed for the one read (#1026), as [`StoredDashboardReads`] answers the Dashboard's. It calls
/// `lib-bills`' Needs Attention rule, `lib-budgets`' over-budget count for the default Budget this
/// month and `lib-documents`' Inbox count, and re-implements none of them. The same struct stands
/// over the adapters and [`Stores::fake`]'s in-memory stores.
#[derive(Clone)]
pub struct StoredShellReads {
    pub budgets: SharedBudgetsStore,
    pub documents: SharedDocumentsStore,
    /// The other domains' stores as the Budget figures read them, which the Needs Attention count
    /// reads its Bills from too.
    pub budget_reads: StoredBudgetReads,
}

impl ShellRailCounts for StoredShellReads {
    fn rail_counts(&self, today: NaiveDate) -> RailCounts {
        let bills_attention = bills_attention(&self.budget_reads.bills, today).ids.len();
        let budgets_over = default_budget_figures(&self.budgets, &self.budget_reads, today)
            .map_or(0, |figures| figures.over_count);
        let documents_inbox =
            lib_documents::inbox_count(&self.documents.borrow().documents().documents());
        RailCounts {
            bills_attention,
            budgets_over,
            documents_inbox,
        }
    }
}

/// `lib-bills`' four read ports answered by the shared stores of the domains Bills reads, each
/// borrowed for the one read (#994), as [`StoredBudgetReads`] answers Budgets'. The same struct
/// stands over the adapters and [`Stores::fake`]'s in-memory stores.
#[derive(Clone)]
pub struct StoredBillReads {
    pub payees: SharedPayeesStore,
    pub accounts: SharedAccountsStore,
    pub categories: SharedCategoriesStore,
    pub transactions: SharedTransactionsStore,
}

impl BillPayees for StoredBillReads {
    fn bill_payees(&self) -> Vec<BillPayee> {
        bill_payees(&self.payees.borrow().payees().payees())
    }
}

impl BillAccounts for StoredBillReads {
    fn bill_accounts(&self) -> Vec<BillAccount> {
        accounts::bill_accounts(&self.accounts.borrow().accounts().accounts())
    }
}

impl BillCategories for StoredBillReads {
    fn bill_categories(&self) -> Vec<BillCategory> {
        categories::bill_categories(&self.categories.borrow().categories().categories())
    }
}

impl BillTransactions for StoredBillReads {
    fn splits_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<BillSplit> {
        transactions::splits_between(&self.transactions.borrow().all_transactions(), from, to)
    }

    fn paid_split(&self, split: SplitRef) -> Option<BillSplit> {
        transactions::paid_split(&self.transactions.borrow().all_transactions(), split)
    }
}

/// `lib-transactions`' five read ports answered by the shared stores of the domains a Transaction
/// names, each borrowed for the one read (#998), as [`StoredBillReads`] answers Bills'. The same
/// struct stands over the adapters and [`Stores::fake`]'s in-memory stores.
#[derive(Clone)]
pub struct StoredTransactionReads {
    pub accounts: SharedAccountsStore,
    pub categories: SharedCategoriesStore,
    pub payees: SharedPayeesStore,
    pub tags: SharedTagsStore,
    pub units: SharedUnitsStore,
}

impl TransactionAccounts for StoredTransactionReads {
    fn transaction_accounts(&self) -> Vec<TransactionAccount> {
        accounts::transaction_accounts(&self.accounts.borrow().accounts().accounts())
    }
}

/// In tree order, each with its path, as the Categories' rule writes it.
impl TransactionCategories for StoredTransactionReads {
    fn transaction_categories(&self) -> Vec<TransactionCategory> {
        categories::transaction_categories(&self.categories.borrow().categories().categories())
    }
}

impl TransactionPayees for StoredTransactionReads {
    fn transaction_payees(&self) -> Vec<TransactionPayee> {
        payees::transaction_payees(&self.payees.borrow().payees().payees())
    }
}

impl TransactionTags for StoredTransactionReads {
    fn transaction_tags(&self) -> Vec<TransactionTag> {
        tags::transaction_tags(&self.tags.borrow().tags().tags())
    }
}

impl TransactionUnits for StoredTransactionReads {
    fn transaction_unit_code(&self, id: UnitId) -> Option<String> {
        self.units.borrow().unit_code(id)
    }
}

/// `lib-transaction-import`'s two read ports answered by the Accounts and Categories stores, each
/// borrowed for the one read (#994).
#[derive(Clone)]
pub struct StoredImportReads {
    pub accounts: SharedAccountsStore,
    pub categories: SharedCategoriesStore,
}

impl ImportAccounts for StoredImportReads {
    fn import_account(&self, id: AccountId) -> Option<ImportAccount> {
        accounts::import_account(&self.accounts.borrow().accounts().accounts(), id)
    }
}

impl ImportCategories for StoredImportReads {
    fn import_categories(&self) -> Vec<PayeeCategory> {
        categories::payee_categories(&self.categories.borrow().categories().categories())
    }
}

/// `lib-institutions`' `InstitutionUsage` and `InstitutionUnits` answered by the Accounts and
/// Units stores, each borrowed for the one read (#994, #1052): the Accounts naming the
/// Institution, and the Units a Usual unit may name.
#[derive(Clone)]
pub struct StoredInstitutionReads {
    pub accounts: SharedAccountsStore,
    pub units: SharedUnitsStore,
}

impl InstitutionUsage for StoredInstitutionReads {
    fn institution_usage(&self, institution: InstitutionId) -> InstitutionUse {
        InstitutionUse {
            accounts: accounts::institution_accounts(
                &self.accounts.borrow().accounts().accounts(),
                institution,
            ),
        }
    }
}

impl InstitutionUnits for StoredInstitutionReads {
    fn institution_units(&self) -> Vec<InstitutionUnit> {
        units::institution_units(&self.units.borrow().units().units())
    }
}

/// `lib-accounts`' `AccountUsage`, `AccountUnits` and `AccountInstitutions` answered by the shared stores of the domains
/// an Account names or is named by, each borrowed for the one read (#993, #1052), as
/// [`StoredPayeeReads`] answers Payees'. The same struct stands over [`Stores::fake`]'s in-memory
/// stores and the adapters. The Transactions store counts the Transactions booked to the Account,
/// the Budgets store the Budgets that count it, and the Units store holds the Units an Account
/// may count in, and the Institutions store the Institutions it may be at. It is also
/// `lib-account-deletion`'s `DeletionReads`: the Delete account Dialog
/// quotes the same counts and the Unit's code.
#[derive(Clone)]
pub struct StoredAccountReads {
    pub transactions: SharedTransactionsStore,
    pub budgets: SharedBudgetsStore,
    pub units: SharedUnitsStore,
    pub institutions: SharedInstitutionsStore,
}

impl AccountUsage for StoredAccountReads {
    fn account_usage(&self, id: AccountId) -> AccountUse {
        let counts = self.transactions.borrow().account_transaction_counts(&[id]);
        AccountUse {
            transactions: counts.get(&id).copied().unwrap_or_default(),
            budgets: self.budgets.borrow().account_budgets(id),
        }
    }
}

impl AccountUnits for StoredAccountReads {
    fn account_units(&self) -> Vec<AccountUnit> {
        units::account_units(&self.units.borrow().units().units())
    }

    fn base_unit(&self) -> Option<UnitId> {
        let store = self.units.borrow();
        units::held_base_unit(&store.units().units(), store.base_unit().base_unit())
    }
}

impl AccountInstitutions for StoredAccountReads {
    fn account_institutions(&self) -> Vec<AccountInstitution> {
        institutions::account_institutions(
            &self.institutions.borrow().institutions().institutions(),
        )
    }
}

impl DeletionReads for StoredAccountReads {
    fn unit_code(&self, unit: UnitId) -> Option<String> {
        self.units.borrow().unit_code(unit)
    }
}

/// `lib-units`' `UnitUsage` and `UnitHoldings` answered by the shared stores of the domains that
/// name a Unit, each borrowed for the one read (#993): the Accounts (with their Transactions
/// counted by the Transactions store), Bill Plans, Budgets, Locations and Institutions.
#[derive(Clone)]
pub struct StoredUnitReads {
    pub accounts: SharedAccountsStore,
    pub transactions: SharedTransactionsStore,
    pub bills: SharedBillsStore,
    pub budgets: SharedBudgetsStore,
    pub inventory: SharedInventoryStore,
    pub institutions: SharedInstitutionsStore,
}

impl UnitUsage for StoredUnitReads {
    fn unit_usage(&self, id: UnitId) -> UnitUse {
        UnitUse {
            accounts: accounts::unit_accounts(&self.accounts.borrow().accounts().accounts(), id),
            bill_plans: bills::unit_bill_plans(&self.bills.borrow().plans().bill_plans(), id),
            budgets: self.budgets.borrow().unit_budgets(id),
            locations: inventory::unit_locations(
                &self.inventory.borrow().inventory().locations(),
                id,
            ),
            institutions: institutions::unit_institutions(
                &self.institutions.borrow().institutions().institutions(),
                id,
            ),
        }
    }
}

impl UnitHoldings for StoredUnitReads {
    /// One count of the Transactions for every Account in the Unit, rather than one per Account.
    fn holdings(&self, id: UnitId) -> Vec<Holding> {
        let rows = self.accounts.borrow().accounts().accounts();
        let counts = self
            .transactions
            .borrow()
            .account_transaction_counts(&accounts::unit_account_ids(&rows, id));
        accounts::holdings(&rows, id, |account| {
            counts.get(&account).copied().unwrap_or_default()
        })
    }
}

/// `lib-bills`' `BillUnits` answered by the Units store, borrowed for the one read (#993).
#[derive(Clone)]
pub struct StoredBillUnits {
    pub units: SharedUnitsStore,
}

impl BillUnits for StoredBillUnits {
    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.units.borrow().unit_code(id)
    }

    fn base_unit(&self) -> Option<UnitId> {
        Some(self.units.borrow().base_unit().base_unit())
    }
}

/// `lib-payees`' two read ports answered by the shared stores of the domains Payees reads, each
/// borrowed for the one read (#992), as [`StoredDocumentReads`] answers Documents'. The same
/// struct stands over [`Stores::fake`]'s in-memory stores and the adapters, so the UI tests read
/// a Payee's use as the app does. Each store counts its own rows naming the Payee; this maps the
/// counts into `lib-payees`' terms.
#[derive(Clone)]
pub struct StoredPayeeReads {
    pub transactions: SharedTransactionsStore,
    pub bills: SharedBillsStore,
    pub categories: SharedCategoriesStore,
}

impl PayeeUsage for StoredPayeeReads {
    /// Unknown when either half can't be counted, never unused (#840). The two counts are made
    /// one after the other, not from one snapshot: that is safe because every write is queued from
    /// the UI thread, which blocks across both, and the foreign keys (`splits.payee_id`,
    /// `bill_plans.payee_id`) back the refusal in the file.
    fn payee_usage(&self, id: PayeeId) -> Option<PayeeUse> {
        Some(PayeeUse {
            splits: self.transactions.borrow().payee_splits(id)?,
            bill_plans: self.bills.borrow().payee_bill_plans(id)?,
        })
    }
}

impl PayeeCategories for StoredPayeeReads {
    fn payee_categories(&self) -> Vec<PayeeCategory> {
        categories::payee_categories(&self.categories.borrow().categories().categories())
    }
}

/// The Documents store, shared by the Documents module's Presenters and the reads that count its
/// Links. Single-threaded, as the ports are.
pub type SharedDocumentsStore = Rc<RefCell<dyn DocumentsStore>>;

/// The Documents repository port, wherever it is kept.
pub trait DocumentsStore {
    fn documents(&self) -> &dyn DocumentRepository;

    fn documents_mut(&mut self) -> &mut dyn DocumentRepository;

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> Result<()>;
}

impl DocumentsStore for DocumentsAdapter {
    fn documents(&self) -> &dyn DocumentRepository {
        self
    }

    fn documents_mut(&mut self) -> &mut dyn DocumentRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        DocumentsAdapter::reload(self)
    }
}

/// A boxed store is the store, so a bin given one can share it ([`SharedDocumentsStore`]).
impl<T: DocumentsStore + ?Sized> DocumentsStore for Box<T> {
    fn documents(&self) -> &dyn DocumentRepository {
        (**self).documents()
    }

    fn documents_mut(&mut self) -> &mut dyn DocumentRepository {
        (**self).documents_mut()
    }

    fn reload(&mut self) -> Result<()> {
        (**self).reload()
    }
}

/// Memory is all there is, so a reload has nothing to read.
impl DocumentsStore for InMemoryDocuments {
    fn documents(&self) -> &dyn DocumentRepository {
        self
    }

    fn documents_mut(&mut self) -> &mut dyn DocumentRepository {
        self
    }

    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}

/// `lib-documents`' five read ports answered by the shared stores of the domains a Link names,
/// each borrowed for the one read (#963), as [`StoredBudgetReads`] answers Budgets'. The Desktop's
/// Documents View reads through it, once per query into `LinkRecords`.
#[derive(Clone)]
pub struct StoredDocumentReads {
    pub transactions: SharedTransactionsStore,
    pub payees: SharedPayeesStore,
    pub accounts: SharedAccountsStore,
    pub institutions: SharedInstitutionsStore,
    pub bills: SharedBillsStore,
    pub inventory: SharedInventoryStore,
}

impl DocumentTransactions for StoredDocumentReads {
    fn document_transactions(&self) -> Vec<DocumentTransaction> {
        transactions::document_transactions(&self.transactions.borrow().all_transactions())
    }
}

impl DocumentPayees for StoredDocumentReads {
    fn document_payees(&self) -> Vec<DocumentPayee> {
        payees::document_payees(&self.payees.borrow().payees().payees())
    }
}

impl DocumentAccounts for StoredDocumentReads {
    fn document_accounts(&self) -> Vec<DocumentAccount> {
        accounts::document_accounts(
            &self.accounts.borrow().accounts().accounts(),
            &self.institutions.borrow().institutions().institutions(),
        )
    }
}

impl DocumentBillPlans for StoredDocumentReads {
    fn document_bill_plans(&self) -> Vec<DocumentBillPlan> {
        bills::document_bill_plans(&self.bills.borrow().plans().bill_plans())
    }
}

impl DocumentItems for StoredDocumentReads {
    fn document_items(&self) -> Vec<DocumentItem> {
        inventory::document_items(&self.inventory.borrow().inventory().items())
    }
}

/// `lib-inventory`'s two read ports answered by the Units, Inventory and Documents stores, each
/// borrowed for the one read (#1014): the fiat Units its Location form offers and the Documents
/// its Remove location Dialog quotes.
#[derive(Clone)]
pub struct StoredInventoryReads {
    pub units: SharedUnitsStore,
    pub inventory: SharedInventoryStore,
    pub documents: SharedDocumentsStore,
}

impl InventoryUnits for StoredInventoryReads {
    /// In the Units' order.
    fn currencies(&self) -> Vec<UnitChoice> {
        self.units
            .borrow()
            .units()
            .units()
            .into_iter()
            .filter(|unit| unit.kind == UnitKind::Currency)
            .map(|unit| UnitChoice {
                id: unit.id,
                code: unit.code,
                name: unit.name,
            })
            .collect()
    }

    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.units.borrow().unit_code(id)
    }
}

impl LocationUsage for StoredInventoryReads {
    /// The Documents with a Link to one of the Location's Items, which the file's cascade drops
    /// when it goes (#964).
    fn location_usage(&self, id: LocationId) -> LocationUse {
        let links =
            inventory::location_item_links(&self.inventory.borrow().inventory().items(), id);
        let documents = self.documents.borrow().documents().documents();
        LocationUse {
            documents: documents
                .iter()
                .filter(|document| document.links.iter().any(|link| links.contains(link)))
                .count(),
        }
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
    fn commit(&self) -> Vec<Domain> {
        self.count.set(self.count.get() + 1);
        self.inner.commit()
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
    pub budgets: InMemoryBudgets,
    pub inventory: InMemoryInventory,
    pub documents: InMemoryDocuments,
}

impl FakeStores {
    /// The Sample Ledger as of `today` (`crate::seed::SampleLedger`), the rows `crate::seed`
    /// writes to a Ledger file, with each Account's usage counted from its Transactions and
    /// Budgets. The one way a test or a Client that runs without a Ledger file reaches it (#1056).
    pub fn sample(today: NaiveDate) -> Self {
        let crate::seed::SampleLedger {
            units,
            institutions,
            accounts,
            categories,
            tags,
            payees,
            transactions,
            bills,
            budgets,
            inventory,
            document_types,
            documents,
        } = crate::seed::SampleLedger::new(today);
        let budgets = InMemoryBudgets { budgets };
        let mut accounts = lib_accounts::fake::InMemoryPorts {
            accounts: lib_accounts::fake::InMemoryAccounts { accounts },
            ..lib_accounts::fake::InMemoryPorts::sample()
        };
        accounts.usage.uses = accounts
            .accounts
            .accounts
            .iter()
            .map(|account| {
                let uses = AccountUse {
                    transactions: crate::seed::account_transactions(&transactions, account.id),
                    budgets: budgets.account_budgets(account.id),
                };
                (account.id, uses)
            })
            .collect();
        Self {
            units,
            institutions: InMemoryInstitutions { institutions },
            accounts,
            categories: InMemoryCategories { categories },
            tags: lib_tags::fake::InMemoryPorts {
                tags: lib_tags::fake::InMemoryTags { tags },
                ..lib_tags::fake::InMemoryPorts::sample()
            },
            payees: lib_payees::fake::InMemoryPorts {
                payees: lib_payees::fake::InMemoryPayees { payees },
                ..lib_payees::fake::InMemoryPorts::sample()
            },
            documents: InMemoryDocuments {
                documents,
                types: document_types,
            },
            transactions: InMemoryTransactions { transactions },
            bills: InMemoryBills {
                plans: bills.plans,
                entries: bills.entries,
            },
            budgets,
            inventory: InMemoryInventory {
                locations: inventory.locations,
                items: inventory.items,
            },
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
            budgets,
            inventory,
            documents,
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
            budgets: Box::new(budgets),
            inventory: Box::new(inventory),
            documents: Box::new(documents),
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
            units: InMemoryPorts::sample(chrono::Utc::now()),
            institutions: InMemoryInstitutions::sample(),
            accounts: lib_accounts::fake::InMemoryPorts::sample(),
            categories: InMemoryCategories::sample(),
            tags: lib_tags::fake::InMemoryPorts::sample(),
            payees: lib_payees::fake::InMemoryPorts::sample(),
            transactions: InMemoryTransactions::default(),
            bills: InMemoryBills::default(),
            budgets: InMemoryBudgets::default(),
            inventory: InMemoryInventory::default(),
            documents: InMemoryDocuments::default(),
        });

        stores.commit.commit();
        stores.commit.commit();

        assert_eq!(count.get(), 2);
    }

    /// Over the in-memory stores the UI tests run on, a Payee's use is what those stores hold, a
    /// Split written to the Transactions store after the reads were built included.
    #[test]
    fn stored_payee_reads_count_the_in_memory_stores_rows() {
        let sample = FakeStores::sample(chrono::Local::now().date_naive());
        let transactions: SharedTransactionsStore = Rc::new(RefCell::new(InMemoryTransactions {
            transactions: sample.transactions.transactions.clone(),
        }));
        let reads = StoredPayeeReads {
            transactions: transactions.clone(),
            bills: Rc::new(RefCell::new(InMemoryBills {
                plans: sample.bills.plans.clone(),
                entries: Vec::new(),
            })),
            categories: Rc::new(RefCell::new(InMemoryCategories::sample())),
        };
        let (woolworths, telstra) = (PayeeId::seed(1), PayeeId::seed(20));
        let before = reads.payee_usage(woolworths).expect("memory always counts");
        assert!(before.splits > 0, "the sample's groceries name Woolworths");
        assert_eq!(
            reads.payee_usage(telstra).map(|uses| uses.bill_plans),
            Some(1)
        );

        let mut booked = sample.transactions.transactions[0].clone();
        booked.id = TransactionId::seed(9_999);
        booked.splits.truncate(1);
        booked.splits[0].payee_id = Some(woolworths);
        transactions.borrow_mut().upsert_transaction(booked);

        assert_eq!(
            reads.payee_usage(woolworths).map(|uses| uses.splits),
            Some(before.splits + 1)
        );
        assert!(
            reads
                .payee_categories()
                .iter()
                .any(|leaf| leaf.id == CategoryId::seed(7)),
            "the Categories store's leaves"
        );
    }

    fn in_memory_sample_stores() -> (FakeStores, SharedTransactionsStore, SharedBillsStore) {
        let sample = FakeStores::sample(chrono::Local::now().date_naive());
        let transactions: SharedTransactionsStore = Rc::new(RefCell::new(InMemoryTransactions {
            transactions: sample.transactions.transactions.clone(),
        }));
        let bills: SharedBillsStore = Rc::new(RefCell::new(InMemoryBills {
            plans: sample.bills.plans.clone(),
            entries: sample.bills.entries.clone(),
        }));
        (sample, transactions, bills)
    }

    /// An Account's use is counted in the Transactions and Budgets stores on each ask.
    #[test]
    fn stored_account_reads_count_the_in_memory_stores_rows() {
        let (sample, transactions, _) = in_memory_sample_stores();
        let budgets: SharedBudgetsStore = Rc::new(RefCell::new(InMemoryBudgets {
            budgets: sample.budgets.budgets.clone(),
        }));
        let reads = StoredAccountReads {
            transactions: transactions.clone(),
            budgets,
            units: sample_units(),
            institutions: sample_institutions(),
        };
        let everyday = AccountId::seed(2);
        let before = reads.account_usage(everyday);
        assert!(before.transactions > 0);
        assert_eq!(before.budgets, 2);
        assert_eq!(reads.account_usage(AccountId::new()), AccountUse::default());

        let mut booked = sample.transactions.transactions[0].clone();
        booked.id = TransactionId::seed(9_999);
        booked.account_id = everyday;
        transactions.borrow_mut().upsert_transaction(booked);

        assert_eq!(
            reads.account_usage(everyday).transactions,
            before.transactions + 1
        );
    }

    /// Bills, Import, Institutions and the Budgets' Category tree read the stores' own rows, in
    /// the reading module's terms, on each ask (#994).
    #[test]
    fn the_bill_import_and_institution_reads_map_the_in_memory_stores_rows() {
        let (sample, transactions, _) = in_memory_sample_stores();
        let accounts: SharedAccountsStore =
            Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::sample()));
        let categories: SharedCategoriesStore = Rc::new(RefCell::new(InMemoryCategories::sample()));
        let payees: SharedPayeesStore =
            Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts::sample()));
        let stored_accounts = accounts.borrow().accounts().accounts();
        let stored_categories = categories.borrow().categories().categories();

        let bills = StoredBillReads {
            payees: payees.clone(),
            accounts: accounts.clone(),
            categories: categories.clone(),
            transactions: transactions.clone(),
        };
        assert_eq!(bills.bill_accounts().len(), stored_accounts.len());
        assert_eq!(bills.bill_categories().len(), stored_categories.len());
        assert_eq!(
            bills.bill_payees().len(),
            payees.borrow().payees().payees().len()
        );
        let (from, to) = (NaiveDate::MIN, NaiveDate::MAX);
        assert_eq!(
            bills.splits_between(from, to).len(),
            sample
                .transactions
                .transactions
                .iter()
                .map(|transaction| transaction.splits.len())
                .sum::<usize>()
        );

        let import = StoredImportReads {
            accounts: accounts.clone(),
            categories: categories.clone(),
        };
        let account = stored_accounts[0].id;
        assert!(import.import_account(account).is_some());
        assert!(import.import_account(AccountId::new()).is_none());
        assert!(!import.import_categories().is_empty());

        let institution = stored_accounts[0].institution;
        let named = StoredInstitutionReads {
            accounts,
            units: sample_units(),
        }
        .institution_usage(institution);
        assert!(named.is_used());
        assert_eq!(
            StoredInstitutionReads {
                accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::default())),
                units: sample_units(),
            }
            .institution_usage(institution),
            InstitutionUse::default()
        );

        let tree = StoredBudgetCategories { categories }.budget_categories();
        assert_eq!(tree.len(), stored_categories.len());
    }

    /// The sample Units, base Unit and Price Sources as an in-memory Units store.
    fn sample_units() -> SharedUnitsStore {
        Rc::new(RefCell::new(InMemoryPorts::sample(chrono::Utc::now())))
    }

    /// The sample Institutions as an in-memory Institutions store.
    fn sample_institutions() -> SharedInstitutionsStore {
        Rc::new(RefCell::new(InMemoryInstitutions::sample()))
    }

    /// The Accounts read their Institution choices from the Institutions store on each ask
    /// (#1053): each Institution's id, name and Usual unit, in the Institutions' order.
    #[test]
    fn stored_account_reads_answer_account_institutions_from_the_institutions_store() {
        let (_, transactions, _) = in_memory_sample_stores();
        let institutions = sample_institutions();
        let reads = StoredAccountReads {
            transactions,
            budgets: Rc::new(RefCell::new(InMemoryBudgets::default())),
            units: sample_units(),
            institutions: institutions.clone(),
        };

        let held = lib_institutions::sample_institutions();
        let offered = reads.account_institutions();
        assert_eq!(
            offered.iter().map(|row| row.id).collect::<Vec<_>>(),
            held.iter().map(|row| row.id).collect::<Vec<_>>(),
            "in the Institutions' order"
        );
        assert_eq!(offered[0].name, held[0].name);

        let mut edited = held[0].clone();
        edited.name = "Renamed".into();
        edited.usual_unit = Some(UnitId::SEED_BTC);
        institutions
            .borrow_mut()
            .institutions_mut()
            .upsert_institution(edited);

        let after = reads.account_institutions();
        assert_eq!(
            after[0].name, "Renamed",
            "written since the reads were built"
        );
        assert_eq!(after[0].usual_unit, Some(UnitId::SEED_BTC));
        assert_eq!(after.len(), held.len());
    }

    /// The Accounts read their Unit choices from the Units store on each ask (#1052): each Unit's
    /// code, active flag and Precision, and the base Unit while the store still holds it.
    #[test]
    fn stored_account_reads_answer_account_units_from_the_units_store() {
        let (sample, transactions, _) = in_memory_sample_stores();
        let units = sample_units();
        let reads = StoredAccountReads {
            transactions,
            budgets: Rc::new(RefCell::new(InMemoryBudgets {
                budgets: sample.budgets.budgets.clone(),
            })),
            units: units.clone(),
            institutions: sample_institutions(),
        };

        let choices = reads.account_units();
        let codes: Vec<&str> = choices.iter().map(|unit| unit.code.as_str()).collect();
        assert_eq!(codes, ["aud", "btc", "vas"], "in the Units' order");
        let aud = choices
            .iter()
            .find(|unit| unit.id == UnitId::SEED_AUD)
            .expect("the sample's aud");
        assert!(aud.is_active);
        assert_eq!(aud.precision, 2);
        assert_eq!(AccountUnits::base_unit(&reads), Some(UnitId::SEED_AUD));

        let mut btc = units
            .borrow()
            .units()
            .units()
            .into_iter()
            .find(|unit| unit.id == UnitId::SEED_BTC)
            .expect("the sample's btc");
        btc.code = "xbt".into();
        btc.is_active = false;
        btc.precision = lib_units::Precision::new(6).expect("six places is within the most");
        let unread = lib_units::fake::InMemoryUnitUsage::default();
        let mut store = units.borrow_mut();
        let ports = store.split_mut(&unread);
        ports.units.upsert_unit(btc);
        ports.base_unit.set_base_unit(UnitId::SEED_BTC);
        drop(store);

        let edited = reads
            .account_units()
            .into_iter()
            .find(|unit| unit.id == UnitId::SEED_BTC)
            .expect("btc is still a Unit");
        assert_eq!(edited.code, "xbt", "written since the reads were built");
        assert!(!edited.is_active);
        assert_eq!(edited.precision, 6);
        assert_eq!(AccountUnits::base_unit(&reads), Some(UnitId::SEED_BTC));

        units
            .borrow_mut()
            .split_mut(&unread)
            .base_unit
            .set_base_unit(UnitId::new());
        assert_eq!(
            AccountUnits::base_unit(&reads),
            None,
            "a base Unit the store doesn't hold is none"
        );
    }

    /// The Delete account Dialog reads a Unit's code from the Units store: a known Unit's code,
    /// and none for a Unit that has gone (#1052).
    #[test]
    fn stored_account_reads_quote_the_delete_dialogs_unit_code() {
        let (_, transactions, _) = in_memory_sample_stores();
        let reads = StoredAccountReads {
            transactions,
            budgets: Rc::new(RefCell::new(InMemoryBudgets::default())),
            units: sample_units(),
            institutions: sample_institutions(),
        };

        assert_eq!(
            DeletionReads::unit_code(&reads, UnitId::SEED_AUD).as_deref(),
            Some("aud")
        );
        assert_eq!(DeletionReads::unit_code(&reads, UnitId::new()), None);
    }

    /// The Institutions read the Units a Usual unit may name from the Units store.
    #[test]
    fn stored_institution_reads_answer_institution_units_from_the_units_store() {
        let reads = StoredInstitutionReads {
            accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::default())),
            units: sample_units(),
        };
        assert_eq!(
            reads.institution_units(),
            lib_institutions::fake::InMemoryInstitutionUnits::sample().units
        );
    }

    /// A Unit's use and holdings are counted in the Accounts, Transactions, Bills and Budgets
    /// stores.
    #[test]
    fn stored_unit_reads_count_the_in_memory_stores_rows() {
        let (sample, transactions, bills) = in_memory_sample_stores();
        let accounts: SharedAccountsStore =
            Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts {
                accounts: lib_accounts::fake::InMemoryAccounts {
                    accounts: sample.accounts.accounts.accounts.clone(),
                },
                ..Default::default()
            }));
        let reads = StoredUnitReads {
            accounts,
            transactions,
            bills,
            budgets: Rc::new(RefCell::new(InMemoryBudgets {
                budgets: sample.budgets.budgets.clone(),
            })),
            inventory: Rc::new(RefCell::new(InMemoryInventory::default())),
            institutions: Rc::new(RefCell::new(InMemoryInstitutions::sample())),
        };
        let aud = UnitId::SEED_AUD;

        let usage = reads.unit_usage(aud);
        assert_eq!(
            usage.accounts,
            sample
                .accounts
                .accounts
                .accounts
                .iter()
                .filter(|a| a.unit == aud)
                .count()
        );
        assert_eq!(
            usage.bill_plans,
            sample.bills.plans.iter().filter(|p| p.unit == aud).count()
        );
        assert_eq!(usage.locations, 0);
        assert_eq!(reads.unit_usage(UnitId::new()), UnitUse::default());

        let holdings = reads.holdings(aud);
        assert_eq!(holdings.len(), usage.accounts);
        let everyday = holdings
            .iter()
            .find(|holding| holding.name == "ANZ Everyday")
            .expect("ANZ Everyday counts in aud");
        assert_eq!(
            usize::try_from(everyday.transactions).ok(),
            Some(
                sample
                    .transactions
                    .transactions
                    .iter()
                    .filter(|t| t.account_id == AccountId::seed(2))
                    .count()
            )
        );
        assert!(reads.holdings(UnitId::new()).is_empty());
    }

    /// Over the in-memory stores [`Stores::fake`] bundles, the Remove tag and Merge tags Dialogs
    /// read every Tag ranked by the Transactions store's rows, a Split booked after the reads were
    /// built included.
    #[test]
    fn stored_tag_removal_reads_rank_the_in_memory_stores_rows() {
        let world = lib_tag_removal::fake::world::world();
        let transactions: SharedTransactionsStore =
            Rc::new(RefCell::new(world.transactions.clone()));
        let reads = StoredTagRemovalReads {
            tags: Rc::new(RefCell::new(lib_tags::fake::InMemoryPorts {
                tags: world.tags.clone(),
                ..Default::default()
            })),
            transactions: transactions.clone(),
        };

        lib_tag_removal::contract::the_reads_rank_every_tag_by_usage(&reads);
        lib_tag_removal::contract::the_reads_count_one_tag(&reads);

        let mut booked = world.transactions.transactions[0].clone();
        booked.id = TransactionId::seed(9_999);
        booked.splits[0].tag_ids = vec![lib_tag_removal::fake::world::ONE_OFF];
        transactions.borrow_mut().upsert_transaction(booked);
        let one_off = reads
            .tags_by_usage()
            .into_iter()
            .find(|count| count.tag.id == lib_tag_removal::fake::world::ONE_OFF)
            .map(|count| count.transactions);
        assert_eq!(one_off, Some(1));
    }

    /// Over the in-memory stores [`Stores::fake`] bundles, the Delete category Dialog quotes a
    /// Category's Splits and Budgets from the stores' rows, a Split booked after the reads were
    /// built included.
    #[test]
    fn stored_category_deletion_reads_quote_the_in_memory_stores_rows() {
        use lib_category_deletion::fake::world::{RENT, world};

        let world = world();
        let transactions: SharedTransactionsStore =
            Rc::new(RefCell::new(world.transactions.clone()));
        let reads = StoredCategoryDeletionReads {
            categories: Rc::new(RefCell::new(world.categories.clone())),
            transactions: transactions.clone(),
            budgets: Rc::new(RefCell::new(world.budgets.clone())),
        };

        lib_category_deletion::contract::the_reads_quote_a_category_in_use(&reads);

        let mut booked = world.transactions.transactions[0].clone();
        booked.id = TransactionId::seed(9_999);
        booked.splits[0].category_id = RENT;
        transactions.borrow_mut().upsert_transaction(booked);
        assert_eq!(
            reads.category_in_use(RENT).map(|in_use| in_use.splits),
            Some(2)
        );
    }

    /// Bills read a Unit's code and the base Unit from the Units store.
    #[test]
    fn stored_bill_units_read_the_units_store() {
        let units: SharedUnitsStore =
            Rc::new(RefCell::new(InMemoryPorts::sample(chrono::Utc::now())));
        let reads = StoredBillUnits { units };
        assert_eq!(reads.unit_code(UnitId::SEED_AUD).as_deref(), Some("aud"));
        assert_eq!(reads.unit_code(UnitId::new()), None);
        assert_eq!(reads.base_unit(), Some(UnitId::SEED_AUD));
    }

    /// The six Budgets read ports are answered by their stores, as the Ledger once answered them.
    #[test]
    fn stored_budget_reads_answer_each_port_from_its_store() {
        let (sample, transactions, bills) = in_memory_sample_stores();
        let reads = StoredBudgetReads {
            categories: Rc::new(RefCell::new(InMemoryCategories::sample())),
            accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::sample())),
            transactions,
            bills,
            units: Rc::new(RefCell::new(InMemoryPorts::sample(chrono::Utc::now()))),
            payees: Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts::sample())),
        };
        let month = lib_core::Period::of(NaiveDate::from_ymd_opt(2026, 9, 21).expect("a date"));

        let splits = reads.budget_splits(month.first_day(), month.last_day());
        let expected: usize = sample
            .transactions
            .transactions
            .iter()
            .filter(|t| month.contains(t.date))
            .map(|t| t.splits.len())
            .sum();
        assert!(expected > 0);
        assert_eq!(splits.len(), expected);
        assert!(splits.iter().all(|split| month.contains(split.date)));

        let open = sample
            .bills
            .entries
            .iter()
            .filter(|entry| entry.is_open())
            .count();
        assert_eq!(reads.open_bills().len(), open);
        assert!(!reads.budget_categories().is_empty());
        assert!(!reads.budget_accounts().is_empty());
        assert_eq!(reads.unit_code(UnitId::SEED_AUD).as_deref(), Some("aud"));
        assert!(reads.payee_name(PayeeId::seed(1)).is_some());
        assert!(reads.active_bill_plans().len() <= sample.bills.plans.len());
    }
}

/// `StoredDashboardReads`, run over the in-memory stores and the adapters with the same
/// assertions (#1023).
#[cfg(test)]
mod dashboard_reads_tests {
    use chrono::TimeZone;
    use lib_core::Period;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::{PendingWrites, Writer};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.from_utc_datetime(&today().and_hms_opt(12, 0, 0).expect("a real time"))
    }

    /// The Sample Ledger's rows in memory: the same rows the seed writes to the file.
    fn in_memory(with_default_budget: bool) -> StoredDashboardReads {
        let sample = crate::seed::sample_bills(today());
        let budgets = if with_default_budget {
            crate::seed::sample_budgets(today())
        } else {
            Vec::new()
        };
        StoredDashboardReads {
            budgets: Rc::new(RefCell::new(InMemoryBudgets { budgets })),
            budget_reads: StoredBudgetReads {
                bills: Rc::new(RefCell::new(InMemoryBills {
                    plans: sample.plans,
                    entries: sample.entries,
                })),
                categories: Rc::new(RefCell::new(InMemoryCategories::sample())),
                units: Rc::new(RefCell::new(InMemoryPorts::sample(now()))),
                accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::sample())),
                transactions: Rc::new(RefCell::new(InMemoryTransactions {
                    transactions: crate::seed::sample_transactions(today()),
                })),
                payees: Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts::sample())),
            },
        }
    }

    /// The Sample Ledger seeded into a test database, then loaded with fresh adapters as start-up does.
    fn on_adapters(
        options: SqliteConnectOptions,
        with_default_budget: bool,
    ) -> StoredDashboardReads {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        crate::seed::write_sample_rows(&writer, today(), false).expect("the sample is written");
        if !with_default_budget {
            writer
                .run(|pool| async move {
                    sqlx::query("UPDATE budgets SET is_default = 0")
                        .execute(&pool)
                        .await?;
                    Ok(())
                })
                .expect("the default is cleared");
        }
        let pending = PendingWrites::new(writer);
        let units = UnitsAdapter::load(pending.clone()).expect("the Units load");
        let categories = CategoriesAdapter::load(pending.clone()).expect("the Categories load");
        let accounts = AccountsAdapter::load(pending.clone()).expect("the Accounts load");
        let payees = PayeesAdapter::load(pending.clone()).expect("the Payees load");
        let transactions =
            TransactionsAdapter::load(pending.clone()).expect("the Transactions load");
        let bills = BillsAdapter::load(pending.clone()).expect("the Bills load");
        let budgets = BudgetsAdapter::load(pending).expect("the Budgets load");
        StoredDashboardReads {
            budgets: Rc::new(RefCell::new(budgets)),
            budget_reads: StoredBudgetReads {
                bills: Rc::new(RefCell::new(bills)),
                categories: Rc::new(RefCell::new(categories)),
                units: Rc::new(RefCell::new(units)),
                accounts: Rc::new(RefCell::new(accounts)),
                transactions: Rc::new(RefCell::new(transactions)),
                payees: Rc::new(RefCell::new(payees)),
            },
        }
    }

    /// Needs Attention rows come in `attention_entries` order with their Plan's name, Unit code and
    /// overdue mark.
    fn needs_attention_rows_follow_the_bills_rule(reads: &StoredDashboardReads) {
        let sample = crate::seed::sample_bills(today());
        let expected = lib_bills::attention_entries(&sample.plans, &sample.entries, today());
        assert!(
            !expected.is_empty(),
            "the sample has Bills needing attention"
        );

        let rows = reads.attention_bills(today());
        let ids: Vec<_> = rows
            .iter()
            .map(|row| (row.id.plan_id, row.id.due))
            .collect();
        let expected_ids: Vec<_> = expected.iter().map(|id| (id.plan_id, id.due)).collect();
        assert_eq!(ids, expected_ids);
        for row in &rows {
            let plan = lib_bills::get(&sample.plans, row.id.plan_id).expect("the Plan is stored");
            assert_eq!(row.plan, plan.name);
            assert_eq!(row.amount, plan.planned_amount);
            assert_eq!(row.overdue, row.id.due < today());
            assert_eq!(row.unit_code, "aud");
        }
        assert_eq!(reads.base_unit_code().as_deref(), Some("aud"));
    }

    /// The default Budget's current month has its fullest top-level bars, named by Category.
    fn the_default_budget_shows_its_bars_with_category_names(reads: &StoredDashboardReads) {
        let month = reads
            .default_budget_month(today())
            .expect("a Budget is the default");
        assert_eq!(month.month, Period::of(today()));
        assert_eq!(month.elapsed.day, 19);
        assert_eq!(month.elapsed.days_in_month, 30);
        assert_eq!(month.elapsed.percent, 63);
        assert!(!month.bars.is_empty() && month.bars.len() <= lib_budgets::DASHBOARD_BARS);
        assert!(month.bars.iter().all(|bar| !bar.category.is_empty()));
        assert!(
            month
                .bars
                .iter()
                .all(|bar| bar.over == (bar.spent.0 > bar.budget.0))
        );
        let share = |bar: &DashboardBudgetBar| bar.spent.0.clone() * 1_000 / bar.budget.0.clone();
        assert!(
            month
                .bars
                .windows(2)
                .all(|pair| share(&pair[0]) >= share(&pair[1])),
            "the fullest bar comes first"
        );
    }

    fn no_default_budget_shows_no_bars(reads: &StoredDashboardReads) {
        assert_eq!(reads.default_budget_month(today()), None);
    }

    #[test]
    fn over_the_in_memory_stores_needs_attention_follows_the_bills_rule() {
        needs_attention_rows_follow_the_bills_rule(&in_memory(true));
    }

    #[test]
    fn over_the_in_memory_stores_the_default_budget_shows_its_bars() {
        the_default_budget_shows_its_bars_with_category_names(&in_memory(true));
    }

    #[test]
    fn over_the_in_memory_stores_no_default_budget_shows_no_bars() {
        no_default_budget_shows_no_bars(&in_memory(false));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn over_the_adapters_needs_attention_follows_the_bills_rule(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        needs_attention_rows_follow_the_bills_rule(&on_adapters(options, true));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn over_the_adapters_the_default_budget_shows_its_bars(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        the_default_budget_shows_its_bars_with_category_names(&on_adapters(options, true));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn over_the_adapters_no_default_budget_shows_no_bars(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        no_default_budget_shows_no_bars(&on_adapters(options, false));
    }
}

/// `StoredShellReads`, run over the in-memory stores and the adapters with the same assertions
/// (#1026).
#[cfg(test)]
mod shell_reads_tests {
    use chrono::TimeZone;
    use lib_core::Period;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::{PendingWrites, Writer};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.from_utc_datetime(&today().and_hms_opt(12, 0, 0).expect("a real time"))
    }

    /// The Sample Ledger's rows in memory: the same rows the seed writes to the file.
    fn in_memory(with_default_budget: bool) -> StoredShellReads {
        let sample = crate::seed::sample_bills(today());
        let budgets = if with_default_budget {
            crate::seed::sample_budgets(today())
        } else {
            Vec::new()
        };
        StoredShellReads {
            budgets: Rc::new(RefCell::new(InMemoryBudgets { budgets })),
            documents: Rc::new(RefCell::new(InMemoryDocuments {
                documents: crate::seed::sample_documents(today(), &[]),
                types: lib_documents::types::default_types(),
            })),
            budget_reads: StoredBudgetReads {
                bills: Rc::new(RefCell::new(InMemoryBills {
                    plans: sample.plans,
                    entries: sample.entries,
                })),
                categories: Rc::new(RefCell::new(InMemoryCategories::sample())),
                accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::sample())),
                transactions: Rc::new(RefCell::new(InMemoryTransactions {
                    transactions: crate::seed::sample_transactions(today()),
                })),
                units: Rc::new(RefCell::new(InMemoryPorts::sample(now()))),
                payees: Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts::sample())),
            },
        }
    }

    /// The Sample Ledger seeded into a test database, Documents included, then loaded with fresh adapters as
    /// start-up does.
    fn on_adapters(options: SqliteConnectOptions, with_default_budget: bool) -> StoredShellReads {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        crate::seed::write_sample_rows(&writer, today(), true).expect("the sample is written");
        if !with_default_budget {
            writer
                .run(|pool| async move {
                    sqlx::query("UPDATE budgets SET is_default = 0")
                        .execute(&pool)
                        .await?;
                    Ok(())
                })
                .expect("the default is cleared");
        }
        let pending = PendingWrites::new(writer);
        let units = UnitsAdapter::load(pending.clone()).expect("the Units load");
        let categories = CategoriesAdapter::load(pending.clone()).expect("the Categories load");
        let accounts = AccountsAdapter::load(pending.clone()).expect("the Accounts load");
        let payees = PayeesAdapter::load(pending.clone()).expect("the Payees load");
        let transactions =
            TransactionsAdapter::load(pending.clone()).expect("the Transactions load");
        let bills = BillsAdapter::load(pending.clone()).expect("the Bills load");
        let documents = DocumentsAdapter::load(pending.clone()).expect("the Documents load");
        let budgets = BudgetsAdapter::load(pending).expect("the Budgets load");
        StoredShellReads {
            budgets: Rc::new(RefCell::new(budgets)),
            documents: Rc::new(RefCell::new(documents)),
            budget_reads: StoredBudgetReads {
                bills: Rc::new(RefCell::new(bills)),
                categories: Rc::new(RefCell::new(categories)),
                accounts: Rc::new(RefCell::new(accounts)),
                transactions: Rc::new(RefCell::new(transactions)),
                units: Rc::new(RefCell::new(units)),
                payees: Rc::new(RefCell::new(payees)),
            },
        }
    }

    /// Each count is its domain's rule over the sample: Needs Attention, the default Budget's
    /// over-budget Categories this month and the Unfiled Documents.
    fn each_count_is_its_domains_rule(reads: &StoredShellReads) {
        let sample = crate::seed::sample_bills(today());
        let attention = lib_bills::attention_entries(&sample.plans, &sample.entries, today()).len();
        assert!(attention > 0, "the sample has Bills needing attention");
        let inbox = lib_documents::inbox_count(&crate::seed::sample_documents(today(), &[]));
        assert!(inbox > 0, "the sample has Documents in the Inbox");
        let budgets = Budgets::from_stored(crate::seed::sample_budgets(today()));
        let budget = budgets
            .default_budget()
            .expect("a sample Budget is the default");
        let over =
            lib_budgets::period_figures(budget, &reads.budget_reads, Period::of(today()), today())
                .over_count;
        assert!(over > 0, "the sample's default Budget is over somewhere");

        assert_eq!(
            reads.rail_counts(today()),
            RailCounts {
                bills_attention: attention,
                budgets_over: over,
                documents_inbox: inbox,
            }
        );
    }

    fn no_default_budget_counts_none_over(reads: &StoredShellReads) {
        assert_eq!(reads.rail_counts(today()).budgets_over, 0);
    }

    /// Filing a Document takes it out of the Inbox count at once.
    fn filing_a_document_drops_the_inbox_count(reads: &StoredShellReads) {
        let before = reads.rail_counts(today()).documents_inbox;
        let unfiled = reads
            .documents
            .borrow()
            .documents()
            .documents()
            .into_iter()
            .find(|document| document.is_unfiled())
            .expect("a Document waits in the Inbox");
        let filed = lib_documents::Document {
            filing: lib_documents::Filing::Filed,
            ..unfiled
        };
        reads
            .documents
            .borrow_mut()
            .documents_mut()
            .upsert_document(&filed);

        assert_eq!(reads.rail_counts(today()).documents_inbox, before - 1);
    }

    #[test]
    fn over_the_in_memory_stores_each_count_is_its_domains_rule() {
        each_count_is_its_domains_rule(&in_memory(true));
    }

    #[test]
    fn over_the_in_memory_stores_no_default_budget_counts_none_over() {
        no_default_budget_counts_none_over(&in_memory(false));
    }

    #[test]
    fn over_the_in_memory_stores_filing_a_document_drops_the_inbox_count() {
        filing_a_document_drops_the_inbox_count(&in_memory(true));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn over_the_adapters_each_count_is_its_domains_rule(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        each_count_is_its_domains_rule(&on_adapters(options, true));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn over_the_adapters_no_default_budget_counts_none_over(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        no_default_budget_counts_none_over(&on_adapters(options, false));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn over_the_adapters_filing_a_document_drops_the_inbox_count(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        filing_a_document_drops_the_inbox_count(&on_adapters(options, true));
    }
}
