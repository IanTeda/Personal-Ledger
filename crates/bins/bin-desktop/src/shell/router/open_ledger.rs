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
//! `build` hands each page the ports and reads it needs from the `LedgerPorts` `lib-database`
//! built (#1057), and keeps those for a failed write's reload, a new month and the test hooks
//! (#1202). The input door announces each input's
//! writes through its [`Announcer`] (#1081).

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{AppContext, Context, Entity, Subscription, Task};
use lib_database::{SharedLedger, Storage, Stores, Writer};
use lib_viper::dialog::DialogKeys;

use super::Root;
use crate::announcer::{Announcer, notify_on};
use crate::settings::state::SettingsView;
use crate::view::active::ViewEntities;

/// What an Open Ledger is built on: the stores, and the file they read and its writer when they are
/// a Ledger file's rather than a test's in-memory ports. The stores are private, so the Desktop's
/// own code never holds a domain's store: [`OpenLedger::build`] builds the ports on them (#1221).
pub struct LedgerFile {
    stores: LedgerStores,
    /// The file the stores read, which the status line shows and the state file remembers at quit.
    pub path: Option<PathBuf>,
    /// The writer the stores save through, flushed when the Ledger is swapped out or the app quits.
    pub writer: Option<Writer>,
}

/// The stores a [`LedgerFile`] holds: unshared for a file the Desktop opened, which only its
/// `LedgerPorts` ever shares out, or shared out already by a caller keeping its own clone of them
/// (a test, #1117).
enum LedgerStores {
    /// A Ledger file's stores, which `LedgerPorts`' `new` shares out and builds on.
    Unshared(Stores),
    /// Stores a caller shared out itself and keeps a handle on, which `LedgerPorts`' `over` builds on.
    Shared(SharedLedger),
}

impl LedgerFile {
    /// The Ledger file at `path`, as `crate::open_ledger_file` opened it.
    pub fn opened(path: PathBuf, storage: Storage) -> Self {
        Self {
            path: Some(path),
            writer: Some(storage.writer.clone()),
            stores: LedgerStores::Unshared(Stores::from(storage)),
        }
    }

    /// `stores` read from the Ledger file at `path` and saved through `writer`, shared out by a
    /// caller that wraps them first and keeps a clone of them: the UI tests' file-backed Shell.
    pub fn on(path: PathBuf, writer: Writer, stores: SharedLedger) -> Self {
        Self {
            stores: LedgerStores::Shared(stores),
            path: Some(path),
            writer: Some(writer),
        }
    }

    /// `stores` with no file behind them: a test's in-memory ports, shared by the test so it keeps
    /// a clone.
    pub fn in_memory(stores: SharedLedger) -> Self {
        Self {
            stores: LedgerStores::Shared(stores),
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
    /// The Ledger's ports, reloaded here when a write to them fails, and which a new month's Bill
    /// Schedule is generated through.
    pub(super) ports: lib_database::LedgerPorts,
    /// The Announcer's count when the rail's counts were last read, so the Shell reads them afresh
    /// only after a change rather than after every input.
    pub(super) rail_counted_at: std::cell::Cell<usize>,
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
    /// Where each price response lands, `None` with no price feed installed.
    pub(super) price_responses:
        Option<Rc<std::cell::RefCell<lib_units::presenter::FetchedPricesPresenter>>>,
    /// Polls the price feed's response channel; dropped with the Ledger, which stops it and drops
    /// the responses still to come.
    _price_responses: Option<Task<()>>,
    /// The minute timer reaching the Units' scheduled price check; dropped with the Ledger.
    _price_checks: Option<Task<()>>,
}

impl OpenLedger {
    /// Builds everything on `stores` under `keybindings`, dated by `clock`, its Price Source keys
    /// read with `api_key_secret`, and installs the stores' commit as the one every input ends in. Wires the shared ports first, then the module Setups,
    /// the Announcer and the Views, and gives `settings` its This Ledger pages.
    pub fn build(
        file: LedgerFile,
        keybindings: &lib_config::KeyBindingConfig,
        api_key_secret: &lib_database::ApiKeySecret,
        clock: &lib_core::Clock,
        settings: &Entity<SettingsView>,
        cx: &mut Context<'_, Root>,
    ) -> Self {
        let LedgerFile {
            stores,
            path,
            writer,
        } = file;
        let (ports, failures) = match stores {
            LedgerStores::Unshared(stores) => {
                lib_database::LedgerPorts::new(stores, clock.clone(), api_key_secret.clone())
            }
            LedgerStores::Shared(shared) => {
                lib_database::LedgerPorts::over(shared, clock.clone(), api_key_secret.clone())
            }
        };
        // The stores are the truth: the input door announces what each input wrote through the
        // Announcer, which holds no rows (#1028, #1057, #1081).
        let announcer = cx.new(|_| Announcer::default());
        crate::input::open_ledger(ports.commit(), announcer.clone(), cx);
        let failures = Rc::new(failures);
        // The Dialog keymap's chords, which every module Dialog's hints read as it opens, and
        // which a rebind replaces.
        let dialog_keys = crate::dialog_keys::LiveDialogKeys::new(DialogKeys::of(
            &lib_shell::dialog_keymap(keybindings),
        ));
        // The Units' price fetching, on a feed of this Ledger's own, dropped with it (ADR-0062).
        let feed = crate::price_feed::feed(cx);
        let fetching = feed
            .as_ref()
            .map(|feed| lib_units::PriceFetching::shared(feed.transport.clone()));
        let price_responses = fetching.clone().map(|fetching| {
            Rc::new(std::cell::RefCell::new(
                lib_units::presenter::FetchedPricesPresenter::new(
                    ports.units(),
                    fetching,
                    crate::units::now_of(clock),
                ),
            ))
        });
        // Fetch every due Unit as the Ledger opens, then every minute while it stays open.
        if let Some(presenter) = &price_responses {
            presenter.borrow_mut().fetch_due();
        }
        let price_checks = price_responses
            .as_ref()
            .map(|_| Root::check_prices_every_minute(cx));
        let units = crate::units::UnitsSetup::new(
            ports.units(),
            fetching,
            ports.unit_deletion(),
            dialog_keys.clone(),
            clock.clone(),
            keybindings,
        );
        let institutions_setup = crate::institutions::InstitutionsSetup::new(
            ports.institutions(),
            dialog_keys.clone(),
            keybindings,
        );
        let tags_setup = crate::tags::TagsSetup::new(
            ports.tags(),
            ports.tag_removal(),
            dialog_keys.clone(),
            clock.clone(),
            keybindings,
        );
        let payments = ports.bill_payments();
        // Account delete writes the Accounts, Transactions and Bills stores together (#854).
        let account_deletion = ports.account_deletion();
        let accounts_setup = crate::accounts::AccountsSetup::new(
            ports.accounts(),
            account_deletion,
            dialog_keys.clone(),
            clock.clone(),
            keybindings,
        );
        let payees_setup = crate::payees::PayeesSetup::new(
            ports.payees(),
            dialog_keys.clone(),
            clock.clone(),
            keybindings,
        );
        let document_types_setup = crate::document_types::DocumentTypesSetup::new(
            ports.documents(),
            dialog_keys.clone(),
            keybindings,
        );
        // Category delete writes the Categories, Transactions, Bills, Payees and Budgets stores
        // together (#1005).
        let categories_setup = crate::categories::CategoriesSetup::new(
            ports.categories(),
            ports.category_save(),
            ports.category_budget_months(),
            ports.category_deletion(),
            dialog_keys.clone(),
            keybindings,
        );
        let inventory_setup = crate::inventory::InventorySetup::new(
            ports.inventory(),
            ports.inventory_reads(),
            dialog_keys.clone(),
            keybindings,
        );
        let bills = crate::bills::BillsSetup::new(
            ports.bills(),
            payments,
            dialog_keys.clone(),
            clock.clone(),
            keybindings,
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
            ledger: ports.settings_ledger(),
        };

        // The Transactions page and the Import step come from `lib-database`'s `LedgerPorts`.
        let transactions_ports = ports.transactions();
        let import_ports = ports.transaction_import(Rc::new(
            lib_transaction_import::SeededStatement::stub(clock.today()),
        ));
        // The Dashboard reads through the reads `LedgerPorts` built.
        let dashboard_reads = ports.dashboard_reads();
        let budgets_ports = ports.budgets();
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
            view.open_ledger(&announcer, keybindings, settings_setup, cx)
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
            import_ports,
            &announcer,
            keybindings,
            clock.clone(),
            cx,
        );
        let (budgets_presenter, budgets_setup) = crate::budgets::BudgetsSetup::new(
            budgets_ports,
            clock.clone(),
            dialog_keys.clone(),
            keybindings,
        );
        let budgets_view =
            crate::budgets::router::budgets_page(budgets_presenter, budgets_setup, &announcer, cx);
        let dashboard_view = crate::dashboard::dashboard_page(
            dashboard_reads,
            &announcer,
            keybindings,
            clock.clone(),
            cx,
        );
        let (bills_presenter, bills) = bills;
        let bills_view = crate::bills::bills_page(bills_presenter, bills, &announcer, cx);
        // The Documents page writes the store through its Interactors, and reads the other
        // domains through their stores (#967).
        let documents_view = crate::documents::router::documents_page(
            &ports.documents(),
            Rc::new(crate::documents::files::DiskFiles),
            ports.document_reads(),
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
            ports,
            // No count read yet, so the first refresh after the Ledger opens reads its counts.
            rail_counted_at: std::cell::Cell::new(usize::MAX),
            views,
            dialog_keys,
            path,
            writer,
            _subscriptions: subscriptions,
            _write_failures: Root::listen_for_write_failures(failures.clone(), cx),
            failures,
            price_responses,
            _price_responses: feed.map(|feed| Root::listen_for_price_responses(feed.responses, cx)),
            _price_checks: price_checks,
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
