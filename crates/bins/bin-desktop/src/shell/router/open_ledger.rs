//! The Open Ledger (#975): everything the Desktop builds on a Ledger, held by `Root` as one
//! `Option` so the Shell's own parts (the frame, Help, the status line, the Toasts, `NavState`) stand
//! without it. [`OpenLedger::build`] is the one way to build it: start-up calls it on the stores
//! `lib_database::open_storage` opened, and `:open` will call it on another file's. Whether a Ledger
//! is open is whether `Root` holds one.
//!
//! Halfway state: start-up always builds one and nothing drops it yet, so `Root` reads it through
//! `Root::open`, which treats its absence as a bug until the no Ledger open start (#977). The
//! Settings View is inside, This device pages and all, since it is built on the Ledger; #976 and
//! #977 split those pages out.

use std::rc::Rc;

use gpui::{AppContext, Context, Entity, Subscription, Task};
use lib_database::Stores;
use lib_ledger::Ledger;

use super::Root;
use crate::ledger_store::{LedgerStore, LegacyApplied, notify_on};
use crate::settings::state::SettingsView;
use crate::transaction_import::{ImportPage, ImportStores};
use crate::transactions::TransactionsPage;
use crate::view::active::ViewEntities;
use crate::view::dashboard::state::{DashboardSources, DashboardView};

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
    /// The Views that render themselves and handle their own input (ADR-0037): the Dashboard,
    /// Bills, Budgets, Transactions with its Import step, Documents, and Settings. Each reaches
    /// `Root` only through `ViewEvent`.
    pub(super) views: ViewEntities,
    /// Keeps each View's event subscription and each Ledger domain's repaint alive: dropping one
    /// silently stops its events.
    _subscriptions: Vec<Subscription>,
    /// Polls the writer's failure channel; dropped with the Ledger, which stops it.
    _write_failures: Task<()>,
}

impl OpenLedger {
    /// Builds everything on `stores` under `keybindings`, dated by `clock`, and installs the stores'
    /// commit as the one every input ends in. Wires the shared ports first, then the module Setups,
    /// the Ledger's read copy and the Views.
    pub fn build(
        stores: Stores,
        keybindings: &lib_config::KeyBindingConfig,
        clock: &lib_core::Clock,
        cx: &mut Context<'_, Root>,
    ) -> Self {
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
        let today = clock.today();

        // The Transactions and Bills stores are shared: Pay it directly writes both (#779). The
        // Payees ports are shared by the Payees module, the Ledger's forwarding shim and the Bills
        // reads (#831, #832). The Categories store is shared by the Categories module, the Ledger's
        // forwarding shim and the Bills and Payees reads (#858, #859).
        let categories: lib_database::SharedCategoriesStore =
            Rc::new(std::cell::RefCell::new(categories));
        let payees = crate::payees::LedgerBackedPorts::shared(payees, categories.clone());
        // The Tags store is shared by the Ledger's forwarding shim and the Tags page's usage reads
        // (#862).
        let tags: lib_database::SharedTagsStore = Rc::new(std::cell::RefCell::new(tags));
        // The Inventory store is shared by the Ledger's forwarding shim and read copy and the
        // Inventory page (#866, #867).
        let inventory: lib_database::SharedInventoryStore =
            Rc::new(std::cell::RefCell::new(inventory));
        // The Budgets store is shared by the Ledger's forwarding shim and read copy, the Accounts,
        // Units and Categories usage counts and the Categories Monthly budget (#952).
        let budgets: lib_database::SharedBudgetsStore = Rc::new(std::cell::RefCell::new(budgets));
        // The Accounts store is shared by the Ledger's forwarding shim and the Units and Bills reads
        // (#853), and the Institutions' reads of which Accounts name one (#876).
        let accounts = crate::accounts::LedgerBackedPorts::shared(accounts, budgets.clone());
        let institutions =
            crate::institutions::LedgerBackedPorts::shared(institutions, accounts.clone());
        let institutions_setup =
            crate::institutions::InstitutionsSetup::new(institutions.clone(), keybindings);
        let transactions: lib_database::SharedTransactionsStore =
            Rc::new(std::cell::RefCell::new(transactions));
        let bills = crate::bills::LedgerBackedPorts::shared(
            Rc::new(std::cell::RefCell::new(bills)),
            payees.clone(),
            accounts.clone(),
            categories.clone(),
            transactions.clone(),
        );
        let tags_setup = crate::tags::TagsSetup::new(tags.clone(), keybindings);
        let inventory_setup = crate::inventory::InventorySetup::new(inventory.clone(), keybindings);
        let categories_setup =
            crate::categories::CategoriesSetup::new(categories.clone(), keybindings);
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
        let units = crate::units::UnitsSetup::new(
            units,
            accounts.clone(),
            inventory.clone(),
            budgets.clone(),
            &institutions,
            keybindings,
        );
        // The Documents store is shared by the Ledger's forwarding shim and read copy and the Document
        // Types page (#963, #966).
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
        };
        let bills = crate::bills::BillsSetup::new(bills, payments, keybindings);
        let settings = crate::settings::pages::SettingsSetup {
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
            storage.accounts.clone(),
            storage.categories.clone(),
            storage.payees.clone(),
            storage.tags.clone(),
            settings.units.ports.clone(),
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
            units: settings.units.ports.clone(),
            payees: storage.payees.clone(),
        });
        let budgets_ports =
            crate::budgets::ports::DesktopPorts::new(storage.budgets.clone(), budget_reads);
        // The Documents page writes the store through its Interactors, and reads the other
        // domains through their stores (#967).
        let documents_store = crate::document_types::ports::shared(&storage.documents);
        // The Transactions and Bills ports are the truth; the Ledger keeps read copies of them
        // (#774).
        let ledger = cx.new(|_| LedgerStore::with_storage(Ledger::stub_on(today), storage));
        // The Units ports are the truth; the Ledger keeps a read copy of them (#746).
        crate::units::ports::refresh(&settings.units.ports, &ledger, cx);
        let units_ports = settings.units.ports.clone();
        // The Dialogs and the Dashboard read the Categories, Units and Institutions straight from the Ledger.
        let categories_subscription = notify_on(&ledger, lib_ledger::Domain::Categories, cx);
        // A View not yet a module wrote through the Ledger from its own click: one input commits it.
        let legacy_applied = cx.subscribe(&ledger, |shell, _, _: &LegacyApplied, cx| {
            shell.send_shell(lib_shell::ShellInput::LegacyApply, cx);
        });
        let units_subscription = notify_on(&ledger, lib_ledger::Domain::Units, cx);
        let tags_subscription = notify_on(&ledger, lib_ledger::Domain::Tags, cx);
        let institutions_subscription = notify_on(&ledger, lib_ledger::Domain::Institutions, cx);
        let accounts_changed = notify_on(&ledger, lib_ledger::Domain::Accounts, cx);
        let payees_changed = notify_on(&ledger, lib_ledger::Domain::Payees, cx);
        // Documents resolve their Links against the Inventory.
        let inventory_changed = notify_on(&ledger, lib_ledger::Domain::Inventory, cx);
        let settings_view = cx.new(|cx| SettingsView::new(&ledger, clock, settings, cx));
        let settings_subscription = Root::view_event_subscription(&settings_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let settings_view_observer = cx.observe(&settings_view, |_, _, cx| cx.notify());
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
        let dashboard_view = cx.new(|cx| {
            DashboardView::new(
                DashboardSources {
                    ledger: ledger.clone(),
                },
                clock,
                cx,
            )
        });
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
            views: ViewEntities {
                dashboard: dashboard_view,
                bills: bills_view,
                budgets: budgets_view,
                transactions: transactions_view,
                import: import_view,
                documents: documents_view,
                settings: settings_view,
            },
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
                settings_subscription,
                settings_view_observer,
                legacy_applied,
            ],
            _write_failures: Root::listen_for_write_failures(failures, cx),
        }
    }
}
