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
use lib_accounts::{AccountRepository, AccountUsage, AccountUse};
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
use lib_core::{
    AccountId, CategoryId, InstitutionId, PayeeId, TagId, Total, TransactionId, UnitId,
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
use lib_institutions::{InstitutionRepository, InstitutionUsage, InstitutionUse};
use lib_inventory::InventoryRepository;
use lib_inventory::fake::InMemoryInventory;
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
    accounts, bills, categories, institutions, inventory, payees, tags, transactions,
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

/// An Accounts store shared by the Ledger's forwarding shim and the Units and Bills reads.
/// Single-threaded, as the ports are.
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

/// A Categories store shared by the Ledger's forwarding shim and the Bills and Payees reads.
/// Single-threaded, as the ports are.
pub type SharedCategoriesStore = Rc<RefCell<dyn CategoriesStore>>;

/// The Categories repository port, wherever it is kept, and the delete Category reassign sends
/// until `lib-reassign` (#813). The reading modules map the Categories' own rows themselves
/// (#994).
pub trait CategoriesStore {
    fn categories(&self) -> &dyn CategoryRepository;

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository;

    /// Removes Category `id`, once nothing names it: Category reassign's delete, which the
    /// Desktop's forwarding shim sends. Not a port write, since the delete re-points other
    /// domains' rows (#813).
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

/// A Tags store shared by the Ledger's forwarding shim, the Tags page and the Tag Removal
/// adapter. Single-threaded, as the ports are.
pub type SharedTagsStore = Rc<RefCell<dyn TagsStore>>;

/// The Tags repository port, wherever it is kept, and the delete Tag remove and merge send. What
/// carries a Tag is other domains' rows, which [`StoredTagReads`] counts in their stores.
pub trait TagsStore {
    fn tags(&self) -> &dyn TagRepository;

    fn tags_mut(&mut self) -> &mut dyn TagRepository;

    /// Removes Tag `id`, once no Split carries it: Tag remove's and merge's delete, which the
    /// Desktop's forwarding shim sends until it hosts `lib-tag-removal` (#1003), whose
    /// [`TagRemovalAdapter`](crate::TagRemovalAdapter) sends it after the Splits. Not a
    /// `lib-tags` port write, since the delete untags other domains' rows.
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

/// An Inventory store shared by the Ledger's forwarding shim and read copy. Single-threaded, as
/// the ports are.
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

/// A Payees store shared by the Ledger's forwarding shim and the Bills reads. Single-threaded, as
/// the ports are.
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

/// A Budgets store shared by the Ledger's forwarding shim and the Accounts, Units and Categories
/// usage counts. Single-threaded, as the ports are.
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
}

/// `lib-budgets`' six read ports answered by the shared stores of the domains Budgets reads, each
/// borrowed for the one read (#952). The Desktop's Budgets View reads through it.
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

/// `lib-dashboard`'s two read ports answered by the shared stores of the domains the Dashboard
/// reads, each borrowed for the one read (#1023), as [`StoredBudgetReads`] answers Budgets'. It
/// calls `lib-bills`' Needs Attention rule and `lib-budgets`' period figures and bars and maps the
/// answers into `lib-dashboard`'s rows; it re-implements no rule and copies nothing from the
/// Ledger. The same struct stands over the adapters and [`Stores::fake`]'s in-memory stores.
#[derive(Clone)]
pub struct StoredDashboardReads {
    pub bills: SharedBillsStore,
    pub budgets: SharedBudgetsStore,
    pub categories: SharedCategoriesStore,
    pub units: SharedUnitsStore,
    /// The stores the Budget figures read besides the three above.
    pub accounts: SharedAccountsStore,
    pub transactions: SharedTransactionsStore,
    pub payees: SharedPayeesStore,
}

impl StoredDashboardReads {
    /// The Budgets read ports over these stores, for one figure.
    fn budget_reads(&self) -> StoredBudgetReads {
        StoredBudgetReads {
            categories: self.categories.clone(),
            accounts: self.accounts.clone(),
            transactions: self.transactions.clone(),
            bills: self.bills.clone(),
            units: self.units.clone(),
            payees: self.payees.clone(),
        }
    }
}

impl DashboardBills for StoredDashboardReads {
    fn attention_bills(&self, today: NaiveDate) -> Vec<DashboardBill> {
        let bills = self.bills.borrow();
        let (plans, entries) = (bills.plans().bill_plans(), bills.entries().entries());
        let units = self.units.borrow();
        lib_bills::attention_entries(&plans, &entries, today)
            .into_iter()
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
        let units = self.units.borrow();
        units.unit_code(units.base_unit().base_unit())
    }
}

impl DashboardBudgets for StoredDashboardReads {
    fn default_budget_month(&self, today: NaiveDate) -> Option<DashboardBudgetMonth> {
        let stored = Budgets::from_stored(self.budgets.borrow().budgets().budgets());
        let budget = stored.default_budget()?;
        let reads = self.budget_reads();
        let figures =
            lib_budgets::period_figures(budget, &reads, lib_core::Period::of(today), today);
        let categories = self.categories.borrow().categories().categories();
        let bars = lib_budgets::dashboard_bars(&figures, &reads)
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
    pub bills: SharedBillsStore,
    pub budgets: SharedBudgetsStore,
    pub documents: SharedDocumentsStore,
    /// The stores the Budget figures read besides the two above.
    pub categories: SharedCategoriesStore,
    pub accounts: SharedAccountsStore,
    pub transactions: SharedTransactionsStore,
    pub units: SharedUnitsStore,
    pub payees: SharedPayeesStore,
}

impl ShellRailCounts for StoredShellReads {
    fn rail_counts(&self, today: NaiveDate) -> RailCounts {
        let bills_attention = {
            let bills = self.bills.borrow();
            lib_bills::attention_entries(
                &bills.plans().bill_plans(),
                &bills.entries().entries(),
                today,
            )
            .len()
        };
        let stored = Budgets::from_stored(self.budgets.borrow().budgets().budgets());
        let budgets_over = stored.default_budget().map_or(0, |budget| {
            let reads = StoredBudgetReads {
                categories: self.categories.clone(),
                accounts: self.accounts.clone(),
                transactions: self.transactions.clone(),
                bills: self.bills.clone(),
                units: self.units.clone(),
                payees: self.payees.clone(),
            };
            lib_budgets::period_figures(budget, &reads, lib_core::Period::of(today), today)
                .over_count
        });
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

/// `lib-institutions`' `InstitutionUsage` answered by the Accounts store, borrowed for the one
/// read (#994): the Accounts naming the Institution.
#[derive(Clone)]
pub struct StoredInstitutionReads {
    pub accounts: SharedAccountsStore,
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

/// `lib-accounts`' `AccountUsage` answered by the shared stores of the domains that name an
/// Account, each borrowed for the one read (#993), as [`StoredPayeeReads`] answers Payees'. The
/// same struct stands over [`Stores::fake`]'s in-memory stores and the adapters. The Transactions
/// store counts the Transactions booked to the Account and the Budgets store the Budgets that
/// count it.
#[derive(Clone)]
pub struct StoredAccountReads {
    pub transactions: SharedTransactionsStore,
    pub budgets: SharedBudgetsStore,
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
    /// Every domain's stub seed as of `today`, with the stub Ledger's Transactions and each
    /// Account's usage counted from them and the stub Budgets. For a Client that runs on the stub without a Ledger file.
    pub fn stub(today: NaiveDate) -> Self {
        let ledger = lib_ledger::Ledger::stub_on(today);
        let budgets = InMemoryBudgets::stub(today);
        let mut accounts = lib_accounts::fake::InMemoryPorts::stub();
        accounts.usage.uses = ledger
            .accounts()
            .iter()
            .map(|account| {
                let uses = AccountUse {
                    transactions: ledger.account_transactions(account.id),
                    budgets: budgets.account_budgets(account.id),
                };
                (account.id, uses)
            })
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
            budgets,
            inventory: InMemoryInventory::stub(today),
            documents: InMemoryDocuments {
                documents: ledger.stub_documents(today),
                types: lib_documents::types::default_types(),
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
            units: InMemoryPorts::stub(chrono::Utc::now()),
            institutions: InMemoryInstitutions::stub(),
            accounts: lib_accounts::fake::InMemoryPorts::stub(),
            categories: InMemoryCategories::stub(),
            tags: lib_tags::fake::InMemoryPorts::stub(),
            payees: lib_payees::fake::InMemoryPorts::stub(),
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
        let ledger = lib_ledger::Ledger::stub();
        let transactions: SharedTransactionsStore = Rc::new(RefCell::new(InMemoryTransactions {
            transactions: ledger.transactions().to_vec(),
        }));
        let reads = StoredPayeeReads {
            transactions: transactions.clone(),
            bills: Rc::new(RefCell::new(InMemoryBills {
                plans: ledger.bill_plans().to_vec(),
                entries: Vec::new(),
            })),
            categories: Rc::new(RefCell::new(InMemoryCategories::stub())),
        };
        let (woolworths, telstra) = (PayeeId::seed(1), PayeeId::seed(20));
        let before = reads.payee_usage(woolworths).expect("memory always counts");
        assert!(before.splits > 0, "the stub's groceries name Woolworths");
        assert_eq!(
            reads.payee_usage(telstra).map(|uses| uses.bill_plans),
            Some(1)
        );

        let mut booked = ledger.transactions()[0].clone();
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

    fn in_memory_stub_stores() -> (
        lib_ledger::Ledger,
        SharedTransactionsStore,
        SharedBillsStore,
    ) {
        let ledger = lib_ledger::Ledger::stub();
        let transactions: SharedTransactionsStore = Rc::new(RefCell::new(InMemoryTransactions {
            transactions: ledger.transactions().to_vec(),
        }));
        let bills: SharedBillsStore = Rc::new(RefCell::new(InMemoryBills {
            plans: ledger.bill_plans().to_vec(),
            entries: ledger.bill_entries().to_vec(),
        }));
        (ledger, transactions, bills)
    }

    /// An Account's use is counted in the Transactions and Budgets stores on each ask.
    #[test]
    fn stored_account_reads_count_the_in_memory_stores_rows() {
        let (ledger, transactions, _) = in_memory_stub_stores();
        let budgets: SharedBudgetsStore = Rc::new(RefCell::new(InMemoryBudgets {
            budgets: ledger.budgets().all().to_vec(),
        }));
        let reads = StoredAccountReads {
            transactions: transactions.clone(),
            budgets,
        };
        let everyday = AccountId::seed(2);
        let before = reads.account_usage(everyday);
        assert!(before.transactions > 0);
        assert_eq!(before.budgets, 2);
        assert_eq!(reads.account_usage(AccountId::new()), AccountUse::default());

        let mut booked = ledger.transactions()[0].clone();
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
        let (ledger, transactions, _) = in_memory_stub_stores();
        let accounts: SharedAccountsStore =
            Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::stub()));
        let categories: SharedCategoriesStore = Rc::new(RefCell::new(InMemoryCategories::stub()));
        let payees: SharedPayeesStore =
            Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts::stub()));
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
            ledger
                .transactions()
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
        let named = StoredInstitutionReads { accounts }.institution_usage(institution);
        assert!(named.is_used());
        assert_eq!(
            StoredInstitutionReads {
                accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::default())),
            }
            .institution_usage(institution),
            InstitutionUse::default()
        );

        let tree = StoredBudgetCategories { categories }.budget_categories();
        assert_eq!(tree.len(), stored_categories.len());
    }

    /// A Unit's use and holdings are counted in the Accounts, Transactions, Bills and Budgets
    /// stores.
    #[test]
    fn stored_unit_reads_count_the_in_memory_stores_rows() {
        let (ledger, transactions, bills) = in_memory_stub_stores();
        let accounts: SharedAccountsStore =
            Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts {
                accounts: lib_accounts::fake::InMemoryAccounts {
                    accounts: ledger.accounts().to_vec(),
                },
                ..Default::default()
            }));
        let reads = StoredUnitReads {
            accounts,
            transactions,
            bills,
            budgets: Rc::new(RefCell::new(InMemoryBudgets {
                budgets: ledger.budgets().all().to_vec(),
            })),
            inventory: Rc::new(RefCell::new(InMemoryInventory::default())),
            institutions: Rc::new(RefCell::new(InMemoryInstitutions::stub())),
        };
        let aud = UnitId::SEED_AUD;

        let usage = reads.unit_usage(aud);
        assert_eq!(
            usage.accounts,
            ledger.accounts().iter().filter(|a| a.unit == aud).count()
        );
        assert_eq!(
            usage.bill_plans,
            ledger.bill_plans().iter().filter(|p| p.unit == aud).count()
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
                ledger
                    .transactions()
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

    /// Bills read a Unit's code and the base Unit from the Units store.
    #[test]
    fn stored_bill_units_read_the_units_store() {
        let units: SharedUnitsStore =
            Rc::new(RefCell::new(InMemoryPorts::stub(chrono::Utc::now())));
        let reads = StoredBillUnits { units };
        assert_eq!(reads.unit_code(UnitId::SEED_AUD).as_deref(), Some("aud"));
        assert_eq!(reads.unit_code(UnitId::new()), None);
        assert_eq!(reads.base_unit(), Some(UnitId::SEED_AUD));
    }

    /// The six Budgets read ports are answered by their stores, as the Ledger once answered them.
    #[test]
    fn stored_budget_reads_answer_each_port_from_its_store() {
        let (ledger, transactions, bills) = in_memory_stub_stores();
        let reads = StoredBudgetReads {
            categories: Rc::new(RefCell::new(InMemoryCategories::stub())),
            accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::stub())),
            transactions,
            bills,
            units: Rc::new(RefCell::new(InMemoryPorts::stub(chrono::Utc::now()))),
            payees: Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts::stub())),
        };
        let month = lib_core::Period::of(NaiveDate::from_ymd_opt(2026, 9, 21).expect("a date"));

        let splits = reads.budget_splits(month.first_day(), month.last_day());
        let expected: usize = ledger
            .transactions()
            .iter()
            .filter(|t| month.contains(t.date))
            .map(|t| t.splits.len())
            .sum();
        assert!(expected > 0);
        assert_eq!(splits.len(), expected);
        assert!(splits.iter().all(|split| month.contains(split.date)));

        let open = ledger
            .bill_entries()
            .iter()
            .filter(|entry| entry.is_open())
            .count();
        assert_eq!(reads.open_bills().len(), open);
        assert!(!reads.budget_categories().is_empty());
        assert!(!reads.budget_accounts().is_empty());
        assert_eq!(reads.unit_code(UnitId::SEED_AUD).as_deref(), Some("aud"));
        assert!(reads.payee_name(PayeeId::seed(1)).is_some());
        assert!(reads.active_bill_plans().len() <= ledger.bill_plans().len());
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

    /// The stub's rows in memory: the same rows the stub seed writes to the file.
    fn in_memory(with_default_budget: bool) -> StoredDashboardReads {
        let stub = crate::seed::stub_bills(today());
        let budgets = if with_default_budget {
            crate::seed::stub_budgets(today())
        } else {
            Vec::new()
        };
        StoredDashboardReads {
            bills: Rc::new(RefCell::new(InMemoryBills {
                plans: stub.plans,
                entries: stub.entries,
            })),
            budgets: Rc::new(RefCell::new(InMemoryBudgets { budgets })),
            categories: Rc::new(RefCell::new(InMemoryCategories::stub())),
            units: Rc::new(RefCell::new(InMemoryPorts::stub(now()))),
            accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::stub())),
            transactions: Rc::new(RefCell::new(InMemoryTransactions {
                transactions: crate::seed::stub_transactions(today()),
            })),
            payees: Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts::stub())),
        }
    }

    /// The stub seeded into a test database, then loaded with fresh adapters as start-up does.
    fn on_adapters(
        options: SqliteConnectOptions,
        with_default_budget: bool,
    ) -> StoredDashboardReads {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        crate::seed::write_stub_rows(&writer, now(), today(), None, false)
            .expect("the stub is written");
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
            bills: Rc::new(RefCell::new(bills)),
            budgets: Rc::new(RefCell::new(budgets)),
            categories: Rc::new(RefCell::new(categories)),
            units: Rc::new(RefCell::new(units)),
            accounts: Rc::new(RefCell::new(accounts)),
            transactions: Rc::new(RefCell::new(transactions)),
            payees: Rc::new(RefCell::new(payees)),
        }
    }

    /// Needs Attention rows come in `attention_entries` order with their Plan's name, Unit code and
    /// overdue mark.
    fn needs_attention_rows_follow_the_bills_rule(reads: &StoredDashboardReads) {
        let stub = crate::seed::stub_bills(today());
        let expected = lib_bills::attention_entries(&stub.plans, &stub.entries, today());
        assert!(!expected.is_empty(), "the stub has Bills needing attention");

        let rows = reads.attention_bills(today());
        let ids: Vec<_> = rows
            .iter()
            .map(|row| (row.id.plan_id, row.id.due))
            .collect();
        let expected_ids: Vec<_> = expected.iter().map(|id| (id.plan_id, id.due)).collect();
        assert_eq!(ids, expected_ids);
        for row in &rows {
            let plan = lib_bills::get(&stub.plans, row.id.plan_id).expect("the Plan is stored");
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

    /// The stub's rows in memory: the same rows the stub seed writes to the file.
    fn in_memory(with_default_budget: bool) -> StoredShellReads {
        let stub = crate::seed::stub_bills(today());
        let budgets = if with_default_budget {
            crate::seed::stub_budgets(today())
        } else {
            Vec::new()
        };
        StoredShellReads {
            bills: Rc::new(RefCell::new(InMemoryBills {
                plans: stub.plans,
                entries: stub.entries,
            })),
            budgets: Rc::new(RefCell::new(InMemoryBudgets { budgets })),
            documents: Rc::new(RefCell::new(InMemoryDocuments {
                documents: crate::seed::stub_documents(today(), &[]),
                types: lib_documents::types::default_types(),
            })),
            categories: Rc::new(RefCell::new(InMemoryCategories::stub())),
            accounts: Rc::new(RefCell::new(lib_accounts::fake::InMemoryPorts::stub())),
            transactions: Rc::new(RefCell::new(InMemoryTransactions {
                transactions: crate::seed::stub_transactions(today()),
            })),
            units: Rc::new(RefCell::new(InMemoryPorts::stub(now()))),
            payees: Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts::stub())),
        }
    }

    /// The stub seeded into a test database, Documents included, then loaded with fresh adapters as
    /// start-up does.
    fn on_adapters(options: SqliteConnectOptions, with_default_budget: bool) -> StoredShellReads {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        crate::seed::write_stub_rows(&writer, now(), today(), None, true)
            .expect("the stub is written");
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
            bills: Rc::new(RefCell::new(bills)),
            budgets: Rc::new(RefCell::new(budgets)),
            documents: Rc::new(RefCell::new(documents)),
            categories: Rc::new(RefCell::new(categories)),
            accounts: Rc::new(RefCell::new(accounts)),
            transactions: Rc::new(RefCell::new(transactions)),
            units: Rc::new(RefCell::new(units)),
            payees: Rc::new(RefCell::new(payees)),
        }
    }

    /// Each count is its domain's rule over the stub: Needs Attention, the default Budget's
    /// over-budget Categories this month and the Unfiled Documents.
    fn each_count_is_its_domains_rule(reads: &StoredShellReads) {
        let stub = crate::seed::stub_bills(today());
        let attention = lib_bills::attention_entries(&stub.plans, &stub.entries, today()).len();
        assert!(attention > 0, "the stub has Bills needing attention");
        let inbox = lib_documents::inbox_count(&crate::seed::stub_documents(today(), &[]));
        assert!(inbox > 0, "the stub has Documents in the Inbox");
        let budgets = Budgets::from_stored(crate::seed::stub_budgets(today()));
        let budget = budgets
            .default_budget()
            .expect("a stub Budget is the default");
        let over = lib_budgets::period_figures(
            budget,
            &StoredBudgetReads {
                categories: reads.categories.clone(),
                accounts: reads.accounts.clone(),
                transactions: reads.transactions.clone(),
                bills: reads.bills.clone(),
                units: reads.units.clone(),
                payees: reads.payees.clone(),
            },
            Period::of(today()),
            today(),
        )
        .over_count;
        assert!(over > 0, "the stub's default Budget is over somewhere");

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
