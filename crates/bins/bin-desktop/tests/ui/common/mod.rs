//! Headless harness for the Desktop UI: repeats `lib::run`'s app-level setup, builds `Root`
//! through the lib's `shell::router::build` with a fixed date and the Sample Ledger (`FakeStores::sample`), and offers
//! key presses, selector clicks and read helpers. Nothing here opens a real window or touches
//! `persistence`, so it runs in a sandbox with no GPU.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test support: a failed lookup or invalid fixed date should fail the test loudly"
)]

use bin_desktop::{
    DeviceConfigFile, WindowSize, WindowSizeExt,
    locale::init_for_tests,
    navigation::nav::{Noun, RailMode},
    persistence::PersistedState,
    shell::router::{
        self, BillsSnapshot, BudgetsSnapshot, DashboardBillSnapshot, DocumentsSnapshot, HelpGroup,
        ImportSnapshot, PageSnapshot, Root, SettingsSnapshot, SettingsUnitsSnapshot, ShellBindings,
        ShellLogs, StatusSnapshot, ToastsSnapshot, TransactionsSnapshot,
    },
    theme,
};
use chrono::NaiveDate;
use gpui::{
    App, Bounds, Entity, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point,
    ScrollDelta, ScrollWheelEvent, Size, TestAppContext, VisualTestContext,
};
use lib_colour_theme::ThemeOverrides;
use lib_core::{AccountId, CategoryId, InstitutionId, PayeeId, UnitId};
use lib_database::{
    CountingCommit, Domain, FakeStores, Recorded, SharedStores, Stores, TransactionsStore, Written,
};
use lib_transactions::TransactionRepository;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The status line's refusal for anything that needs a Ledger with none open (#977).
pub const NO_LEDGER: &str = "No Ledger open: :open or :new first";

/// Creates the Sample Ledger in a file at `path` (`seed::sample_ledger`: start-up never creates one,
/// and `:new` creates an empty one), and returns the configuration `main` would hold, pointed at
/// it, for a smoke test on the real start-up.
pub fn new_ledger(path: &std::path::Path) -> lib_config::Config {
    init_for_tests();
    lib_database::seed::sample_ledger(path, today()).expect("a new Ledger file");
    config_for(path)
}

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

/// A Transactions store whose Splits naming a Payee are counted as `payee_splits` says, whatever
/// it holds: `Some(0)`, as a count taken before a Split naming one landed would, so a delete it
/// allows reaches the file; `None`, as a Ledger file that can't be read would.
struct MiscountedTransactions {
    inner: Box<dyn TransactionsStore>,
    payee_splits: Option<usize>,
}

impl TransactionsStore for MiscountedTransactions {
    fn transactions(&self) -> &dyn TransactionRepository {
        self.inner.transactions()
    }

    fn transactions_mut(&mut self) -> &mut dyn TransactionRepository {
        self.inner.transactions_mut()
    }

    fn category_splits(&self, id: CategoryId) -> usize {
        self.inner.category_splits(id)
    }

    fn payee_splits(&self, _id: PayeeId) -> Option<usize> {
        self.payee_splits
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.inner.reload()
    }
}

/// Leaves no Unit in use, so a seeded Unit can be deleted rather than deactivated: no Account,
/// Bill Plan, Budget or Location names one. For [`Harness::arrange`].
pub fn no_unit_users(fakes: &mut FakeStores) {
    fakes.accounts.accounts.accounts.clear();
    fakes.bills.plans.clear();
    fakes.bills.entries.clear();
    fakes.budgets.budgets.clear();
    fakes.inventory.locations.clear();
    fakes.inventory.items.clear();
}

/// Empties the Inventory, so the Inventory page shows its empty state. For [`Harness::arrange`].
pub fn empty_inventory(fakes: &mut FakeStores) {
    fakes.inventory.locations.clear();
    fakes.inventory.items.clear();
}

/// Removes every Tag, and the Tags Splits carried, so the Tags page has nothing to select. For
/// [`Harness::arrange`].
pub fn empty_tags(fakes: &mut FakeStores) {
    fakes.tags.tags.tags.clear();
    for transaction in &mut fakes.transactions.transactions {
        for split in &mut transaction.splits {
            split.tag_ids.clear();
        }
    }
}

/// Adds a Location priced in `unit`, which then names it. For [`Harness::arrange`].
pub fn add_location_in(fakes: &mut FakeStores, unit: UnitId) {
    let mut scratch = lib_inventory::Inventory::default();
    let draft = lib_inventory::LocationDraft {
        name: "Beach shack".to_string(),
        ..lib_inventory::LocationDraft::default()
    };
    scratch
        .add_location(&draft, unit)
        .expect("a valid Location");
    fakes.inventory.locations.append(&mut scratch.locations);
}

/// Adds an empty default Budget in `unit`, which then names it. For [`Harness::arrange`].
pub fn add_budget_in(fakes: &mut FakeStores, unit: UnitId) {
    fakes
        .budgets
        .budgets
        .push(lib_budgets::personal_spending(unit, &[]));
}

/// Moves every Bill Plan to pay from Account `to`, so the Accounts they paid from can be deleted
/// rather than refused. For [`Harness::arrange`].
pub fn move_bill_plans(fakes: &mut FakeStores, to: AccountId) {
    for plan in &mut fakes.bills.plans {
        plan.account_id = to;
    }
}

/// Leaves no Unit quoted in another, so a seeded Unit can be deleted rather than refused. For
/// [`Harness::arrange`].
pub fn unquote_units(fakes: &mut FakeStores) {
    for unit in &mut fakes.units.units.units {
        unit.link.quote = unit.id;
    }
}

/// Gives Institution `id` Unit `unit` as its Usual unit. For [`Harness::arrange`].
pub fn name_usual_unit(fakes: &mut FakeStores, id: InstitutionId, unit: UnitId) {
    for institution in &mut fakes.institutions.institutions {
        if institution.id == id {
            institution.usual_unit = Some(unit);
        }
    }
}

/// A Data folder holding the folders `archive` and `finance`, a Ledger file and a CSV: the folder
/// picker lists `archive`, `finance`, `expenses.csv`, `teda.pldb`, in that order.
pub fn data_folder() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary directory");
    for folder in ["archive", "finance"] {
        std::fs::create_dir(dir.path().join(folder)).expect("a folder");
    }
    for file in ["teda.pldb", "expenses.csv"] {
        std::fs::write(dir.path().join(file), b"stub").expect("a file");
    }
    dir
}

/// A device's settings as start-up read them with `folder` the Data folder in force.
pub fn browsing(folder: &std::path::Path) -> lib_config::Config {
    let mut config = lib_config::Config::default();
    config.general.data = folder.to_path_buf();
    config
}

/// The device a Harness builds the Shell on, as `run` hands start-up's to `build`: a device's
/// settings as start-up read them, the `PERSONAL_LEDGER_` variables above the file, what the
/// Harness's own conf file holds before start-up reads it, and the file Settings writes.
#[derive(Default)]
pub struct Device<'c> {
    pub config: lib_config::Config,
    pub env: &'c [(&'c str, &'c str)],
    /// The conf file's text; empty for no file yet.
    pub conf: &'c str,
    /// The configuration file in place of the Harness's own, `Some(None)` for none. A file named
    /// here is only read, never written, by a test that only looks.
    pub file: Option<Option<std::path::PathBuf>>,
}

/// The default `[keybindings]` with `command` bound to `chord`.
pub fn overriding(command: &str, chord: &str) -> lib_config::KeyBindingConfig {
    let mut config = lib_config::KeyBindingConfig::default();
    config
        .bindings
        .insert(command.to_string(), chord.to_string());
    config
}

/// The date every test is anchored to, so scopes and seeded relative dates never drift.
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
    /// A clone of the stores the Shell was built on, `None` with no Ledger open: what
    /// [`Harness::write_outside`] writes and [`Harness::stores`] reads (#1117).
    stores: Option<SharedStores>,
    /// Holds this Shell's own `personal-ledger.conf`, which Settings writes a device's settings to
    /// (ADR-0053), so no test writes the real one.
    conf_dir: tempfile::TempDir,
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
    /// An Edit's fixed balance, as shown; `None` on an Add.
    pub balance: Option<String>,
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
        Self::build(
            app,
            logs,
            configured_level,
            Stores::fake(FakeStores::sample(today())),
        )
    }

    /// As [`Harness::new`], on the Sample Ledger as `arrange` leaves it: the state a test needs set
    /// on the in-memory stores before the Shell is built on them.
    pub fn arrange(app: &'a mut TestAppContext, arrange: impl FnOnce(&mut FakeStores)) -> Self {
        let mut fakes = FakeStores::sample(today());
        arrange(&mut fakes);
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            Stores::fake(fakes),
        )
    }

    /// Writes to the stores as something outside the app would, a sync or a second window: into
    /// memory, where the Shell reads, with no commit and no `LedgerChanged`, so nothing repaints
    /// until the test moves the Shell to read afresh.
    ///
    /// # Panics
    /// With no Ledger open, since there are no stores to write.
    pub fn write_outside(&mut self, write: impl FnOnce(&SharedStores)) {
        write(self.stores.as_ref().expect("a Ledger is open"));
    }

    /// Reads the stores the Shell was built on, as persisted state rather than as drawn.
    ///
    /// # Panics
    /// With no Ledger open, since there are no stores to read.
    pub fn stores<T>(&self, read: impl FnOnce(&SharedStores) -> T) -> T {
        read(self.stores.as_ref().expect("a Ledger is open"))
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
            Stores::fake(FakeStores::sample(today())),
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

    /// As [`Harness::with_storage`], with a Transactions store that counts no Split naming any
    /// Payee, so a delete of a Payee only Splits name reaches the file and its foreign keys refuse
    /// it. [`Harness::settle_writes`], after the writer is flushed, lets the Shell hear of the
    /// failure.
    pub fn with_storage_counting_payees_unused(
        app: &'a mut TestAppContext,
        storage: lib_database::Storage,
    ) -> Self {
        let mut stores = Stores::from(storage);
        stores.transactions = Recorded::new(
            Domain::Transactions,
            Written::default(),
            Box::new(MiscountedTransactions {
                inner: Box::new(stores.transactions),
                payee_splits: Some(0),
            }),
        );
        Self::build(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            counted(stores),
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
        Self::build_on(
            app,
            logs,
            configured_level,
            (Some(router::LedgerFile::in_memory(stores.share())), commits),
            keybindings,
            PersistedState::default(),
            Device::default(),
        )
    }

    /// As [`Harness::new`], on `device`.
    pub fn on_device(app: &'a mut TestAppContext, device: Device<'_>) -> Self {
        let (stores, commits) = Stores::fake(FakeStores::sample(today()));
        Self::build_on(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            (Some(router::LedgerFile::in_memory(stores.share())), commits),
            lib_config::KeyBindingConfig::default(),
            PersistedState::default(),
            device,
        )
    }

    /// What the file explorer draws, `None` while it is closed.
    pub fn explorer(&mut self) -> Option<lib_shell::ExplorerViewModel> {
        self.read(|shell| shell.explorer_snapshot())
    }

    /// Points `:open`, `:new` and `:data`'s file explorer at `folder`, a folder made after the
    /// Shell was built, leaving the Data folder the device read as it is. A test that needs
    /// Settings › General to show the same folder builds on it ([`browsing`]).
    pub fn browse_from(&mut self, folder: &std::path::Path) {
        let folder = folder.to_path_buf();
        self.shell
            .update(self.cx, |shell, _| shell.browse_from_for_test(folder));
    }

    /// The status line's right-hand side as drawn at `size`.
    pub fn status_right(&mut self, size: WindowSize) -> String {
        self.read_app(|shell, cx| shell.status_right_snapshot(size, cx))
    }

    /// The Open Ledger's file, `None` with none open.
    pub fn ledger_path(&mut self) -> Option<std::path::PathBuf> {
        self.read(|shell| shell.ledger_path().map(std::path::Path::to_path_buf))
    }

    /// Whether the status line shows the refusal for something that needs a Ledger with none open.
    pub fn refused_without_ledger(&mut self) -> bool {
        self.read(Root::status_message).as_deref() == Some(NO_LEDGER)
    }

    /// The Shell as start-up leaves it with no Ledger named: no Ledger open, nothing persisted.
    pub fn without_ledger(app: &'a mut TestAppContext) -> Self {
        Self::starting(
            app,
            bin_desktop::Started::NoLedger,
            PersistedState::default(),
        )
    }

    /// The Shell as `run` builds it on what start-up made of the Ledger file, from `persisted`
    /// state: on the Ledger `started` opened, or with none open, raising the Toast a failed open
    /// raises.
    pub fn starting(
        app: &'a mut TestAppContext,
        started: bin_desktop::Started,
        persisted: PersistedState,
    ) -> Self {
        Self::starting_with(app, started, persisted, &lib_config::Config::default())
    }

    /// As [`Harness::starting`], on a device's settings as start-up read them from `config`, so
    /// the Shell lands where its Start view says.
    pub fn starting_with(
        app: &'a mut TestAppContext,
        started: bin_desktop::Started,
        persisted: PersistedState,
        config: &lib_config::Config,
    ) -> Self {
        Self::starting_on(
            app,
            started,
            persisted,
            Device {
                config: config.clone(),
                ..Device::default()
            },
        )
    }

    /// As [`Harness::starting`], on `device`.
    pub fn starting_on(
        app: &'a mut TestAppContext,
        started: bin_desktop::Started,
        persisted: PersistedState,
        device: Device<'_>,
    ) -> Self {
        let failure = match &started {
            bin_desktop::Started::Failed(failure) => Some(failure.clone()),
            _ => None,
        };
        let (stores, commits) = match started {
            bin_desktop::Started::Opened { path, storage } => {
                let writer = storage.writer.clone();
                let (stores, commits) = counted(Stores::from(*storage));
                (Some(router::LedgerFile::on(path, writer, stores)), commits)
            }
            _ => (None, Rc::new(Cell::new(0))),
        };
        let ui = Self::build_on(
            app,
            lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
            None,
            (stores, commits),
            lib_config::KeyBindingConfig::default(),
            persisted,
            device,
        );
        if let Some(failure) = failure {
            ui.shell
                .update(ui.cx, |shell, _| shell.ledger_open_failed(&failure));
        }
        ui
    }

    /// The Shell on `stores`, or with no Ledger open, from `persisted` state, built on `device` as
    /// `run` builds it on start-up's.
    fn build_on(
        app: &'a mut TestAppContext,
        logs: lib_tracing::LogBuffer,
        configured_level: Option<lib_tracing::Levels>,
        (stores, commits): (Option<router::LedgerFile>, Rc<Cell<usize>>),
        keybindings: lib_config::KeyBindingConfig,
        persisted: PersistedState,
        device: Device<'_>,
    ) -> Self {
        // Pins the process-wide Locale to en-US (and loads this bin's Messages), so rendered text
        // never depends on the host; `main` is never run in a test.
        init_for_tests();
        let Device {
            config,
            env,
            conf,
            file,
        } = device;
        let conf_dir = tempfile::tempdir().expect("a temporary directory");
        let conf_path = conf_dir.path().join("personal-ledger.conf");
        if !conf.is_empty() {
            std::fs::write(&conf_path, conf).expect("the conf file is written");
        }
        let writer = lib_config::ConfigWriter::new(
            file.unwrap_or(Some(conf_path)),
            lib_config::Overrides::environment(
                env.iter()
                    .map(|(name, value)| ((*name).to_string(), (*value).to_string())),
            ),
        );
        app.update(|cx| {
            gpui_component::init(cx);
            theme::colours::init(ThemeOverrides::default(), cx);
            theme::colours::set_device_colours(&config, cx);
        });
        let (clock, date) = lib_core::Clock::settable(today());
        let shared = stores.as_ref().map(|ledger| ledger.stores.stores.clone());
        let (shell, cx) = app.add_window_view(|window, cx| {
            let focus_handle = cx.focus_handle();
            window.focus(&focus_handle);
            router::build(
                persisted,
                config.general.start_view(),
                Rc::new(DeviceConfigFile::new(&config, writer)),
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
        cx.run_until_parked();
        Self {
            shell,
            cx,
            logs,
            date,
            commits,
            stores: shared,
            conf_dir,
        }
    }

    /// As [`Harness::new`], on a device's settings as start-up read them from `config`, under
    /// `env`'s `PERSONAL_LEDGER_` variables.
    pub fn with_configuration(
        app: &'a mut TestAppContext,
        config: &lib_config::Config,
        env: &[(&str, &str)],
    ) -> Self {
        Self::on_device(
            app,
            Device {
                config: config.clone(),
                env,
                ..Device::default()
            },
        )
    }

    /// As [`Harness::new`], with the conf file holding `text` before start-up reads it.
    pub fn with_conf_file(app: &'a mut TestAppContext, text: &str) -> Self {
        Self::on_device(
            app,
            Device {
                conf: text,
                ..Device::default()
            },
        )
    }

    /// The `personal-ledger.conf` Settings writes a device's settings to.
    pub fn conf_path(&self) -> std::path::PathBuf {
        self.conf_dir.path().join("personal-ledger.conf")
    }

    /// What Settings has written to [`Harness::conf_path`]; empty before the first write.
    pub fn conf(&self) -> String {
        std::fs::read_to_string(self.conf_path()).unwrap_or_default()
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

    /// Answers the platform's open save dialog with `path`, `None` cancelling it, then lets what
    /// waits on the answer run.
    pub fn answer_save_dialog(&mut self, path: Option<std::path::PathBuf>) {
        self.cx.simulate_new_path_selection(|_| path);
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

    /// Applies `config` while the app runs, as saving `[keybindings]` in Settings does.
    pub fn rebind(&mut self, config: &lib_config::KeyBindingConfig) {
        self.shell.update(self.cx, |shell, cx| {
            shell.set_keymaps(config, cx);
            cx.notify();
        });
        self.cx.run_until_parked();
    }

    /// The code of Institution `id`'s Usual unit, read through the stores; `None` for none.
    pub fn usual_unit_code(&mut self, id: InstitutionId) -> Option<String> {
        self.stores(|stores| {
            let unit = stores
                .institutions
                .borrow()
                .institutions()
                .institution(id)?
                .usual_unit?;
            Some(stores.units.borrow().units().unit(unit)?.code)
        })
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
    pub fn settings_frame(&mut self) -> SettingsSnapshot {
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

    /// A Settings page's state, read back for assertions: `h.settings::<SettingsTagsSnapshot>()`,
    /// or `Option<UnitPricesSnapshot>` for the Unit prices view, `None` while it is closed.
    pub fn settings<S: PageSnapshot>(&mut self) -> S {
        self.read_app(Root::settings_page_snapshot::<S>)
    }

    /// The Units table's rows as `(code, name, type)`, as drawn.
    pub fn unit_rows(&mut self) -> Vec<(String, String, String)> {
        self.settings::<SettingsUnitsSnapshot>()
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
        self.settings::<SettingsUnitsSnapshot>()
            .units
            .into_iter()
            .find(|unit| unit.is_base)
            .map(|unit| unit.code)
    }

    /// The codes of the deactivated Units, drawn dimmed and tagged.
    pub fn inactive_unit_codes(&mut self) -> Vec<String> {
        self.settings::<SettingsUnitsSnapshot>()
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
            balance: choice("balance"),
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
        self.read_app(Root::help_group)
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

    /// A count of the Dashboard's redraws from now on, wherever the Shell is showing.
    pub fn count_dashboard_redraws(&mut self) -> Rc<Cell<usize>> {
        self.shell
            .update(self.cx, |shell, cx| shell.count_dashboard_redraws(cx))
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
