//! `Shell` -- the desktop window's root render tree (`docs/ux/desktop-mockups/README.md`'s
//! "Component tree"), replacing the feasibility cycle's `TabBar`-driven screen cycling as the
//! real navigation entry point (ADR-0016).
//!
//! Assembles the static chrome from `docs/ux/desktop-mockups/01-shell/README.md`'s "1a"
//! spec (issue #148) and drives it with real keyboard interaction: `Tab`/`Shift-Tab`
//! focus-zone cycling and `j`/`k`/`Down`/`Up`/`gg`/`G`/`Ctrl-d`/`Ctrl-u`/`Enter` movement,
//! scoped strictly to whichever zone (`NavState::focus`) currently has it (issue #149); the
//! `g`-prefix jump chords (`g d`, `g t`, ...) and the `Normal`/`Insert`/`Command`/`Search`
//! mode transitions (issue #150); the command palette (issue #151); the collapsed rail, its
//! `b`-key/click toggle, and its hover tooltip (issue #152). `?`'s help overlay (issue #169,
//! `InputMode::Help`). No `View` trait (ADR-0016): `Shell` forwards to the active View through
//! `view::active::ActiveViewEntity` (ADR-0037), and `render_view` in `app/render.rs` draws the
//! Views not yet moved onto it.
//!
//! This is the root of the `app` module, so the `Shell` struct's own module and its child files
//! sit in `app/`. The file holds the `Shell` struct, construction, persisted setters, the accessor
//! surface, `impl Focusable`, per-domain wiring and the unit tests. Each cross-cutting concern
//! lives in a bare-named child file (`commands`, `dialogs`, `explorer`, `focus`, `key_dispatch`,
//! `log_feed`, `rail`, `render`, `status`, `toasts`) under `app/` that is an `impl Shell` block,
//! not the real module of the same name; a `_ui` suffix (`documents_ui`, `bills_ui`, ...)
//! marks per-domain wiring.
//!
//! `Shell` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

mod bills_ui;
mod budgets_ui;

mod commands;
mod dialogs;
mod documents_ui;
mod explorer;
mod focus;
mod history;
mod key_dispatch;
mod log_feed;
mod rail;
mod render;
mod snapshots;
mod status;
mod toasts;
mod transactions_ui;
mod view_events;

#[cfg(test)]
use commands::record_history;
pub use key_dispatch::PENDING_G_TIMEOUT;
use lib_core::LocationId;
pub use log_feed::LOG_COALESCE;
use log_feed::LOG_LIST_OVERDRAW;
#[doc(hidden)]
pub use snapshots::{
    AccountFormSnapshot, BillRowSnapshot, BillsSnapshot, BudgetRowSnapshot, BudgetsSnapshot,
    ChipSnapshot, DashboardBillSnapshot, DocumentsSnapshot, ImportRowSnapshot, ImportSnapshot,
    PaletteSnapshot, SettingsAccountsSnapshot, SettingsCategoriesSnapshot, SettingsDisplaySnapshot,
    SettingsDocumentTypesSnapshot, SettingsInstitutionsSnapshot, SettingsInventorySnapshot,
    SettingsPayeesSnapshot, SettingsSnapshot, SettingsTagsSnapshot, SettingsTracingSnapshot,
    SettingsUnitsSnapshot, StatusSnapshot, ToastSnapshot, ToastsSnapshot, TransactionsSnapshot,
};

use std::time::Duration;

use crate::ledger_store::{LedgerStore, notify_on};
use crate::settings::institutions::InstitutionsPage;
use crate::settings::inventory::InventoryPage;
use crate::settings::state::{SettingsSources, SettingsView};
use crate::settings::units::UnitsPage;
use crate::settings::{about::AboutPage, data_backup::DataBackupPage, sync_server::SyncServerPage};
use crate::view::transactions::state::{TransactionsSources, TransactionsView};

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, ListAlignment, ListState,
    ScrollHandle, Subscription, Task,
};

use lib_ledger::Ledger;
use lib_toast::Toasts;

use crate::settings::accounts::AccountsPage;
use crate::settings::categories::CategoriesPage;
use crate::settings::documents::DocumentsPage;
use crate::settings::payees::{PayeesPage, PayeesSources};
use crate::settings::tags::TagsPage;
use crate::view::active::{ActiveViewEntity, ViewEntities};
use crate::view::bills::state::{BillsSources, BillsView};
use crate::view::budgets::state::{BudgetsSources, BudgetsView};
use crate::view::documents::state::{DocumentsSources, DocumentsView};
use crate::view::event::ViewMode;
use crate::{
    budgets,
    chrome::dialog_host::OpenDialog,
    chrome::state::ChromeState,
    chrome::{rail as chrome_rail, window_size::WindowSize},
    documents::{DocumentsMode, LibraryScope, LibrarySort},
    navigation::active_view::ActiveView,
    navigation::explorer::{ExplorerFilters, FileExplorer},
    navigation::key_router,
    navigation::nav::{NavState, RailMode},
    settings::tracing::{
        TracingPage,
        log::{LogView, TracingLevel},
    },
    settings::{
        SettingsPage,
        display::{DisplayPage, DisplayPreferences},
    },
    view::dashboard::state::{DashboardSources, DashboardView},
};
use lib_categories::Category;

/// The collapsed rail's own hover-reveal delay (`docs/ux/desktop-mockups/01-shell/README.md`'s
/// "1c" tooltip spec) -- deliberately the same 500ms `gpui`'s own built-in `.tooltip()` uses,
/// even though this tooltip is hand-rolled (row-anchored, not cursor-anchored -- see
/// `chrome::rail::primary::collapsed_tooltip`'s doc) rather than that builtin.
const TOOLTIP_REVEAL_DELAY: Duration = Duration::from_millis(500);

/// Owns the shell's render tree and the live `NavState`.
pub struct Shell {
    nav: NavState,
    focus_handle: FocusHandle,
    /// The View area's sideways scroll, for a View wider than the space the Chrome leaves it
    /// (ADR-0042). The Chrome itself never scrolls sideways.
    view_x_scroll_handle: ScrollHandle,
    /// The timer of a pressed `g` -- `Some` while a completing chord (`gg`, or `g` + a jump
    /// key) is still possible. Cleared by the next keypress regardless of outcome, so an
    /// abandoned `g` never leaks into the one after it, or by the timer itself once
    /// [`PENDING_G_TIMEOUT`] passes. The executor's clock, not the wall clock, so a loaded test
    /// run can't lapse a `g` between two simulated keys.
    pending_g: Option<Task<()>>,
    /// The mode the active View entered itself (`ViewEvent::EnterMode`), so that mode's keys are
    /// its to read. The router's own `a` enters an `Insert` no View asked for, which swallows every
    /// key as before.
    view_mode: Option<ViewMode>,
    /// Chrome's own state (the status message, Toasts, palette, Dialog and rail tooltip), see
    /// [`ChromeState`] and ADR-0032.
    chrome: ChromeState,
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
    /// The only copy of the Ledger (ADR-0038). Every per-domain store below reads through it and
    /// holds no rows of its own.
    ledger: Entity<LedgerStore>,
    /// "Today" for the seed data and, later, the `this year` filter -- read once at construction so
    /// everything derived from it (the seeded dates, the default range) agrees for the whole run.
    today: chrono::NaiveDate,
    /// The Budgets, read through their store Entity (ADR-0032). Shared so the Budgets surface and
    /// Categories 5c read and write the same Category Limits, and so they survive leaving and
    /// re-entering either view.
    /// The Transactions, read through their store Entity (ADR-0032): saved-in-memory state that
    /// survives leaving and re-entering the page. Deleting an account deletes its transactions
    /// with it.
    /// The Bill Plans and Bill Schedule, read through their store Entity (ADR-0032).
    /// The Views that render themselves and handle their own input (ADR-0037): Bills, Budgets,
    /// Transactions with its Import step, Documents, and Settings. Each reaches `Shell` only
    /// through `ViewEvent`.
    views: ViewEntities,
    /// The Documents and Document Types, read through their store Entity (ADR-0032). Shared: Settings › Documents edits the Types, and the Documents page, the rail
    /// badge and the Inventory removal read the Documents.
    /// Keeps each View's event subscription alive: dropping one silently stops its events.
    view_subscriptions: Vec<Subscription>,
}

impl Shell {
    /// Builds the Shell with the date the seeded stub data is anchored to supplied, so tests are
    /// not at the mercy of the wall clock. Creates the Accounts Entities, so it takes a `Context`.
    pub fn with_today(
        nav: NavState,
        focus_handle: FocusHandle,
        today: chrono::NaiveDate,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today)));
        // The Dialogs and the Dashboard read the Categories, Units and Institutions straight from the Ledger.
        let categories_subscription = notify_on(&ledger, lib_ledger::Domain::Categories, cx);
        let units_subscription = notify_on(&ledger, lib_ledger::Domain::Units, cx);
        let tags_subscription = notify_on(&ledger, lib_ledger::Domain::Tags, cx);
        let institutions_subscription = notify_on(&ledger, lib_ledger::Domain::Institutions, cx);
        let accounts_changed = notify_on(&ledger, lib_ledger::Domain::Accounts, cx);
        let categories_page = cx.new(|cx| CategoriesPage::new(ledger.clone(), today, cx));
        let categories_page_subscription = Self::view_event_subscription(&categories_page, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let categories_page_observer = cx.observe(&categories_page, |_, _, cx| cx.notify());
        let accounts_view = cx.new(|cx| AccountsPage::new(ledger.clone(), cx));
        let accounts_subscription = Self::view_event_subscription(&accounts_view, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let accounts_view_observer = cx.observe(&accounts_view, |_, _, cx| cx.notify());
        let payees_changed = notify_on(&ledger, lib_ledger::Domain::Payees, cx);
        let payees_view = cx.new(|cx| {
            PayeesPage::new(
                PayeesSources {
                    ledger: ledger.clone(),
                },
                cx,
            )
        });
        let payees_view_subscription = Self::view_event_subscription(&payees_view, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let payees_view_observer = cx.observe(&payees_view, |_, _, cx| cx.notify());
        let tags_page = cx.new(|cx| TagsPage::new(ledger.clone(), cx));
        let tags_page_subscription = Self::view_event_subscription(&tags_page, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let tags_page_observer = cx.observe(&tags_page, |_, _, cx| cx.notify());
        let documents_page = cx.new(|cx| DocumentsPage::new(ledger.clone(), cx));
        let documents_page_subscription = Self::view_event_subscription(&documents_page, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let documents_page_observer = cx.observe(&documents_page, |_, _, cx| cx.notify());
        let inventory_page = cx.new(|cx| InventoryPage::new(ledger.clone(), cx));
        let inventory_page_subscription = Self::view_event_subscription(&inventory_page, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let inventory_page_observer = cx.observe(&inventory_page, |_, _, cx| cx.notify());
        // Documents resolve their Links against the Inventory.
        let inventory_changed = notify_on(&ledger, lib_ledger::Domain::Inventory, cx);
        let display_page = cx.new(|_| DisplayPage::default());
        let display_page_subscription = Self::view_event_subscription(&display_page, cx);
        // The status line reads the page's focus, so a change in it repaints `Shell` too.
        let display_page_observer = cx.observe(&display_page, |_, _, cx| cx.notify());
        let units_page = cx.new(|cx| UnitsPage::new(ledger.clone(), cx));
        let units_page_subscription = Self::view_event_subscription(&units_page, cx);
        let institutions_page = cx.new(|cx| InstitutionsPage::new(ledger.clone(), cx));
        let institutions_page_subscription = Self::view_event_subscription(&institutions_page, cx);
        let sync_server_page = cx.new(|_| SyncServerPage);
        let sync_server_page_subscription = Self::view_event_subscription(&sync_server_page, cx);
        let data_backup_page = cx.new(|_| DataBackupPage);
        let data_backup_page_subscription = Self::view_event_subscription(&data_backup_page, cx);
        let about_page = cx.new(|_| AboutPage);
        // A private, empty capture until `set_log_capture` hands over the real one.
        let tracing_page = cx.new(|_| {
            TracingPage::new(
                LogView::new(
                    lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
                    TracingLevel::default(),
                ),
                ListState::new(0, ListAlignment::Top, LOG_LIST_OVERDRAW),
            )
        });
        let tracing_page_subscription = Self::view_event_subscription(&tracing_page, cx);
        let settings_view = cx.new(|cx| {
            SettingsView::new(
                SettingsSources {
                    display: display_page,
                    institutions: institutions_page,
                    sync_server: sync_server_page,
                    data_backup: data_backup_page,
                    about: about_page,
                    units: units_page,
                    accounts: accounts_view,
                    categories: categories_page,
                    payees: payees_view,
                    tags: tags_page,
                    documents: documents_page,
                    inventory: inventory_page,
                    tracing: tracing_page,
                },
                cx,
            )
        });
        let settings_subscription = Self::view_event_subscription(&settings_view, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let settings_view_observer = cx.observe(&settings_view, |_, _, cx| cx.notify());
        let transactions_view = cx.new(|cx| {
            TransactionsView::new(
                TransactionsSources {
                    ledger: ledger.clone(),
                },
                today,
                cx,
            )
        });
        let transactions_subscription = Self::view_event_subscription(&transactions_view, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
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
                today,
                cx,
            )
        });
        let budgets_subscription = Self::view_event_subscription(&budgets_view, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let budgets_view_observer = cx.observe(&budgets_view, |_, _, cx| cx.notify());
        let dashboard_view = cx.new(|cx| {
            DashboardView::new(
                DashboardSources {
                    ledger: ledger.clone(),
                },
                today,
                cx,
            )
        });
        let dashboard_subscription = Self::view_event_subscription(&dashboard_view, cx);
        let dashboard_view_observer = cx.observe(&dashboard_view, |_, _, cx| cx.notify());
        let bills_view = cx.new(|cx| {
            BillsView::new(
                BillsSources {
                    ledger: ledger.clone(),
                },
                today,
                cx,
            )
        });
        let bills_subscription = Self::view_event_subscription(&bills_view, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let bills_view_observer = cx.observe(&bills_view, |_, _, cx| cx.notify());
        let documents_view = cx.new(|cx| {
            DocumentsView::new(
                DocumentsSources {
                    ledger: ledger.clone(),
                },
                today,
                cx,
            )
        });
        let documents_subscription = Self::view_event_subscription(&documents_view, cx);
        // The status line reads the page's state, so a change in it repaints `Shell` too.
        let documents_view_observer = cx.observe(&documents_view, |_, _, cx| cx.notify());
        Self {
            nav,
            focus_handle,
            view_x_scroll_handle: ScrollHandle::new(),
            pending_g: None,
            view_mode: None,
            chrome: ChromeState {
                status_message: None,
                toasts: Toasts::default(),
                toasts_hovered: false,
                dismiss_toasts_binding: key_router::DEFAULT_DISMISS_TOASTS.to_string(),
                toast_history_binding: None,
                #[cfg(debug_assertions)]
                debug_toast_kind: 0,
                palette: None,
                collapsed_rail_tooltip: None,
                rail_override: None,
                hover_generation: 0,
                dialog: None,
                dialog_confirm: None,
                pending_colour_change: None,
                undo_menu_open: false,
            },
            command_history: Vec::new(),
            file_explorer: None,
            explorer_filters: ExplorerFilters::default(),
            ledger,
            today,
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
                accounts_subscription,
                accounts_view_observer,
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
                payees_view_subscription,
                payees_view_observer,
                tags_page_subscription,
                tags_page_observer,
                documents_page_subscription,
                documents_page_observer,
                categories_subscription,
                units_subscription,
                accounts_changed,
                units_page_subscription,
                tracing_page_subscription,
                display_page_subscription,
                display_page_observer,
                institutions_subscription,
                institutions_page_subscription,
                sync_server_page_subscription,
                data_backup_page_subscription,
                inventory_changed,
                inventory_page_subscription,
                inventory_page_observer,
                categories_page_subscription,
                categories_page_observer,
                settings_subscription,
                settings_view_observer,
            ],
        }
    }

    /// The Categories tree, read straight from the Ledger (ADR-0038).
    fn categories<'a>(&self, cx: &'a App) -> &'a [Category] {
        self.ledger.read(cx).ledger().categories()
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
        self.nav.restore_settings_page(section);
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
        chrome_rail::rail_mode(
            window_size,
            self.start_sidebar_minimised(cx),
            self.chrome.rail_override,
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

    pub fn set_dismiss_toasts_binding(&mut self, spec: String) {
        self.chrome.dismiss_toasts_binding = spec;
    }

    pub fn set_toast_history_binding(&mut self, spec: Option<String>) {
        self.chrome.toast_history_binding = spec;
    }

    /// Stands in for opening a ledger, so a test can reach the context rail without the file
    /// explorer's filesystem walk.
    #[doc(hidden)]
    pub fn open_ledger_for_test(&mut self) {
        self.nav.open_ledger();
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

    /// The status line's flash message (e.g. "not yet built"), if one is showing.
    #[doc(hidden)]
    pub fn status_message(&self) -> Option<&str> {
        self.chrome.status_message.as_deref()
    }

    /// Previously run palette commands, most recent first.
    #[doc(hidden)]
    pub fn command_history(&self) -> &[String] {
        &self.command_history
    }

    pub fn nav(&self) -> &NavState {
        &self.nav
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Which View owns the keyboard, page status and main pane right now.
    fn active_view(&self, cx: &App) -> ActiveView {
        ActiveView::derive(
            self.nav.noun(),
            self.views.settings.read(cx).selected_section(),
            self.views.transactions.read(cx).import_open(),
        )
    }

    /// The active View as `Shell` forwards to it (ADR-0037).
    fn active_view_entity(&self, cx: &App) -> ActiveViewEntity {
        ActiveViewEntity::new(self.active_view(cx), &self.views)
    }

    /// Hands the Display Preferences to the Views that read them and the Toasts Preference to
    /// the Toasts, after a Settings change.
    fn push_display_preferences(&mut self, display: DisplayPreferences, cx: &mut App) {
        self.chrome.toasts.set_display(lib_toast::Display {
            toasts_on: display.toasts_on,
            ..self.chrome.toasts.display()
        });
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

impl Focusable for Shell {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::accounts::hints::accounts_hints;
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
    fn the_accounts_count_is_a_plural_selector() {
        crate::locale::init_for_tests();
        assert_eq!(crate::msg::desktop_accounts_count(1), "1 account");
        assert_eq!(crate::msg::desktop_accounts_count(0), "0 accounts");
        assert_eq!(crate::msg::desktop_accounts_count(7), "7 accounts");
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
            for (_, action) in accounts_hints()
                .into_iter()
                .chain(filter_hints())
                .chain(transactions_hints())
            {
                assert!(action.starts_with('['), "{action}");
            }
        });
    }

    #[test]
    fn command_flashes_carry_the_typed_command_and_name() {
        crate::locale::init_for_tests();
        assert_eq!(
            crate::msg::desktop_status_no_accounts(":accounts edit"),
            ":accounts edit \u{2014} no accounts"
        );
        assert_eq!(
            crate::msg::desktop_status_no_account_named(":accounts edit", "Rainy"),
            ":accounts edit \u{2014} no account named \"Rainy\""
        );
        assert_eq!(
            crate::msg::desktop_status_account_ambiguous(
                ":accounts edit",
                "ANZ",
                "ANZ Offset, ANZ Everyday"
            ),
            ":accounts edit \u{2014} \"ANZ\" matches ANZ Offset, ANZ Everyday"
        );
    }
}
