//! The Open Ledger (#975): everything the Desktop builds on a Ledger, held by `Root` as one
//! `Option` so the Shell's own parts (the frame, Help, the status line, the Toasts, `NavState`) stand
//! without it. [`OpenLedger::build`] is the one way to build it: start-up calls it on the stores
//! `lib_database::open_storage` opened, and `:open` calls it on the file picked. Whether a Ledger
//! is open is whether `Root` holds one.
//!
//! With no Ledger open there is none: start-up builds one only on a Ledger file it opened (#977).
//! The Settings View stands outside it, since its This device pages work with no Ledger open;
//! building the Open Ledger hands that View its This Ledger pages.
//!
//! It is the one place a Ledger's stores and stored reads are assembled ([`LedgerStores`], #1057):
//! `build` hands each page the reads it needs, and keeps the stores for a failed write's reload, a
//! new month and the test hooks. The input door announces each input's writes through its
//! [`Announcer`] (#1081).

mod stores;

pub use stores::{BuiltStores, LedgerStores};

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{AppContext, Context, Entity, Subscription, Task};
use lib_database::{SharedLedger, Storage, Stores, Writer};
use lib_viper::dialog::DialogKeys;

use super::Root;
use crate::announcer::{Announcer, notify_on};
use crate::settings::state::SettingsView;
use crate::view::active::ViewEntities;

/// What an Open Ledger is built on: the stores, shared out so the caller can keep a clone of them
/// (#1117), and the file they read and its writer when they are a Ledger file's rather than a test's
/// in-memory ports.
pub struct LedgerFile {
    pub stores: SharedLedger,
    /// The file the stores read, which the status line shows and the state file remembers at quit.
    pub path: Option<PathBuf>,
    /// The writer the stores save through, flushed when the Ledger is swapped out or the app quits.
    pub writer: Option<Writer>,
}

impl LedgerFile {
    /// The Ledger file at `path`, as `crate::open_ledger_file` opened it.
    pub fn opened(path: PathBuf, storage: Storage) -> Self {
        Self::on(path, storage.writer.clone(), Stores::from(storage))
    }

    /// `stores` read from the Ledger file at `path` and saved through `writer`, for a caller that
    /// wraps the stores first.
    pub fn on(path: PathBuf, writer: Writer, stores: Stores) -> Self {
        Self {
            stores: stores.share(),
            path: Some(path),
            writer: Some(writer),
        }
    }

    /// `stores` with no file behind them: a test's in-memory ports, shared by the test so it keeps
    /// a clone.
    pub fn in_memory(stores: SharedLedger) -> Self {
        Self {
            stores,
            path: None,
            writer: None,
        }
    }
}

/// Everything built on one Ledger file: its stores and the ports over them, the Views that read
/// them, the Announcer the input door announces their writes through, and the listener for its writer's
/// failures. Its commit is installed in `input.rs`'s door as it is built, since `send` can't reach
/// `Root` mid-update.
pub struct OpenLedger {
    /// What the input door announces each input's writes through, which the pages repaint on. It holds no
    /// rows: every page reads the stores below.
    pub(super) announcer: Entity<Announcer>,
    /// The shared stores, reloaded here when a write to them fails, and which a new month's Bill
    /// Schedule is generated through.
    pub(super) stores: LedgerStores,
    /// The Units ports the Units page and the General page share, reloaded here when a write to
    /// them fails.
    pub(super) units: lib_units::SharedPorts,
    /// The Announcer's count when the rail's counts were last read, so the Shell reads them afresh
    /// only after a change rather than after every input.
    pub(super) rail_counted_at: std::cell::Cell<usize>,
    /// The rail's badge counts over the stores, which the Shell's Presenter reads after each input.
    pub(super) rail_reads: lib_database::StoredShellReads,
    /// The Views that render themselves and handle their own input (ADR-0037): the Dashboard,
    /// Bills, Budgets, Transactions with its Import step, and Documents. Each reaches `Root` only
    /// through `ViewEvent`. Settings is `Root`'s own, given its This Ledger pages here.
    pub(super) views: ViewEntities,
    /// The Dialog keys every page's Router hands a Dialog it opens, replaced on a rebind.
    pub(super) dialog_keys: crate::dialog_keys::LiveDialogKeys,
    /// The Ledger file, `None` for a test's in-memory stores.
    path: Option<PathBuf>,
    /// The writer the stores save through, `None` for a test's in-memory stores.
    writer: Option<Writer>,
    /// Keeps each View's event subscription and each Ledger domain's repaint alive: dropping one
    /// silently stops its events.
    _subscriptions: Vec<Subscription>,
    /// The writer's failure channel, which the task below polls and `:close` drains once more.
    failures: Rc<std::sync::mpsc::Receiver<lib_database::WriteFailure>>,
    /// Polls the writer's failure channel; dropped with the Ledger, which stops it.
    _write_failures: Task<()>,
}

impl OpenLedger {
    /// Builds everything on `stores` under `keybindings`, dated by `clock`, and installs the stores'
    /// commit as the one every input ends in. Wires the shared ports first, then the module Setups,
    /// the Announcer and the Views, and gives `settings` its This Ledger pages.
    pub fn build(
        file: LedgerFile,
        keybindings: &lib_config::KeyBindingConfig,
        clock: &lib_core::Clock,
        settings: &Entity<SettingsView>,
        cx: &mut Context<'_, Root>,
    ) -> Self {
        let LedgerFile {
            stores,
            path,
            writer,
        } = file;
        let BuiltStores {
            stores: storage,
            commit,
            failures,
        } = LedgerStores::build(lib_database::LedgerPorts::over(stores, clock.clone()));
        // The stores are the truth: the input door announces what each input wrote through the
        // Announcer, which holds no rows (#1028, #1057, #1081).
        let announcer = cx.new(|_| Announcer::default());
        crate::input::open_ledger(commit, announcer.clone(), cx);
        let failures = Rc::new(failures);
        // The Dialog keymap's chords, which every module Dialog's hints read as it opens, and
        // which a rebind replaces.
        let dialog_keys = crate::dialog_keys::LiveDialogKeys::new(DialogKeys::of(
            &lib_shell::dialog_keymap(keybindings),
        ));
        let units = crate::units::UnitsSetup::new(
            storage.ports.units.clone(),
            storage.ports.unit_deletion.clone(),
            dialog_keys.clone(),
        );
        let institutions_setup = crate::institutions::InstitutionsSetup::new(
            storage.ports.institutions.clone(),
            dialog_keys.clone(),
            keybindings,
        );
        let tags_setup = crate::tags::TagsSetup::new(
            storage.ports.tags.clone(),
            crate::tag_removal::RemovalStores::of(&storage.ports),
            dialog_keys.clone(),
            keybindings,
        );
        let payments = storage.ports.bill_payments.clone();
        // Account delete writes the Accounts, Transactions and Bills stores together (#854).
        let account_deletion = crate::account_deletion::DeletionStores::of(&storage.ports);
        let accounts_setup = crate::accounts::AccountsSetup::new(
            storage.ports.accounts.clone(),
            account_deletion,
            dialog_keys.clone(),
            keybindings,
        );
        let payees_setup = crate::payees::PayeesSetup::new(
            storage.ports.payees.clone(),
            dialog_keys.clone(),
            keybindings,
        );
        let document_types_setup = crate::document_types::DocumentTypesSetup::new(
            storage.ports.documents.clone(),
            dialog_keys.clone(),
            keybindings,
        );
        // Category delete writes the Categories, Transactions, Bills, Payees and Budgets stores
        // together (#1005).
        let categories_setup = crate::categories::CategoriesSetup::new(
            storage.ports.categories.clone(),
            storage.ports.category_save.clone(),
            storage.ports.stores.budgets.clone(),
            crate::category_deletion::DeletionStores::of(&storage.ports),
            dialog_keys.clone(),
            keybindings,
        );
        let inventory_setup = crate::inventory::InventorySetup::new(
            storage.ports.inventory.clone(),
            storage.ports.inventory_reads.clone(),
            dialog_keys.clone(),
            keybindings,
        );
        let bills = crate::bills::BillsSetup::new(
            storage.ports.bills.clone(),
            payments,
            dialog_keys.clone(),
        );
        let settings_setup = crate::settings::pages::SettingsSetup {
            units,
            institutions: institutions_setup,
            payees: payees_setup,
            document_types: document_types_setup,
            accounts: accounts_setup,
            categories: categories_setup,
            tags: tags_setup,
            inventory: inventory_setup,
            ledger: crate::settings::ledger::LedgerPageSetup::new(&storage.ports),
        };

        // The Transactions page and the Import step come from `lib-database`'s `LedgerPorts`.
        let transactions_ports = storage.ports.transactions.clone();
        let import_stores = crate::transaction_import::ImportStores::of(&storage.ports);
        // The Dashboard and the rail's badges read through the reads `LedgerPorts` built.
        let dashboard_reads = storage.ports.dashboard_reads.clone();
        let rail_reads = storage.ports.shell_reads.clone();
        let budgets_ports = storage.ports.budgets.clone();
        let units_ports = settings_setup.units.ports.clone();
        // The pages repaint when a domain they read is announced.
        let categories_subscription = notify_on(&announcer, lib_database::Domain::Categories, cx);
        let units_subscription = notify_on(&announcer, lib_database::Domain::Units, cx);
        let tags_subscription = notify_on(&announcer, lib_database::Domain::Tags, cx);
        let institutions_subscription =
            notify_on(&announcer, lib_database::Domain::Institutions, cx);
        let accounts_changed = notify_on(&announcer, lib_database::Domain::Accounts, cx);
        let payees_changed = notify_on(&announcer, lib_database::Domain::Payees, cx);
        // Documents resolve their Links against the Inventory.
        let inventory_changed = notify_on(&announcer, lib_database::Domain::Inventory, cx);
        settings.update(cx, |view, cx| {
            view.open_ledger(&announcer, clock, keybindings, settings_setup, cx)
        });
        let transactions_view = crate::transactions::transactions_page(
            transactions_ports,
            &announcer,
            keybindings,
            clock.clone(),
            cx,
        );
        crate::transactions::router::install(&transactions_view, cx);
        let import_view = crate::transaction_import::import_page(
            import_stores,
            &announcer,
            keybindings,
            clock.clone(),
            cx,
        );
        let budgets_view = crate::budgets::router::budgets_page(
            budgets_ports,
            &announcer,
            keybindings,
            clock.clone(),
            dialog_keys.clone(),
            cx,
        );
        let dashboard_view = crate::dashboard::dashboard_page(
            dashboard_reads,
            &announcer,
            keybindings,
            clock.clone(),
            cx,
        );
        let bills_view =
            crate::bills::bills_page(bills, &announcer, keybindings, clock.clone(), cx);
        // The Documents page writes the store through its Interactors, and reads the other
        // domains through their stores (#967).
        let documents_view = crate::documents::view::page::documents_page(
            &storage.ports.documents,
            Rc::new(crate::documents::files::DiskFiles),
            storage.ports.document_reads.clone(),
            announcer.clone(),
            keybindings,
            clock.clone(),
            dialog_keys.clone(),
            cx,
        );
        let views = ViewEntities {
            dashboard: dashboard_view,
            bills: bills_view,
            budgets: budgets_view,
            transactions: transactions_view,
            import: import_view,
            documents: documents_view,
        };
        // Each View's events reach `Root`, and a change in one repaints it: the status line reads
        // the View's state, and which View shows turns on the Import step being open.
        let signals = Root::page_signals(cx);
        let mut subscriptions: Vec<Subscription> = views
            .all()
            .flat_map(|view| view.subscribe(&signals, cx))
            .collect();
        subscriptions.extend([
            tags_subscription,
            payees_changed,
            categories_subscription,
            units_subscription,
            accounts_changed,
            institutions_subscription,
            inventory_changed,
        ]);
        Self {
            announcer,
            stores: storage,
            units: units_ports,
            // No count read yet, so the first refresh after the Ledger opens reads its counts.
            rail_counted_at: std::cell::Cell::new(usize::MAX),
            rail_reads,
            views,
            dialog_keys,
            path,
            writer,
            _subscriptions: subscriptions,
            _write_failures: Root::listen_for_write_failures(failures.clone(), cx),
            failures,
        }
    }

    /// The Ledger file, `None` for a test's in-memory stores.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Waits for the writes sent so far to land, then takes the failures not yet reported: a write
    /// that failed since the last poll. Nothing is rolled back, since the Ledger is closing.
    pub fn flush_and_take_failures(&self) -> Vec<lib_database::WriteFailure> {
        self.flush();
        self.failures.try_iter().collect()
    }

    /// Waits for every write sent so far to land in the file. A failure is logged, since the
    /// Ledger it belongs to is closing or the app quitting, with nothing left to roll back.
    pub fn flush(&self) {
        if let Some(writer) = &self.writer {
            crate::quit(writer);
        }
    }
}

/// A Ledger swapped out, or dropped with the window, flushes its writes first, so nothing it took
/// is lost however it goes.
impl Drop for OpenLedger {
    fn drop(&mut self) {
        self.flush();
    }
}
