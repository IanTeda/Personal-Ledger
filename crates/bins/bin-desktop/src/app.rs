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
mod commands;
mod dialogs;
mod document_types_ui;
mod documents_ui;
mod explorer;
mod focus;
mod inventory_ui;
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
use key_dispatch::PENDING_G_TIMEOUT;
pub use log_feed::LOG_COALESCE;
use log_feed::{LOG_LINE_STEP, LOG_LIST_OVERDRAW};
#[doc(hidden)]
pub use snapshots::{
    AccountFormSnapshot, BillRowSnapshot, BillsSnapshot, BudgetRowSnapshot, BudgetsSnapshot,
    ChipSnapshot, DashboardBillSnapshot, DocumentsSnapshot, ImportRowSnapshot, ImportSnapshot,
    PaletteSnapshot, SettingsSnapshot, ToastSnapshot, ToastsSnapshot, TransactionsSnapshot,
};

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::view::settings::hints::{
    settings_categories_hints, settings_colour_grid_hints, settings_display_hints,
    settings_documents_hints, settings_index_hints, settings_inventory_hints,
    settings_payees_hints, settings_plain_page_hints, settings_tags_hints, settings_tracing_hints,
};
use crate::view::settings::inventory::{self as inventory_view, InventoryRow};
use crate::view::transactions::state::{TransactionsState, TransactionsView};
use crate::view::{accounts::hints::accounts_hints, settings::hints::confirm_dialog_hints};

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Keystroke, ListAlignment, ListState,
    ScrollHandle, ScrollStrategy, Subscription, UniformListScrollHandle, point, px,
};

use lib_core::{CategoryTypes, DateStyle};
use lib_toast::{ToastKind, Toasts};

use crate::view::accounts::state::{
    ACCOUNTS_HALF_PAGE, AccountsEvent, AccountsStore, AccountsView,
};
use crate::view::bills::state::{BillsEvent, BillsView};
use crate::view::budgets::state::{BudgetsEvent, BudgetsState, BudgetsStore, BudgetsView};
use crate::view::documents::state::{
    DocumentsEvent, DocumentsState, DocumentsStore, DocumentsView,
};
use crate::{
    accounts::{self},
    bills::{self, BillsStore},
    budgets,
    categories::{self, Category},
    chrome::dialog_host::OpenDialog,
    chrome::state::ChromeState,
    documents::{self, DocumentsMode, LibraryScope, LibrarySort},
    form::field::TextField,
    import::ImportState,
    institutions::{self, AccountType, InstitutionRow, form::AddInstitutionForm},
    inventory,
    navigation::active_view::ActiveView,
    navigation::explorer::{ExplorerFilters, FileExplorer},
    navigation::key_router::{self, Movement},
    navigation::nav::{FocusZone, InputMode, NavState, Noun},
    payees::{self, Payee},
    settings::tracing_log::LogView,
    settings::{
        DISPLAY_FIELD_COUNT, DISPLAY_FIELD_SIDEBAR, SettingsDialog, SettingsFocus, SettingsSection,
        display::{DATE_STYLE_CHOICES, RowDensity, StatusGlyphs},
        step_choice,
        tracing_log::TracingLevel,
    },
    tags::{self, Tag},
    theme::colours::ColourChange,
    transactions::{
        self, Transaction, TransactionsStore, chips::FilterField, edit_transactions,
        query::TransactionFilters,
    },
    units::{
        self, PriceSourceRow, UnitKind, UnitRow,
        form::{AddUnitField, DeleteUnitForm, UnitForm},
    },
    view::format,
    view::{
        budgets::{self as budgets_view},
        dashboard::{self},
        settings::{self as settings_view},
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

/// `Ctrl-d`/`Ctrl-u` on the Settings Categories page: rows per half page.
const CATEGORIES_HALF_PAGE: usize = 5;
/// `Ctrl-d`/`Ctrl-u` on the Settings Tags page: rows per half page.
const SETTINGS_TAGS_HALF_PAGE: isize = 5;
/// `Ctrl-d`/`Ctrl-u` on the Settings Payees page: rows per half page.
const SETTINGS_PAYEES_HALF_PAGE: isize = 5;
/// `Ctrl-d`/`Ctrl-u` on the Settings Documents page: rows per half page.
const SETTINGS_DOCUMENTS_HALF_PAGE: isize = 5;
/// `Ctrl-d`/`Ctrl-u` on the Settings Inventory page: rows per half page.
const SETTINGS_INVENTORY_HALF_PAGE: usize = 5;

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
    /// The Settings page on show, and the index rail's highlighted entry. Changed by a row click,
    /// `j`/`k` on the index, or `:settings <page>`. Deliberately *not* reset when a noun is
    /// entered: it is the last-visited page, which `g s` reopens and persistence keeps.
    settings_selected_section: SettingsSection,
    /// Whether keyboard focus is on the index rail or the page, while the View zone has it.
    settings_focus: SettingsFocus,
    /// Set when focus has just moved onto the Display page, so the next key handler (which has
    /// an `App` to read the chosen Colour Theme from) puts the grid's focus on that card.
    /// The Display page's focused control (`DISPLAY_FIELD_COUNT` of them, above the Colour Theme
    /// grid); `None` off that page or while the grid has focus.
    settings_display_field: Option<usize>,
    /// The Display section's own "Date format" segmented control (issue #179) -- a stored
    /// preference, not reset on noun change (same reasoning as [`Self::settings_units`]).
    settings_date_style: Option<DateStyle>,
    /// The same section's "Row density" segmented control -- also drives the PREVIEW table's own
    /// row padding (`view::settings::display`'s own doc), unlike a purely-cosmetic preference.
    settings_row_density: RowDensity,
    /// The Colour Theme card the keyboard is on, while Settings' Colour Theme grid has focus.
    colour_theme_focus: Option<usize>,
    /// A Colour Theme or Colour Appearance picked by a keystroke or palette command, applied by
    /// the caller once it has an `App` (the key handling runs without one).
    pending_colour_change: Option<ColourChange>,
    /// The same section's "Status glyphs" radio group.
    settings_status_glyphs: StatusGlyphs,
    /// The same section's "Start Sidebar minimised" toggle -- persisted across restarts (see
    /// `persistence::PersistedState`) and applied to the primary rail at launch.
    settings_start_sidebar_minimised: bool,
    /// The `:open` explorer's footer checkboxes, kept here so they outlive each dialog and reach
    /// `persistence::PersistedState` at quit.
    explorer_filters: ExplorerFilters,
    /// The Units section's own table rows (issue #177), seeded from `units::default_units()`.
    /// A real, mutable `Vec` so the Add/Edit/Delete unit dialogs (issues #184-#186) can
    /// push/update/remove rows once they land -- unlike
    /// [`Self::settings_selected_section`], not reset by
    /// [`Self::reset_view_scroll`]: it represents saved-in-memory state, not navigational UI
    /// state, so it must survive leaving and re-entering Settings the way real saved data would.
    settings_units: Vec<UnitRow>,
    /// The same section's own "Price Sources" subsection rows (issue #189), seeded from
    /// `units::default_price_sources()` -- same reasoning as [`Self::settings_units`], though
    /// nothing on this map's own dialog tickets mutates this `Vec` yet (test/edit/delete/add are
    /// all clearly-marked stubs, see `view::settings::units`'s own doc).
    settings_price_sources: Vec<PriceSourceRow>,
    /// The Institutions section's own table rows (issue #178), seeded from
    /// `institutions::default_institutions()` -- same reasoning as [`Self::settings_units`].
    settings_institutions: Vec<InstitutionRow>,
    /// The Tracing page's mirror of the live log capture and its level filter (session only).
    settings_log: LogView,
    /// The Tracing page's virtualised log list. Kept in step with `settings_log` by splicing,
    /// so a scrolled-down reader stays anchored as entries arrive.
    settings_log_list: ListState,
    /// The Accounts rows, owned by their store Entity and read through it (ADR-0032). Shared:
    /// Transactions, Budgets, Bills and Documents read the same rows.
    accounts: Entity<AccountsStore>,
    /// The Accounts page's state: its selected row. Its events are handled in
    /// `handle_accounts_event`.
    accounts_view: Entity<AccountsView>,
    /// "Today" for the seed data and, later, the `this year` filter -- read once at construction so
    /// everything derived from it (the seeded dates, the default range) agrees for the whole run.
    today: chrono::NaiveDate,
    /// The shared stub Categories tree, Payees and Tags. Owned here so the Categories, Payees and
    /// Tags views the later maps build can read and grow the same data the Transactions view uses.
    categories: Vec<Category>,
    /// The shared stub Budgets, seeded from `budgets::default_budgets()`, owned by their store
    /// Entity and read through it (ADR-0032). Shared so the Budgets surface and Categories 5c
    /// read and write the same Category Limits, and so they survive leaving and re-entering
    /// either view.
    budgets_store: Entity<BudgetsStore>,
    /// The Budgets page's state (Budget, tab, month, cursors). Its events are handled in
    /// `handle_budgets_event`.
    budgets_view: Entity<BudgetsView>,
    /// The selected category row in the tree view (the position in a depth-first enumeration).
    categories_selected: usize,
    /// The selected category ID for keyboard navigation, if any.
    categories_selected_id: Option<u32>,
    /// Which category nodes are expanded in the tree view.
    categories_expanded: Vec<u32>,
    payees: Vec<Payee>,
    /// The selected row on the Payees page.
    payees_selected: usize,
    /// The selected row on Settings' Payees page, by Payee id: that page lists A–Z.
    settings_payees_selected: Option<u32>,
    settings_documents_selected: Option<u32>,
    /// Settings' Inventory page: the selected row and the Properties shown open (session-only).
    settings_inventory_selected: Option<InventoryRow>,
    settings_inventory_expanded: HashSet<u32>,
    /// The stubbed 6e Import "match payees" step, `Some` while it shows in place of the
    /// Transactions page (`:import`). Dropped on leaving Transactions.
    import: Option<ImportState>,
    tags: Vec<Tag>,
    /// The selected row on the Tags page, a position in `tags::sorted_by_usage`'s order.
    tags_selected: usize,
    /// The selected row on Settings' Tags page, by Tag id: that page lists A–Z, not by usage.
    settings_tags_selected: Option<u32>,
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
        let payees = payees::default_payees();
        let tags = tags::default_tags();
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
            },
            command_history: Vec::new(),
            file_explorer: None,
            settings_selected_section: SettingsSection::default(),
            settings_focus: SettingsFocus::default(),
            settings_display_field: None,
            settings_date_style: None,
            settings_row_density: RowDensity::default(),
            colour_theme_focus: None,
            pending_colour_change: None,
            settings_status_glyphs: StatusGlyphs::default(),
            settings_start_sidebar_minimised: false,
            explorer_filters: ExplorerFilters::default(),
            settings_units: units::default_units(),
            settings_price_sources: units::default_price_sources(),
            settings_institutions: institutions::default_institutions(),
            // A private, empty capture until `set_log_capture` hands over the real one.
            settings_log: LogView::new(
                lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
                TracingLevel::default(),
            ),
            settings_log_list: ListState::new(0, ListAlignment::Top, LOG_LIST_OVERDRAW),
            accounts,
            accounts_view,
            today,
            categories,
            budgets_store,
            budgets_view,
            categories_selected: 0,
            categories_selected_id: None,
            categories_expanded: vec![1, 3, 6], // Housing, Utilities, Food expanded by default
            payees,
            payees_selected: 0,
            settings_payees_selected: None,
            settings_documents_selected: None,
            settings_inventory_selected: None,
            settings_inventory_expanded: HashSet::new(),
            import: None,
            tags,
            tags_selected: 0,
            settings_tags_selected: None,
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
            ],
        }
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
    pub fn settings_page(&self) -> SettingsSection {
        self.settings_selected_section
    }

    pub fn set_settings_page(&mut self, section: SettingsSection) {
        self.settings_selected_section = section;
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

    /// Settings' Colour Theme grid (`docs/colour-themes-design.md` "Settings"): Focus arrives from
    /// the Display page (`l`/`enter` on the index) at the chosen card; arrows or `h`/`j`/`k`/`l` move focus,
    /// `Enter` selects, and `Esc` or `Tab` leaves it (`Tab` going on to the next zone). Moving
    /// focus never previews. `false` for any key the grid does not take.
    fn handle_colour_theme_grid_key(&mut self, keystroke: &Keystroke) -> bool {
        let key = keystroke.key.as_str();
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        if self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Settings
            || self.nav.focus() != FocusZone::View
            || self.settings_focus != SettingsFocus::Page
            || self.settings_selected_section != SettingsSection::Display
            || keystroke.modifiers.control
            || pending_g_active
        {
            self.colour_theme_focus = None;
            return false;
        }
        let Some(index) = self.colour_theme_focus else {
            return false;
        };
        match key {
            "escape" => {
                self.chrome.status_message = None;
                self.leave_colour_theme_grid();
                true
            }
            "tab" => {
                self.colour_theme_focus = None;
                false
            }
            "enter" => {
                if let Some(theme) = lib_colour_theme::ColourTheme::built_in().get(index) {
                    self.pending_colour_change = Some(ColourChange::Theme(theme.id));
                }
                true
            }
            _ => {
                let columns = settings_view::colour_theme::grid_columns(f32::from(
                    self.view_scroll_handle.bounds().size.width,
                ));
                let len = lib_colour_theme::ColourTheme::built_in().len();
                // `k` on the top row climbs back to the last control above the grid.
                if matches!(key, "k" | "up") && index < columns {
                    self.leave_colour_theme_grid();
                    return true;
                }
                // `h` at the first column steps back out to the index.
                if matches!(key, "h" | "left") && index % columns.max(1) == 0 {
                    self.focus_settings_index();
                    return true;
                }
                match settings_view::colour_theme::grid_move(index, len, columns, key) {
                    Some(next) => {
                        self.colour_theme_focus = Some(next);
                        true
                    }
                    // Any other key leaves the grid and goes on to the usual handling.
                    None => {
                        self.colour_theme_focus = None;
                        false
                    }
                }
            }
        }
    }

    /// The status-line legend for Settings' current focus: the index rail's keys, or the open
    /// page's. The four list pages are handled before this (they own dialogs too), so it covers
    /// the index and the form and plain pages.
    fn settings_hints(&self) -> Vec<(&'static str, String)> {
        if self.settings_focus == SettingsFocus::Index {
            return settings_index_hints();
        }
        match self.settings_selected_section {
            SettingsSection::Display if self.colour_theme_focus.is_some() => {
                settings_colour_grid_hints()
            }
            SettingsSection::Display => settings_display_hints(),
            SettingsSection::Tracing if self.settings_dialog().is_some() => confirm_dialog_hints(),
            SettingsSection::Tracing => settings_tracing_hints(),
            _ => settings_plain_page_hints(),
        }
    }

    /// The `?` cheat-sheet's Settings group as `(action, keys)`: the index keys, then the open
    /// page's. Empty off the Settings noun, so the group is left out.
    fn settings_cheat_sheet(&self) -> Vec<(String, &'static str)> {
        if self.nav.noun() != Noun::Settings {
            return Vec::new();
        }
        let page = match self.settings_selected_section {
            SettingsSection::Accounts => accounts_hints(),
            SettingsSection::Categories => settings_categories_hints(),
            SettingsSection::Tags => settings_tags_hints(),
            SettingsSection::Payees => settings_payees_hints(),
            SettingsSection::Documents => settings_documents_hints(),
            SettingsSection::Inventory => {
                settings_inventory_hints(self.settings_inventory_selected_row())
            }
            SettingsSection::Display => {
                let mut keys = settings_display_hints();
                keys.extend(settings_colour_grid_hints().into_iter().take(2));
                keys
            }
            SettingsSection::Tracing => settings_tracing_hints(),
            _ => settings_plain_page_hints(),
        };
        settings_index_hints()
            .into_iter()
            .chain(page)
            .map(|(keys, action)| (action, keys))
            .collect()
    }

    /// Steps from the Colour Theme grid back up to the Display page's last control.
    fn leave_colour_theme_grid(&mut self) {
        self.colour_theme_focus = None;
        self.settings_display_field = Some(DISPLAY_FIELD_COUNT - 1);
    }

    /// Swaps the Settings page on show. Each page starts at its top.
    /// Opens a Settings page with focus in it -- what `:settings <page>`, the old `:accounts` /
    /// `:categories` / `:payees` / `:tags` aliases and every hand-off to a moved noun land on
    /// (the keyboard model's "commands and hand-offs land in the page"). Leaves the view's scroll
    /// alone when the page is already showing.
    fn open_settings_page(&mut self, section: SettingsSection, cx: &mut App) {
        let noun_before = self.nav.noun();
        self.nav.set_noun(Noun::Settings);
        if noun_before != Noun::Settings {
            self.reset_view_scroll(cx);
        }
        self.select_settings_page(section);
        self.focus_settings_page();
    }

    fn select_settings_page(&mut self, section: SettingsSection) {
        if self.settings_selected_section != section {
            self.view_scroll_handle.set_offset(gpui::Point::default());
        }
        self.settings_selected_section = section;
        self.colour_theme_focus = None;
        self.settings_display_field = None;
    }

    /// Moves focus from the index rail into the open page (`l`/`enter`, `:settings <page>`). A
    /// page with nothing to focus keeps focus on the index, as a quiet no-op.
    fn focus_settings_page(&mut self) {
        if !self.settings_selected_section.has_controls() {
            return;
        }
        self.settings_focus = SettingsFocus::Page;
        self.settings_display_field =
            (self.settings_selected_section == SettingsSection::Display).then_some(0);
    }

    fn focus_settings_index(&mut self) {
        self.settings_focus = SettingsFocus::Index;
        self.colour_theme_focus = None;
        self.settings_display_field = None;
    }

    /// The Display page's form keys, ahead of Settings' focus keys: `j`/`k` walk the controls and
    /// on past the last into the Colour Theme grid, `h`/`l` change a segmented or radio control
    /// (or clear/tick the checkbox) in place, `enter`/`space` toggles the checkbox. `esc` is left
    /// to the focus keys, which step back to the index. `false` for any key it does not take.
    fn handle_settings_form_key(&mut self, keystroke: &Keystroke, chosen: usize) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        let Some(field) = self.settings_display_field else {
            return false;
        };
        if self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Settings
            || self.nav.focus() != FocusZone::View
            || self.settings_focus != SettingsFocus::Page
            || self.settings_selected_section != SettingsSection::Display
            || keystroke.modifiers.control
            || keystroke.modifiers.shift
            || pending_g_active
        {
            return false;
        }
        let delta = match keystroke.key.as_str() {
            "h" | "left" => -1,
            "l" | "right" => 1,
            _ => 0,
        };
        match keystroke.key.as_str() {
            "j" | "down" => {
                self.chrome.status_message = None;
                if field + 1 >= DISPLAY_FIELD_COUNT {
                    self.settings_display_field = None;
                    self.colour_theme_focus = Some(chosen);
                } else {
                    self.settings_display_field = Some(field + 1);
                }
                true
            }
            "k" | "up" => {
                self.settings_display_field = Some(field.saturating_sub(1));
                true
            }
            "h" | "left" | "l" | "right" => {
                self.chrome.status_message = None;
                match field {
                    0 => {
                        self.settings_date_style =
                            step_choice(&DATE_STYLE_CHOICES, self.settings_date_style, delta);
                    }
                    1 => {
                        self.settings_row_density =
                            step_choice(&RowDensity::ALL, self.settings_row_density, delta);
                    }
                    2 => {
                        self.settings_status_glyphs =
                            step_choice(&StatusGlyphs::ALL, self.settings_status_glyphs, delta);
                    }
                    DISPLAY_FIELD_SIDEBAR => self.settings_start_sidebar_minimised = delta > 0,
                    _ => self.set_toasts_on(delta < 0),
                }
                true
            }
            "enter" | "space" => {
                if field == DISPLAY_FIELD_SIDEBAR {
                    self.settings_start_sidebar_minimised = !self.settings_start_sidebar_minimised;
                }
                true
            }
            _ => false,
        }
    }

    /// The Tracing page's keys while it has focus, ahead of Settings' focus keys so `h` steps the
    /// level rather than leaving (Display's radio grammar; `esc` leaves): `h`/`l` the level,
    /// `j`/`k` a line, `J`/`K` a page, `G` the oldest entry, `c` Clear logs. Focus never enters
    /// the box itself. `false` for any key it does not take.
    fn handle_settings_tracing_key(&mut self, keystroke: &Keystroke) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        let modifiers = &keystroke.modifiers;
        if self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Settings
            || self.nav.focus() != FocusZone::View
            || self.settings_focus != SettingsFocus::Page
            || self.settings_selected_section != SettingsSection::Tracing
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
            || pending_g_active
        {
            return false;
        }
        let list = &self.settings_log_list;
        let page = list.viewport_bounds().size.height.max(LOG_LINE_STEP);
        match (modifiers.shift, keystroke.key.as_str()) {
            (false, "h" | "left") => self.step_tracing_level(-1),
            (false, "l" | "right") => self.step_tracing_level(1),
            (false, "j" | "down") => list.scroll_by(LOG_LINE_STEP),
            (false, "k" | "up") => list.scroll_by(-LOG_LINE_STEP),
            (true, "j") => list.scroll_by(page),
            (true, "k") => list.scroll_by(-page),
            (true, "g") => {
                if let Some(last) = list.item_count().checked_sub(1) {
                    list.scroll_to_reveal_item(last);
                }
            }
            (false, "c") => self.open_clear_logs_dialog(),
            _ => return false,
        }
        true
    }

    fn step_tracing_level(&mut self, delta: isize) {
        self.chrome.status_message = None;
        let level = step_choice(&TracingLevel::ALL, self.settings_log.level(), delta);
        self.set_tracing_level(level);
    }

    /// Settings' own focus keys, ahead of the Colour Theme grid and the global keymap: `l`/`right`
    /// /`enter` on the index step into the page, `h`/`left`/`esc` on the page step back out.
    /// `false` for any key it does not take.
    fn handle_settings_focus_key(&mut self, keystroke: &Keystroke) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        if self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Settings
            || self.nav.focus() != FocusZone::View
            || keystroke.modifiers.control
            || keystroke.modifiers.shift
            || pending_g_active
        {
            return false;
        }
        match (self.settings_focus, keystroke.key.as_str()) {
            (SettingsFocus::Index, "l" | "right" | "enter") => {
                self.chrome.status_message = None;
                self.focus_settings_page();
                true
            }
            // `left` on the Categories tree collapses or climbs first; only a top-level row with
            // nothing to fold hands it back to the index. `h` always leaves.
            (SettingsFocus::Page, "left")
                if self.settings_categories_page_has_focus()
                    && self.settings_categories_left_is_local() =>
            {
                false
            }
            (SettingsFocus::Page, "left")
                if self.settings_inventory_page_has_focus()
                    && self.settings_inventory_left_is_local() =>
            {
                false
            }
            (SettingsFocus::Page, "h" | "left") if self.colour_theme_focus.is_none() => {
                self.chrome.status_message = None;
                self.focus_settings_index();
                true
            }
            (SettingsFocus::Page, "escape") if self.colour_theme_focus.is_none() => {
                self.focus_settings_index();
                true
            }
            _ => false,
        }
    }

    /// `j`/`k`/`g g`/`G` on the Settings index step the highlight through the pages
    /// and swap the page live, like an index click.
    fn apply_settings_section_movement(&mut self, movement: Movement) {
        let visible: Vec<SettingsSection> = SettingsSection::ALL.into_iter().collect();
        let Some(last) = visible.len().checked_sub(1) else {
            return;
        };
        let current = visible
            .iter()
            .position(|section| *section == self.settings_selected_section)
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => (current + 1).min(last),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            _ => last,
        };
        self.select_settings_page(visible[next]);
    }

    /// Which View owns the keyboard, page status and main pane right now.
    fn active_view(&self) -> ActiveView {
        ActiveView::derive(
            self.nav.noun(),
            self.settings_selected_section,
            self.import.is_some(),
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

    /// Whether Settings' Tags list owns the keyboard: the page, not the index, has focus.
    fn settings_tags_page_has_focus(&self) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_focus == SettingsFocus::Page
            && self.settings_selected_section == SettingsSection::Tags
    }

    /// The Settings Tags page's selected Tag: the stored id while it still exists, else the first
    /// row, so a removed or merged-away Tag never leaves the page with nothing under the cursor.
    fn settings_tags_selected_id(&self) -> Option<u32> {
        let sorted = tags::sorted_by_name(&self.tags);
        self.settings_tags_selected
            .filter(|id| sorted.iter().any(|tag| tag.id == *id))
            .or_else(|| sorted.first().map(|tag| tag.id))
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the Tags list A–Z. `enter` has no hand-off here.
    fn apply_settings_tags_movement(&mut self, movement: Movement) {
        let sorted = tags::sorted_by_name(&self.tags);
        let len = sorted.len();
        let current = self
            .settings_tags_selected_id()
            .and_then(|id| sorted.iter().position(|tag| tag.id == id))
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => accounts::step_selection(current, len, 1),
            Movement::Prev => accounts::step_selection(current, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => {
                accounts::step_selection(current, len, SETTINGS_TAGS_HALF_PAGE)
            }
            Movement::HalfPageUp => {
                accounts::step_selection(current, len, -SETTINGS_TAGS_HALF_PAGE)
            }
            Movement::Enter => return,
        };
        self.settings_tags_selected = sorted.get(next).map(|tag| tag.id);
    }

    /// Whether Settings' Payees list owns the keyboard: the page, not the index, has focus.
    fn settings_payees_page_has_focus(&self) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_focus == SettingsFocus::Page
            && self.settings_selected_section == SettingsSection::Payees
    }

    /// The Settings Payees page's selected Payee: the stored id while it still exists, else the
    /// first row, so a deleted Payee never leaves the page with nothing under the cursor.
    fn settings_payees_selected_id(&self) -> Option<u32> {
        let sorted = payees::sorted_by_name(&self.payees);
        self.settings_payees_selected
            .filter(|id| sorted.iter().any(|payee| payee.id == *id))
            .or_else(|| sorted.first().map(|payee| payee.id))
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the Payees list A–Z. `enter` has no hand-off here.
    fn apply_settings_payees_movement(&mut self, movement: Movement) {
        let sorted = payees::sorted_by_name(&self.payees);
        let len = sorted.len();
        let current = self
            .settings_payees_selected_id()
            .and_then(|id| sorted.iter().position(|payee| payee.id == id))
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => accounts::step_selection(current, len, 1),
            Movement::Prev => accounts::step_selection(current, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => {
                accounts::step_selection(current, len, SETTINGS_PAYEES_HALF_PAGE)
            }
            Movement::HalfPageUp => {
                accounts::step_selection(current, len, -SETTINGS_PAYEES_HALF_PAGE)
            }
            Movement::Enter => return,
        };
        self.settings_payees_selected = sorted.get(next).map(|payee| payee.id);
    }

    /// Whether Settings' Documents table owns the keyboard: the page, not the index, has focus.
    fn settings_documents_page_has_focus(&self) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_focus == SettingsFocus::Page
            && self.settings_selected_section == SettingsSection::Documents
    }

    /// The Documents page's selected type: the stored id while it still exists, else the first
    /// row, so the cursor is never lost.
    fn settings_documents_selected_id(&self, cx: &App) -> Option<u32> {
        self.settings_documents_selected
            .filter(|id| documents::types::position(self.document_types(cx), *id).is_some())
            .or_else(|| self.document_types(cx).first().map(|row| row.id))
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the types in the user's order.
    fn apply_settings_documents_movement(&mut self, movement: Movement, cx: &mut App) {
        let len = self.document_types(cx).len();
        let current = self
            .settings_documents_selected_id(cx)
            .and_then(|id| documents::types::position(self.document_types(cx), id))
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => accounts::step_selection(current, len, 1),
            Movement::Prev => accounts::step_selection(current, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => {
                accounts::step_selection(current, len, SETTINGS_DOCUMENTS_HALF_PAGE)
            }
            Movement::HalfPageUp => {
                accounts::step_selection(current, len, -SETTINGS_DOCUMENTS_HALF_PAGE)
            }
            Movement::Enter => return,
        };
        self.settings_documents_selected = self.document_types(cx).get(next).map(|row| row.id);
    }

    /// `J`/`K` on the Documents page move the selected type a place, which is also its place in
    /// the Documents Type filter.
    fn handle_settings_documents_reorder_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        if !self.settings_documents_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || !modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "j" => self.move_selected_document_type(1, cx),
            "k" => self.move_selected_document_type(-1, cx),
            _ => return false,
        }
        true
    }

    /// The Documents page's own `n`/`e`/`x`, which open the Add, Edit and Remove dialogs.
    fn handle_settings_documents_key(&mut self, keystroke: &Keystroke, cx: &mut App) -> bool {
        if !self.settings_documents_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        match (keystroke.key.as_str(), modifiers.shift) {
            ("x", false) => self.remove_selected_document_type(cx),
            ("n", false) => self.open_add_document_type_dialog(cx),
            ("e", false) => {
                if let Some(id) = self.settings_documents_selected_id(cx) {
                    self.open_edit_document_type_dialog(id, cx);
                }
            }
            _ => return false,
        }
        true
    }

    fn move_selected_document_type(&mut self, delta: isize, cx: &mut App) {
        if let Some(id) = self.settings_documents_selected_id(cx) {
            self.mutate_documents(cx, |data| {
                documents::types::move_by(&mut data.types, id, delta)
            });
            self.settings_documents_selected = Some(id);
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

    /// Whether Settings' Inventory table owns the keyboard: the page, not the index, has focus.
    fn settings_inventory_page_has_focus(&self) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_focus == SettingsFocus::Page
            && self.settings_selected_section == SettingsSection::Inventory
    }

    /// The Inventory page's selected row: the stored one while it is still on screen, else the
    /// first row, so the cursor is never lost. `None` only with no Properties.
    fn settings_inventory_selected_row(&self) -> Option<InventoryRow> {
        let rows = inventory_view::visible_rows(&self.inventory, &self.settings_inventory_expanded);
        self.settings_inventory_selected
            .filter(|row| rows.contains(row))
            .or_else(|| rows.first().copied())
    }

    /// Whether `left` has something to do on the Inventory page: close an open Property, or
    /// climb from a Room to its Property. Anything else hands it back to the index.
    fn settings_inventory_left_is_local(&self) -> bool {
        match self.settings_inventory_selected_row() {
            Some(InventoryRow::Room(_)) => true,
            Some(InventoryRow::Property(id)) => self.settings_inventory_expanded.contains(&id),
            None => false,
        }
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the visible Property and Room rows as one list.
    fn apply_settings_inventory_movement(&mut self, movement: Movement) {
        let rows = inventory_view::visible_rows(&self.inventory, &self.settings_inventory_expanded);
        let Some(last) = rows.len().checked_sub(1) else {
            return;
        };
        let current = self
            .settings_inventory_selected_row()
            .and_then(|row| rows.iter().position(|r| *r == row))
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => (current + 1).min(last),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            Movement::Last => last,
            Movement::HalfPageDown => (current + SETTINGS_INVENTORY_HALF_PAGE).min(last),
            Movement::HalfPageUp => current.saturating_sub(SETTINGS_INVENTORY_HALF_PAGE),
            Movement::Enter => return,
        };
        self.settings_inventory_selected = rows.get(next).copied();
    }

    /// `J`/`K` on a Room move it a place within its Property; inert on a Property row.
    fn handle_settings_inventory_reorder_key(&mut self, keystroke: &Keystroke) -> bool {
        if !self.settings_inventory_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || !modifiers.shift {
            return false;
        }
        let delta = match keystroke.key.as_str() {
            "j" => 1,
            "k" => -1,
            _ => return false,
        };
        if let Some(InventoryRow::Room(id)) = self.settings_inventory_selected_row() {
            inventory::move_room(&mut self.inventory, id, delta);
        }
        true
    }

    /// The Inventory page's own keys: right and left open, close and climb, and `n`/`r`/`e`/`x`
    /// ask for the Add, Edit and Remove dialogs, which are still placeholders.
    fn handle_settings_inventory_key(&mut self, keystroke: &Keystroke, cx: &App) -> bool {
        if !self.settings_inventory_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_property_dialog(),
            "r" => self.add_inventory_room_for_selection(),
            "e" => {
                if let Some(row) = self.settings_inventory_selected_row() {
                    self.open_edit_inventory_row(row);
                }
            }
            "x" => {
                if let Some(row) = self.settings_inventory_selected_row() {
                    self.open_remove_inventory_row(row, cx);
                }
            }
            "right" => self.step_settings_inventory_in(),
            "left" => self.step_settings_inventory_out(),
            _ => return false,
        }
        true
    }

    /// `r`: Add room for the selected Property (or the selected Room's), opening it so the new
    /// Room shows. With no Property it is a Toast.
    fn add_inventory_room_for_selection(&mut self) {
        let property = match self.settings_inventory_selected_row() {
            Some(InventoryRow::Property(id)) => id,
            Some(InventoryRow::Room(id)) => match self.inventory.room(id) {
                Some((property, _)) => property.id,
                None => return,
            },
            None => {
                self.raise_toast(
                    ToastKind::Info,
                    crate::msg::desktop_inventory_toast_no_property(),
                );
                return;
            }
        };
        self.settings_inventory_expanded.insert(property);
        self.open_add_room_dialog(property);
    }

    /// `right`: open a closed Property, or step from an open one to its first Room.
    fn step_settings_inventory_in(&mut self) {
        let Some(InventoryRow::Property(id)) = self.settings_inventory_selected_row() else {
            return;
        };
        if self.settings_inventory_expanded.insert(id) {
            return;
        }
        if let Some(room) = self
            .inventory
            .property(id)
            .and_then(|property| property.rooms.first())
        {
            self.settings_inventory_selected = Some(InventoryRow::Room(room.id));
        }
    }

    /// `left`: climb from a Room to its Property, or close an open Property.
    fn step_settings_inventory_out(&mut self) {
        match self.settings_inventory_selected_row() {
            Some(InventoryRow::Room(id)) => {
                if let Some((property, _)) = self.inventory.room(id) {
                    self.settings_inventory_selected = Some(InventoryRow::Property(property.id));
                }
            }
            Some(InventoryRow::Property(id)) => {
                self.settings_inventory_expanded.remove(&id);
            }
            None => {}
        }
    }

    /// Whether Settings' Categories tree owns the keyboard: the page, not the index, has focus.
    fn settings_categories_page_has_focus(&self) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_focus == SettingsFocus::Page
            && self.settings_selected_section == SettingsSection::Categories
    }

    /// Whether `left` has something to do inside the Categories tree: fold an open parent, or
    /// climb from a nested row to its parent.
    fn settings_categories_left_is_local(&self) -> bool {
        let Some(id) = self.categories_selected_id else {
            return false;
        };
        let open_parent =
            self.categories_expanded.contains(&id) && !categories::is_leaf(&self.categories, id);
        open_parent
            || self
                .categories
                .iter()
                .any(|category| category.id == id && category.parent.is_some())
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the Categories tree's visible rows, Expense then
    /// Income. With no row selected (or the selected one folded away) the first press lands on
    /// the first row.
    fn apply_settings_categories_movement(&mut self, movement: Movement) {
        let rows = categories::settings_rows(&self.categories, &self.categories_expanded);
        let Some(last) = rows.len().checked_sub(1) else {
            return;
        };
        let current = self
            .categories_selected_id
            .and_then(|id| rows.iter().position(|row| row.id == id));
        let next = match movement {
            Movement::Next => current.map_or(0, |index| (index + 1).min(last)),
            Movement::Prev => current.map_or(0, |index| index.saturating_sub(1)),
            Movement::First => 0,
            Movement::Last => last,
            Movement::HalfPageDown => {
                current.map_or(0, |index| (index + CATEGORIES_HALF_PAGE).min(last))
            }
            Movement::HalfPageUp => {
                current.map_or(0, |index| index.saturating_sub(CATEGORIES_HALF_PAGE))
            }
            Movement::Enter => return,
        };
        self.categories_selected_id = rows.get(next).map(|row| row.id);
    }

    /// `right`: opens a folded parent, or steps into an open one's first child. `left`: folds an
    /// open parent, or climbs to the parent of a nested row.
    fn step_settings_categories_fold(&mut self, forward: bool) {
        let Some(id) = self.categories_selected_id else {
            return;
        };
        let is_parent = !categories::is_leaf(&self.categories, id);
        let open = self.categories_expanded.contains(&id);
        if forward {
            if !is_parent {
                return;
            }
            if open {
                self.apply_settings_categories_movement(Movement::Next);
            } else {
                self.categories_expanded.push(id);
            }
        } else if is_parent && open {
            self.categories_expanded.retain(|&other| other != id);
        } else if let Some(parent) = self
            .categories
            .iter()
            .find(|category| category.id == id)
            .and_then(|category| category.parent)
        {
            self.categories_selected_id = Some(parent);
        }
    }

    /// Keyboard input while on the Categories page: `n` adds a top-level category, `N` (shift+n)
    /// adds a sub-category to the selected one, `e` edits the selected category, `d` deletes it,
    /// `enter` opens Transactions filtered to the selected category.
    fn handle_categories_key(&mut self, keystroke: &Keystroke, cx: &mut App) -> bool {
        let on_settings_page = self.settings_categories_page_has_focus();
        if !on_settings_page {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        let shift = modifiers.shift;
        let has_mod = modifiers.control || modifiers.alt || modifiers.platform;
        if has_mod {
            return false;
        }

        let selected_category = self
            .categories_selected_id
            .and_then(|id| self.categories.iter().find(|c| c.id == id));

        match keystroke.key.as_str() {
            "n" => {
                if shift {
                    // N (shift+n): add sub-category to selected
                    if let Some(category) = selected_category {
                        self.open_add_categories_dialog(Some(category.id), cx);
                    }
                } else {
                    // n: add top-level category
                    self.open_add_categories_dialog(None, cx);
                }
                true
            }
            "e" => {
                if let Some(category) = selected_category {
                    self.open_edit_categories_dialog(category.id, cx);
                }
                true
            }
            "d" => {
                if let Some(category) = selected_category {
                    if !categories::is_leaf(&self.categories, category.id) {
                        self.chrome.status_message =
                            Some(crate::msg::desktop_status_delete_children_first());
                    } else {
                        let id = category.id;
                        self.open_delete_categories_dialog(id);
                    }
                }
                true
            }
            // Settings' tree has no ledger hand-off: `enter` is the standalone page's only.
            "enter" if !on_settings_page => {
                if let Some(category) = selected_category {
                    self.open_category_transactions(category.id, cx);
                }
                true
            }
            "right" | "left" if on_settings_page => {
                self.step_settings_categories_fold(keystroke.key == "right");
                true
            }
            _ => false,
        }
    }

    /// Why the Categories dialogs' Monthly budget field is read-only for `category_id` (`None`
    /// for a Category being added): the Personal spending Budget it reads and writes is archived,
    /// or the Category is a parent and so only rolls up.
    fn categories_budget_lock(
        &self,
        category_id: Option<u32>,
        cx: &App,
    ) -> Option<categories::form::BudgetLock> {
        if category_id.is_some_and(|id| !categories::is_leaf(&self.categories, id)) {
            return Some(categories::form::BudgetLock::Parent);
        }
        self.budgets(cx)
            .get(budgets::PERSONAL_SPENDING_ID)
            .filter(|budget| budget.is_archived())
            .map(|budget| categories::form::BudgetLock::Archived(budget.name.clone()))
    }

    /// Opens the Add categories dialog pre-scoped to parent_id (None for top-level).
    fn open_add_categories_dialog(&mut self, parent_id: Option<u32>, cx: &App) {
        // A child takes its parent's type, locked in the form; a top-level one starts as Expense.
        let category_type = match parent_id {
            Some(id) => self
                .categories
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.category_type.clone()),
            None => Some(CategoryTypes::Expense),
        };
        let form = categories::form::CategoryForm {
            name: TextField::default(),
            parent_id,
            category_type,
            budget: TextField::default(),
            budget_lock: self.categories_budget_lock(None, cx),
            focused: categories::form::CategoryField::Name,
        };
        self.open_dialog(OpenDialog::Categories(
            categories::form::CategoriesDialog::Add { parent_id, form },
        ));
    }

    /// Opens the Edit categories dialog on `id`, pre-filled. Monthly budget shows the current
    /// month's amount in the Personal spending Budget (a parent's is its children's sum).
    fn open_edit_categories_dialog(&mut self, category_id: u32, cx: &App) {
        if let Some(category) = self.categories.iter().find(|c| c.id == category_id) {
            let budget_str = self
                .budgets(cx)
                .monthly_limit(
                    budgets::PERSONAL_SPENDING_ID,
                    &self.categories,
                    category_id,
                    self.today,
                )
                .map(|amount| amount.0.to_string())
                .unwrap_or_default();
            let form = categories::form::CategoryForm {
                name: TextField::new(category.name.as_str()),
                parent_id: category.parent,
                category_type: Some(category.category_type.clone()),
                budget: TextField::new(budget_str),
                budget_lock: self.categories_budget_lock(Some(category_id), cx),
                focused: categories::form::CategoryField::Name,
            };
            self.open_dialog(OpenDialog::Categories(
                categories::form::CategoriesDialog::Edit(category_id, form),
            ));
        }
    }

    /// Categories 5c's write on a saved dialog: the Monthly budget text as an Onward amount from
    /// the current month in the Personal spending Budget, or a Stop when `clears` and it is
    /// blank. A locked field (a parent's rollup, an archived Budget) writes nothing.
    fn save_category_budget(
        &mut self,
        category_id: u32,
        form: &categories::form::CategoryForm,
        clears: bool,
        cx: &mut App,
    ) {
        if form.budget_lock.is_some() {
            return;
        }
        let amount = if form.budget.is_blank() {
            if !clears {
                return;
            }
            None
        } else {
            match form.budget.text().trim().parse::<lib_core::Money>() {
                Ok(amount) => Some(amount),
                Err(_) => return,
            }
        };
        // A Category that can't hold a Budget Amount (an Income one) is left without one.
        let _ = self.mutate_budgets(cx, |budgets| {
            budgets.set_monthly_limit(
                budgets::PERSONAL_SPENDING_ID,
                &self.categories,
                category_id,
                amount,
                self.today,
            )
        });
    }

    fn open_category_transactions(&mut self, id: u32, cx: &mut App) {
        self.edit_transactions_state(cx, |s| {
            s.filters = TransactionFilters::for_category(self.today, id)
        });
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.reset_transactions_selection(cx);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }

    /// "View transactions" on the Payees page: Transactions filtered to exactly this Payee's id, not
    /// a name substring that would over-match ("BP").
    fn open_payee_transactions(&mut self, id: u32, cx: &mut App) {
        self.edit_transactions_state(cx, |s| {
            s.filters = TransactionFilters::for_payee(self.today, id)
        });
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.reset_transactions_selection(cx);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }

    /// "View transactions" on the Tags page: Transactions filtered to exactly this Tag's id, not a
    /// name substring that would over-match ("trip").
    fn open_tag_transactions(&mut self, id: u32, cx: &mut App) {
        self.edit_transactions_state(cx, |s| {
            s.filters = TransactionFilters::for_tag(self.today, id)
        });
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.reset_transactions_selection(cx);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }

    /// The selected Tag: `tags_selected` indexes the page's usage order, not `self.tags`.
    fn selected_tag_id(&self, cx: &App) -> Option<u32> {
        if self.settings_tags_page_has_focus() {
            return self.settings_tags_selected_id();
        }
        let sorted = tags::sorted_by_usage(&self.tags, self.transactions(cx));
        sorted
            .get(self.tags_selected.min(sorted.len().saturating_sub(1)))
            .map(|tag| tag.id)
    }

    /// Selects the Tag with `id`, if it still exists.
    fn select_tag(&mut self, id: u32, cx: &App) {
        self.settings_tags_selected = Some(id);
        if let Some(index) = tags::sorted_by_usage(&self.tags, self.transactions(cx))
            .iter()
            .position(|tag| tag.id == id)
        {
            self.tags_selected = index;
        }
    }

    /// The Tags page's own `n`/`e`/`x`/`m` (only while it is the active noun and the view has
    /// focus, in `Normal` mode).
    fn handle_tags_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if !self.settings_tags_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_tag_dialog(),
            "e" => {
                if let Some(id) = self.selected_tag_id(cx) {
                    self.open_edit_tag_dialog(id);
                }
            }
            "x" => {
                if let Some(id) = self.selected_tag_id(cx) {
                    self.open_remove_tag_dialog(id, cx);
                }
            }
            "m" => {
                if let Some(id) = self.selected_tag_id(cx) {
                    self.open_merge_tags_dialog(Some(id), cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// The Dashboard's budget list, worded from the default Budget's current-month figures.
    fn dashboard_budget_list(&self, figures: &budgets::PeriodFigures) -> dashboard::BudgetList {
        use bigdecimal::{ToPrimitive, Zero};
        let bars = budgets::dashboard_bars(figures, &self.categories)
            .into_iter()
            .map(|bar| dashboard::BudgetBar {
                category: self
                    .categories
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

    fn open_add_tag_dialog(&mut self) {
        let form = tags::form::TagForm::new(self.tags.clone());
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Add(form)));
    }

    /// The form behind the open Add or Edit tag dialog, if that is what's open.
    fn tag_form_mut(&mut self) -> Option<&mut tags::form::TagForm> {
        match self.tags_dialog_mut() {
            Some(tags::form::TagsDialog::Add(form) | tags::form::TagsDialog::Edit(_, form)) => {
                Some(form)
            }
            _ => None,
        }
    }

    /// Applies a confirmed Tags dialog (`Enter` and the confirm button): adds or saves the Tag
    /// and selects it, removes it, or merges it into another, toasting the last two.
    fn apply_tags_dialog(&mut self, dialog: tags::form::TagsDialog, cx: &mut Context<'_, Self>) {
        match dialog {
            tags::form::TagsDialog::Add(form) => {
                let Some(draft) = form.draft() else {
                    return;
                };
                // `is_valid` ran the same name check, so a refusal can only leave the dialog open.
                match tags::insert_tag(&mut self.tags, &draft) {
                    Ok(id) => self.select_tag(id, cx),
                    Err(_) => self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Add(form))),
                }
            }
            tags::form::TagsDialog::Edit(id, form) => {
                let Some(draft) = form.draft() else {
                    return;
                };
                let saved = tags::edit_tag(&mut self.tags, id, &draft)
                    .and_then(|()| tags::set_active(&mut self.tags, id, form.is_active));
                match saved {
                    Ok(()) => self.select_tag(id, cx),
                    Err(_) => {
                        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Edit(id, form)))
                    }
                }
            }
            tags::form::TagsDialog::Remove(id, _) => self.apply_remove_tag(id, cx),
            tags::form::TagsDialog::Merge(form) => {
                if let Some((source, target)) = form.pair() {
                    self.apply_merge_tags(source, target, cx);
                }
            }
        }
    }

    fn handle_tags_dialog_field_click(
        &mut self,
        field: tags::form::TagField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.tag_form_mut() {
            form.focus(field);
        }
        cx.notify();
    }

    fn handle_tags_dialog_pick(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.tag_form_mut() {
            form.focus(tags::form::TagField::Swatches);
            form.pick(index);
        }
        cx.notify();
    }

    fn handle_tags_dialog_toggle_active(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.tag_form_mut() {
            form.toggle_active();
        }
        cx.notify();
    }

    fn handle_tags_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    fn handle_tags_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    fn open_edit_tag_dialog(&mut self, id: u32) {
        let Some(tag) = tags::get(&self.tags, id) else {
            return;
        };
        let form = tags::form::TagForm::for_edit(tag, self.tags.clone());
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Edit(id, form)));
    }

    fn open_remove_tag_dialog(&mut self, id: u32, cx: &App) {
        let Some(tag) = tags::get(&self.tags, id) else {
            return;
        };
        let form = tags::form::RemoveTagForm::new(
            &tag.name,
            tags::transaction_count(self.transactions(cx), id),
        );
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Remove(id, form)));
    }

    /// Untags every Split, deletes the Tag and toasts it. The selection keeps its position, so it
    /// lands on the next Tag in usage order (or the new last one).
    fn apply_remove_tag(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(name) = tags::get(&self.tags, id).map(|tag| tag.name.clone()) else {
            return;
        };
        let removed = edit_transactions(&self.transactions_store, cx, |transactions| {
            tags::remove_tag(&mut self.tags, transactions, id)
        });
        let (kind, text) = match removed {
            Ok(()) => (
                ToastKind::Success,
                lib_locale::msg::toast_tag_deleted(&name),
            ),
            Err(error) => (
                ToastKind::Error,
                lib_locale::msg::toast_save_failed(
                    &lib_locale::msg::toast_entity_tag(),
                    &error.to_string(),
                ),
            ),
        };
        self.raise_toast(kind, text);
        self.tags_selected = self.tags_selected.min(self.tags.len().saturating_sub(1));
    }

    /// The 7e selects' options, labelled `Shared (9 txns)`.
    fn merge_tag_options(&self, cx: &App) -> Vec<tags::form::MergeOption> {
        tags::form::merge_options(&self.tags, self.transactions(cx), |name, count| {
            crate::msg::desktop_tags_merge_option(name, i64::try_from(count).unwrap_or(i64::MAX))
        })
    }

    /// Opens 7e with `source` as the source Tag and, when it is flagged as a likely duplicate, its
    /// suggested target (#354). `None` (the palette's `tags merge`, or the subline link with
    /// nothing flagged) leaves both selects empty.
    fn open_merge_tags_dialog(&mut self, source: Option<u32>, cx: &App) {
        let groups = tags::duplicate_groups(&self.tags, self.transactions(cx));
        let target = source.and_then(|id| tags::duplicate_of(&groups, id));
        let form = tags::form::MergeTagsForm::new(self.merge_tag_options(cx), source, target);
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Merge(form)));
    }

    /// Retags the source's Splits with the target, deletes the source, toasts it and selects the
    /// target.
    fn apply_merge_tags(&mut self, source: u32, target: u32, cx: &mut Context<'_, Self>) {
        let (Some(source_name), Some(target_name)) = (
            tags::get(&self.tags, source).map(|tag| tag.name.clone()),
            tags::get(&self.tags, target).map(|tag| tag.name.clone()),
        ) else {
            return;
        };
        let transactions = tags::transaction_count(self.transactions(cx), source);
        let (kind, text) = match edit_transactions(&self.transactions_store, cx, |transactions| {
            tags::merge_tags(&mut self.tags, transactions, source, target)
        }) {
            Ok(()) => (
                ToastKind::Success,
                lib_locale::msg::toast_tag_merged(
                    &source_name,
                    &target_name,
                    i64::try_from(transactions).unwrap_or(i64::MAX),
                ),
            ),
            Err(error) => (
                ToastKind::Error,
                lib_locale::msg::toast_save_failed(
                    &lib_locale::msg::toast_entity_tag(),
                    &error.to_string(),
                ),
            ),
        };
        self.raise_toast(kind, text);
        self.select_tag(target, cx);
    }

    fn handle_merge_tags_field_click(
        &mut self,
        field: tags::form::MergeField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(tags::form::TagsDialog::Merge(form)) = self.tags_dialog_mut() {
            form.toggle(field);
        }
        cx.notify();
    }

    fn handle_merge_tags_option_click(
        &mut self,
        field: tags::form::MergeField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(tags::form::TagsDialog::Merge(form)) = self.tags_dialog_mut() {
            form.choose(field, index);
        }
        cx.notify();
    }

    fn handle_tags_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_tag_dialog();
        cx.notify();
    }

    /// A click on a row of Settings' Tags list: selects it and moves focus into the page.
    fn handle_settings_tags_row_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id, cx);
        self.focus_settings_page();
        cx.notify();
    }

    fn handle_tags_duplicate_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id, cx);
        self.open_merge_tags_dialog(Some(id), cx);
        cx.notify();
    }

    fn handle_tags_edit_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id, cx);
        self.open_edit_tag_dialog(id);
        cx.notify();
    }

    fn handle_tags_remove_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id, cx);
        self.open_remove_tag_dialog(id, cx);
        cx.notify();
    }

    fn selected_payee_id(&self) -> Option<u32> {
        if self.settings_payees_page_has_focus() {
            return self.settings_payees_selected_id();
        }
        self.payees
            .get(
                self.payees_selected
                    .min(self.payees.len().saturating_sub(1)),
            )
            .map(|payee| payee.id)
    }

    /// Selects the Payee with `id`, if it still exists.
    fn select_payee(&mut self, id: u32) {
        self.settings_payees_selected = Some(id);
        if let Some(index) = self.payees.iter().position(|payee| payee.id == id) {
            self.payees_selected = index;
        }
    }

    /// The Payees page's own `n`/`e`/`d` (only while it is the active noun and the view has focus,
    /// in `Normal` mode): the Add, Edit and Delete dialogs.
    fn handle_payees_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if !self.settings_payees_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_payee_dialog(),
            "e" => {
                if let Some(id) = self.selected_payee_id() {
                    self.open_edit_payee_dialog(id);
                }
            }
            "d" => {
                if let Some(id) = self.selected_payee_id() {
                    self.open_delete_payee_dialog(id, cx);
                }
            }
            _ => return false,
        }
        true
    }

    fn payee_dialog_options(&self) -> payees::form::PayeeOptions {
        payees::form::PayeeOptions::new(
            &self.categories,
            crate::msg::desktop_payees_category_none(),
        )
    }

    fn open_add_payee_dialog(&mut self) {
        let form = payees::form::PayeeForm::new(&self.payee_dialog_options(), &self.payees);
        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Add(form)));
    }

    /// Applies a confirmed Payees dialog (reached through [`Self::confirm_open_dialog`] from the
    /// **Add payee** / **Save** / **Delete** buttons or `Enter`). Add and Edit store the Payee and
    /// select it; a refused submit reopens the dialog with the error shown. Delete applies its
    /// [`payees::form::DeleteAction`], toasts the outcome and keeps the selection in range.
    fn apply_payees_dialog(
        &mut self,
        dialog: payees::form::PayeesDialog,
        cx: &mut Context<'_, Self>,
    ) {
        match dialog {
            payees::form::PayeesDialog::Delete(id, form) => {
                let Some(name) = payees::get(&self.payees, id).map(|p| p.name.clone()) else {
                    return;
                };
                let action = form.action();
                let (kind, text) = match payees::apply_delete_action(
                    &mut self.payees,
                    self.transactions_store.read(cx).transactions(),
                    id,
                    action,
                ) {
                    Ok(()) => (
                        ToastKind::Success,
                        match action {
                            payees::form::DeleteAction::Delete => {
                                lib_locale::msg::toast_payee_deleted(&name)
                            }
                            payees::form::DeleteAction::Deactivate => {
                                lib_locale::msg::toast_payee_deactivated(&name)
                            }
                            payees::form::DeleteAction::Reactivate => {
                                lib_locale::msg::toast_payee_reactivated(&name)
                            }
                        },
                    ),
                    Err(error) => (
                        ToastKind::Error,
                        lib_locale::msg::toast_save_failed(
                            &lib_locale::msg::toast_entity_payee(),
                            &error.to_string(),
                        ),
                    ),
                };
                self.raise_toast(kind, text);
                self.payees_selected = self
                    .payees_selected
                    .min(self.payees.len().saturating_sub(1));
            }
            payees::form::PayeesDialog::Add(mut form) => {
                match payees::insert_payee(&mut self.payees, &form.draft()) {
                    Ok(id) => self.select_payee(id),
                    Err(error) => {
                        form.error = Some(error);
                        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Add(form)));
                    }
                }
            }
            payees::form::PayeesDialog::Edit(id, mut form) => {
                match payees::edit_payee(&mut self.payees, id, &form.draft()) {
                    Ok(()) => self.select_payee(id),
                    Err(error) => {
                        form.error = Some(error);
                        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Edit(
                            id, form,
                        )));
                    }
                }
            }
        }
    }

    fn with_payee_form(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut payees::form::PayeeForm),
    ) {
        if let Some(form) = self
            .payees_dialog_mut()
            .and_then(payees::form::PayeesDialog::form_mut)
        {
            change(form);
        }
        cx.notify();
    }

    fn handle_payees_dialog_field_click(
        &mut self,
        field: payees::form::PayeeField,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_payee_form(cx, |form| {
            if field == payees::form::PayeeField::DefaultCategory {
                form.click_select();
            } else {
                form.focus(field);
            }
        });
    }

    fn handle_payees_dialog_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.with_payee_form(cx, |form| form.choose_category(index));
    }

    fn handle_payees_dialog_add_rule(&mut self, cx: &mut Context<'_, Self>) {
        self.with_payee_form(cx, |form| {
            form.focus(payees::form::PayeeField::Rule);
            form.add_rule();
        });
    }

    fn handle_payees_dialog_remove_rule(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.with_payee_form(cx, |form| form.remove_rule(index));
    }

    fn handle_payees_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    fn handle_payees_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// Opens the Edit dialog pre-filled from Payee `id`.
    fn open_edit_payee_dialog(&mut self, id: u32) {
        let Some(payee) = payees::get(&self.payees, id) else {
            return;
        };
        let form =
            payees::form::PayeeForm::from_payee(payee, &self.payee_dialog_options(), &self.payees);
        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Edit(
            id, form,
        )));
    }

    /// Opens the Delete dialog on Payee `id`, copying its name and action in so the form validates
    /// without `Shell`. A no-op if the Payee is gone.
    fn open_delete_payee_dialog(&mut self, id: u32, cx: &App) {
        let Some(payee) = payees::get(&self.payees, id) else {
            return;
        };
        let action = payees::form::DeleteAction::for_payee(payee, self.transactions(cx));
        let form = payees::form::DeletePayeeForm::new(payee, action);
        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Delete(
            id, form,
        )));
    }

    /// The Payee the Delete dialog is open on and what confirming it would do (#283).
    fn delete_payee_target(&self) -> Option<(&Payee, payees::form::DeleteAction)> {
        let Some(payees::form::PayeesDialog::Delete(id, form)) = self.payees_dialog() else {
            return None;
        };
        Some((payees::get(&self.payees, *id)?, form.action()))
    }

    fn handle_payees_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_payee_dialog();
        cx.notify();
    }

    /// A click on a row of Settings' Payees list: selects it and moves focus into the page.
    fn handle_settings_payees_row_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_payee(id);
        self.focus_settings_page();
        cx.notify();
    }

    /// A click on a row of Settings' Documents table: selects it and moves focus into the page.
    fn handle_settings_documents_row_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.settings_documents_selected = Some(id);
        self.focus_settings_page();
        cx.notify();
    }

    /// **edit** on a Documents row: selects it and opens the Edit dialog.
    fn handle_settings_documents_edit_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.settings_documents_selected = Some(id);
        self.focus_settings_page();
        self.open_edit_document_type_dialog(id, cx);
        cx.notify();
    }

    /// **remove** on a Documents row: selects it and opens the Remove dialog.
    fn handle_settings_documents_remove_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.settings_documents_selected = Some(id);
        self.focus_settings_page();
        self.open_remove_document_type_dialog(id, cx);
        cx.notify();
    }

    /// **+ Add document type**: opens the Add dialog.
    fn handle_settings_documents_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.focus_settings_page();
        self.open_add_document_type_dialog(cx);
        cx.notify();
    }

    /// A click on a row of Settings' Inventory table: selects it and moves focus into the page.
    fn handle_settings_inventory_row_click(
        &mut self,
        row: InventoryRow,
        cx: &mut Context<'_, Self>,
    ) {
        self.settings_inventory_selected = Some(row);
        self.focus_settings_page();
        cx.notify();
    }

    /// The disclosure control of a Property row: opens or closes it. Closing keeps the selection
    /// on screen by moving a hidden Room's selection up to its Property.
    fn handle_settings_inventory_toggle_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.focus_settings_page();
        if self.settings_inventory_expanded.remove(&id) {
            if let Some(InventoryRow::Room(room)) = self.settings_inventory_selected_row()
                && self
                    .inventory
                    .room(room)
                    .is_some_and(|(property, _)| property.id == id)
            {
                self.settings_inventory_selected = Some(InventoryRow::Property(id));
            }
        } else {
            self.settings_inventory_expanded.insert(id);
        }
        cx.notify();
    }

    /// **edit** on an Inventory row: selects it and asks for the Edit dialog.
    fn handle_settings_inventory_edit_click(
        &mut self,
        row: InventoryRow,
        cx: &mut Context<'_, Self>,
    ) {
        self.settings_inventory_selected = Some(row);
        self.focus_settings_page();
        self.open_edit_inventory_row(row);
        cx.notify();
    }

    /// **remove** on an Inventory row: selects it and asks for the Remove dialog.
    fn handle_settings_inventory_remove_click(
        &mut self,
        row: InventoryRow,
        cx: &mut Context<'_, Self>,
    ) {
        self.settings_inventory_selected = Some(row);
        self.focus_settings_page();
        self.open_remove_inventory_row(row, cx);
        cx.notify();
    }

    /// **+ Add property**.
    fn handle_settings_inventory_add_property_click(&mut self, cx: &mut Context<'_, Self>) {
        self.focus_settings_page();
        self.open_add_property_dialog();
        cx.notify();
    }

    /// **+ Add room** under an open Property.
    fn handle_settings_inventory_add_room_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.focus_settings_page();
        self.open_add_room_dialog(id);
        cx.notify();
    }

    fn handle_payees_edit_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_payee(id);
        self.open_edit_payee_dialog(id);
        cx.notify();
    }

    fn handle_payees_delete_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_payee(id);
        self.open_delete_payee_dialog(id, cx);
        cx.notify();
    }

    fn handle_categories_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_categories_dialog(None, cx);
        cx.notify();
    }

    fn handle_categories_add_sub_click(&mut self, parent_id: u32, cx: &mut Context<'_, Self>) {
        self.categories_selected_id = Some(parent_id);
        self.open_add_categories_dialog(Some(parent_id), cx);
        cx.notify();
    }

    fn handle_categories_edit_click(&mut self, category_id: u32, cx: &mut Context<'_, Self>) {
        if self.categories.iter().any(|c| c.id == category_id) {
            self.categories_selected_id = Some(category_id);
            self.open_edit_categories_dialog(category_id, cx);
            cx.notify();
        }
    }

    fn handle_categories_delete_click(&mut self, category_id: u32, cx: &mut Context<'_, Self>) {
        if !categories::is_leaf(&self.categories, category_id) {
            self.chrome.status_message = Some(crate::msg::desktop_status_delete_children_first());
            cx.notify();
            return;
        }

        self.open_delete_categories_dialog(category_id);
        cx.notify();
    }

    /// Opens the Delete category dialog on `category_id`, copying its name in so the form
    /// validates without `Shell`. A no-op if the category is gone.
    fn open_delete_categories_dialog(&mut self, category_id: u32) {
        let Some(category) = self.categories.iter().find(|c| c.id == category_id) else {
            return;
        };
        let form = categories::form::DeleteCategoryForm::new(category.name.as_str());
        self.open_dialog(OpenDialog::Categories(
            categories::form::CategoriesDialog::Delete(category_id, form),
        ));
    }

    fn handle_categories_dialog_field_click(
        &mut self,
        field: categories::form::CategoryField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(dialog) = self.categories_dialog_mut()
            && let Some(form) = dialog.form_mut()
        {
            form.focus_field(field);
            cx.notify();
        }
    }

    fn handle_categories_dialog_parent_change(
        &mut self,
        parent_id: Option<u32>,
        cx: &mut Context<'_, Self>,
    ) {
        // A child takes its parent's type.
        let parent_type = parent_id.and_then(|parent_id| {
            self.categories
                .iter()
                .find(|c| c.id == parent_id)
                .map(|parent| parent.category_type.clone())
        });
        if let Some(dialog) = self.categories_dialog_mut()
            && let Some(form) = dialog.form_mut()
        {
            form.parent_id = parent_id;
            if parent_type.is_some() {
                form.category_type = parent_type;
            }
            cx.notify();
        }
    }

    fn handle_categories_dialog_type_change(
        &mut self,
        category_type: CategoryTypes,
        cx: &mut Context<'_, Self>,
    ) {
        let can_change_type = match self.categories_dialog() {
            Some(categories::form::CategoriesDialog::Add { form, .. }) => form.parent_id.is_none(),
            Some(categories::form::CategoriesDialog::Edit(id, _)) => self
                .categories
                .iter()
                .find(|c| c.id == *id)
                .is_some_and(|c| c.parent.is_none()),
            _ => false,
        };
        if !can_change_type {
            return;
        }
        let edited_id = match self.categories_dialog() {
            Some(categories::form::CategoriesDialog::Edit(id, _)) => Some(*id),
            _ => None,
        };
        if let Some(dialog) = self.categories_dialog_mut()
            && let Some(form) = dialog.form_mut()
        {
            form.category_type = Some(category_type.clone());
        }
        // For Edit dialogs, cascade the type change to descendants
        if let Some(id) = edited_id {
            let _ = categories::change_category_type(&mut self.categories, id, category_type);
        }
        cx.notify();
    }

    fn handle_categories_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    fn handle_categories_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// Applies a confirmed Categories dialog (reached through [`Self::confirm_open_dialog`] from
    /// the Add/Save/Delete button or `Enter`): Add inserts the category and saves its Monthly
    /// budget; Edit renames, re-parents and saves the budget; Delete removes it and keeps the
    /// selection in range. The form has already validated.
    fn apply_categories_dialog(
        &mut self,
        dialog: categories::form::CategoriesDialog,
        cx: &mut Context<'_, Self>,
    ) {
        match dialog {
            categories::form::CategoriesDialog::Add { form, .. } => {
                let category_type = form.category_type.clone().unwrap_or(CategoryTypes::Expense);
                // A rejected insert (e.g. depth) closes the dialog without adding anything.
                if let Ok(category_id) = categories::insert_category(
                    &mut self.categories,
                    form.name.text().trim().to_string(),
                    form.parent_id,
                    category_type,
                ) {
                    self.save_category_budget(category_id, &form, false, cx);
                }
            }
            categories::form::CategoriesDialog::Edit(id, form) => {
                let _ = categories::edit_category(
                    &mut self.categories,
                    id,
                    form.name.text().trim().to_string(),
                );
                // `None` moves it to top level when the parent was cleared.
                let _ = categories::move_category(&mut self.categories, id, form.parent_id);
                self.save_category_budget(id, &form, true, cx);
            }
            categories::form::CategoriesDialog::Delete(category_id, _) => {
                let (kind, text) = match prepare_category_delete(&mut self.categories, category_id)
                {
                    Err(refusal) => refusal,
                    Ok(uncategorised_id) => {
                        let moved =
                            edit_transactions(&self.transactions_store, cx, |transactions| {
                                move_category_splits(transactions, category_id, uncategorised_id)
                            });
                        self.budgets_store.update(cx, |store, cx| {
                            store.mutate(cx, |budgets| {
                                finish_category_delete(
                                    &mut self.categories,
                                    budgets,
                                    category_id,
                                    moved,
                                )
                            })
                        })
                    }
                };
                self.raise_toast(kind, text);
                // Keep the selection in range
                let tree_rows = categories::tree_rows(&self.categories, &self.categories_expanded);
                let filtered_rows = tree_rows
                    .iter()
                    .filter(|row| {
                        self.categories
                            .iter()
                            .find(|c| c.id == row.id)
                            .is_some_and(|c| c.category_type == CategoryTypes::Expense)
                    })
                    .count();
                self.categories_selected = self
                    .categories_selected
                    .min(filtered_rows.saturating_sub(1));
            }
        }
    }

    fn handle_categories_disclosure_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        categories::toggle_expanded(&mut self.categories_expanded, id, false);
        cx.notify();
    }

    /// A click on a row of Settings' Categories tree: selects it and moves focus into the page.
    fn handle_settings_categories_row_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.categories_selected_id = Some(id);
        self.focus_settings_page();
        cx.notify();
    }

    /// The Units section's own "+ Add unit" button (issue #184, replacing the stub #177 left
    /// behind): opens the Add unit dialog rather than flashing a status message.
    fn handle_add_unit_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_dialog(OpenDialog::Settings(SettingsDialog::AddUnit(
            UnitForm::default(),
        )));
        cx.notify();
    }

    /// The Units table's own row "edit" button (issue #185, replacing the stub #177 left
    /// behind): opens the Edit unit dialog pre-filled from the clicked row
    /// (`UnitForm::from_row`) rather than flashing a status message. A no-op if `index` is
    /// somehow out of bounds (defensive only -- every caller is a row's own click handler, so
    /// this should never actually happen).
    fn handle_unit_edit_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.settings_units.get(index) else {
            return;
        };
        let form = UnitForm::from_row(row);
        self.open_dialog(OpenDialog::Settings(SettingsDialog::EditUnit(index, form)));
        cx.notify();
    }

    /// Shared by the Add/Edit unit dialogs' own field-focus clicks (issues #184/#185) -- which
    /// field a click targets doesn't depend on which dialog variant is open.
    fn handle_unit_dialog_field_click(&mut self, field: AddUnitField, cx: &mut Context<'_, Self>) {
        if let Some(dialog) = self.settings_dialog_mut() {
            match dialog {
                SettingsDialog::AddUnit(form) | SettingsDialog::EditUnit(_, form) => {
                    form.focused_field = field;
                    cx.notify();
                }
                // Neither has more than one text field, always implicitly focused -- nothing to
                // click into.
                SettingsDialog::DeleteUnit(..)
                | SettingsDialog::AddInstitution(_)
                | SettingsDialog::ClearLogs => {}
            }
        }
    }

    /// Shared by the Add/Edit unit dialogs' own Type segmented control (issues #184/#185).
    fn handle_unit_dialog_kind_click(&mut self, kind: UnitKind, cx: &mut Context<'_, Self>) {
        if let Some(dialog) = self.settings_dialog_mut() {
            match dialog {
                SettingsDialog::AddUnit(form) | SettingsDialog::EditUnit(_, form) => {
                    form.kind = kind;
                    cx.notify();
                }
                // Neither has a Type selector at all.
                SettingsDialog::DeleteUnit(..)
                | SettingsDialog::AddInstitution(_)
                | SettingsDialog::ClearLogs => {}
            }
        }
    }

    /// Shared by the Add/Edit unit dialogs' own Cancel button -- discards whatever was typed,
    /// same as `Esc` (`Self::handle_key_down`'s `ClosePopupsAndExitMode` arm).
    fn handle_settings_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    /// Applies a confirmed Settings dialog (reached through [`Self::confirm_open_dialog`], from
    /// the Add/Save/Delete button or `Enter`): the README's own "Dialog lifecycle" rows -- Add
    /// "validate -> append to Units table -> close", Edit "Save -> update the in-memory row ->
    /// close" (a changed code just relabels the row here; rewriting real references is out of
    /// scope per the map's own Destination), Delete "confirm -> remove + close". The form has
    /// already validated.
    fn apply_settings_dialog(&mut self, dialog: SettingsDialog) {
        match dialog {
            SettingsDialog::AddUnit(form) => {
                self.settings_units.push(UnitRow {
                    code: form.code.into_text(),
                    name: form.name.into_text(),
                    kind: form.kind.label().to_string(),
                    // Neither field exists in the Add unit dialog (issue #184's own fields are
                    // just Code/Name/Type) -- a dialog-created unit has no real price-source
                    // integration yet, and can't be the ledger's base/default unit (nothing
                    // lets a user change which one that is).
                    source: "Manual entry".to_string(),
                    is_base: false,
                    is_default: false,
                });
            }
            SettingsDialog::EditUnit(index, form) => {
                if let Some(existing) = self.settings_units.get_mut(index) {
                    // `source`/`is_base`/`is_default` aren't Edit unit dialog fields either
                    // (issue #185's own body: "same form as Add unit") -- preserved from the
                    // row being edited rather than reset, unlike `code`/`name`/`kind`.
                    *existing = UnitRow {
                        code: form.code.into_text(),
                        name: form.name.into_text(),
                        kind: form.kind.label().to_string(),
                        source: existing.source.clone(),
                        is_base: existing.is_base,
                        is_default: existing.is_default,
                    };
                }
            }
            SettingsDialog::DeleteUnit(index, form) => {
                // Defensive only: the dialog is modal, so the row it opened on is still there.
                if self
                    .settings_units
                    .get(index)
                    .is_some_and(|row| row.code == form.code)
                {
                    let unit = self.settings_units.remove(index);
                    self.raise_toast(
                        ToastKind::Success,
                        lib_locale::msg::toast_unit_deleted(&unit.code),
                    );
                }
            }
            SettingsDialog::AddInstitution(form) => {
                let account_type = form
                    .account_types
                    .iter()
                    .map(|account_type| account_type.label())
                    .collect::<Vec<_>>()
                    .join(" \u{b7} ");
                self.settings_institutions.push(InstitutionRow {
                    name: form.name.into_text(),
                    account_type,
                });
            }
            SettingsDialog::ClearLogs => {
                self.settings_log.clear();
                self.settings_log_list.reset(0);
                self.raise_toast(
                    ToastKind::Info,
                    crate::msg::desktop_settings_tracing_toast_cleared(),
                );
            }
        }
    }

    fn handle_settings_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// The settings index rail's own row click (`chrome::rail::settings_index::OnEntryClick`):
    /// swaps the settings body to the clicked page and takes the active dark treatment
    /// (`docs/ux/desktop-mockups/16-settings/README.md`'s "Navigation" bullet).
    fn handle_settings_index_click(
        &mut self,
        section: SettingsSection,
        cx: &mut Context<'_, Self>,
    ) {
        self.select_settings_page(section);
        self.settings_focus = SettingsFocus::Index;
        cx.notify();
    }

    /// The Price Sources table's own row "test"/"edit"/"delete" buttons and its own "+ Add price
    /// source" button (issue #189): no dialog exists for any of these anywhere on this map (same
    /// reasoning as Institutions' own row edit/delete, `Self::handle_institution_edit_click`'s
    /// own doc), so each flashes a plain "not yet built" status message naming no issue.
    fn handle_price_source_test_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index;
        self.chrome.status_message =
            Some(crate::msg::desktop_status_test_price_source_not_yet_built());
        cx.notify();
    }

    fn handle_price_source_edit_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index;
        self.chrome.status_message =
            Some(crate::msg::desktop_status_edit_price_source_not_yet_built());
        cx.notify();
    }

    fn handle_price_source_delete_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index;
        self.chrome.status_message =
            Some(crate::msg::desktop_status_delete_price_source_not_yet_built());
        cx.notify();
    }

    fn handle_add_price_source_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message =
            Some(crate::msg::desktop_status_add_price_source_not_yet_built());
        cx.notify();
    }

    /// The Units table's own row "delete" button (issue #186, replacing the stub #177 left
    /// behind): opens the destructive Delete unit confirm dialog rather than flashing a status
    /// message. A no-op if `index` is somehow out of bounds (defensive only, same reasoning as
    /// [`Self::handle_unit_edit_click`]).
    fn handle_unit_delete_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.settings_units.get(index) else {
            return;
        };
        let form = DeleteUnitForm::new(row.code.as_str());
        self.open_dialog(OpenDialog::Settings(SettingsDialog::DeleteUnit(
            index, form,
        )));
        cx.notify();
    }

    /// The Institutions table's own row "edit"/"delete" buttons (issue #178). Unlike
    /// [`Self::handle_unit_edit_click`]/[`Self::handle_unit_delete_click`], neither stub names an
    /// issue: no `EditInstitution`/`DeleteInstitution` dialog is specified anywhere on this map
    /// (the README's own "Dialog lifecycle" table and `State` block only ever mention
    /// `AddInstitution`), so there is no ticket to point at.
    fn handle_institution_edit_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index; // no row-scoped state until a future ticket specifies this dialog
        self.chrome.status_message =
            Some(crate::msg::desktop_status_edit_institution_not_yet_built());
        cx.notify();
    }

    fn handle_institution_delete_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index; // no row-scoped state until a future ticket specifies this dialog
        self.chrome.status_message =
            Some(crate::msg::desktop_status_delete_institution_not_yet_built());
        cx.notify();
    }

    /// The Institutions table's own "+ Add institution" button (issue #187, replacing the stub
    /// #178 left behind): opens the real Add institution dialog rather than flashing a status
    /// message. `AddInstitutionForm::new` seeds Default unit from `self.settings_units`' own
    /// first entry, so this dialog reads Units' live state even though the two sections are
    /// otherwise independent.
    fn handle_add_institution_click(&mut self, cx: &mut Context<'_, Self>) {
        let form = AddInstitutionForm::new(&self.settings_units);
        self.open_dialog(OpenDialog::Settings(SettingsDialog::AddInstitution(form)));
        cx.notify();
    }

    fn handle_add_institution_account_type_click(
        &mut self,
        account_type: AccountType,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(SettingsDialog::AddInstitution(form)) = self.settings_dialog_mut() {
            form.toggle_account_type(account_type);
            cx.notify();
        }
    }

    fn handle_add_institution_unit_click(&mut self, code: String, cx: &mut Context<'_, Self>) {
        if let Some(SettingsDialog::AddInstitution(form)) = self.settings_dialog_mut() {
            form.default_unit_code = Some(code);
            cx.notify();
        }
    }

    /// The Sync server section's own "Sync now" button (issue #180): unlike the "+ Add"
    /// buttons above, this has no future ticket that will give it real behaviour -- the map's
    /// own Out-of-scope names it a permanent stand-in -- so the stub message names no issue.
    fn handle_sync_now_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message = Some(crate::msg::desktop_status_sync_now_not_implemented());
        cx.notify();
    }

    /// The Data & backup section's own "Backup now"/"Export ledger (CSV)" buttons (issue #181)
    /// -- same permanently-out-of-scope reasoning as [`Self::handle_sync_now_click`].
    fn handle_backup_now_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message = Some(crate::msg::desktop_status_backup_now_not_implemented());
        cx.notify();
    }

    fn handle_export_ledger_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message =
            Some(crate::msg::desktop_status_export_ledger_not_implemented());
        cx.notify();
    }

    /// The Tracing page's level radios: filters the log box at once, including what is already
    /// there, and starts it back at the newest entry.
    fn handle_tracing_level_click(&mut self, level: TracingLevel, cx: &mut Context<'_, Self>) {
        self.set_tracing_level(level);
        cx.notify();
    }

    fn set_tracing_level(&mut self, level: TracingLevel) {
        self.settings_log.set_level(level);
        self.settings_log_list
            .reset(self.settings_log.visible().len());
    }

    /// The Display section's own "Date format" segmented control (issue #179) -- a stored
    /// preference that also re-renders the PREVIEW table's own DATE column.
    fn handle_date_style_click(&mut self, style: Option<DateStyle>, cx: &mut Context<'_, Self>) {
        self.settings_date_style = style;
        cx.notify();
    }

    /// The same section's "Row density" segmented control -- also re-renders the PREVIEW table's
    /// own row padding (`view::settings::display`'s own doc: the one field this map gives a real
    /// visual effect to, not just a stored preference).
    fn handle_row_density_click(&mut self, density: RowDensity, cx: &mut Context<'_, Self>) {
        self.settings_row_density = density;
        // The Transactions table's scroll offset is in pixels, so a new row height would leave it
        // pointing at a different row: re-anchor on the selected one for its next paint.
        self.transactions_state(cx)
            .scroll
            .scroll_to_item_strict(self.transactions_state(cx).selected, ScrollStrategy::Center);
        cx.notify();
    }

    /// The same section's "Status glyphs" radio group -- also re-renders the PREVIEW table's own
    /// leftmost glyph column.
    fn handle_status_glyphs_click(&mut self, glyphs: StatusGlyphs, cx: &mut Context<'_, Self>) {
        self.settings_status_glyphs = glyphs;
        cx.notify();
    }

    /// The same section's "Start Sidebar minimised" toggle -- takes effect at the next launch, so
    /// the current rail state is left alone.
    fn handle_start_sidebar_minimised_click(&mut self, cx: &mut Context<'_, Self>) {
        self.settings_start_sidebar_minimised = !self.settings_start_sidebar_minimised;
        cx.notify();
    }

    /// The Tracing page's **Clear logs** button (and `c`): asks first, since the capture is
    /// emptied for good (issue #502).
    fn handle_clear_logs_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_clear_logs_dialog();
        cx.notify();
    }

    fn open_clear_logs_dialog(&mut self) {
        self.chrome.status_message = None;
        self.open_dialog(OpenDialog::Settings(SettingsDialog::ClearLogs));
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

/// The refusal Toast for a category delete that the store will not make.
fn category_refused(error: categories::CategoryError) -> (ToastKind, String) {
    (
        ToastKind::Error,
        lib_locale::msg::toast_save_failed(
            &lib_locale::msg::toast_entity_category(),
            &error.to_string(),
        ),
    )
}

/// The checks a category delete makes before it moves any Splits: the Category must exist and be a
/// leaf. Returns the Uncategorised Category its Splits move to, or the refusal Toast. The store's
/// own refusal can still come after the Splits have moved, as it did before this split.
fn prepare_category_delete(
    categories: &mut Vec<Category>,
    id: u32,
) -> Result<u32, (ToastKind, String)> {
    let Some(category) = categories.iter().find(|c| c.id == id) else {
        return Err(category_refused(categories::CategoryError::NotFound));
    };
    if !categories::is_leaf(categories, id) {
        return Err(category_refused(categories::CategoryError::NonLeafDeletion));
    }
    let category_type = category.category_type.clone();
    Ok(categories::get_or_create_uncategorised(
        categories,
        category_type,
    ))
}

/// Re-points every Split on category `from` to `to`, returning how many moved. Runs on the
/// Transactions store, so it borrows no Category or Budget state.
fn move_category_splits(transactions: &mut [Transaction], from: u32, to: u32) -> i64 {
    let mut moved = 0_i64;
    for split in transactions.iter_mut().flat_map(|t| t.splits.iter_mut()) {
        if split.category_id == from {
            split.category_id = to;
            moved += 1;
        }
    }
    moved
}

/// Drops category `id`'s budget and the Category itself, and returns the Toast counting the
/// `moved` Splits: Success, or the `toast-save-failed` Error if the store refuses (the dialog
/// only opens on a leaf, so that means the tree changed underneath it).
fn finish_category_delete(
    categories: &mut Vec<Category>,
    budgets: &mut budgets::Budgets,
    id: u32,
    moved: i64,
) -> (ToastKind, String) {
    let name = categories
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    budgets.remove_category(id);
    match categories::delete_category(categories, id) {
        Ok(()) => (
            ToastKind::Success,
            lib_locale::msg::toast_category_deleted(&name, moved),
        ),
        Err(error) => category_refused(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::transactions::hints::{filter_hints, transactions_hints};
    use chrono::Local;
    use lib_accounts::{Account, AccountService};

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

    /// The category delete the Shell runs, in its order: check, re-point the Splits, then drop the
    /// budget and the Category.
    fn delete_category(
        categories: &mut Vec<Category>,
        transactions: &mut [Transaction],
        budgets: &mut budgets::Budgets,
        id: u32,
    ) -> (ToastKind, String) {
        match prepare_category_delete(categories, id) {
            Err(refusal) => refusal,
            Ok(uncategorised) => {
                let moved = move_category_splits(transactions, id, uncategorised);
                finish_category_delete(categories, budgets, id, moved)
            }
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
