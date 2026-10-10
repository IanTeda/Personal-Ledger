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
//! `InputMode::Help`). No `View` trait (ADR-0016): `Shell` dispatches on the `ActiveView` enum,
//! and `render_view` in `app/render.rs` is its plain exhaustive match.
//!
//! This is the root of the `app` module, so the `Shell` struct's own module and its child files
//! sit in `app/`. The file holds the `Shell` struct, construction, persisted setters, the accessor
//! surface, `impl Focusable`, per-domain wiring and the unit tests. Each cross-cutting concern
//! lives in a bare-named child file (`commands`, `dialogs`, `explorer`, `focus`, `key_dispatch`,
//! `log_feed`, `rail`, `render`, `status`, `toasts`) under `app/` that is an `impl Shell` block,
//! not the real module of the same name; a `_ui` suffix (`documents_ui`, `inventory_ui`, ...)
//! marks per-domain wiring.
//!
//! `Shell` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

mod accounts_ui;
mod bills_ui;
mod budgets_ui;
mod categories_ui;
mod commands;
mod dialogs;
mod document_types_ui;
mod documents_ui;
mod explorer;
mod focus;
mod inventory_ui;
mod key_dispatch;
mod log_feed;
mod payees_ui;
mod rail;
mod render;
mod settings_ui;
mod snapshots;
mod status;
mod tags_ui;
mod toasts;
mod transactions_ui;
mod view_events;

#[cfg(test)]
use commands::record_history;
use key_dispatch::PENDING_G_TIMEOUT;
pub use log_feed::LOG_COALESCE;
use log_feed::LOG_LIST_OVERDRAW;
#[doc(hidden)]
pub use snapshots::{
    AccountFormSnapshot, BillRowSnapshot, BillsSnapshot, BudgetRowSnapshot, BudgetsSnapshot,
    ChipSnapshot, DashboardBillSnapshot, DocumentsSnapshot, ImportRowSnapshot, ImportSnapshot,
    PaletteSnapshot, SettingsSnapshot, ToastSnapshot, ToastsSnapshot, TransactionsSnapshot,
};

use std::time::{Duration, Instant};

use crate::view::settings::state::{InstitutionsStore, LedgerDataEvent, SettingsView, UnitsStore};
use crate::view::transactions::state::{TransactionsState, TransactionsView};

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, ListAlignment, ListState,
    ScrollHandle, Subscription, UniformListScrollHandle, point, px,
};

use lib_core::DateStyle;
use lib_toast::Toasts;

use crate::view::accounts::state::{
    ACCOUNTS_HALF_PAGE, AccountsEvent, AccountsStore, AccountsView,
};
use crate::view::bills::state::{BillsEvent, BillsView};
use crate::view::budgets::state::{BudgetsEvent, BudgetsState, BudgetsStore, BudgetsView};
use crate::view::categories::state::{CategoriesEvent, CategoriesStore, CategoriesView};
use crate::view::documents::state::{
    DocumentsEvent, DocumentsState, DocumentsStore, DocumentsView,
};
use crate::view::payees::state::{PayeesEvent, PayeesStore, PayeesView};
use crate::view::tags::state::{TagsEvent, TagsStore, TagsView};
use crate::{
    bills::{self, BillsStore},
    budgets,
    categories::{self, Category},
    chrome::dialog_host::OpenDialog,
    chrome::state::ChromeState,
    documents::{self, DocumentsMode, LibraryScope, LibrarySort},
    institutions::{self},
    inventory,
    navigation::active_view::ActiveView,
    navigation::explorer::{ExplorerFilters, FileExplorer},
    navigation::key_router::{self, Movement},
    navigation::nav::NavState,
    payees::{self},
    settings::tracing_log::LogView,
    settings::{
        SettingsSection,
        display::{RowDensity, StatusGlyphs},
        tracing_log::TracingLevel,
    },
    tags::{self},
    transactions::{self, TransactionsStore, chips::FilterField, query::TransactionFilters},
    units::{self},
    view::format,
    view::{
        budgets::{self as budgets_view},
        dashboard::{self},
    },
};

/// The collapsed rail's own hover-reveal delay (`docs/ux/desktop-mockups/01-shell/README.md`'s
/// "1c" tooltip spec) -- deliberately the same 500ms `gpui`'s own built-in `.tooltip()` uses,
/// even though this tooltip is hand-rolled (row-anchored, not cursor-anchored -- see
/// `chrome::rail::primary::collapsed_tooltip`'s doc) rather than that builtin.
const TOOLTIP_REVEAL_DELAY: Duration = Duration::from_millis(500);

/// A single `j`/`k`/`Down`/`Up` step in the `View` zone's own scroll, in logical pixels --
/// there's no literal "row height" for a dashboard of charts and figures, so this is a plain
/// reading-sized increment, not a computed value.
const VIEW_LINE_STEP: f32 = 40.0;

/// Owns the shell's render tree and the live `NavState`.
pub struct Shell {
    nav: NavState,
    focus_handle: FocusHandle,
    /// Drives the `View` zone's own scroll when it's focused (`Movement::*`) -- rails don't
    /// scroll at all yet (their content always fits; overflow is a future ticket's problem).
    view_scroll_handle: ScrollHandle,
    /// When this `g` was pressed -- `Some` while a completing chord (`gg`, or `g` + a jump
    /// key) is still possible. Cleared by the next keypress regardless of outcome, so an
    /// abandoned `g` never leaks into the one after it. Checked against
    /// [`PENDING_G_TIMEOUT`] rather than driven by a timer: nothing needs to happen on its
    /// own with no further keypress, so a lazily-checked timestamp is enough.
    pending_g: Option<Instant>,
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
    /// The Display section's own "Date format" segmented control (issue #179) -- a stored
    /// preference, not reset on noun change (same reasoning as [`Self::units_store`]).
    settings_date_style: Option<DateStyle>,
    /// The same section's "Row density" segmented control -- also drives the PREVIEW table's own
    /// row padding (`view::settings::display`'s own doc), unlike a purely-cosmetic preference.
    settings_row_density: RowDensity,
    /// The same section's "Status glyphs" radio group.
    settings_status_glyphs: StatusGlyphs,
    /// The same section's "Start Sidebar minimised" toggle -- persisted across restarts (see
    /// `persistence::PersistedState`) and applied to the primary rail at launch.
    settings_start_sidebar_minimised: bool,
    /// The `:open` explorer's footer checkboxes, kept here so they outlive each dialog and reach
    /// `persistence::PersistedState` at quit.
    explorer_filters: ExplorerFilters,
    /// The Units table and Price Sources, owned by their store Entity and read through it
    /// (ADR-0032). Shared: Settings edits them, and Accounts and the Institutions dialog read the
    /// same rows.
    units_store: Entity<UnitsStore>,
    /// The Institutions table, owned by its store Entity and read through it (ADR-0032). Shared:
    /// Settings edits it, and the Accounts form reads it.
    institutions_store: Entity<InstitutionsStore>,
    /// The Accounts rows, owned by their store Entity and read through it (ADR-0032). Shared:
    /// Transactions, Budgets, Bills and Documents read the same rows.
    accounts: Entity<AccountsStore>,
    /// The Accounts page's state: its selected row. Its events are handled in
    /// `handle_accounts_event`.
    accounts_view: Entity<AccountsView>,
    /// "Today" for the seed data and, later, the `this year` filter -- read once at construction so
    /// everything derived from it (the seeded dates, the default range) agrees for the whole run.
    today: chrono::NaiveDate,
    /// The Categories tree, owned by its store Entity and read through it (ADR-0032). Shared:
    /// Budgets, Bills, Transactions, Documents, Payees and Settings read the same tree.
    categories_store: Entity<CategoriesStore>,
    /// The Settings Categories tree's cursor, selected id and expanded nodes, owned by their view
    /// Entity.
    categories_view: Entity<CategoriesView>,
    /// The Settings View's own state (the page, focus, Display control and Documents and
    /// Inventory selections), owned by its view Entity (ADR-0032).
    settings_view: Entity<SettingsView>,
    /// The shared stub Budgets, seeded from `budgets::default_budgets()`, owned by their store
    /// Entity and read through it (ADR-0032). Shared so the Budgets surface and Categories 5c
    /// read and write the same Category Limits, and so they survive leaving and re-entering
    /// either view.
    budgets_store: Entity<BudgetsStore>,
    /// The Budgets page's state (Budget, tab, month, cursors). Its events are handled in
    /// `handle_budgets_event`.
    budgets_view: Entity<BudgetsView>,
    /// The Payees rows, owned by their store Entity and read through it (ADR-0032). Shared:
    /// Transactions, Bills, Documents and Budgets read the same rows.
    payees_store: Entity<PayeesStore>,
    /// The Payees page's and Settings Payees list's selections, owned by their view Entity.
    payees_view: Entity<PayeesView>,
    /// Settings' Inventory page: the selected row and the Properties shown open (session-only).
    /// The Tags rows, owned by their store Entity and read through it (ADR-0032). Shared:
    /// Transactions reads the same rows for its chips, filter form and Split tag picker.
    tags_store: Entity<TagsStore>,
    /// The Tags page's and Settings Tags list's selections, owned by their view Entity.
    tags_view: Entity<TagsView>,
    /// The Transactions, owned by their store Entity and read through it (ADR-0032). Seeded from
    /// `transactions::default_transactions`; saved-in-memory state that survives leaving and
    /// re-entering the page. Deleting an account deletes its transactions with it.
    transactions_store: Entity<TransactionsStore>,
    /// The Bill Plans and Bill Schedule, owned by their store Entity and read through it
    /// (ADR-0032). Seeded by `bills::default_bills`, which also writes their settling
    /// Transactions into the Transactions store.
    bills_store: Entity<BillsStore>,
    /// The Bills page's state (tab, selection, period, All, filters and filter focus), owned by
    /// its Entity. Its events are handled in `handle_bills_event`.
    bills_view: Entity<BillsView>,
    /// The Transactions page's state (selection, scroll, filters, search and the popover draft),
    /// owned by its Entity (ADR-0032).
    transactions_view: Entity<TransactionsView>,
    /// The Documents and Document Types, owned by their store Entity and read through it
    /// (ADR-0032). Shared: Settings › Documents edits the Types, and the Documents page, the rail
    /// badge and the Inventory removal read the Documents.
    documents_store: Entity<DocumentsStore>,
    /// The Documents page's state (mode, scope, sort, search, selection). Its events are handled
    /// in `handle_documents_event`.
    documents_view: Entity<DocumentsView>,
    /// The Inventory the Documents' Links resolve against.
    inventory: inventory::Inventory,
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
        let accounts = cx.new(|_| AccountsStore::seeded());
        let accounts_view = cx.new(|_| AccountsView::new(accounts.clone()));
        let accounts_subscription =
            cx.subscribe(&accounts_view, |shell, _view, event: &AccountsEvent, cx| {
                shell.handle_accounts_event(*event, cx);
            });
        let seeded_accounts = accounts.read(cx).accounts().to_vec();
        let categories = categories::default_categories();
        let categories_store = cx.new(|_| CategoriesStore::new(categories.clone()));
        let categories_view = cx.new(|_| CategoriesView::new());
        let units_store =
            cx.new(|_| UnitsStore::new(units::default_units(), units::default_price_sources()));
        let institutions_store =
            cx.new(|_| InstitutionsStore::new(institutions::default_institutions()));
        let units_subscription = cx
            .subscribe(&units_store, |_, _store, _event: &LedgerDataEvent, cx| {
                cx.notify()
            });
        let institutions_subscription = cx.subscribe(
            &institutions_store,
            |_, _store, _event: &LedgerDataEvent, cx| cx.notify(),
        );
        let categories_subscription = cx.subscribe(
            &categories_store,
            |_, _store, _event: &CategoriesEvent, cx| cx.notify(),
        );
        let categories_view_observer = cx.observe(&categories_view, |_, _, cx| cx.notify());
        // A private, empty capture until `set_log_capture` hands over the real one.
        let settings_view = cx.new(|_| {
            SettingsView::new(
                LogView::new(
                    lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
                    TracingLevel::default(),
                ),
                ListState::new(0, ListAlignment::Top, LOG_LIST_OVERDRAW),
            )
        });
        let settings_view_observer = cx.observe(&settings_view, |_, _, cx| cx.notify());
        let payees = payees::default_payees();
        let payees_store = cx.new(|_| PayeesStore::new(payees.clone()));
        let payees_view = cx.new(|_| PayeesView::new());
        let payees_subscription = cx
            .subscribe(&payees_store, |_, _store, _event: &PayeesEvent, cx| {
                cx.notify()
            });
        // The Payees View's selections live in their own Entity, so a selection change repaints
        // Shell too, not just the View that moved.
        let payees_view_observer = cx.observe(&payees_view, |_, _, cx| cx.notify());
        let tags = tags::default_tags();
        let tags_store = cx.new(|_| TagsStore::new(tags.clone()));
        let tags_view = cx.new(|_| TagsView::new(tags_store.clone()));
        let tags_subscription =
            cx.subscribe(&tags_store, |_, _store, _event: &TagsEvent, cx| cx.notify());
        let mut transactions = transactions::default_transactions(
            &seeded_accounts,
            &categories,
            &payees,
            &tags,
            today,
        );
        let bills_seed = bills::default_bills(
            &seeded_accounts,
            &categories,
            &payees,
            &mut transactions,
            today,
        );
        let inventory = inventory::default_inventory(today);
        let documents_seed = documents::default_documents(
            &seeded_accounts,
            &categories,
            &payees,
            &bills_seed.plans,
            &inventory,
            &mut transactions,
            today,
        );
        let transactions_store = cx.new(|_| TransactionsStore::new(transactions));
        let transactions_view = cx.new(|_| {
            TransactionsView::new(TransactionsState {
                selected: 0,
                scroll: UniformListScrollHandle::new(),
                filters: TransactionFilters::defaults(today),
                search: String::new(),
                filter_form: None,
                filter_anchor: FilterField::Account,
                chip_bounds: Default::default(),
                import: None,
            })
        });
        let seeded_budgets = budgets::default_budgets(&seeded_accounts, &categories, today);
        let budgets_current = seeded_budgets
            .default_budget()
            .map_or(budgets::PERSONAL_SPENDING_ID, |budget| budget.id);
        let budgets_store = cx.new(|_| BudgetsStore::new(seeded_budgets));
        let budgets_view = cx.new(|_| BudgetsView::new(BudgetsState::new(budgets_current, today)));
        let budgets_subscription =
            cx.subscribe(&budgets_view, |shell, _view, event: &BudgetsEvent, cx| {
                shell.handle_budgets_event(*event, cx);
            });
        let bills_store = cx.new(|_| BillsStore::new(bills_seed.plans, bills_seed.entries));
        let bills_view = cx.new(|_| BillsView::new(bills_store.clone(), today));
        let bills_subscription =
            cx.subscribe(&bills_view, |shell, _view, event: &BillsEvent, cx| {
                shell.handle_bills_event(*event, cx);
            });
        let documents_store = cx.new(|_| {
            DocumentsStore::new(documents_seed.documents, documents::types::default_types())
        });
        let documents_view = cx.new(|_| DocumentsView::new(DocumentsState::default()));
        let documents_subscription = cx.subscribe(
            &documents_view,
            |shell, _view, event: &DocumentsEvent, cx| {
                shell.handle_documents_event(event, cx);
            },
        );
        Self {
            nav,
            focus_handle,
            view_scroll_handle: ScrollHandle::new(),
            pending_g: None,
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
                hover_generation: 0,
                dialog: None,
                pending_colour_change: None,
            },
            command_history: Vec::new(),
            file_explorer: None,
            settings_date_style: None,
            settings_row_density: RowDensity::default(),
            settings_status_glyphs: StatusGlyphs::default(),
            settings_start_sidebar_minimised: false,
            explorer_filters: ExplorerFilters::default(),
            units_store,
            institutions_store,
            accounts,
            accounts_view,
            today,
            categories_store,
            categories_view,
            settings_view,
            budgets_store,
            budgets_view,
            payees_store,
            payees_view,
            tags_store,
            tags_view,
            transactions_store,
            transactions_view,
            bills_store,
            bills_view,
            documents_store,
            documents_view,
            inventory,
            view_subscriptions: vec![
                accounts_subscription,
                budgets_subscription,
                bills_subscription,
                documents_subscription,
                tags_subscription,
                payees_subscription,
                payees_view_observer,
                categories_subscription,
                units_subscription,
                institutions_subscription,
                categories_view_observer,
                settings_view_observer,
            ],
        }
    }

    /// The Categories tree, read through its store Entity (ADR-0032).
    fn categories<'a>(&self, cx: &'a App) -> &'a [Category] {
        self.categories_store.read(cx).categories()
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
    pub fn settings_page(&self, cx: &App) -> SettingsSection {
        self.settings_view.read(cx).selected_section()
    }

    pub fn set_settings_page(&mut self, section: SettingsSection, cx: &mut App) {
        self.settings_view
            .update(cx, |v, cx| v.set_selected_section(cx, section));
    }

    pub fn explorer_filters(&self) -> ExplorerFilters {
        self.explorer_filters
    }

    pub fn set_explorer_filters(&mut self, filters: ExplorerFilters) {
        self.explorer_filters = filters;
    }

    pub fn start_sidebar_minimised(&self) -> bool {
        self.settings_start_sidebar_minimised
    }

    pub fn set_start_sidebar_minimised(&mut self, minimised: bool) {
        self.settings_start_sidebar_minimised = minimised;
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

    /// Empties the Inventory, standing in for removing every Property until the Remove dialog
    /// lands, so a test can reach the Inventory page's empty state.
    #[doc(hidden)]
    pub fn empty_inventory_for_test(&mut self) {
        self.inventory = inventory::Inventory::default();
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
            self.settings_view.read(cx).selected_section(),
            self.import_state(cx).is_some(),
        )
    }

    fn scroll_view(&mut self, movement: Movement) {
        let offset = self.view_scroll_handle.offset();
        let max_height = f32::from(self.view_scroll_handle.max_offset().height);
        let viewport_height = f32::from(self.view_scroll_handle.bounds().size.height);
        let current_y = f32::from(offset.y);

        let new_y = match movement {
            Movement::Next => current_y - VIEW_LINE_STEP,
            Movement::Prev => current_y + VIEW_LINE_STEP,
            Movement::First => 0.0,
            Movement::Last => -max_height,
            Movement::HalfPageDown => current_y - viewport_height / 2.0,
            Movement::HalfPageUp => current_y + viewport_height / 2.0,
            // No action is defined for `Enter` in the View zone by the handoff's "Movement"
            // bullet (only the primary and context rails are named) -- a no-op until one is.
            Movement::Enter => current_y,
        };
        let clamped_y = new_y.clamp(-max_height, 0.0);
        self.view_scroll_handle
            .set_offset(point(offset.x, px(clamped_y)));
    }

    fn move_selected_document_type(&mut self, delta: isize, cx: &mut App) {
        if let Some(id) = self.settings_documents_selected_id(cx) {
            self.mutate_documents(cx, |data| {
                documents::types::move_by(&mut data.types, id, delta)
            });
            self.settings_view
                .update(cx, |v, cx| v.set_documents_selected(cx, Some(id)));
        }
    }

    /// The Remove action. Other, the Default, is never removed: it gets a notice dialog and says
    /// so on the status line.
    fn remove_selected_document_type(&mut self, cx: &mut App) {
        let Some(id) = self.settings_documents_selected_id(cx) else {
            return;
        };
        let is_default = documents::types::position(self.document_types(cx), id)
            .and_then(|position| self.document_types(cx).get(position))
            .is_some_and(|row| row.is_default);
        if is_default {
            self.chrome.status_message =
                Some(crate::msg::desktop_document_types_hint_default_kept());
        }
        self.open_remove_document_type_dialog(id, cx);
    }

    /// The Dashboard's budget list, worded from the default Budget's current-month figures.
    fn dashboard_budget_list(
        &self,
        figures: &budgets::PeriodFigures,
        cx: &App,
    ) -> dashboard::BudgetList {
        use bigdecimal::{ToPrimitive, Zero};
        let bars = budgets::dashboard_bars(figures, self.categories(cx))
            .into_iter()
            .map(|bar| dashboard::BudgetBar {
                category: self
                    .categories(cx)
                    .iter()
                    .find(|category| category.id == bar.category_id)
                    .map(|category| category.name.clone())
                    .unwrap_or_default(),
                figures: crate::msg::desktop_budgets_history_pair(
                    &format::amount(&bar.spent).1,
                    &format::amount(&bar.budget).1,
                ),
                // A 0.00 budget is full as soon as anything is spent against it.
                fraction: if bar.budget.0.is_zero() {
                    if bar.over { 1.0 } else { 0.0 }
                } else {
                    (bar.spent.0.clone() / bar.budget.0.clone())
                        .to_f32()
                        .unwrap_or(0.0)
                },
                over: bar.over,
            })
            .collect();
        dashboard::BudgetList {
            period: crate::msg::desktop_dashboard_budgets_period(
                &budgets_view::period_label(figures.month),
                &figures.elapsed.day.to_string(),
                &figures.elapsed.days_in_month.to_string(),
                &figures.elapsed.percent.to_string(),
            ),
            elapsed: figures.elapsed.percent as f32 / 100.0,
            bars,
        }
    }

    /// The help overlay's own Close button.
    fn handle_help_close(&mut self, cx: &mut Context<'_, Self>) {
        self.nav.exit_mode();
        cx.notify();
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
    use crate::accounts;
    use crate::transactions::Transaction;
    use crate::view::accounts::hints::accounts_hints;
    use crate::view::settings::hints::{
        settings_colour_grid_hints, settings_display_hints, settings_index_hints,
        settings_plain_page_hints,
    };
    use crate::view::transactions::hints::{filter_hints, transactions_hints};
    use categories_ui::category_refused;
    use chrono::Local;
    use lib_accounts::{Account, AccountService};
    use lib_toast::ToastKind;
    use transactions_ui::move_category_splits;

    fn seeded_ledger() -> (Vec<Account>, Vec<Category>, Vec<Transaction>) {
        let accounts = accounts::default_accounts();
        let categories = categories::default_categories();
        let transactions = transactions::default_transactions(
            &accounts,
            &categories,
            &payees::default_payees(),
            &tags::default_tags(),
            Local::now().date_naive(),
        );
        (accounts, categories, transactions)
    }

    /// The category delete the Shell runs, in its order: check and get the Uncategorised Category,
    /// re-point the Splits, drop the budget, then drop the Category. Mirrors `Shell::delete_category`
    /// over plain rows, so the order is tested without a window.
    fn delete_category(
        categories: &mut Vec<Category>,
        transactions: &mut [Transaction],
        budgets: &mut budgets::Budgets,
        id: u32,
    ) -> (ToastKind, String) {
        let mut service = lib_categories::CategoryService::new(categories.clone());
        let name = categories
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let outcome = service.prepare_delete(id).map(|uncategorised| {
            let moved = move_category_splits(transactions, id, uncategorised);
            budgets.remove_category(id);
            (service.delete(id), moved)
        });
        *categories = service.categories().to_vec();
        match outcome {
            Err(error) => category_refused(error),
            Ok((Err(error), _)) => category_refused(error),
            Ok((Ok(()), moved)) => (
                ToastKind::Success,
                lib_locale::msg::toast_category_deleted(&name, moved),
            ),
        }
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
    fn deleting_an_account_raises_a_success_toast_counting_its_transactions() {
        crate::locale::init_for_tests();
        let (accounts, _, mut transactions) = seeded_ledger();
        let mut accounts = AccountService::from_rows(accounts);
        let account = accounts.accounts()[0].clone();
        let booked = transactions
            .iter()
            .filter(|t| t.account_id == account.id)
            .count();
        let before = transactions.len();
        transactions.retain(|t| t.account_id != account.id);
        let (kind, text) =
            accounts_ui::delete_account(&mut accounts, before - transactions.len(), account.id);
        assert_eq!(kind, ToastKind::Success);
        assert_eq!(
            text,
            lib_locale::msg::toast_account_deleted(&account.name, booked as i64)
        );
        assert!(transactions.iter().all(|t| t.account_id != account.id));
    }

    #[test]
    fn a_refused_category_delete_raises_an_error_toast_and_changes_nothing() {
        crate::locale::init_for_tests();
        let (_, mut categories, mut transactions) = seeded_ledger();
        let mut budgets = budgets::Budgets::default();
        let food = categories::find_by_name(&categories, "Food").expect("seeded");
        let before = categories.len();
        let (kind, text) = delete_category(&mut categories, &mut transactions, &mut budgets, food);
        assert_eq!(kind, ToastKind::Error);
        assert_eq!(
            text,
            "Couldn't save category: delete or move its children first"
        );
        assert_eq!(categories.len(), before);
    }

    #[test]
    fn deleting_a_leaf_category_raises_a_success_toast() {
        crate::locale::init_for_tests();
        let (_, mut categories, mut transactions) = seeded_ledger();
        let mut budgets = budgets::Budgets::default();
        let rent = categories::find_by_name(&categories, "Rent").expect("seeded");
        let (kind, text) = delete_category(&mut categories, &mut transactions, &mut budgets, rent);
        assert_eq!(kind, ToastKind::Success);
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
