//! The Shell's root Router: [`Root`], the desktop window's root render tree
//! (`docs/ux/desktop-mockups/README.md`'s "Component tree"), and [`build`], which both the app and
//! the UI tests build it through. `Root` replaced the feasibility cycle's `TabBar`-driven screen
//! cycling as the real navigation entry point (ADR-0016).
//!
//! Assembles the static Shell from `docs/ux/desktop-mockups/01-shell/README.md`'s "1a"
//! spec (issue #148) and drives it with real keyboard interaction: `Tab`/`Shift-Tab`
//! focus-zone cycling and `j`/`k`/`Down`/`Up`/`gg`/`G`/`Ctrl-d`/`Ctrl-u`/`Enter` movement,
//! scoped strictly to whichever zone (`NavState::focus`) currently has it (issue #149); the
//! `g`-prefix jump chords (`g d`, `g t`, ...) and the `Normal`/`Insert`/`Command`/`Search`
//! mode transitions (issue #150); the command palette (issue #151); the collapsed rail, its
//! `b`-key/click toggle, and its hover tooltip (issue #152). `?`'s help overlay (issue #169, a
//! Dialog in `InputMode::Dialog`). No `View` trait (ADR-0016): `Root` forwards to the active View
//! through `view::active::ActiveViewEntity` (ADR-0037), and `render_view` in `router/render.rs`
//! draws the Views not yet moved onto it.
//!
//! The file holds the `Root` struct, [`build`], construction, persisted setters, the accessor
//! surface, `impl Focusable`, per-domain wiring and the unit tests. Each cross-cutting concern
//! lives in a bare-named child file (`commands`, `dialogs`, `explorer`, `focus`, `key_dispatch`,
//! `log_feed`, `rail`, `render`, `root_router`, `status`, `toasts`) under `router/` that is an
//! `impl Root` block, not the real module of the same name; a `_ui` suffix (`documents_ui`,
//! `budgets_ui`, ...) marks per-domain wiring.
//!
//! `Root` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

mod budgets_ui;

mod commands;
mod date_clock;
mod dialogs;
mod documents_ui;
mod explorer;
mod focus;
mod history;
mod key_dispatch;
mod log_feed;
mod rail;
mod render;
mod root_router;
mod snapshots;
mod status;
mod toasts;
mod transactions_ui;
mod view_events;
mod write_failures;

#[cfg(test)]
use commands::record_history;
pub use date_clock::{DATE_POLL, wall_clock};
pub use key_dispatch::PENDING_G_TIMEOUT;
use lib_core::LocationId;
pub use log_feed::LOG_COALESCE;
#[doc(hidden)]
pub use snapshots::{
    BillRowSnapshot, BillsSnapshot, BudgetRowSnapshot, BudgetsSnapshot, ChipSnapshot,
    DashboardBillSnapshot, DocumentsSnapshot, HelpGroup, ImportRowSnapshot, ImportSnapshot,
    PaletteSnapshot, SettingsAccountsSnapshot, SettingsCategoriesSnapshot, SettingsDisplaySnapshot,
    SettingsDocumentTypesSnapshot, SettingsGeneralSnapshot, SettingsInstitutionsSnapshot,
    SettingsInventorySnapshot, SettingsPayeesSnapshot, SettingsSnapshot, SettingsTagsSnapshot,
    SettingsTracingSnapshot, SettingsUnitsSnapshot, StatusSnapshot, ToastSnapshot, ToastsSnapshot,
    TransactionsSnapshot,
};
pub use write_failures::WRITE_FAILURE_POLL;

use std::rc::Rc;
use std::time::Duration;

use crate::persistence::PersistedState;
use lib_database::Stores;

/// The Shell's Presenter over the Desktop's Settings pages.
type ShellPresenter = lib_shell::ShellPresenter<SettingsPage>;

use crate::ledger_store::{LedgerStore, LegacyApplied, notify_on};
use crate::settings::state::SettingsView;
use crate::view::transactions::state::{TransactionsSources, TransactionsView};

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, ScrollHandle, Subscription, Task,
};

use lib_ledger::Ledger;
use lib_shell::{NavInput, ToastInput};

use crate::view::active::{ActiveViewEntity, ViewEntities};
use crate::view::budgets::state::{BudgetsSources, BudgetsView};
use crate::view::documents::state::{DocumentsSources, DocumentsView};
use crate::view::event::ViewMode;
use crate::{
    budgets,
    documents::{DocumentsMode, LibraryScope, LibrarySort},
    navigation::active_view::ActiveView,
    navigation::explorer::{ExplorerFilters, FileExplorer},
    navigation::nav::{NavState, RailMode},
    settings::{SettingsPage, display::DisplayPreferences},
    shell::state::ShellState,
    shell::view::dialog_host::OpenDialog,
    shell::view::{rail as rail_view, window_size::WindowSize},
    view::dashboard::state::{DashboardSources, DashboardView},
};
use lib_categories::Category;

/// The collapsed rail's own hover-reveal delay (`docs/ux/desktop-mockups/01-shell/README.md`'s
/// "1c" tooltip spec) -- deliberately the same 500ms `gpui`'s own built-in `.tooltip()` uses,
/// even though this tooltip is hand-rolled (row-anchored, not cursor-anchored -- see
/// `shell::view::rail::primary::collapsed_tooltip`'s doc) rather than that builtin.
const TOOLTIP_REVEAL_DELAY: Duration = Duration::from_millis(500);

/// Keybinding overrides from config that `Root` needs at construction.
pub struct ShellBindings {
    /// The whole `[keybindings]` section, which the Shell's global and Dialog keymaps and every
    /// VIPER module's keymap read their overrides from.
    pub keybindings: lib_config::KeyBindingConfig,
}

/// The live log capture the Tracing page shows, and the configured `log` level it opens on.
pub struct ShellLogs {
    pub buffer: lib_tracing::LogBuffer,
    pub configured_level: Option<lib_tracing::Levels>,
}

/// Builds `Root` from persisted state the way the app does, so tests construct it identically.
/// `clock` is a parameter so tests can pin, and move, the date the seeded stub data and scopes hang
/// off. `stores` is where the Units, the Institutions, the Accounts, the Categories, the Tags, the Payees, the
/// Transactions, the Bills and the Inventory are kept, the app's SQLite adapters or a test's in-memory ports, with the commit
/// every input ends in (`crate::input::send`) and the writer's failure channel, on which a failed
/// write rolls back.
pub fn build(
    persisted: PersistedState,
    bindings: ShellBindings,
    logs: ShellLogs,
    stores: Stores,
    clock: lib_core::Clock,
    focus_handle: FocusHandle,
    cx: &mut Context<'_, Root>,
) -> Root {
    let Stores {
        units,
        institutions,
        accounts,
        categories,
        tags,
        payees,
        transactions,
        bills,
        inventory,
        commit,
        failures,
    } = stores;
    crate::input::install(commit, cx.weak_entity(), cx);
    let today = clock.today();
    let nav = NavState::starting_at(persisted.noun);
    let documents_state = (
        DocumentsMode::from_id(persisted.documents_mode.as_deref()),
        LibraryScope::from_id(
            persisted.documents_scope.as_deref().unwrap_or("all"),
            &crate::documents::types::default_types(),
            today,
        ),
        LibrarySort::from_id(persisted.documents_sort.as_deref()),
    );
    let settings_page = persisted
        .settings_page
        .as_deref()
        .map(SettingsPage::from_id)
        .unwrap_or_default();

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
    // The Inventory store is the Ledger's forwarding shim's and read copy's alone until the
    // Inventory page holds it (#866, #867).
    let inventory: lib_database::SharedInventoryStore = Rc::new(std::cell::RefCell::new(inventory));
    let institutions = crate::institutions::LedgerBackedPorts::shared(institutions);
    // The Accounts store is shared by the Ledger's forwarding shim and the Units and Bills reads
    // (#853).
    let accounts = crate::accounts::LedgerBackedPorts::shared(accounts);
    let institutions_setup =
        crate::institutions::InstitutionsSetup::new(institutions.clone(), &bindings.keybindings);
    let transactions: lib_database::SharedTransactionsStore =
        Rc::new(std::cell::RefCell::new(transactions));
    let bills = crate::bills::LedgerBackedPorts::shared(
        Rc::new(std::cell::RefCell::new(bills)),
        payees.clone(),
        accounts.clone(),
        categories.clone(),
    );
    let payees_setup = crate::payees::PayeesSetup::new(payees.clone(), &bindings.keybindings);
    let tags_setup = crate::tags::TagsSetup::new(tags.clone(), &bindings.keybindings);
    let categories_setup =
        crate::categories::CategoriesSetup::new(categories.clone(), &bindings.keybindings);
    let payments = crate::bill_payments::payments(transactions.clone(), &bills);
    // Account delete writes the Accounts, Transactions and Bills stores together (#854).
    let account_deletion = crate::account_deletion::DeletionStores {
        deletion: crate::account_deletion::deletion(accounts.clone(), transactions.clone(), &bills),
        bills: bills.clone(),
    };
    let accounts_setup = crate::accounts::AccountsSetup::new(
        accounts.clone(),
        account_deletion,
        &bindings.keybindings,
    );
    let units = crate::units::UnitsSetup::new(units, accounts.clone(), &bindings.keybindings);
    let storage = crate::ledger_store::LedgerSetup {
        institutions,
        accounts,
        categories,
        tags,
        payees,
        transactions,
        bills: bills.clone(),
        inventory,
    };
    let bills = crate::bills::BillsSetup::new(bills, payments, &bindings.keybindings);
    let settings = crate::settings::pages::SettingsSetup {
        units,
        institutions: institutions_setup,
        payees: payees_setup,
        accounts: accounts_setup,
        categories: categories_setup,
        tags: tags_setup,
    };
    let mut shell = Root::with_clock(nav, focus_handle, clock, settings, bills, storage, cx);
    shell.set_start_sidebar_minimised(persisted.start_sidebar_minimised, cx);
    shell.set_settings_page(settings_page, cx);
    shell.set_explorer_filters(persisted.explorer_filters);
    shell.set_documents_state(documents_state.0, documents_state.1, documents_state.2, cx);
    shell.set_keymaps(&bindings.keybindings);
    shell.start_toast_clock(cx);
    shell.start_date_clock(cx);
    shell.listen_for_write_failures(failures, cx);
    shell.set_log_capture(
        logs.buffer,
        crate::settings::tracing::log::TracingLevel::from_configured(logs.configured_level),
        cx,
    );
    shell
}

/// Owns the shell's render tree, and the Shell's Presenter with where the person is.
pub struct Root {
    focus_handle: FocusHandle,
    /// The View area's sideways scroll, for a View wider than the space the Shell leaves it
    /// (ADR-0042). The Shell itself never scrolls sideways.
    view_x_scroll_handle: ScrollHandle,
    /// The timer of a pressed prefix (`g`) -- `Some` while the Shell's Presenter waits for the
    /// key that finishes the sequence (`g g`, or `g` and a jump key). Dropped by the next keypress
    /// regardless of outcome, which cancels it, or run out once [`PENDING_G_TIMEOUT`] passes,
    /// which sends the Presenter `PrefixExpired`. The executor's clock, not the wall clock, so a
    /// loaded test run can't lapse a `g` between two simulated keys.
    prefix_timer: Option<Task<()>>,
    /// The mode the active View entered itself (`ViewEvent::EnterMode`), so that mode's keys are
    /// its to read. The Shell's own `a` enters an `Insert` no View asked for, which swallows every
    /// key as before.
    view_mode: Option<ViewMode>,
    /// The Shell's own state (the palette, Dialog and rail tooltip), see [`ShellState`] and
    /// ADR-0032. The status message and Toasts are the Presenter's.
    state: ShellState,
    /// Debug builds only: the Kind `F9` raises next, so each can be eyeballed.
    #[cfg(debug_assertions)]
    debug_toast_kind: usize,
    /// Previously run palette command names, most-recent-first, deduplicated -- outlives any
    /// one `Palette` (see [`record_history`]), cloned into a fresh `Palette` on every `:` open
    /// so `^r` can reach commands run in an earlier palette session.
    command_history: Vec<String>,
    /// The "1e" file explorer's own state -- `Some` only while `:open`'s dialog is on screen,
    /// mirroring `Option<Palette>`. `NavState::mode` stays `InputMode::Command` for as long as
    /// this is `Some` (see [`Self::run_command`]'s own doc), so the two together -- rather than
    /// a third `InputMode` variant -- are what "the file explorer is open" means.
    file_explorer: Option<FileExplorer>,
    /// The `:open` explorer's footer checkboxes, kept here so they outlive each dialog and reach
    /// `persistence::PersistedState` at quit.
    explorer_filters: ExplorerFilters,
    /// The only copy of the Ledger (ADR-0038). Every page below reads through it and holds no rows
    /// of its own.
    ledger: Entity<LedgerStore>,
    /// The Units ports the Units page and the General page share, reloaded here when a write to
    /// them fails.
    units: crate::units::DesktopPorts,
    /// "Today" for the seed data and, later, the `this year` filter -- read at construction and
    /// moved on only when [`Self::presenter`] sees the Clock cross into a new calendar month, so
    /// everything derived from it (the seeded dates, the default range) agrees within a month.
    today: chrono::NaiveDate,
    /// The Shell as a `lib-shell` module: the status line, the Toasts, where the person is (the
    /// noun, focus and mode), the global keymap, and the Clock (the local date, or a test's pinned
    /// day; every View is built with it), whose month turn it notices.
    presenter: ShellPresenter,
    /// The Bills ports the Bills module and `LedgerStore` share, which a new month's Bill
    /// Schedule is generated through.
    bills: crate::bills::DesktopPorts,
    /// The Budgets, read through their store Entity (ADR-0032). Shared so the Budgets surface and
    /// Categories 5c read and write the same Category Limits, and so they survive leaving and
    /// re-entering either view.
    /// The Transactions, read through their store Entity (ADR-0032): saved-in-memory state that
    /// survives leaving and re-entering the page. Deleting an account deletes its transactions
    /// with it.
    /// The Bill Plans and Bill Schedule, read through their store Entity (ADR-0032).
    /// The Views that render themselves and handle their own input (ADR-0037): Bills, Budgets,
    /// Transactions with its Import step, Documents, and Settings. Each reaches `Root` only
    /// through `ViewEvent`.
    views: ViewEntities,
    /// The Documents and Document Types, read through their store Entity (ADR-0032). Shared:
    /// Settings › Documents edits the Types, and the Documents page, the rail badge and the
    /// Inventory removal read the Documents. Keeps each View's event subscription alive: dropping
    /// one silently stops its events.
    view_subscriptions: Vec<Subscription>,
}

impl Root {
    /// Builds the Shell on `clock`, which dates the seeded stub data from the day it reads now, so
    /// tests pin it rather than live at the mercy of the wall clock. Creates the Accounts Entities,
    /// so it takes a `Context`.
    pub fn with_clock(
        nav: NavState,
        focus_handle: FocusHandle,
        clock: lib_core::Clock,
        settings: crate::settings::pages::SettingsSetup,
        bills: crate::bills::BillsSetup,
        storage: crate::ledger_store::LedgerSetup,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let today = clock.today();
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
        let settings_view = cx.new(|cx| SettingsView::new(&ledger, &clock, settings, cx));
        let settings_subscription = Self::view_event_subscription(&settings_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let settings_view_observer = cx.observe(&settings_view, |_, _, cx| cx.notify());
        let transactions_view = cx.new(|cx| {
            TransactionsView::new(
                TransactionsSources {
                    ledger: ledger.clone(),
                },
                &clock,
                cx,
            )
        });
        let transactions_subscription = Self::view_event_subscription(&transactions_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let transactions_view_observer = cx.observe(&transactions_view, |_, _, cx| cx.notify());
        let budgets_current = ledger
            .read(cx)
            .ledger()
            .budgets()
            .default_budget()
            .map_or(budgets::PERSONAL_SPENDING_ID, |budget| budget.id);
        let budgets_view = cx.new(|cx| {
            BudgetsView::new(
                BudgetsSources {
                    ledger: ledger.clone(),
                },
                budgets_current,
                &clock,
                cx,
            )
        });
        let budgets_subscription = Self::view_event_subscription(&budgets_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let budgets_view_observer = cx.observe(&budgets_view, |_, _, cx| cx.notify());
        let dashboard_view = cx.new(|cx| {
            DashboardView::new(
                DashboardSources {
                    ledger: ledger.clone(),
                },
                &clock,
                cx,
            )
        });
        let dashboard_subscription = Self::view_event_subscription(&dashboard_view, cx);
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
        let bills_subscription = Self::view_event_subscription(&bills_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let bills_view_observer = cx.observe(&bills_view, |_, _, cx| cx.notify());
        let documents_view = cx.new(|cx| {
            DocumentsView::new(
                DocumentsSources {
                    ledger: ledger.clone(),
                },
                &clock,
                cx,
            )
        });
        let documents_subscription = Self::view_event_subscription(&documents_view, cx);
        // The status line reads the page's state, so a change in it repaints `Root` too.
        let documents_view_observer = cx.observe(&documents_view, |_, _, cx| cx.notify());
        Self {
            focus_handle,
            view_x_scroll_handle: ScrollHandle::new(),
            prefix_timer: None,
            view_mode: None,
            state: ShellState {
                palette: None,
                collapsed_rail_tooltip: None,
                rail_override: None,
                hover_generation: 0,
                dialog: None,
                dialog_confirm: None,
                hosted: None,
                dialog_keymap: lib_shell::dialog_keymap(&lib_config::KeyBindingConfig::default()),
                pending_colour_change: None,
                undo_menu_open: false,
            },
            #[cfg(debug_assertions)]
            debug_toast_kind: 0,
            command_history: Vec::new(),
            file_explorer: None,
            explorer_filters: ExplorerFilters::default(),
            ledger,
            units: units_ports,
            today,
            presenter: ShellPresenter::new(clock, nav),
            bills: bills_ports,
            views: ViewEntities {
                dashboard: dashboard_view,
                bills: bills_view,
                budgets: budgets_view,
                transactions: transactions_view,
                documents: documents_view,
                settings: settings_view,
            },
            view_subscriptions: vec![
                dashboard_subscription,
                dashboard_view_observer,
                budgets_subscription,
                budgets_view_observer,
                bills_subscription,
                bills_view_observer,
                transactions_subscription,
                transactions_view_observer,
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
        }
    }

    /// The Categories tree, read straight from the Ledger (ADR-0038).
    fn categories<'a>(&self, cx: &'a App) -> &'a [Category] {
        self.ledger.read(cx).ledger().categories()
    }

    /// The Bill Plans, from the Ledger's read copy of the Bills ports.
    fn bill_plans<'a>(&self, cx: &'a App) -> &'a [crate::bills::BillPlan] {
        self.ledger.read(cx).ledger().bill_plans()
    }

    /// The Bill Schedule entries, from the Ledger's read copy of the Bills ports.
    fn bill_entries<'a>(&self, cx: &'a App) -> &'a [crate::bills::BillScheduleEntry] {
        self.ledger.read(cx).ledger().bill_entries()
    }

    /// Restores the Documents mode, Library scope and sort from the last run.
    pub fn set_documents_state(
        &self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
        cx: &mut App,
    ) {
        self.set_documents_persisted(mode, scope, sort, cx);
    }

    /// The last-visited Settings page, for persistence.
    pub fn settings_page(&self, cx: &App) -> SettingsPage {
        self.views.settings.read(cx).selected_section()
    }

    pub fn set_settings_page(&mut self, section: SettingsPage, cx: &mut App) {
        self.navigate(NavInput::RestoreSettingsPage(section));
        self.views
            .settings
            .update(cx, |v, cx| v.set_selected_section(cx, section));
    }

    pub fn explorer_filters(&self) -> ExplorerFilters {
        self.explorer_filters
    }

    pub fn set_explorer_filters(&mut self, filters: ExplorerFilters) {
        self.explorer_filters = filters;
    }

    /// The primary rail drawn at `window_size`: a `b` override made at that Window Size, or the
    /// Window Size's own default.
    pub fn primary_rail(&self, window_size: WindowSize, cx: &App) -> RailMode {
        rail_view::rail_mode(
            window_size,
            self.start_sidebar_minimised(cx),
            self.state.rail_override,
        )
    }

    pub fn start_sidebar_minimised(&self, cx: &App) -> bool {
        self.display(cx).start_sidebar_minimised
    }

    pub fn set_start_sidebar_minimised(&mut self, minimised: bool, cx: &mut App) {
        let display = DisplayPreferences {
            start_sidebar_minimised: minimised,
            ..self.display(cx)
        };
        self.views
            .settings
            .update(cx, |view, cx| view.set_display(display, cx));
    }

    /// The Display Preferences, which the Settings View owns (ADR-0037).
    fn display(&self, cx: &App) -> DisplayPreferences {
        self.views.settings.read(cx).display(cx)
    }

    /// The global keys and the keys every module Dialog shares, under `config`'s overrides.
    pub fn set_keymaps(&mut self, config: &lib_config::KeyBindingConfig) {
        self.presenter.set_keymap(config);
        self.state.dialog_keymap = lib_shell::dialog_keymap(config);
    }

    /// Stands in for opening a ledger, so a test can reach the context rail without the file
    /// explorer's filesystem walk.
    #[doc(hidden)]
    pub fn open_ledger_for_test(&mut self) {
        self.navigate(NavInput::OpenLedger);
    }

    /// Empties the Inventory, standing in for removing every Location until the Remove dialog
    /// lands, so a test can reach the Inventory page's empty state.
    #[doc(hidden)]
    pub fn empty_inventory_for_test(&mut self, cx: &mut App) {
        let ids: Vec<LocationId> = self
            .ledger
            .read(cx)
            .ledger()
            .inventory()
            .locations
            .iter()
            .map(|location| location.id)
            .collect();
        self.ledger.update(cx, |store, cx| {
            for id in ids {
                assert!(
                    store.apply(cx, lib_ledger::RemoveLocation(id)).is_ok(),
                    "a Location listed a moment ago must still be removable"
                );
            }
        });
    }

    /// Leaves no Unit in use by an Account, a Bill Plan or a Budget, so a test can delete a
    /// seeded Unit instead of meeting the in-use refusal.
    #[doc(hidden)]
    pub fn drop_unit_users_for_test(&mut self, cx: &mut App) {
        self.ledger
            .update(cx, |store, cx| store.drop_unit_users_for_test(cx));
    }

    /// Moves every Bill Plan to pay from Account `to`, so a test can delete the Account another Plan
    /// paid from instead of meeting the Bill Plans refusal.
    #[doc(hidden)]
    pub fn move_bill_plans_for_test(&mut self, to: lib_core::AccountId, cx: &mut App) {
        {
            let mut ports = self.bills.borrow_mut();
            for mut plan in lib_bills::BillsPorts::bills(&*ports).bill_plans() {
                plan.account_id = to;
                lib_bills::BillsPorts::split_mut(&mut *ports)
                    .bills
                    .upsert_bill_plan(plan);
            }
        }
        crate::bills::ports::refresh(&self.bills, &self.ledger, cx);
    }

    /// Leaves no Unit quoted in another, so a test can delete a seeded Unit instead of meeting the
    /// quoted refusal.
    #[doc(hidden)]
    pub fn unquote_units_for_test(&mut self, cx: &mut App) {
        crate::units::ports::unquote_for_test(&self.units, &self.ledger, cx);
    }

    /// The status line's flash message (e.g. "not yet built"), if one is showing.
    #[doc(hidden)]
    pub fn status_message(&self) -> Option<String> {
        self.presenter.view_model().status
    }

    /// Previously run palette commands, most recent first.
    #[doc(hidden)]
    pub fn command_history(&self) -> &[String] {
        &self.command_history
    }

    pub fn nav(&self) -> &NavState {
        self.presenter.nav()
    }

    /// Moves the person to another View, zone or mode, through the Shell's Presenter.
    fn navigate(&mut self, input: NavInput<SettingsPage>) {
        self.presenter.navigate(input);
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Which View owns the keyboard, page status and main pane right now.
    fn active_view(&self, cx: &App) -> ActiveView {
        ActiveView::derive(
            self.nav().noun(),
            self.views.settings.read(cx).selected_section(),
            self.views.transactions.read(cx).import_open(),
        )
    }

    /// The active View as `Root` forwards to it (ADR-0037).
    fn active_view_entity(&self, cx: &App) -> ActiveViewEntity {
        ActiveViewEntity::new(self.active_view(cx), &self.views)
    }

    /// Hands the Display Preferences to the Views that read them and the Toasts Preference to
    /// the Toasts, after a Settings change.
    fn push_display_preferences(&mut self, display: DisplayPreferences, cx: &mut App) {
        self.presenter
            .toast(ToastInput::Display(lib_toast::Display {
                toasts_on: display.toasts_on,
                ..self.presenter.toasts().display()
            }));
        let (date_style, glyphs, density) = (
            display.date_style,
            display.status_glyphs,
            display.row_density,
        );
        self.views
            .dashboard
            .update(cx, |view, cx| view.set_display(date_style, glyphs, cx));
        self.views
            .bills
            .update(cx, |view, cx| view.set_display(date_style, glyphs, cx));
        self.views.transactions.update(cx, |view, cx| {
            view.set_display(date_style, glyphs, density, cx)
        });
        self.views
            .documents
            .update(cx, |view, cx| view.set_display(date_style, cx));
    }
}

impl Focusable for Root {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::hints::{
        settings_colour_grid_hints, settings_display_hints, settings_index_hints,
        settings_plain_page_hints,
    };
    use crate::transactions;
    use crate::transactions::Transaction;
    use crate::view::transactions::hints::{filter_hints, transactions_hints};
    use chrono::Local;
    use lib_accounts::Account;

    fn seeded_ledger() -> (Vec<Account>, Vec<Category>, Vec<Transaction>) {
        let accounts = lib_accounts::default_accounts();
        let categories = lib_categories::default_categories();
        let transactions = transactions::default_transactions(
            &accounts,
            &categories,
            &lib_payees::default_payees(),
            &lib_tags::default_tags(),
            Local::now().date_naive(),
        );
        (accounts, categories, transactions)
    }

    #[test]
    fn each_settings_focus_has_its_own_legend() {
        crate::locale::init_for_tests();
        let keys = |hints: Vec<(&'static str, String)>| -> Vec<&'static str> {
            hints.into_iter().map(|(key, _)| key).collect()
        };
        assert_eq!(keys(settings_index_hints()), ["j/k", "l/enter"]);
        assert_eq!(
            keys(settings_display_hints()),
            ["j/k", "h/l", "enter", "esc"]
        );
        assert_eq!(keys(settings_colour_grid_hints())[1], "enter");
        assert_eq!(keys(settings_plain_page_hints()), ["j/k", "h"]);
        assert!(
            settings_display_hints()
                .iter()
                .all(|(_, action)| !action.is_empty())
        );
    }

    #[test]
    fn deleting_a_leaf_category_toasts_the_splits_it_moved() {
        crate::locale::init_for_tests();
        let mut ledger = Ledger::stub();
        let rent = lib_categories::find_by_name(ledger.categories(), "Rent").expect("seeded");
        let deleted = ledger.delete_category(rent).expect("a leaf");
        let text =
            lib_locale::msg::toast_category_deleted(&deleted.name, deleted.splits_moved as i64);
        assert!(text.starts_with("Deleted category Rent · "), "{text}");
    }

    #[test]
    fn record_history_pushes_a_new_entry_to_the_front() {
        let mut history = vec!["accounts".to_string()];
        record_history(&mut history, "dashboard");
        assert_eq!(
            history,
            vec!["dashboard".to_string(), "accounts".to_string()]
        );
    }

    #[test]
    fn record_history_moves_a_repeated_entry_to_the_front_without_duplicating_it() {
        let mut history = vec![
            "accounts".to_string(),
            "dashboard".to_string(),
            "reports".to_string(),
        ];
        record_history(&mut history, "reports");
        assert_eq!(
            history,
            vec![
                "reports".to_string(),
                "accounts".to_string(),
                "dashboard".to_string(),
            ]
        );
    }

    #[test]
    fn the_empty_state_hint_tags_both_commands_as_clickable_spans() {
        crate::locale::init_for_tests();
        let tagged: Vec<(Option<String>, String)> =
            crate::msg::desktop_empty_state_hint(":open", ":new")
                .into_iter()
                .filter(|segment| segment.tag.is_some())
                .map(|segment| (segment.tag, segment.text))
                .collect();
        assert_eq!(
            tagged,
            vec![
                (Some("open".to_string()), ":open".to_string()),
                (Some("new".to_string()), ":new".to_string()),
            ]
        );
    }

    #[test]
    fn the_empty_state_and_hint_strips_render_in_the_pseudo_locale() {
        crate::locale::init_for_tests();
        lib_locale::with_locale(lib_locale::Locale::EnXa, || {
            assert!(crate::msg::desktop_empty_state_title().starts_with('['));
            let hint = crate::msg::desktop_empty_state_hint(":open", ":new");
            assert!(hint.iter().any(|s| s.tag.as_deref() == Some("open")));
            assert!(hint.iter().any(|s| s.tag.as_deref() == Some("new")));
            for (_, action) in settings_plain_page_hints()
                .into_iter()
                .chain(filter_hints())
                .chain(transactions_hints())
            {
                assert!(action.starts_with('['), "{action}");
            }
        });
    }
}
