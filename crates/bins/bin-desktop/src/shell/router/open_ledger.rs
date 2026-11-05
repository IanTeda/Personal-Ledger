//! The Open Ledger (#975): everything the Desktop builds on a Ledger, held by `Root` as one
//! `Option` so the Shell's own parts (the frame, Help, the status line, the Toasts, `NavState`) stand
//! without it. [`OpenLedger::build`] is the one way to build it: start-up calls it on the stores
//! `lib_database::open_storage` opened, and `:open` calls it on the file picked. Whether a Ledger
//! is open is whether `Root` holds one.
//!
//! With no Ledger open there is none: start-up builds one only on a Ledger file it opened (#977).
//! The Settings View stands outside it, since its This device pages work with no Ledger open;
//! building the Open Ledger hands that View its This Ledger pages.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{AppContext, Context, Entity, Subscription, Task};
use lib_database::{Storage, Stores, Writer};
use lib_ledger::Ledger;

use super::Root;
use crate::dashboard::DashboardPage;
use crate::ledger_store::{LedgerStore, notify_on};
use crate::settings::state::SettingsView;
use crate::transaction_import::{ImportPage, ImportStores};
use crate::transactions::TransactionsPage;
use crate::view::active::ViewEntities;

/// What an Open Ledger is built on: the stores, and the file they read and its writer when they are
/// a Ledger file's rather than a test's in-memory ports.
pub struct LedgerFile {
    pub stores: Stores,
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
            stores,
            path: Some(path),
            writer: Some(writer),
        }
    }

    /// `stores` with no file behind them: a test's in-memory ports.
    pub fn in_memory(stores: Stores) -> Self {
        Self {
            stores,
            path: None,
            writer: None,
        }
    }
}

/// Everything built on one Ledger file: its stores (through the Ledger's read copy and the ports
/// kept here), the Views that read them, and the listener for its writer's failures. Its commit is
/// installed in `input.rs`'s door as it is built, since `send` can't reach `Root` mid-update.
pub struct OpenLedger {
    /// The only copy of the Ledger (ADR-0038). Every page below reads through it and holds no rows
    /// of its own.
    pub(super) ledger: Entity<LedgerStore>,
    /// The Units ports the Units page and the General page share, reloaded here when a write to
    /// them fails.
    pub(super) units: crate::units::DesktopPorts,
    /// The Bills ports the Bills module and `LedgerStore` share, which a new month's Bill
    /// Schedule is generated through.
    pub(super) bills: crate::bills::DesktopPorts,
    /// The rail's badge counts over the stores, which the Shell's Presenter reads after each input.
    pub(super) rail_reads: lib_database::StoredShellReads,
    /// The Views that render themselves and handle their own input (ADR-0037): the Dashboard,
    /// Bills, Budgets, Transactions with its Import step, and Documents. Each reaches `Root` only
    /// through `ViewEvent`. Settings is `Root`'s own, given its This Ledger pages here.
    pub(super) views: ViewEntities,
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
    /// the Ledger's read copy and the Views, and gives `settings` its This Ledger pages.
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
        let Stores {
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
            commit,
            failures,
        } = stores;
        crate::input::open_ledger(commit, cx);
        let failures = Rc::new(failures);
        let today = clock.today();

        // The Transactions and Bills stores are shared: Pay it directly writes both (#779), and
        // Payees counts what names a Payee in them (#992). The Payees ports are shared by the
        // Payees module, the cross-domain adapters, the Ledger's read copies and the Bills reads
        // (#831, #832). The Categories store is shared by the Categories module, Category delete,
        // the Ledger's read copy and the Bills and Payees reads (#858, #859, #1005).
        let categories: lib_database::SharedCategoriesStore =
            Rc::new(std::cell::RefCell::new(categories));
        let transactions: lib_database::SharedTransactionsStore =
            Rc::new(std::cell::RefCell::new(transactions));
        let bills: lib_database::SharedBillsStore = Rc::new(std::cell::RefCell::new(bills));
        let payees = crate::payees::LedgerBackedPorts::shared(
            payees,
            lib_database::StoredPayeeReads {
                transactions: transactions.clone(),
                bills: bills.clone(),
                categories: categories.clone(),
            },
        );
        // The Tags store is shared by the Tags page and Tag remove and merge (#862, #1003).
        let tags: lib_database::SharedTagsStore = Rc::new(std::cell::RefCell::new(tags));
        // The Inventory store is shared by the Inventory page and the Documents reads (#866, #867).
        let inventory: lib_database::SharedInventoryStore =
            Rc::new(std::cell::RefCell::new(inventory));
        // The Budgets store is shared by the Budgets module, the Ledger's read copy, Category
        // delete, the Accounts, Units and Categories usage counts and the Categories Monthly budget
        // (#952).
        let budgets: lib_database::SharedBudgetsStore = Rc::new(std::cell::RefCell::new(budgets));
        // The Accounts store is shared by the Accounts module, the Ledger's read copy and the Units and Bills reads
        // (#853), and the Institutions' reads of which Accounts name one (#876).
        let accounts = crate::accounts::LedgerBackedPorts::shared(
            accounts,
            lib_database::StoredAccountReads {
                transactions: transactions.clone(),
                budgets: budgets.clone(),
            },
        );
        let institutions =
            crate::institutions::LedgerBackedPorts::shared(institutions, accounts.clone());
        let institutions_setup =
            crate::institutions::InstitutionsSetup::new(institutions.clone(), keybindings);
        // The Units ports count what names each Unit in the Accounts, Transactions, Bills,
        // Budgets, Inventory and Institutions stores, and answer the Bills reads of the Unit codes
        // (#993), so they are built before the Bills ports.
        let units = crate::units::UnitsSetup::new(
            units,
            lib_database::StoredUnitReads {
                accounts: accounts.clone(),
                transactions: transactions.clone(),
                bills: bills.clone(),
                budgets: budgets.clone(),
                inventory: inventory.clone(),
                institutions: institutions.clone(),
            },
            &institutions,
            keybindings,
        );
        let bills = crate::bills::LedgerBackedPorts::shared(
            bills,
            payees.clone(),
            accounts.clone(),
            categories.clone(),
            transactions.clone(),
            units.ports.clone(),
        );
        let tags_setup = crate::tags::TagsSetup::new(
            tags.clone(),
            lib_database::StoredTagReads {
                transactions: transactions.clone(),
                accounts: accounts.clone(),
                units: units.ports.clone(),
            },
            keybindings,
        );
        let inventory_setup = crate::inventory::InventorySetup::new(inventory.clone(), keybindings);
        // Category delete writes the Categories, Transactions, Bills, Payees and Budgets stores
        // together (#1005).
        let categories_setup = crate::categories::CategoriesSetup::new(
            categories.clone(),
            crate::category_deletion::DeletionStores::new(
                categories.clone(),
                transactions.clone(),
                &bills,
                payees.clone(),
                budgets.clone(),
            ),
            keybindings,
        );
        let payments = crate::bill_payments::payments(transactions.clone(), &bills);
        // Account delete writes the Accounts, Transactions and Bills stores together (#854).
        let account_deletion = crate::account_deletion::DeletionStores {
            deletion: crate::account_deletion::deletion(
                accounts.clone(),
                transactions.clone(),
                &bills,
            ),
            bills: bills.clone(),
        };
        let accounts_setup =
            crate::accounts::AccountsSetup::new(accounts.clone(), account_deletion, keybindings);
        // The Documents store is shared by the Documents page, the Document Types page and the
        // Ledger's Documents reads (#963, #966).
        let documents: lib_database::SharedDocumentsStore =
            Rc::new(std::cell::RefCell::new(documents));
        let payees_setup = crate::payees::PayeesSetup::new(payees.clone(), keybindings);
        let document_types_setup =
            crate::document_types::DocumentTypesSetup::new(documents.clone(), keybindings);
        let storage = crate::ledger_store::LedgerSetup {
            institutions,
            accounts,
            categories,
            tags,
            payees,
            transactions,
            bills: bills.clone(),
            budgets,
            inventory,
            documents,
            units: units.ports.clone(),
        };
        let bills = crate::bills::BillsSetup::new(bills, payments, keybindings);
        let settings_setup = crate::settings::pages::SettingsSetup {
            units,
            institutions: institutions_setup,
            payees: payees_setup,
            document_types: document_types_setup,
            accounts: accounts_setup,
            categories: categories_setup,
            tags: tags_setup,
            inventory: inventory_setup,
        };

        // The Transactions page reads the stores, not the Ledger's read copies (#923).
        let transactions_ports = crate::transactions::ports::DesktopPorts::shared(
            storage.transactions.clone(),
            lib_database::StoredTransactionReads {
                accounts: storage.accounts.clone(),
                categories: storage.categories.clone(),
                payees: storage.payees.clone(),
                tags: storage.tags.clone(),
                units: settings_setup.units.ports.clone(),
            },
        );
        // The Import writes the Payees and Transactions stores and reads the Accounts and
        // Categories, as their modules hold them.
        let import_stores = ImportStores {
            payees: storage.payees.clone(),
            transactions: storage.transactions.clone(),
            accounts: storage.accounts.clone(),
            categories: storage.categories.clone(),
        };
        // The Budgets page reads the other domains through their stores, not the Ledger's read
        // copies (#952).
        let budget_reads = Rc::new(lib_database::StoredBudgetReads {
            categories: storage.categories.clone(),
            accounts: storage.accounts.clone(),
            transactions: storage.transactions.clone(),
            bills: storage.bills.borrow().store(),
            units: settings_setup.units.ports.clone(),
            payees: storage.payees.clone(),
        });
        let budgets_ports =
            crate::budgets::ports::DesktopPorts::new(storage.budgets.clone(), budget_reads);
        // The Dashboard reads its Bills and Budgets through the stores too, not the Ledger's read
        // copies (#1024).
        let dashboard_reads = Rc::new(lib_database::StoredDashboardReads {
            bills: storage.bills.borrow().store(),
            budgets: storage.budgets.clone(),
            categories: storage.categories.clone(),
            units: settings_setup.units.ports.clone(),
            accounts: storage.accounts.clone(),
            transactions: storage.transactions.clone(),
            payees: storage.payees.clone(),
        });
        // The rail's badges read their counts through the Shell's port, answered over the stores
        // (#1026).
        let rail_reads = lib_database::StoredShellReads {
            bills: storage.bills.borrow().store(),
            budgets: storage.budgets.clone(),
            documents: storage.documents.clone(),
            categories: storage.categories.clone(),
            accounts: storage.accounts.clone(),
            transactions: storage.transactions.clone(),
            units: settings_setup.units.ports.clone(),
            payees: storage.payees.clone(),
        };
        // The Documents page writes the store through its Interactors, and reads the other
        // domains through their stores (#967).
        let documents_store = crate::document_types::ports::shared(&storage.documents);
        // The Transactions and Bills ports are the truth; the Ledger keeps read copies of them
        // (#774).
        let ledger = cx.new(|_| LedgerStore::with_storage(Ledger::stub_on(today), storage));
        // The Units ports are the truth; the Ledger keeps a read copy of them (#746).
        crate::units::ports::refresh(&settings_setup.units.ports, &ledger, cx);
        let units_ports = settings_setup.units.ports.clone();
        // The Dialogs read the Categories, Units and Institutions straight from the Ledger.
        let categories_subscription = notify_on(&ledger, lib_ledger::Domain::Categories, cx);
        let units_subscription = notify_on(&ledger, lib_ledger::Domain::Units, cx);
        let tags_subscription = notify_on(&ledger, lib_ledger::Domain::Tags, cx);
        let institutions_subscription = notify_on(&ledger, lib_ledger::Domain::Institutions, cx);
        let accounts_changed = notify_on(&ledger, lib_ledger::Domain::Accounts, cx);
        let payees_changed = notify_on(&ledger, lib_ledger::Domain::Payees, cx);
        // Documents resolve their Links against the Inventory.
        let inventory_changed = notify_on(&ledger, lib_ledger::Domain::Inventory, cx);
        settings.update(cx, |view, cx| {
            view.open_ledger(&ledger, clock, settings_setup, cx)
        });
        let transactions_view =
            cx.new(|cx| TransactionsPage::new(transactions_ports, &ledger, clock.clone(), cx));
        crate::transactions::router::install(&transactions_view, cx);
        let transactions_subscription = Root::view_event_subscription(&transactions_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let transactions_view_observer = cx.observe(&transactions_view, |_, _, cx| cx.notify());
        let import_view = cx.new(|_| ImportPage::new(import_stores, ledger.clone(), clock.clone()));
        // The status line reads the step's state, and which View shows turns on it being open.
        let import_view_observer = cx.observe(&import_view, |_, _, cx| cx.notify());
        let budgets_view = crate::budgets::router::budgets_page(
            budgets_ports,
            ledger.clone(),
            lib_budgets::keymap::budgets_page_keymap(keybindings),
            clock.clone(),
            cx,
        );
        let budgets_subscription = Root::view_event_subscription(&budgets_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let budgets_view_observer = cx.observe(&budgets_view, |_, _, cx| cx.notify());
        let dashboard_view =
            cx.new(|cx| DashboardPage::new(dashboard_reads, &ledger, clock.clone(), cx));
        let dashboard_subscription = Root::view_event_subscription(&dashboard_view, cx);
        let dashboard_view_observer = cx.observe(&dashboard_view, |_, _, cx| cx.notify());
        let bills_ports = bills.ports.clone();
        let bills_view = crate::bills::bills_page(
            bills.ports,
            bills.payments,
            ledger.clone(),
            bills.keymap,
            clock.clone(),
            cx,
        );
        let bills_subscription = Root::view_event_subscription(&bills_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let bills_view_observer = cx.observe(&bills_view, |_, _, cx| cx.notify());
        let documents_view = crate::documents::view::page::documents_page(
            documents_store,
            Rc::new(crate::documents::files::DiskFiles),
            ledger.clone(),
            lib_documents::keymap::documents_page_keymap(keybindings),
            clock.clone(),
            cx,
        );
        let documents_subscription = Root::view_event_subscription(&documents_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let documents_view_observer = cx.observe(&documents_view, |_, _, cx| cx.notify());
        Self {
            ledger,
            units: units_ports,
            bills: bills_ports,
            rail_reads,
            views: ViewEntities {
                dashboard: dashboard_view,
                bills: bills_view,
                budgets: budgets_view,
                transactions: transactions_view,
                import: import_view,
                documents: documents_view,
            },
            path,
            writer,
            _subscriptions: vec![
                dashboard_subscription,
                dashboard_view_observer,
                budgets_subscription,
                budgets_view_observer,
                bills_subscription,
                bills_view_observer,
                transactions_subscription,
                transactions_view_observer,
                import_view_observer,
                documents_subscription,
                documents_view_observer,
                tags_subscription,
                payees_changed,
                categories_subscription,
                units_subscription,
                accounts_changed,
                institutions_subscription,
                inventory_changed,
            ],
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
