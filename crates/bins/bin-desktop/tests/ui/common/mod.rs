//! Headless harness for the Desktop UI: repeats `lib::run`'s app-level setup, builds `Shell`
//! through the lib's `build_shell` with a fixed date and the deterministic stub seed, and offers
//! key presses, selector clicks and read helpers. Nothing here opens a real window or touches
//! `persistence`, so it runs in a sandbox with no GPU.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test support: a failed lookup or invalid fixed date should fail the test loudly"
)]

use bin_desktop::{
    ShellBindings, ShellLogs, WindowSize,
    app::{
        BillsSnapshot, BudgetsSnapshot, DashboardBillSnapshot, DocumentsSnapshot, ImportSnapshot,
        SettingsAccountsSnapshot, SettingsCategoriesSnapshot, SettingsDisplaySnapshot,
        SettingsDocumentTypesSnapshot, SettingsGeneralSnapshot, SettingsInstitutionsSnapshot,
        SettingsInventorySnapshot, SettingsPayeesSnapshot, SettingsSnapshot, SettingsTagsSnapshot,
        SettingsTracingSnapshot, SettingsUnitsSnapshot, Shell, StatusSnapshot, ToastsSnapshot,
        TransactionsSnapshot,
    },
    build_shell,
    locale::init_for_tests,
    navigation::nav::{Noun, RailMode},
    persistence::PersistedState,
    theme,
};
use chrono::NaiveDate;
use gpui::{
    App, Bounds, Entity, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point,
    ScrollDelta, ScrollWheelEvent, Size, TestAppContext, VisualTestContext,
};
use lib_bills::fake::InMemoryBills;
use lib_bills::{BillPlan, BillPlanRepository, BillScheduleEntry, BillScheduleRepository};
use lib_colour_theme::ThemeOverrides;
use lib_core::TransactionId;
use lib_core::{PriceSourceId, UnitId};
use lib_database::{
    BillsStore, Commit, Domain, InMemoryCommit, TransactionsStore, UnitsStore, WriteFailure,
};
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

/// A failure channel nothing will report on: the in-memory ports have no writer.
fn no_failures() -> mpsc::Receiver<WriteFailure> {
    mpsc::channel().1
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
pub fn today() -> NaiveDate {
    date(2026, 6, 15)
}

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("fixed test date is valid")
}

/// A `Shell` in a headless window, with helpers to drive and read it.
pub struct Harness<'a> {
    pub shell: Entity<Shell>,
    pub cx: &'a mut VisualTestContext,
    /// The Shell's live log capture: push entries here, then [`Harness::settle_logs`].
    pub logs: lib_tracing::LogBuffer,
    /// The day the Shell's clock reads, [`today`] until [`Harness::turn_date_to`] moves it.
    date: Rc<Cell<NaiveDate>>,
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
    /// Boots the app-level globals and a `Shell` with default (nothing persisted) state.
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
        Self::build(
            app,
            logs,
            configured_level,
            Box::new(InMemoryPorts::stub(chrono::Utc::now())),
            Box::new(stub_transactions()),
            Box::new(stub_bills()),
            Rc::new(InMemoryCommit),
            no_failures(),
        )
    }

    /// As [`Harness::new`], on the `storage` the Desktop's real start-up opened, its failure
    /// channel included.
    pub fn with_storage(app: &'a mut TestAppContext, storage: lib_database::Storage) -> Self {
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            Box::new(storage.units),
            Box::new(storage.transactions),
            Box::new(storage.bills),
            Rc::new(storage.pending),
            storage.failures,
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
            Box::new(FailingUnits::new(
                InMemoryPorts::stub(chrono::Utc::now()),
                &written,
                reload_fails,
            )),
            Box::new(stub_transactions()),
            Box::new(stub_bills()),
            Rc::new(FailingCommit {
                domains: vec![Domain::Units],
                written,
                report,
            }),
            failures,
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
            Box::new(InMemoryPorts::stub(chrono::Utc::now())),
            Box::new(FailingTransactions {
                transactions: Marked {
                    inner: stored.clone(),
                    written: written.clone(),
                },
                stored,
            }),
            Box::new(stub_bills()),
            Rc::new(FailingCommit {
                domains: vec![Domain::Transactions],
                written,
                report,
            }),
            failures,
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
            Box::new(InMemoryPorts::stub(chrono::Utc::now())),
            Box::new(stub_transactions()),
            Box::new(FailingBills {
                bills: Marked {
                    inner: stored.clone(),
                    written: written.clone(),
                },
                stored,
            }),
            Rc::new(FailingCommit {
                domains: vec![Domain::Bills],
                written,
                report,
            }),
            failures,
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
            Box::new(InMemoryPorts::stub(chrono::Utc::now())),
            Box::new(FailingTransactions {
                transactions: Marked {
                    inner: transactions.clone(),
                    written: written.clone(),
                },
                stored: transactions,
            }),
            Box::new(FailingBills {
                bills: Marked {
                    inner: bills.clone(),
                    written: written.clone(),
                },
                stored: bills,
            }),
            Rc::new(FailingCommit {
                domains: vec![Domain::Transactions, Domain::Bills],
                written,
                report,
            }),
            failures,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one per store build_shell takes, which each constructor varies"
    )]
    fn build(
        app: &'a mut TestAppContext,
        logs: lib_tracing::LogBuffer,
        configured_level: Option<lib_tracing::Levels>,
        units: Box<dyn UnitsStore>,
        transactions: Box<dyn TransactionsStore>,
        bills: Box<dyn BillsStore>,
        commit: Rc<dyn Commit>,
        failures: mpsc::Receiver<WriteFailure>,
    ) -> Self {
        // Pins the process-wide Locale to en-US (and loads this bin's Messages), so rendered text
        // never depends on the host; `main` is never run in a test.
        init_for_tests();
        app.update(|cx| {
            gpui_component::init(cx);
            theme::colours::init(ThemeOverrides::default(), cx);
        });
        let date = Rc::new(Cell::new(today()));
        let read = date.clone();
        let (shell, cx) = app.add_window_view(|window, cx| {
            let focus_handle = cx.focus_handle();
            window.focus(&focus_handle);
            build_shell(
                PersistedState::default(),
                ShellBindings {
                    dismiss_toasts: "ctrl+l".to_string(),
                    toast_history: None,
                    keybindings: lib_config::KeyBindingConfig::default(),
                },
                ShellLogs {
                    buffer: logs.clone(),
                    configured_level,
                },
                units,
                transactions,
                bills,
                commit,
                failures,
                Rc::new(move || read.get()),
                focus_handle,
                cx,
            )
        });
        Self {
            shell,
            cx,
            logs,
            date,
        }
    }

    /// Moves the Shell's clock to `day`, then lets the Shell notice: past one poll of its clock.
    pub fn turn_date_to(&mut self, day: NaiveDate) {
        self.date.set(day);
        self.advance_clock(bin_desktop::app::DATE_POLL);
    }

    /// Lets the Shell hear of a failed write: past one poll of the writer's failure channel.
    pub fn settle_writes(&mut self) {
        self.advance_clock(bin_desktop::app::WRITE_FAILURE_POLL);
    }

    /// Lets the Tracing page's live feed pull what was pushed: past its coalescing window.
    pub fn settle_logs(&mut self) {
        self.cx.run_until_parked();
        self.cx
            .executor()
            .advance_clock(bin_desktop::app::LOG_COALESCE * 2);
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
        self.read_app(Shell::documents_snapshot)
    }

    /// The Bills page's state, read back for assertions.
    pub fn bills(&mut self) -> BillsSnapshot {
        self.read_app(Shell::bills_snapshot)
    }

    /// The Budgets page's state, read back for assertions.
    pub fn budgets(&mut self) -> BudgetsSnapshot {
        self.read_app(Shell::budgets_snapshot)
    }

    /// The Settings shell's state (page and open dialog), read back for assertions.
    pub fn settings(&mut self) -> SettingsSnapshot {
        self.read_app(Shell::settings_snapshot)
    }

    /// The open hosted Dialog's kind (`AddTag`, `DeleteCategory`), read back through the Dialog
    /// itself; `None` when none is open.
    pub fn dialog(&mut self) -> Option<&'static str> {
        self.read_app(Shell::dialog_readback)
            .map(|readback| readback.kind)
    }

    /// The open hosted Dialog's value labelled `label` (`name`, `confirm`), if it shows one.
    pub fn dialog_value(&mut self, label: &str) -> Option<String> {
        self.read_app(Shell::dialog_readback)?
            .value(label)
            .map(str::to_string)
    }

    /// The Settings Accounts page's state, read back for assertions.
    pub fn settings_accounts(&mut self) -> SettingsAccountsSnapshot {
        self.read_app(Shell::settings_accounts_snapshot)
    }

    /// The Settings Categories page's state, read back for assertions.
    pub fn settings_categories(&mut self) -> SettingsCategoriesSnapshot {
        self.read_app(Shell::settings_categories_snapshot)
    }

    /// The Settings Display page's state, read back for assertions.
    pub fn settings_display(&mut self) -> SettingsDisplaySnapshot {
        self.read_app(Shell::settings_display_snapshot)
    }

    /// The Settings DocumentTypes page's state, read back for assertions.
    pub fn settings_document_types(&mut self) -> SettingsDocumentTypesSnapshot {
        self.read_app(Shell::settings_document_types_snapshot)
    }

    /// The Settings Institutions page's state, read back for assertions.
    pub fn settings_institutions(&mut self) -> SettingsInstitutionsSnapshot {
        self.read_app(Shell::settings_institutions_snapshot)
    }

    /// The Settings Inventory page's state, read back for assertions.
    pub fn settings_inventory(&mut self) -> SettingsInventorySnapshot {
        self.read_app(Shell::settings_inventory_snapshot)
    }

    /// The Settings Payees page's state, read back for assertions.
    pub fn settings_payees(&mut self) -> SettingsPayeesSnapshot {
        self.read_app(Shell::settings_payees_snapshot)
    }

    /// The Settings Tags page's state, read back for assertions.
    pub fn settings_tags(&mut self) -> SettingsTagsSnapshot {
        self.read_app(Shell::settings_tags_snapshot)
    }

    /// The Settings Tracing page's state, read back for assertions.
    pub fn settings_tracing(&mut self) -> SettingsTracingSnapshot {
        self.read_app(Shell::settings_tracing_snapshot)
    }

    /// The Settings General page's state, read back for assertions.
    pub fn settings_general(&mut self) -> SettingsGeneralSnapshot {
        self.read_app(Shell::settings_general_snapshot)
    }

    /// The Settings Units page's state, read back for assertions.
    pub fn settings_units(&mut self) -> SettingsUnitsSnapshot {
        self.read_app(Shell::settings_units_snapshot)
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
            .read_app(Shell::dialog_readback)
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
        self.read_app(Shell::transactions_snapshot)
    }

    /// The status line's hints and right-hand text for the View on show (or the open Dialog's
    /// hints); `None` when there are none.
    pub fn status(&mut self) -> Option<StatusSnapshot> {
        self.read_app(Shell::status_snapshot)
    }

    /// The Toast layer and its history, read back for assertions.
    pub fn toasts(&mut self) -> ToastsSnapshot {
        self.read(Shell::toasts_snapshot)
    }

    /// The Import step's state; `None` while it is not showing.
    pub fn import(&mut self) -> Option<ImportSnapshot> {
        self.read_app(Shell::import_snapshot)
    }

    /// The Dashboard's Needs Attention Bill rows.
    pub fn dashboard_bills(&mut self) -> Vec<DashboardBillSnapshot> {
        self.read_app(Shell::dashboard_bills_snapshot)
    }

    /// Queues `notice` as a module Router does from inside a Presenter's `handle`. Nothing shows
    /// until `Shell` drains the queue, after the next key or Dialog click.
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
    pub fn read<T>(&mut self, f: impl FnOnce(&Shell) -> T) -> T {
        self.shell.read_with(self.cx, |shell, _| f(shell))
    }

    /// As [`Self::read`], for the snapshots that read Entities (ADR-0032) and so need the `App`.
    pub fn read_app<T>(&mut self, f: impl FnOnce(&Shell, &App) -> T) -> T {
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
