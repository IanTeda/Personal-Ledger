//! Headless harness for the Desktop UI: repeats `lib::run`'s app-level setup, builds `Root`
//! through the lib's `shell::router::build` with a fixed date and the deterministic stub seed, and offers
//! key presses, selector clicks and read helpers. Nothing here opens a real window or touches
//! `persistence`, so it runs in a sandbox with no GPU.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test support: a failed lookup or invalid fixed date should fail the test loudly"
)]

use bin_desktop::{
    WindowSize, WindowSizeExt,
    locale::init_for_tests,
    navigation::nav::{Noun, RailMode},
    persistence::PersistedState,
    shell::router::{
        self, BillsSnapshot, BudgetsSnapshot, DashboardBillSnapshot, DocumentsSnapshot, HelpGroup,
        ImportSnapshot, Root, SettingsAccountsSnapshot, SettingsCategoriesSnapshot,
        SettingsDisplaySnapshot, SettingsDocumentTypesSnapshot, SettingsGeneralSnapshot,
        SettingsInstitutionsSnapshot, SettingsInventorySnapshot, SettingsPayeesSnapshot,
        SettingsSnapshot, SettingsTagsSnapshot, SettingsTracingSnapshot, SettingsUnitsSnapshot,
        ShellBindings, ShellLogs, StatusSnapshot, ToastsSnapshot, TransactionsSnapshot,
    },
    theme,
};
use chrono::NaiveDate;
use gpui::{
    App, Bounds, Entity, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point,
    ScrollDelta, ScrollWheelEvent, Size, TestAppContext, VisualTestContext,
};
use lib_accounts::fake::{InMemoryAccountUsage, InMemoryAccounts};
use lib_accounts::{Account, AccountRepository};
use lib_bills::fake::InMemoryBills;
use lib_bills::{BillPlan, BillPlanRepository, BillScheduleEntry, BillScheduleRepository};
use lib_categories::fake::InMemoryCategories;
use lib_categories::{Category, CategoryRepository};
use lib_colour_theme::ThemeOverrides;
use lib_core::TransactionId;
use lib_core::{AccountId, CategoryId, InstitutionId, PayeeId, PriceSourceId, TagId, UnitId};
use lib_database::{
    AccountsStore, BillsStore, CategoriesStore, Commit, CountingCommit, Domain, FakeStores,
    InstitutionsStore, InventoryStore, PayeesStore, Stores, TagsStore, TransactionsStore,
    UnitsStore, WriteFailure,
};
use lib_institutions::fake::InMemoryInstitutions;
use lib_institutions::{Institution, InstitutionRepository};
use lib_inventory::fake::InMemoryInventory;
use lib_inventory::{InventoryRepository, Item, Location};
use lib_payees::{Payee, PayeeRepository, PayeeUsage};
use lib_tags::{Tag, TagRepository, TagUsage};
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{Transaction, TransactionRepository};
use lib_units::fake::{InMemoryBaseUnit, InMemoryPorts, InMemoryPriceSources, InMemoryUnits};
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRecord, PriceSourceRepository, UnitRepository,
    UnitRow, UnitUsage,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

/// The configuration `main` would hold, pointed at the Ledger file `path`, for a smoke test on
/// the real start-up.
pub fn config_for(path: &std::path::Path) -> lib_config::Config {
    let mut config = lib_config::Config::default();
    lib_config::ConfigArgs {
        file: Some(path.to_path_buf()),
        ..Default::default()
    }
    .apply_overrides(&mut config);
    config
}

/// `stores` with its commit counted, so a test asserts one per input.
fn counted(mut stores: Stores) -> (Stores, Rc<Cell<usize>>) {
    let commit = CountingCommit::new(stores.commit);
    let count = commit.count();
    stores.commit = Rc::new(commit);
    (stores, count)
}

/// Marks `written` on every write through it, so the commit after it knows there is one to fail.
struct Marked<T> {
    inner: T,
    written: Rc<Cell<bool>>,
}

impl<T: UnitRepository> UnitRepository for Marked<T> {
    fn units(&self) -> Vec<UnitRow> {
        self.inner.units()
    }

    fn upsert_unit(&mut self, unit: UnitRow) {
        self.written.set(true);
        self.inner.upsert_unit(unit);
    }

    fn delete_unit(&mut self, id: UnitId) {
        self.written.set(true);
        self.inner.delete_unit(id);
    }
}

impl<T: PriceSourceRepository> PriceSourceRepository for Marked<T> {
    fn price_source_records(&self) -> Vec<PriceSourceRecord> {
        self.inner.price_source_records()
    }

    fn upsert_price_source(&mut self, record: PriceSourceRecord) {
        self.written.set(true);
        self.inner.upsert_price_source(record);
    }

    fn delete_price_source(&mut self, id: PriceSourceId) {
        self.written.set(true);
        self.inner.delete_price_source(id);
    }
}

impl<T: BaseUnitRepository> BaseUnitRepository for Marked<T> {
    fn base_unit(&self) -> UnitId {
        self.inner.base_unit()
    }

    fn set_base_unit(&mut self, id: UnitId) {
        self.written.set(true);
        self.inner.set_base_unit(id);
    }
}

impl<T: TransactionRepository> TransactionRepository for Marked<T> {
    fn transactions(&self) -> Vec<Transaction> {
        self.inner.transactions()
    }

    fn upsert_transaction(&mut self, transaction: Transaction) {
        self.written.set(true);
        self.inner.upsert_transaction(transaction);
    }

    fn delete_transaction(&mut self, id: TransactionId) {
        self.written.set(true);
        self.inner.delete_transaction(id);
    }
}

impl<T: PayeeRepository> PayeeRepository for Marked<T> {
    fn payees(&self) -> Vec<Payee> {
        self.inner.payees()
    }

    fn upsert_payee(&mut self, payee: Payee) {
        self.written.set(true);
        self.inner.upsert_payee(payee);
    }

    fn delete_payee(&mut self, id: PayeeId) {
        self.written.set(true);
        self.inner.delete_payee(id);
    }
}

impl<T: InstitutionRepository> InstitutionRepository for Marked<T> {
    fn institutions(&self) -> Vec<Institution> {
        self.inner.institutions()
    }

    fn upsert_institution(&mut self, institution: Institution) {
        self.written.set(true);
        self.inner.upsert_institution(institution);
    }

    fn delete_institution(&mut self, id: InstitutionId) {
        self.written.set(true);
        self.inner.delete_institution(id);
    }
}

/// An Institutions port whose writes never reach `stored`, the rows as they are on disk: a reload
/// puts memory back to `stored`. Each write marks `written` for [`FailingCommit`].
struct FailingInstitutions {
    institutions: Marked<InMemoryInstitutions>,
    stored: InMemoryInstitutions,
}

impl InstitutionsStore for FailingInstitutions {
    fn institutions(&self) -> &dyn InstitutionRepository {
        &self.institutions
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        &mut self.institutions
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.institutions.inner = self.stored.clone();
        Ok(())
    }
}

impl<T: AccountRepository> AccountRepository for Marked<T> {
    fn accounts(&self) -> Vec<Account> {
        self.inner.accounts()
    }

    fn upsert_account(&mut self, account: Account) {
        self.written.set(true);
        self.inner.upsert_account(account);
    }

    fn delete_account(&mut self, id: AccountId) {
        self.written.set(true);
        self.inner.delete_account(id);
    }
}

/// An Accounts store whose writes never reach `stored`, the rows as they are on disk: a reload
/// puts memory back to `stored`. Each write marks `written` for [`FailingCommit`]. Its usage is
/// in memory, recounted from the Ledger by its holder.
struct FailingAccounts {
    accounts: Marked<InMemoryAccounts>,
    usage: InMemoryAccountUsage,
    stored: InMemoryAccounts,
}

impl AccountsStore for FailingAccounts {
    fn accounts(&self) -> &dyn AccountRepository {
        &self.accounts
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    fn usage(&self) -> &dyn lib_accounts::AccountUsage {
        &self.usage
    }

    fn usage_to_count(&mut self) -> Option<&mut InMemoryAccountUsage> {
        Some(&mut self.usage)
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.accounts.inner = self.stored.clone();
        Ok(())
    }
}

/// A Payees store whose usage counts every Payee as unused, as a count taken before a Split
/// naming one landed would, so a delete it allows reaches the file.
struct CountedUnusedPayees {
    inner: Box<dyn PayeesStore>,
    usage: lib_payees::fake::InMemoryPayeeUsage,
}

impl PayeesStore for CountedUnusedPayees {
    fn payees(&self) -> &dyn PayeeRepository {
        self.inner.payees()
    }

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        self.inner.payees_mut()
    }

    fn usage(&self) -> &dyn PayeeUsage {
        &self.usage
    }

    fn split_mut(&mut self) -> lib_payees::PortsMut<'_> {
        lib_payees::PortsMut {
            payees: self.inner.payees_mut(),
            usage: &self.usage,
        }
    }

    fn bill_payees(&self) -> Vec<lib_bills::BillPayee> {
        self.inner.bill_payees()
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.inner.reload()
    }
}

impl<T: CategoryRepository> CategoryRepository for Marked<T> {
    fn categories(&self) -> Vec<Category> {
        self.inner.categories()
    }

    fn upsert_category(&mut self, category: Category) {
        self.written.set(true);
        self.inner.upsert_category(category);
    }
}

/// A Categories store whose writes never reach `stored`, the rows as they are on disk: a reload
/// puts memory back to `stored`. Each write marks `written` for [`FailingCommit`].
struct FailingCategories {
    categories: Marked<InMemoryCategories>,
    stored: InMemoryCategories,
}

impl CategoriesStore for FailingCategories {
    fn categories(&self) -> &dyn CategoryRepository {
        &self.categories
    }

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository {
        &mut self.categories
    }

    fn delete_category(&mut self, id: CategoryId) {
        self.categories.written.set(true);
        self.categories.inner.delete_category(id);
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.categories.inner = self.stored.clone();
        Ok(())
    }
}

impl<T: TagRepository> TagRepository for Marked<T> {
    fn tags(&self) -> Vec<Tag> {
        self.inner.tags()
    }

    fn upsert_tag(&mut self, tag: Tag) {
        self.written.set(true);
        self.inner.upsert_tag(tag);
    }
}

/// A Tags store whose writes never reach `stored`, the rows as they are on disk: a reload puts
/// memory back to `stored`. Each write marks `written` for [`FailingCommit`].
struct FailingTags {
    tags: Marked<lib_tags::fake::InMemoryTags>,
    usage: lib_tags::fake::InMemoryTagUsage,
    stored: lib_tags::fake::InMemoryTags,
}

impl TagsStore for FailingTags {
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
        self.tags.written.set(true);
        self.tags.inner.tags.retain(|tag| tag.id != id);
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.tags.inner = self.stored.clone();
        Ok(())
    }
}

impl<T: InventoryRepository> InventoryRepository for Marked<T> {
    fn locations(&self) -> Vec<Location> {
        self.inner.locations()
    }

    fn items(&self) -> Vec<Item> {
        self.inner.items()
    }

    fn upsert_location(&mut self, location: Location) {
        self.written.set(true);
        self.inner.upsert_location(location);
    }

    fn upsert_item(&mut self, item: Item) {
        self.written.set(true);
        self.inner.upsert_item(item);
    }

    fn delete_item(&mut self, id: lib_core::ItemId) {
        self.written.set(true);
        self.inner.delete_item(id);
    }
}

/// An Inventory store whose writes never reach `stored`, the rows as they are on disk: a reload
/// puts memory back to `stored`. Each write marks `written` for [`FailingCommit`].
struct FailingInventory {
    inventory: Marked<InMemoryInventory>,
    stored: InMemoryInventory,
}

impl InventoryStore for FailingInventory {
    fn inventory(&self) -> &dyn InventoryRepository {
        &self.inventory
    }

    fn inventory_mut(&mut self) -> &mut dyn InventoryRepository {
        &mut self.inventory
    }

    fn delete_location(&mut self, id: lib_core::LocationId) {
        self.inventory.written.set(true);
        InventoryStore::delete_location(&mut self.inventory.inner, id);
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.inventory.inner = self.stored.clone();
        Ok(())
    }
}

/// A Payees port whose writes never reach `stored`, the rows as they are on disk: a reload puts
/// memory back to `stored`. Each write marks `written` for [`FailingCommit`].
struct FailingPayees {
    payees: Marked<lib_payees::fake::InMemoryPayees>,
    usage: lib_payees::fake::InMemoryPayeeUsage,
    stored: lib_payees::fake::InMemoryPayees,
}

impl PayeesStore for FailingPayees {
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
        lib_payees::PortsMut {
            payees: &mut self.payees,
            usage: &self.usage,
        }
    }

    fn usage_to_count(&mut self) -> Option<&mut lib_payees::fake::InMemoryPayeeUsage> {
        Some(&mut self.usage)
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.payees.inner = self.stored.clone();
        Ok(())
    }
}

/// A Transactions port whose writes never reach `stored`, the rows as they are on disk: a reload
/// puts memory back to `stored`. Each write marks `written` for [`FailingCommit`].
struct FailingTransactions {
    transactions: Marked<InMemoryTransactions>,
    stored: InMemoryTransactions,
}

impl TransactionsStore for FailingTransactions {
    fn transactions(&self) -> &dyn TransactionRepository {
        &self.transactions
    }

    fn transactions_mut(&mut self) -> &mut dyn TransactionRepository {
        &mut self.transactions
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.transactions.inner = self.stored.clone();
        Ok(())
    }
}

impl<T: BillPlanRepository> BillPlanRepository for Marked<T> {
    fn bill_plans(&self) -> Vec<BillPlan> {
        self.inner.bill_plans()
    }

    fn upsert_bill_plan(&mut self, plan: BillPlan) {
        self.written.set(true);
        self.inner.upsert_bill_plan(plan);
    }
}

impl<T: BillScheduleRepository> BillScheduleRepository for Marked<T> {
    fn entries(&self) -> Vec<BillScheduleEntry> {
        self.inner.entries()
    }

    fn upsert_entry(&mut self, entry: BillScheduleEntry) {
        self.written.set(true);
        self.inner.upsert_entry(entry);
    }
}

/// Bills ports whose writes never reach `stored`, the rows as they are on disk: a reload puts
/// memory back to `stored`. Each write marks `written` for [`FailingCommit`].
struct FailingBills {
    bills: Marked<InMemoryBills>,
    stored: InMemoryBills,
}

impl BillsStore for FailingBills {
    fn plans(&self) -> &dyn BillPlanRepository {
        &self.bills
    }

    fn plans_mut(&mut self) -> &mut dyn BillPlanRepository {
        &mut self.bills
    }

    fn entries(&self) -> &dyn BillScheduleRepository {
        &self.bills
    }

    fn entries_mut(&mut self) -> &mut dyn BillScheduleRepository {
        &mut self.bills
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.bills.inner = self.stored.clone();
        Ok(())
    }
}

/// The stub Ledger's Bill Plans and Bill Schedule as of the test date, as the in-memory ports
/// hold them.
fn stub_bills() -> InMemoryBills {
    let ledger = lib_ledger::Ledger::stub_on(today());
    InMemoryBills {
        plans: ledger.bill_plans().to_vec(),
        entries: ledger.bill_entries().to_vec(),
    }
}

/// The stub Ledger's Inventory as of the test date, as the in-memory port holds it.
fn stub_inventory() -> InMemoryInventory {
    InMemoryInventory::stub(today())
}

/// Every domain's stub in memory, as [`Harness::new`] runs on, for a test to override one of.
fn stub_fakes() -> FakeStores {
    FakeStores {
        units: InMemoryPorts::stub(chrono::Utc::now()),
        institutions: InMemoryInstitutions::stub(),
        accounts: lib_accounts::fake::InMemoryPorts::stub(),
        categories: InMemoryCategories::stub(),
        tags: lib_tags::fake::InMemoryPorts::stub(),
        payees: lib_payees::fake::InMemoryPorts::stub(),
        transactions: stub_transactions(),
        bills: stub_bills(),
        inventory: stub_inventory(),
    }
}

/// The stub Ledger's Transactions as of the test date, as the in-memory port holds them.
fn stub_transactions() -> InMemoryTransactions {
    InMemoryTransactions {
        transactions: lib_ledger::Ledger::stub_on(today()).transactions().to_vec(),
    }
}

/// Units ports whose writes never reach `stored`, the rows as they are on disk: a reload puts
/// memory back to `stored`. Each write marks `written` for [`FailingCommit`].
struct FailingUnits {
    units: Marked<InMemoryUnits>,
    price_sources: Marked<InMemoryPriceSources>,
    base_unit: Marked<InMemoryBaseUnit>,
    stored: InMemoryPorts,
    reload_fails: bool,
}

impl FailingUnits {
    fn new(stored: InMemoryPorts, written: &Rc<Cell<bool>>, reload_fails: bool) -> Self {
        Self {
            units: Marked {
                inner: stored.units.clone(),
                written: written.clone(),
            },
            price_sources: Marked {
                inner: stored.price_sources.clone(),
                written: written.clone(),
            },
            base_unit: Marked {
                inner: stored.base_unit,
                written: written.clone(),
            },
            stored,
            reload_fails,
        }
    }
}

impl UnitsStore for FailingUnits {
    fn units(&self) -> &dyn UnitRepository {
        &self.units
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        &self.price_sources
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        &self.base_unit
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage,
        }
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        if self.reload_fails {
            return Err(lib_database::Error::Generic(
                "the test's reload fails".into(),
            ));
        }
        self.units.inner = self.stored.units.clone();
        self.price_sources.inner = self.stored.price_sources.clone();
        self.base_unit.inner = self.stored.base_unit;
        Ok(())
    }
}

/// A commit with writes since the last reports a failure naming `domains`, as the writer would
/// when SQLite refuses them; a commit with nothing new reports nothing.
struct FailingCommit {
    domains: Vec<Domain>,
    written: Rc<Cell<bool>>,
    report: mpsc::Sender<WriteFailure>,
}

impl Commit for FailingCommit {
    fn commit(&self) {
        if self.written.replace(false) {
            let _ = self.report.send(WriteFailure {
                domains: self.domains.clone(),
                error: lib_database::Error::Generic("the test's write fails".into()),
            });
        }
    }
}

/// The date every test is anchored to, so scopes and seeded relative dates never drift.
/// The default `[keybindings]` with `command` bound to `chord`.
pub fn overriding(command: &str, chord: &str) -> lib_config::KeyBindingConfig {
    let mut config = lib_config::KeyBindingConfig::default();
    config
        .bindings
        .insert(command.to_string(), chord.to_string());
    config
}

pub fn today() -> NaiveDate {
    date(2026, 6, 15)
}

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("fixed test date is valid")
}

/// A `Root` in a headless window, with helpers to drive and read it.
pub struct Harness<'a> {
    pub shell: Entity<Root>,
    pub cx: &'a mut VisualTestContext,
    /// The Shell's live log capture: push entries here, then [`Harness::settle_logs`].
    pub logs: lib_tracing::LogBuffer,
    /// The day the Shell's clock reads, [`today`] until [`Harness::turn_date_to`] moves it.
    date: Rc<Cell<NaiveDate>>,
    /// How many times the Shell has committed, once per input (`input::send`).
    commits: Rc<Cell<usize>>,
}

/// The open Add or Edit account dialog's form: plain values, as `Harness::account_form` reads
/// them back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountFormSnapshot {
    /// The focused field, as its `AccountField` variant name.
    pub focused: String,
    pub name: String,
    pub institution: Option<String>,
    pub account_type: Option<String>,
    pub unit: Option<String>,
    pub opening_balance: String,
    pub account_number: String,
    /// Whether any select's option list is showing.
    pub select_open: bool,
    /// Whether the form would confirm.
    pub valid: bool,
}

impl<'a> Harness<'a> {
    /// Boots the app-level globals and a `Root` with default (nothing persisted) state.
    pub fn new(app: &'a mut TestAppContext) -> Self {
        Self::with_logs(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
        )
    }

    /// As [`Harness::new`], with a log capture already holding entries and a configured `log`
    /// level, as the app would have at start.
    pub fn with_logs(
        app: &'a mut TestAppContext,
        logs: lib_tracing::LogBuffer,
        configured_level: Option<lib_tracing::Levels>,
    ) -> Self {
        Self::build(app, logs, configured_level, Stores::fake(stub_fakes()))
    }

    /// As [`Harness::new`], under `[keybindings]` overrides.
    pub fn with_keybindings(
        app: &'a mut TestAppContext,
        keybindings: lib_config::KeyBindingConfig,
    ) -> Self {
        Self::build_with(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            Stores::fake(stub_fakes()),
            keybindings,
        )
    }

    /// As [`Harness::new`], on the `storage` the Desktop's real start-up opened, its failure
    /// channel included.
    pub fn with_storage(app: &'a mut TestAppContext, storage: lib_database::Storage) -> Self {
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores::from(storage)),
        )
    }

    /// As [`Harness::new`], on a Payees usage whose every count fails, as a Ledger file that
    /// can't be read would.
    pub fn with_unreadable_payee_usage(app: &'a mut TestAppContext) -> Self {
        let mut payees = lib_payees::fake::InMemoryPorts::stub();
        payees.usage.unreadable = true;
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            Stores::fake(FakeStores {
                payees,
                ..stub_fakes()
            }),
        )
    }

    /// As [`Harness::with_storage`], with a Payees usage that counts every Payee as unused, so a
    /// delete of a Payee in use reaches the file and its foreign keys refuse it.
    /// [`Harness::settle_writes`], after the writer is flushed, lets the Shell hear of the failure.
    pub fn with_storage_counting_payees_unused(
        app: &'a mut TestAppContext,
        storage: lib_database::Storage,
    ) -> Self {
        let mut stores = Stores::from(storage);
        stores.payees = Box::new(CountedUnusedPayees {
            inner: stores.payees,
            usage: lib_payees::fake::InMemoryPayeeUsage::default(),
        });
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(stores),
        )
    }

    /// As [`Harness::new`], on Units ports whose every write fails, as SQLite refusing it would;
    /// with `reload_fails`, so does the reload after it. [`Harness::settle_writes`] lets the
    /// Shell hear of the failure.
    pub fn with_failing_units(app: &'a mut TestAppContext, reload_fails: bool) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(FailingUnits::new(
                    InMemoryPorts::stub(chrono::Utc::now()),
                    &written,
                    reload_fails,
                )),
                institutions: Box::new(InMemoryInstitutions::stub()),
                accounts: Box::new(lib_accounts::fake::InMemoryPorts::stub()),
                categories: Box::new(InMemoryCategories::stub()),
                tags: Box::new(lib_tags::fake::InMemoryPorts::stub()),
                payees: Box::new(lib_payees::fake::InMemoryPorts::stub()),
                transactions: Box::new(stub_transactions()),
                bills: Box::new(stub_bills()),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Units],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// As [`Harness::new`], on an Institutions port whose every write fails, as SQLite refusing it
    /// would; the reload after it puts the stored rows back. [`Harness::settle_writes`] lets the
    /// Shell hear of the failure.
    pub fn with_failing_institutions(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let stored = InMemoryInstitutions::stub();
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(InMemoryPorts::stub(chrono::Utc::now())),
                institutions: Box::new(FailingInstitutions {
                    institutions: Marked {
                        inner: stored.clone(),
                        written: written.clone(),
                    },
                    stored,
                }),
                accounts: Box::new(lib_accounts::fake::InMemoryPorts::stub()),
                categories: Box::new(InMemoryCategories::stub()),
                tags: Box::new(lib_tags::fake::InMemoryPorts::stub()),
                payees: Box::new(lib_payees::fake::InMemoryPorts::stub()),
                transactions: Box::new(stub_transactions()),
                bills: Box::new(stub_bills()),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Institutions],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// As [`Harness::new`], on an Accounts store whose every write fails, as SQLite refusing it
    /// would; the reload after it puts the stored rows back. [`Harness::settle_writes`] lets the
    /// Shell hear of the failure.
    pub fn with_failing_accounts(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let stored = InMemoryAccounts::stub();
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(InMemoryPorts::stub(chrono::Utc::now())),
                institutions: Box::new(InMemoryInstitutions::stub()),
                accounts: Box::new(FailingAccounts {
                    accounts: Marked {
                        inner: stored.clone(),
                        written: written.clone(),
                    },
                    usage: InMemoryAccountUsage::default(),
                    stored,
                }),
                categories: Box::new(InMemoryCategories::stub()),
                tags: Box::new(lib_tags::fake::InMemoryPorts::stub()),
                payees: Box::new(lib_payees::fake::InMemoryPorts::stub()),
                transactions: Box::new(stub_transactions()),
                bills: Box::new(stub_bills()),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Accounts],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// As [`Harness::new`], on a Categories store whose every write fails, as SQLite refusing it
    /// would; the reload after it puts the stored rows back. [`Harness::settle_writes`] lets the
    /// Shell hear of the failure.
    pub fn with_failing_categories(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let stored = InMemoryCategories::stub();
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(InMemoryPorts::stub(chrono::Utc::now())),
                institutions: Box::new(InMemoryInstitutions::stub()),
                accounts: Box::new(lib_accounts::fake::InMemoryPorts::stub()),
                categories: Box::new(FailingCategories {
                    categories: Marked {
                        inner: stored.clone(),
                        written: written.clone(),
                    },
                    stored,
                }),
                tags: Box::new(lib_tags::fake::InMemoryPorts::stub()),
                payees: Box::new(lib_payees::fake::InMemoryPorts::stub()),
                transactions: Box::new(stub_transactions()),
                bills: Box::new(stub_bills()),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Categories],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// As [`Harness::new`], on a Tags store whose every write fails, as SQLite refusing it would;
    /// the reload after it puts the stored rows back. [`Harness::settle_writes`] lets the Shell
    /// hear of the failure.
    pub fn with_failing_tags(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let stored = lib_tags::fake::InMemoryTags::stub();
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(InMemoryPorts::stub(chrono::Utc::now())),
                institutions: Box::new(InMemoryInstitutions::stub()),
                accounts: Box::new(lib_accounts::fake::InMemoryPorts::stub()),
                categories: Box::new(InMemoryCategories::stub()),
                tags: Box::new(FailingTags {
                    tags: Marked {
                        inner: stored.clone(),
                        written: written.clone(),
                    },
                    usage: lib_tags::fake::InMemoryTagUsage::default(),
                    stored,
                }),
                payees: Box::new(lib_payees::fake::InMemoryPorts::stub()),
                transactions: Box::new(stub_transactions()),
                bills: Box::new(stub_bills()),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Tags],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// As [`Harness::new`], on an Inventory store whose every write fails, as SQLite refusing it
    /// would; the reload after it puts the stored rows back. [`Harness::settle_writes`] lets the
    /// Shell hear of the failure.
    pub fn with_failing_inventory(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let stored = stub_inventory();
        let (mut stores, _) = Stores::fake(stub_fakes());
        stores.inventory = Box::new(FailingInventory {
            inventory: Marked {
                inner: stored.clone(),
                written: written.clone(),
            },
            stored,
        });
        stores.commit = Rc::new(FailingCommit {
            domains: vec![Domain::Inventory],
            written,
            report,
        });
        stores.failures = failures;
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(stores),
        )
    }

    /// As [`Harness::new`], on a Payees port whose every write fails, as SQLite refusing it would;
    /// the reload after it puts the stored rows back. [`Harness::settle_writes`] lets the Shell
    /// hear of the failure.
    pub fn with_failing_payees(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let stored = lib_payees::fake::InMemoryPorts::stub().payees;
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(InMemoryPorts::stub(chrono::Utc::now())),
                institutions: Box::new(InMemoryInstitutions::stub()),
                accounts: Box::new(lib_accounts::fake::InMemoryPorts::stub()),
                categories: Box::new(InMemoryCategories::stub()),
                tags: Box::new(lib_tags::fake::InMemoryPorts::stub()),
                payees: Box::new(FailingPayees {
                    payees: Marked {
                        inner: stored.clone(),
                        written: written.clone(),
                    },
                    usage: lib_payees::fake::InMemoryPayeeUsage::default(),
                    stored,
                }),
                transactions: Box::new(stub_transactions()),
                bills: Box::new(stub_bills()),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Payees],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// As [`Harness::new`], on a Transactions port whose every write fails, as SQLite refusing it
    /// would; the reload after it puts the stored rows back. [`Harness::settle_writes`] lets the
    /// Shell hear of the failure.
    pub fn with_failing_transactions(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let stored = stub_transactions();
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(InMemoryPorts::stub(chrono::Utc::now())),
                institutions: Box::new(InMemoryInstitutions::stub()),
                accounts: Box::new(lib_accounts::fake::InMemoryPorts::stub()),
                categories: Box::new(InMemoryCategories::stub()),
                tags: Box::new(lib_tags::fake::InMemoryPorts::stub()),
                payees: Box::new(lib_payees::fake::InMemoryPorts::stub()),
                transactions: Box::new(FailingTransactions {
                    transactions: Marked {
                        inner: stored.clone(),
                        written: written.clone(),
                    },
                    stored,
                }),
                bills: Box::new(stub_bills()),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Transactions],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// As [`Harness::new`], on Bills ports whose every write fails, as SQLite refusing it would;
    /// the reload after it puts the stored rows back. [`Harness::settle_writes`] lets the Shell
    /// hear of the failure.
    pub fn with_failing_bills(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let stored = stub_bills();
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(InMemoryPorts::stub(chrono::Utc::now())),
                institutions: Box::new(InMemoryInstitutions::stub()),
                accounts: Box::new(lib_accounts::fake::InMemoryPorts::stub()),
                categories: Box::new(InMemoryCategories::stub()),
                tags: Box::new(lib_tags::fake::InMemoryPorts::stub()),
                payees: Box::new(lib_payees::fake::InMemoryPorts::stub()),
                transactions: Box::new(stub_transactions()),
                bills: Box::new(FailingBills {
                    bills: Marked {
                        inner: stored.clone(),
                        written: written.clone(),
                    },
                    stored,
                }),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Bills],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// As [`Harness::new`], on Transactions and Bills ports whose every write fails, the one
    /// commit naming both, as SQLite refusing a payment would; the reloads after it put the stored
    /// rows back. [`Harness::settle_writes`] lets the Shell hear of the failure.
    pub fn with_failing_payments(app: &'a mut TestAppContext) -> Self {
        let (report, failures) = mpsc::channel();
        let written = Rc::new(Cell::new(false));
        let (transactions, bills) = (stub_transactions(), stub_bills());
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(Stores {
                units: Box::new(InMemoryPorts::stub(chrono::Utc::now())),
                institutions: Box::new(InMemoryInstitutions::stub()),
                accounts: Box::new(lib_accounts::fake::InMemoryPorts::stub()),
                categories: Box::new(InMemoryCategories::stub()),
                tags: Box::new(lib_tags::fake::InMemoryPorts::stub()),
                payees: Box::new(lib_payees::fake::InMemoryPorts::stub()),
                transactions: Box::new(FailingTransactions {
                    transactions: Marked {
                        inner: transactions.clone(),
                        written: written.clone(),
                    },
                    stored: transactions,
                }),
                bills: Box::new(FailingBills {
                    bills: Marked {
                        inner: bills.clone(),
                        written: written.clone(),
                    },
                    stored: bills,
                }),
                inventory: Box::new(stub_inventory()),
                commit: Rc::new(FailingCommit {
                    domains: vec![Domain::Transactions, Domain::Bills],
                    written,
                    report,
                }),
                failures,
            }),
        )
    }

    /// The Shell on `stores`, whose commits `commits` counts.
    fn build(
        app: &'a mut TestAppContext,
        logs: lib_tracing::LogBuffer,
        configured_level: Option<lib_tracing::Levels>,
        stores: (Stores, Rc<Cell<usize>>),
    ) -> Self {
        Self::build_with(
            app,
            logs,
            configured_level,
            stores,
            lib_config::KeyBindingConfig::default(),
        )
    }

    /// As [`Harness::build`], under `keybindings`.
    fn build_with(
        app: &'a mut TestAppContext,
        logs: lib_tracing::LogBuffer,
        configured_level: Option<lib_tracing::Levels>,
        (stores, commits): (Stores, Rc<Cell<usize>>),
        keybindings: lib_config::KeyBindingConfig,
    ) -> Self {
        // Pins the process-wide Locale to en-US (and loads this bin's Messages), so rendered text
        // never depends on the host; `main` is never run in a test.
        init_for_tests();
        app.update(|cx| {
            gpui_component::init(cx);
            theme::colours::init(ThemeOverrides::default(), cx);
        });
        let (clock, date) = lib_core::Clock::settable(today());
        let (shell, cx) = app.add_window_view(|window, cx| {
            let focus_handle = cx.focus_handle();
            window.focus(&focus_handle);
            router::build(
                PersistedState::default(),
                ShellBindings { keybindings },
                ShellLogs {
                    buffer: logs.clone(),
                    configured_level,
                },
                stores,
                clock,
                focus_handle,
                cx,
            )
        });
        Self {
            shell,
            cx,
            logs,
            date,
            commits,
        }
    }

    /// How many times the Shell has committed since it was built: once per input.
    pub fn commits(&self) -> usize {
        self.commits.get()
    }

    /// Moves the Shell's clock to `day`, then lets the Shell notice: past one poll of its clock.
    pub fn turn_date_to(&mut self, day: NaiveDate) {
        self.date.set(day);
        self.advance_clock(router::DATE_POLL);
    }

    /// Lets the Shell hear of a failed write: past one poll of the writer's failure channel.
    pub fn settle_writes(&mut self) {
        self.advance_clock(router::WRITE_FAILURE_POLL);
    }

    /// Lets the Tracing page's live feed pull what was pushed: past its coalescing window.
    pub fn settle_logs(&mut self) {
        self.cx.run_until_parked();
        self.cx.executor().advance_clock(router::LOG_COALESCE * 2);
        self.cx.run_until_parked();
    }

    /// Resizes the window to `window_size`'s reference frame (ADR-0042) and lets it redraw. The
    /// window opens maximised on the test display, 1920 × 1080, which is X-Large.
    pub fn resize_to(&mut self, window_size: WindowSize) {
        self.resize(window_size.reference_frame());
    }

    /// Resizes the window to `size` and lets it redraw: for a case a reference frame doesn't
    /// reach, such as Medium's 720px floor.
    pub fn resize(&mut self, size: Size<Pixels>) {
        self.cx.simulate_resize(size);
        self.cx.run_until_parked();
    }

    /// Where the element tagged `debug_selector(selector)` was drawn in the last frame.
    pub fn bounds(&mut self, selector: &str) -> Bounds<Pixels> {
        let key: &'static str = Box::leak(selector.to_string().into_boxed_str());
        self.cx
            .debug_bounds(key)
            .unwrap_or_else(|| panic!("no element tagged `{selector}` was drawn"))
    }

    /// Scrolls the wheel by `delta` over `position`, then lets the window redraw.
    pub fn scroll_at(&mut self, position: Point<Pixels>, delta: Point<Pixels>) {
        self.cx.simulate_event(ScrollWheelEvent {
            position,
            delta: ScrollDelta::Pixels(delta),
            ..Default::default()
        });
        self.cx.run_until_parked();
    }

    /// The primary rail as the window is drawn now.
    pub fn rail(&mut self) -> RailMode {
        let shell = self.shell.clone();
        self.cx
            .update(|window, cx| shell.read(cx).primary_rail(WindowSize::of(window), cx))
    }

    /// Moves the executor's clock on, firing any timer it passes (a pending `g`'s timeout).
    pub fn advance_clock(&mut self, by: Duration) {
        self.cx.executor().advance_clock(by);
        self.cx.run_until_parked();
    }

    /// Presses a space-separated keystroke sequence, e.g. `press("g f")`.
    pub fn press(&mut self, keys: &str) {
        self.cx.simulate_keystrokes(keys);
        self.cx.run_until_parked();
    }

    /// Clicks the centre of the element tagged `debug_selector(selector)`.
    pub fn click(&mut self, selector: &str) {
        // gpui's `debug_bounds` wants a `'static` key and index-tagged selectors are built at run
        // time; leaking a handful of short strings per test process is the cheap way through.
        let key: &'static str = Box::leak(selector.to_string().into_boxed_str());
        let bounds = self
            .cx
            .debug_bounds(key)
            .unwrap_or_else(|| panic!("no element tagged `{selector}` was drawn"));
        self.cx.simulate_click(bounds.center(), Modifiers::none());
        self.cx.run_until_parked();
    }

    /// Right-clicks the centre of the element tagged `debug_selector(selector)`, as opening a
    /// row's context menu does.
    pub fn right_click(&mut self, selector: &str) {
        let position = self.bounds(selector).center();
        self.cx.simulate_event(MouseDownEvent {
            button: MouseButton::Right,
            position,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        });
        self.cx.simulate_event(MouseUpEvent {
            button: MouseButton::Right,
            position,
            modifiers: Modifiers::none(),
            click_count: 1,
        });
        self.cx.run_until_parked();
    }

    /// Double-clicks the centre of the element tagged `debug_selector(selector)`: two presses, the
    /// second reporting a click count of 2, as the platform does.
    pub fn double_click(&mut self, selector: &str) {
        let position = self.bounds(selector).center();
        for click_count in [1, 2] {
            self.cx.simulate_event(MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::none(),
                click_count,
                first_mouse: false,
            });
            self.cx.simulate_event(MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::none(),
                click_count,
            });
        }
        self.cx.run_until_parked();
    }

    /// Empties the Inventory so the Settings Inventory page shows its empty state.
    pub fn empty_inventory(&mut self) {
        self.shell.update(self.cx, |shell, cx| {
            shell.empty_inventory_for_test(cx);
            cx.notify();
        });
        self.cx.run_until_parked();
    }

    /// Leaves no Unit in use, so a seeded Unit can be deleted rather than deactivated.
    pub fn drop_unit_users(&mut self) {
        self.shell.update(self.cx, |shell, cx| {
            shell.drop_unit_users_for_test(cx);
            cx.notify();
        });
        self.cx.run_until_parked();
    }

    /// Adds a Location priced in `unit`, which then names it.
    pub fn add_location_in(&mut self, unit: UnitId) {
        self.shell.update(self.cx, |shell, cx| {
            shell.add_location_in_for_test(unit, cx);
            cx.notify();
        });
        self.cx.run_until_parked();
    }

    /// Moves every Bill Plan to pay from Account `to`, so the Accounts they paid from can be
    /// deleted rather than refused.
    pub fn move_bill_plans_to(&mut self, to: AccountId) {
        self.shell.update(self.cx, |shell, cx| {
            shell.move_bill_plans_for_test(to, cx);
            cx.notify();
        });
        self.cx.run_until_parked();
    }

    /// Leaves no Unit quoted in another, so the base Unit can be deleted rather than refused.
    pub fn unquote_units(&mut self) {
        self.shell.update(self.cx, |shell, cx| {
            shell.unquote_units_for_test(cx);
            cx.notify();
        });
        self.cx.run_until_parked();
    }

    /// The Noun the Shell is showing.
    pub fn noun(&mut self) -> Noun {
        self.read(|shell| shell.nav().noun())
    }

    /// The Documents page's state, read back for assertions.
    pub fn documents(&mut self) -> DocumentsSnapshot {
        self.read_app(Root::documents_snapshot)
    }

    /// The Bills page's state, read back for assertions.
    pub fn bills(&mut self) -> BillsSnapshot {
        self.read_app(Root::bills_snapshot)
    }

    /// The Budgets page's state, read back for assertions.
    pub fn budgets(&mut self) -> BudgetsSnapshot {
        self.read_app(Root::budgets_snapshot)
    }

    /// The Settings shell's state (page and open dialog), read back for assertions.
    pub fn settings(&mut self) -> SettingsSnapshot {
        self.read_app(Root::settings_snapshot)
    }

    /// The open hosted Dialog's kind (`AddTag`, `DeleteCategory`), read back through the Dialog
    /// itself; `None` when none is open.
    pub fn dialog(&mut self) -> Option<&'static str> {
        self.read_app(Root::dialog_readback)
            .map(|readback| readback.kind)
    }

    /// The open hosted Dialog's value labelled `label` (`name`, `confirm`), if it shows one.
    pub fn dialog_value(&mut self, label: &str) -> Option<String> {
        self.read_app(Root::dialog_readback)?
            .value(label)
            .map(str::to_string)
    }

    /// The Settings Accounts page's state, read back for assertions.
    pub fn settings_accounts(&mut self) -> SettingsAccountsSnapshot {
        self.read_app(Root::settings_accounts_snapshot)
    }

    /// The Settings Categories page's state, read back for assertions.
    pub fn settings_categories(&mut self) -> SettingsCategoriesSnapshot {
        self.read_app(Root::settings_categories_snapshot)
    }

    /// The Settings Display page's state, read back for assertions.
    pub fn settings_display(&mut self) -> SettingsDisplaySnapshot {
        self.read_app(Root::settings_display_snapshot)
    }

    /// The Settings DocumentTypes page's state, read back for assertions.
    pub fn settings_document_types(&mut self) -> SettingsDocumentTypesSnapshot {
        self.read_app(Root::settings_document_types_snapshot)
    }

    /// The Settings Institutions page's state, read back for assertions.
    pub fn settings_institutions(&mut self) -> SettingsInstitutionsSnapshot {
        self.read_app(Root::settings_institutions_snapshot)
    }

    /// The Settings Inventory page's state, read back for assertions.
    pub fn settings_inventory(&mut self) -> SettingsInventorySnapshot {
        self.read_app(Root::settings_inventory_snapshot)
    }

    /// The Settings Payees page's state, read back for assertions.
    pub fn settings_payees(&mut self) -> SettingsPayeesSnapshot {
        self.read_app(Root::settings_payees_snapshot)
    }

    /// The Settings Tags page's state, read back for assertions.
    pub fn settings_tags(&mut self) -> SettingsTagsSnapshot {
        self.read_app(Root::settings_tags_snapshot)
    }

    /// The Settings Tracing page's state, read back for assertions.
    pub fn settings_tracing(&mut self) -> SettingsTracingSnapshot {
        self.read_app(Root::settings_tracing_snapshot)
    }

    /// The Settings General page's state, read back for assertions.
    pub fn settings_general(&mut self) -> SettingsGeneralSnapshot {
        self.read_app(Root::settings_general_snapshot)
    }

    /// The Settings Units page's state, read back for assertions.
    pub fn settings_units(&mut self) -> SettingsUnitsSnapshot {
        self.read_app(Root::settings_units_snapshot)
    }

    /// The Units table's rows as `(code, name, type)`, as drawn.
    pub fn unit_rows(&mut self) -> Vec<(String, String, String)> {
        self.settings_units()
            .units
            .into_iter()
            .map(|unit| (unit.code, unit.name, unit.kind))
            .collect()
    }

    /// The Units' codes in table order.
    pub fn unit_codes(&mut self) -> Vec<String> {
        self.unit_rows()
            .into_iter()
            .map(|(code, ..)| code)
            .collect()
    }

    /// The code of the Unit the ◆ marks: the base Unit.
    pub fn base_unit_code(&mut self) -> Option<String> {
        self.settings_units()
            .units
            .into_iter()
            .find(|unit| unit.is_base)
            .map(|unit| unit.code)
    }

    /// The codes of the deactivated Units, drawn dimmed and tagged.
    pub fn inactive_unit_codes(&mut self) -> Vec<String> {
        self.settings_units()
            .units
            .into_iter()
            .filter(|unit| unit.inactive_tag.is_some())
            .map(|unit| unit.code)
            .collect()
    }

    /// The open Add or Edit account dialog's form, read back through the Dialog itself; panics if
    /// none is open.
    pub fn account_form(&mut self) -> AccountFormSnapshot {
        let readback = self
            .read_app(Root::dialog_readback)
            .filter(|readback| matches!(readback.kind, "AddAccount" | "EditAccount"))
            .expect("an Add or Edit account dialog is open");
        let text = |label: &str| readback.value(label).unwrap_or_default().to_string();
        let choice = |label: &str| Some(text(label)).filter(|value| !value.is_empty());
        AccountFormSnapshot {
            focused: text("focused"),
            name: text("name"),
            institution: choice("institution"),
            account_type: choice("type"),
            unit: choice("unit"),
            opening_balance: text("opening_balance"),
            account_number: text("account_number"),
            select_open: text("select_open") == "true",
            valid: text("valid") == "true",
        }
    }

    /// The Colour Theme id and Appearance now chosen, read from the colours Global.
    pub fn colour_choice(&mut self) -> (Option<String>, Option<String>) {
        self.cx.read(|cx| {
            let state = theme::colours::colours(cx);
            (
                state.colour_theme().map(str::to_string),
                state.colour_appearance().map(|a| format!("{a:?}")),
            )
        })
    }

    /// The Transactions page's state, read back for assertions.
    pub fn transactions(&mut self) -> TransactionsSnapshot {
        self.read_app(Root::transactions_snapshot)
    }

    /// The status line's hints and right-hand text for the View on show (or the open Dialog's
    /// hints); `None` when there are none.
    pub fn status(&mut self) -> Option<StatusSnapshot> {
        self.read_app(Root::status_snapshot)
    }

    /// The open `?` Help Dialog's page group; `None` while it is not open or the View on show
    /// has no group.
    pub fn help_group(&mut self) -> Option<HelpGroup> {
        self.read(Root::help_group)
    }

    /// The status line's hints as the `?` cheat sheet words them, `(action, keys)`; empty when
    /// the line has no page content.
    pub fn status_as_shortcuts(&mut self) -> Vec<(String, String)> {
        self.status()
            .map(|status| {
                status
                    .hints
                    .into_iter()
                    .map(|(keys, action)| (action, keys))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Opens `?` and asserts its page group is headed `heading` and lists every hint the status
    /// line showed before it.
    #[expect(
        clippy::expect_used,
        reason = "test support: a `?` that opens no page group should fail the test loudly"
    )]
    pub fn assert_cheat_sheet_matches_status(&mut self, heading: &str) {
        let legend = self.status_as_shortcuts();
        assert!(!legend.is_empty(), "the page shows a legend");
        self.press("?");
        assert!(self.read(Root::help_open), "`?` opens the Help Dialog");
        let group = self
            .help_group()
            .expect("the View on show has a page group");
        assert_eq!(group.heading, heading);
        for hint in &legend {
            assert!(
                group.keys.contains(hint),
                "{hint:?} is on the status line but not in {:?}",
                group.keys
            );
        }
    }

    /// The Toast layer and its history, read back for assertions.
    pub fn toasts(&mut self) -> ToastsSnapshot {
        self.read(Root::toasts_snapshot)
    }

    /// The Import step's state; `None` while it is not showing.
    pub fn import(&mut self) -> Option<ImportSnapshot> {
        self.read_app(Root::import_snapshot)
    }

    /// The Dashboard's Needs Attention Bill rows.
    pub fn dashboard_bills(&mut self) -> Vec<DashboardBillSnapshot> {
        self.read_app(Root::dashboard_bills_snapshot)
    }

    /// Queues `notice` as a module Router does from inside a Presenter's `handle`. Nothing shows
    /// until `Root` drains the queue, after the next key or Dialog click.
    pub fn queue_notice(&mut self, notice: lib_toast::Notice) {
        use lib_toast::Notify as _;
        bin_desktop::notices::Queue.notify(notice);
    }

    /// The visible Toasts' text, oldest first.
    pub fn toast_texts(&mut self) -> Vec<String> {
        self.toasts()
            .visible
            .into_iter()
            .map(|toast| toast.text)
            .collect()
    }

    /// Raises a Toast the way a call site would, then settles.
    pub fn raise_toast(&mut self, kind: lib_toast::ToastKind, text: &str) {
        self.shell.update(self.cx, |shell, cx| {
            shell.raise_toast(kind, text);
            cx.notify();
        });
        self.cx.run_until_parked();
    }

    /// Whether an element tagged `debug_selector(selector)` was drawn in the last frame.
    pub fn is_drawn(&mut self, selector: &str) -> bool {
        let key: &'static str = Box::leak(selector.to_string().into_boxed_str());
        self.cx.debug_bounds(key).is_some()
    }

    /// Reads from the Shell without mutating it.
    pub fn read<T>(&mut self, f: impl FnOnce(&Root) -> T) -> T {
        self.shell.read_with(self.cx, |shell, _| f(shell))
    }

    /// As [`Self::read`], for the snapshots that read Entities (ADR-0032) and so need the `App`.
    pub fn read_app<T>(&mut self, f: impl FnOnce(&Root, &App) -> T) -> T {
        self.shell.read_with(self.cx, |shell, cx| f(shell, cx))
    }
}

/// A captured log entry, as the capture layer would record it.
pub fn log_entry(
    level: tracing::Level,
    target: &'static str,
    message: &str,
) -> lib_tracing::LogEntry {
    lib_tracing::LogEntry {
        seq: 0,
        time: std::time::SystemTime::now(),
        level,
        target: std::borrow::Cow::Borrowed(target),
        message: message.to_string(),
        fields: Vec::new(),
    }
}
