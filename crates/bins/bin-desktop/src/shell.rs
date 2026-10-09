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
//! and `render_view` in `shell/render.rs` is its plain exhaustive match.
//!
//! The file holds the `Shell` struct, construction, persisted setters, the accessor surface,
//! `impl Focusable`, per-domain wiring and the unit tests. Each cross-cutting concern lives in a
//! bare-named child file (`commands`, `dialogs`, `explorer`, `focus`, `key_dispatch`, `log_feed`,
//! `rail`, `render`, `status`, `toasts`) that is an `impl Shell` block, not the real module of
//! the same name; a `_ui` suffix (`documents_ui`, `inventory_ui`, ...) marks per-domain wiring.
//!
//! `Shell` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

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
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::view::settings::hints::{
    settings_categories_hints, settings_colour_grid_hints, settings_display_hints,
    settings_documents_hints, settings_index_hints, settings_inventory_hints,
    settings_payees_hints, settings_plain_page_hints, settings_tags_hints, settings_tracing_hints,
};
use crate::view::settings::inventory::{self as inventory_view, InventoryRow};
use crate::view::{accounts::hints::accounts_hints, settings::hints::confirm_dialog_hints};

use gpui::{
    Context, FocusHandle, Focusable, Keystroke, ListAlignment, ListState, ScrollHandle,
    ScrollStrategy, UniformListScrollHandle, point, px,
};

use chrono::Local;
use lib_core::{CategoryTypes, DateStyle};
use lib_toast::{ToastKind, Toasts};

use crate::{
    accounts::{
        self, Account, NameLookup,
        form::{AccountField, AccountForm, AccountOptions, AccountsDialog, DeleteAccountForm},
    },
    bills::{self, pay_form::PayForm},
    budgets,
    categories::{self, Category},
    chrome::dialog_host::OpenDialog,
    chrome::state::ChromeState,
    documents::types::DocumentTypeRow,
    documents::{self, DocumentsMode, LibraryScope, LibrarySort},
    form::field::TextField,
    import::{self, ImportState, RowSelect},
    institutions::{self, AccountType, InstitutionRow, form::AddInstitutionForm},
    inventory,
    navigation::active_view::ActiveView,
    navigation::command::AccountsVerb,
    navigation::explorer::{ExplorerFilters, FileExplorer},
    navigation::key_router::{self, Movement},
    navigation::nav::{FocusZone, InputMode, NavState, Noun},
    payees::{self, Payee},
    period::Period,
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
        self, Transaction,
        chips::FilterField,
        filter_form::{FilterForm, FormField, FormOptions, SelectKey as FilterSelectKey},
        query::{Ledger, TransactionFilters},
        rows::DisplayPrefs,
    },
    units::{
        self, PriceSourceRow, UnitKind, UnitRow,
        form::{AddUnitField, DeleteUnitForm, UnitForm},
    },
    view::format,
    view::{
        bills as bills_view,
        budgets::{self as budgets_view},
        dashboard::{self},
        documents::DocumentsFocus,
        import as import_view,
        settings::{self as settings_view},
        transactions as transactions_view,
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

/// `Ctrl-d`/`Ctrl-u` on the Accounts page: half of a typical screenful of rows.
pub(super) const ACCOUNTS_HALF_PAGE: isize = 5;
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
    /// The Accounts page's rows, seeded from `accounts::default_accounts()`. A real, mutable
    /// `Vec` the Add/Edit/Delete dialogs push to, update and remove from -- saved-in-memory
    /// state like [`Self::settings_units`], so it survives leaving and re-entering Accounts.
    accounts: Vec<Account>,
    /// The selected row as a position in `accounts::display_order(&self.accounts)` -- what
    /// `j`/`k` move -- not an index into [`Self::accounts`], since the page shows accounts
    /// grouped by type rather than in insertion order.
    accounts_selected: usize,
    /// "Today" for the seed data and, later, the `this year` filter -- read once at construction so
    /// everything derived from it (the seeded dates, the default range) agrees for the whole run.
    today: chrono::NaiveDate,
    /// The shared stub Categories tree, Payees and Tags. Owned here so the Categories, Payees and
    /// Tags views the later maps build can read and grow the same data the Transactions view uses.
    categories: Vec<Category>,
    /// The shared stub Budgets, seeded from `budgets::default_budgets()`. Owned here so the
    /// Budgets surface and Categories 5c read and write the same Category Limits, and so they
    /// survive leaving and re-entering either view.
    budgets: budgets::Budgets,
    /// The Budgets destination's own view state (Budget, tab, month, cursors).
    budgets_state: budgets::BudgetsState,
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
    /// The Ledger's Document Types in the user's order (mock data until persistence lands).
    document_types: Vec<DocumentTypeRow>,
    settings_documents_selected: Option<u32>,
    /// Settings' Inventory page: the selected row and the Properties shown open (session-only).
    settings_inventory_selected: Option<InventoryRow>,
    settings_inventory_expanded: HashSet<u32>,
    /// The id the next added Document Type takes; only counts up, so ids are never reused.
    document_types_next_id: u32,
    /// The stubbed 6e Import "match payees" step, `Some` while it shows in place of the
    /// Transactions page (`:import`). Dropped on leaving Transactions.
    import: Option<ImportState>,
    tags: Vec<Tag>,
    /// The selected row on the Tags page, a position in `tags::sorted_by_usage`'s order.
    tags_selected: usize,
    /// The selected row on Settings' Tags page, by Tag id: that page lists A–Z, not by usage.
    settings_tags_selected: Option<u32>,
    /// The Transactions view's stub dataset, newest first (`transactions::default_transactions`).
    /// A real, mutable `Vec`, like [`Self::accounts`]: saved-in-memory state that survives leaving
    /// and re-entering the page. Deleting an account deletes its transactions with it.
    transactions: Vec<Transaction>,
    /// The Bills surface's stub Bill Plans and Bill Schedule, seeded by `bills::default_bills`
    /// (which also writes their settling Transactions into [`Self::transactions`]).
    bill_plans: Vec<bills::BillPlan>,
    bill_entries: Vec<bills::BillScheduleEntry>,
    /// Which Bills tab shows, and the selected row on each (a position in that tab's list).
    bills_tab: bills::BillsTab,
    bills_selected: usize,
    /// The Schedule tab's calendar month; starts at today's. Kept while `bills_all` shows every
    /// row, so the arrows return to it.
    bills_period: Period,
    bills_all: bool,
    /// The Schedule tab's filters, and the filter-row select `f` has focused (open or closed).
    bills_filters: bills::history::BillFilters,
    bills_filter_focus: Option<(
        bills_view::filters::FilterField,
        crate::form::select::SelectState,
    )>,
    /// The selected row as a position in [`Self::transactions`] (the table shows them in this
    /// order), clamped wherever it is read. Once filters land it becomes a position in the
    /// filtered list.
    transactions_selected: usize,
    /// The table's `uniform_list` scroll state: scrolls the selected row into view and reports the
    /// viewport height for half-page moves.
    transactions_scroll: UniformListScrollHandle,
    /// What the table is filtered by; starts at the defaults (this year, everything else empty).
    transactions_filters: TransactionFilters,
    /// The search box's text, separate from the filters.
    transactions_search: String,
    /// The filter popover's draft, `Some` while it is open (`NavState::mode` is then
    /// `InputMode::Filter`). Kept apart from [`Self::transactions_filters`] until **apply**.
    transactions_filter_form: Option<FilterForm>,
    /// The chip that opened the popover, which it anchors under.
    transactions_filter_anchor: FilterField,
    /// Where each chip was last painted; the header writes it, the popover reads it.
    transactions_chip_bounds: transactions_view::ChipBounds,
    /// The Documents surface's stub Documents, and the Inventory their Links resolve against.
    documents: Vec<documents::Document>,
    inventory: inventory::Inventory,
    /// Inbox or Library, the Library's scope and sort. These three persist across restarts.
    documents_mode: DocumentsMode,
    documents_scope: LibraryScope,
    documents_sort: LibrarySort,
    /// The Library's search text: session-only, and kept across an `i` round-trip.
    documents_query: String,
    /// Index rail or list, while the View zone has focus.
    documents_focus: DocumentsFocus,
    /// The selected Library row, a position in the listed rows; clamped wherever it is read.
    documents_selected: usize,
    documents_scroll: ScrollHandle,
    /// The focused Inbox row, a position in the Inbox's rows; clamped wherever it is read. Kept
    /// apart from the Library's so an `i` round-trip returns to both.
    documents_inbox_selected: usize,
    /// The last filing action as a unit, for `u`. Session-only; a new filing action replaces it.
    documents_undo: Option<documents::FilingUndo>,
    /// A file to hand to the OS, taken by the key-down listener, which has the `App` it needs.
    pending_file_action: Option<documents_ui::FileAction>,
}

impl Shell {
    pub fn new(nav: NavState, focus_handle: FocusHandle) -> Self {
        Self::with_today(nav, focus_handle, Local::now().date_naive())
    }

    /// As [`Self::new`], with the date the seeded stub data is anchored to supplied, so tests
    /// are not at the mercy of the wall clock.
    pub fn with_today(nav: NavState, focus_handle: FocusHandle, today: chrono::NaiveDate) -> Self {
        let seeded_accounts = accounts::default_accounts();
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
        let seeded_budgets = budgets::default_budgets(&seeded_accounts, &categories, today);
        let budgets_current = seeded_budgets
            .default_budget()
            .map_or(budgets::PERSONAL_SPENDING_ID, |budget| budget.id);
        let document_types_seed = documents::types::default_types();
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
            accounts: accounts::default_accounts(),
            accounts_selected: 0,
            today,
            categories,
            budgets: seeded_budgets,
            budgets_state: budgets::BudgetsState::new(budgets_current, today),
            categories_selected: 0,
            categories_selected_id: None,
            categories_expanded: vec![1, 3, 6], // Housing, Utilities, Food expanded by default
            payees,
            payees_selected: 0,
            settings_payees_selected: None,
            document_types_next_id: documents::types::first_free_id(&document_types_seed),
            document_types: document_types_seed,
            settings_documents_selected: None,
            settings_inventory_selected: None,
            settings_inventory_expanded: HashSet::new(),
            import: None,
            tags,
            tags_selected: 0,
            settings_tags_selected: None,
            transactions,
            bill_plans: bills_seed.plans,
            bill_entries: bills_seed.entries,
            bills_tab: bills::BillsTab::default(),
            bills_selected: 0,
            bills_period: Period::of(today),
            bills_all: false,
            bills_filters: bills::history::BillFilters::default(),
            bills_filter_focus: None,
            transactions_selected: 0,
            transactions_scroll: UniformListScrollHandle::new(),
            transactions_filters: TransactionFilters::defaults(today),
            transactions_search: String::new(),
            transactions_filter_form: None,
            transactions_filter_anchor: FilterField::Account,
            transactions_chip_bounds: Default::default(),
            documents: documents_seed.documents,
            inventory,
            documents_mode: DocumentsMode::default(),
            documents_scope: LibraryScope::default(),
            documents_sort: LibrarySort::default(),
            documents_query: String::new(),
            documents_focus: DocumentsFocus::default(),
            documents_selected: 0,
            documents_scroll: documents_ui::new_scroll(),
            documents_inbox_selected: 0,
            documents_undo: None,
            pending_file_action: None,
        }
    }

    /// Restores the Documents mode, Library scope and sort from the last run.
    pub fn set_documents_state(
        &mut self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
    ) {
        self.set_documents_persisted(mode, scope, sort);
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
    fn open_settings_page(&mut self, section: SettingsSection) {
        let noun_before = self.nav.noun();
        self.nav.set_noun(Noun::Settings);
        if noun_before != Noun::Settings {
            self.reset_view_scroll();
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

    /// The reference data the Transactions engine reads, borrowed from the shared stubs.
    fn transactions_ledger(&self) -> Ledger<'_> {
        Ledger {
            accounts: &self.accounts,
            categories: &self.categories,
            payees: &self.payees,
            tags: &self.tags,
        }
    }

    /// The Display preferences the table's text is formatted with.
    fn transactions_prefs(&self) -> DisplayPrefs {
        DisplayPrefs {
            date_style: self.settings_date_style,
            glyphs: self.settings_status_glyphs,
        }
    }

    /// How many rows the current filters and search leave visible.
    fn transactions_visible_len(&self) -> usize {
        transactions::query::query(
            &self.transactions_ledger(),
            &self.transactions,
            &self.transactions_filters,
            &self.transactions_search,
        )
        .rows
        .len()
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` move the table's row selection and keep it in view;
    /// `Enter` would open the transaction, which the bundle designs no screen for.
    ///
    /// The scroll strategy follows the research note: a non-strict `scroll_to_item` applies its
    /// strategy only when the row was off-screen, so stepping down uses `Bottom` and stepping up
    /// `Top` (the new row lands at the edge it came from), and jumps use `Center`.
    fn apply_transactions_movement(&mut self, movement: Movement) {
        let len = self.transactions_visible_len();
        if len == 0 {
            return;
        }
        let selected = transactions::rows::clamp_selection(self.transactions_selected, len);
        let last = len - 1;
        let viewport = f32::from(
            self.transactions_scroll
                .0
                .borrow()
                .base_handle
                .bounds()
                .size
                .height,
        );
        let half = transactions::rows::half_page_rows(
            viewport,
            format::row_height_px(self.settings_row_density),
        );
        let (next, strategy) = match movement {
            Movement::Next => ((selected + 1).min(last), ScrollStrategy::Bottom),
            Movement::Prev => (selected.saturating_sub(1), ScrollStrategy::Top),
            Movement::First => (0, ScrollStrategy::Top),
            Movement::Last => (last, ScrollStrategy::Bottom),
            Movement::HalfPageDown => ((selected + half).min(last), ScrollStrategy::Center),
            Movement::HalfPageUp => (selected.saturating_sub(half), ScrollStrategy::Center),
            Movement::Enter => {
                self.chrome.status_message =
                    Some(crate::msg::desktop_status_open_transaction_not_yet_built());
                return;
            }
        };
        self.transactions_selected = next;
        self.transactions_scroll.scroll_to_item(next, strategy);
    }

    /// The Transactions page's `n` and `e` (only while it is the active noun and the view has focus,
    /// in `Normal` mode): the bundle designs no add or edit flow, so both say so.
    fn handle_transactions_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.noun() != Noun::Transactions || self.nav.focus() != FocusZone::View {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        if keystroke.key == "f" {
            self.open_filter_popover(None);
            return true;
        }
        let message = match keystroke.key.as_str() {
            "n" => crate::msg::desktop_status_add_transaction_not_yet_built(),
            "e" => crate::msg::desktop_status_edit_transaction_not_yet_built(),
            _ => return false,
        };
        self.chrome.status_message = Some(message);
        true
    }

    /// Puts the table back on its first row: called whenever the filters or the search change what
    /// is visible, since the old selection no longer points at the same row.
    fn reset_transactions_selection(&mut self) {
        self.transactions_selected = 0;
        self.transactions_scroll
            .scroll_to_item(0, ScrollStrategy::Top);
    }

    /// Keys while `InputMode::Search` is active on the Transactions page: typing filters live,
    /// `Backspace` edits, `Enter` keeps the text and returns to browsing the (filtered) rows, and
    /// `Esc` (handled with the other modes' exit) clears it.
    fn handle_transactions_search_key(&mut self, keystroke: &Keystroke) -> bool {
        match keystroke.key.as_str() {
            "enter" => {
                self.nav.exit_mode();
                true
            }
            "backspace" => {
                self.transactions_search.pop();
                self.reset_transactions_selection();
                true
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                match keystroke.key_char.as_deref() {
                    Some(text)
                        if text.chars().count() == 1
                            && text.chars().next().is_some_and(|ch| !ch.is_control()) =>
                    {
                        self.transactions_search.push_str(text);
                        self.reset_transactions_selection();
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// A click on a filter chip (or its `▾`): opens the popover on that chip's field.
    fn handle_transactions_chip_click(&mut self, field: FilterField, cx: &mut Context<'_, Self>) {
        self.open_filter_popover(Some(field));
        cx.notify();
    }

    /// The Account and Category selects' options, read live from the stub data.
    fn filter_form_options(&self) -> FormOptions {
        FormOptions::new(&self.accounts, &self.categories, &self.payees, &self.tags)
    }

    /// Opens the filter popover on a draft of the applied filters. A chip focuses its own field
    /// (the date chip focuses From) and the card anchors under it; `f` (no chip) focuses the first
    /// field and anchors under the first chip.
    fn open_filter_popover(&mut self, chip: Option<FilterField>) {
        let options = self.filter_form_options();
        let mut form = FilterForm::from_filters(
            &self.transactions_filters,
            &options,
            self.today,
            self.settings_date_style,
        );
        form.focused = chip.map(FormField::for_chip).unwrap_or_default();
        self.transactions_filter_anchor = chip.unwrap_or(FilterField::Account);
        self.transactions_filter_form = Some(form);
        self.nav.enter_mode(InputMode::Filter);
    }

    /// Keys while the filter popover is open. `Tab` / `Shift-Tab` move between fields; a focused
    /// select takes `Up` / `Down` / `Enter` / `Space` as the shared dropdown does; the Status control
    /// steps with `Left` / `Right` (or `Space`); text fields take typing and `Backspace`; `Enter`
    /// applies (from a select it opens or commits the list instead, so `Tab` off it first); `Ctrl-r`
    /// resets the draft. `Esc` never reaches here: it is handled with the other modes' exit.
    fn handle_filter_key(&mut self, keystroke: &Keystroke) -> bool {
        let options = self.filter_form_options();
        let (today, date_style) = (self.today, self.settings_date_style);
        let Some(form) = self.transactions_filter_form.as_mut() else {
            return false;
        };
        let modifiers = &keystroke.modifiers;
        let mut apply = false;

        if modifiers.control && keystroke.key == "r" {
            form.reset(&options, today, date_style);
            return true;
        }
        match keystroke.key.as_str() {
            "tab" => form.cycle_focus(modifiers.shift, &options),
            "up" => {
                form.handle_select_key(FilterSelectKey::Up, &options);
            }
            "down" => {
                form.handle_select_key(FilterSelectKey::Down, &options);
            }
            "left" if form.focused == FormField::Status => form.step_status(-1),
            "right" if form.focused == FormField::Status => form.step_status(1),
            "space" if form.focused.is_select() => {
                form.handle_select_key(FilterSelectKey::Activate, &options);
            }
            "space" if form.focused == FormField::Status => form.step_status(1),
            "enter" if form.focused.is_select() => {
                form.handle_select_key(FilterSelectKey::Activate, &options);
            }
            "enter" => apply = true,
            "backspace" => form.backspace(),
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(text) = keystroke.key_char.as_deref()
                    && text.chars().count() == 1
                    && let Some(ch) = text.chars().next()
                {
                    form.push_char(ch);
                }
            }
        }
        if apply {
            self.apply_filter_form();
        }
        true
    }

    /// **apply**: commits the draft to the applied filters, closes the popover and puts the table
    /// back on its first row. A no-op while a date does not parse.
    fn apply_filter_form(&mut self) {
        let options = self.filter_form_options();
        let Some(filters) = self
            .transactions_filter_form
            .as_ref()
            .and_then(|form| form.to_filters(&options, self.today, self.settings_date_style))
        else {
            return;
        };
        self.transactions_filters = filters;
        self.transactions_filter_form = None;
        self.nav.exit_mode();
        self.reset_transactions_selection();
    }

    /// A click on a popover field: a text field takes focus; a select takes focus and toggles its
    /// list.
    fn handle_filter_field_click(&mut self, field: FormField, cx: &mut Context<'_, Self>) {
        let options = self.filter_form_options();
        if let Some(form) = self.transactions_filter_form.as_mut() {
            if field.is_select() {
                form.click_select(field, &options);
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    fn handle_filter_option_click(
        &mut self,
        field: FormField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.filter_form_options();
        if let Some(form) = self.transactions_filter_form.as_mut() {
            form.choose_option(field, index, &options);
        }
        cx.notify();
    }

    fn handle_filter_status_click(
        &mut self,
        status: transactions::query::StatusFilter,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.transactions_filter_form.as_mut() {
            form.focus(FormField::Status);
            form.status = status;
        }
        cx.notify();
    }

    /// `reset`: the draft back to the defaults; the applied filters are untouched.
    fn handle_filter_reset(&mut self, cx: &mut Context<'_, Self>) {
        let options = self.filter_form_options();
        let (today, date_style) = (self.today, self.settings_date_style);
        if let Some(form) = self.transactions_filter_form.as_mut() {
            form.reset(&options, today, date_style);
        }
        cx.notify();
    }

    fn handle_filter_apply(&mut self, cx: &mut Context<'_, Self>) {
        self.apply_filter_form();
        cx.notify();
    }

    /// A click outside the card: discards the draft, like `Esc`.
    fn handle_filter_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.transactions_filter_form = None;
        self.nav.exit_mode();
        cx.notify();
    }

    /// The `✕` on an accent chip: resets just that filter.
    fn handle_transactions_chip_clear(&mut self, field: FilterField, cx: &mut Context<'_, Self>) {
        transactions::chips::clear_field(&mut self.transactions_filters, field, self.today);
        self.reset_transactions_selection();
        cx.notify();
    }

    /// `clear filters`: every filter back to its default. The search text is separate state and is
    /// left alone.
    fn handle_transactions_clear_all(&mut self, cx: &mut Context<'_, Self>) {
        self.transactions_filters = TransactionFilters::defaults(self.today);
        self.reset_transactions_selection();
        cx.notify();
    }

    /// The header's **add transaction** button: the same message `n` gives.
    fn handle_transactions_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message =
            Some(crate::msg::desktop_status_add_transaction_not_yet_built());
        cx.notify();
    }

    /// A click on the search box: the same as pressing `/`.
    fn handle_transactions_search_click(&mut self, cx: &mut Context<'_, Self>) {
        self.nav.enter_mode(InputMode::Search);
        cx.notify();
    }

    /// A click on a table row selects it.
    fn handle_transactions_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.transactions_selected = index;
        cx.notify();
    }

    /// Whether the Accounts rows own the keyboard: Settings' Accounts page with focus in the page
    /// rather than on the index.
    fn accounts_page_has_focus(&self) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_focus == SettingsFocus::Page
            && self.settings_selected_section == SettingsSection::Accounts
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
    fn settings_documents_selected_id(&self) -> Option<u32> {
        self.settings_documents_selected
            .filter(|id| documents::types::position(&self.document_types, *id).is_some())
            .or_else(|| self.document_types.first().map(|row| row.id))
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the types in the user's order.
    fn apply_settings_documents_movement(&mut self, movement: Movement) {
        let len = self.document_types.len();
        let current = self
            .settings_documents_selected_id()
            .and_then(|id| documents::types::position(&self.document_types, id))
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
        self.settings_documents_selected = self.document_types.get(next).map(|row| row.id);
    }

    /// `J`/`K` on the Documents page move the selected type a place, which is also its place in
    /// the Documents Type filter.
    fn handle_settings_documents_reorder_key(&mut self, keystroke: &Keystroke) -> bool {
        if !self.settings_documents_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || !modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "j" => self.move_selected_document_type(1),
            "k" => self.move_selected_document_type(-1),
            _ => return false,
        }
        true
    }

    /// The Documents page's own `n`/`e`/`x`, which open the Add, Edit and Remove dialogs.
    fn handle_settings_documents_key(&mut self, keystroke: &Keystroke) -> bool {
        if !self.settings_documents_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        match (keystroke.key.as_str(), modifiers.shift) {
            ("x", false) => self.remove_selected_document_type(),
            ("n", false) => self.open_add_document_type_dialog(),
            ("e", false) => {
                if let Some(id) = self.settings_documents_selected_id() {
                    self.open_edit_document_type_dialog(id);
                }
            }
            _ => return false,
        }
        true
    }

    fn move_selected_document_type(&mut self, delta: isize) {
        if let Some(id) = self.settings_documents_selected_id() {
            documents::types::move_by(&mut self.document_types, id, delta);
            self.settings_documents_selected = Some(id);
        }
    }

    /// The Remove action. Other, the Default, is never removed: it gets a notice dialog and says
    /// so on the status line.
    fn remove_selected_document_type(&mut self) {
        let Some(id) = self.settings_documents_selected_id() else {
            return;
        };
        let is_default = documents::types::position(&self.document_types, id)
            .and_then(|position| self.document_types.get(position))
            .is_some_and(|row| row.is_default);
        if is_default {
            self.chrome.status_message =
                Some(crate::msg::desktop_document_types_hint_default_kept());
        }
        self.open_remove_document_type_dialog(id);
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
    fn handle_settings_inventory_key(&mut self, keystroke: &Keystroke) -> bool {
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
                    self.open_remove_inventory_row(row);
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

    /// The selected account's index in [`Self::accounts`], `None` when there are none. The stored
    /// position is clamped, so removing accounts can never leave it pointing past the end.
    fn selected_account_index(&self) -> Option<usize> {
        let order = accounts::display_order(&self.accounts);
        order
            .get(self.accounts_selected.min(order.len().saturating_sub(1)))
            .copied()
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the Accounts page's row selection instead of
    /// scrolling it; `Enter` opens the account's ledger (Transactions filtered to it).
    fn apply_accounts_movement(&mut self, movement: Movement) {
        let len = self.accounts.len();
        let selected = self.accounts_selected;
        self.accounts_selected = match movement {
            Movement::Next => accounts::step_selection(selected, len, 1),
            Movement::Prev => accounts::step_selection(selected, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => accounts::step_selection(selected, len, ACCOUNTS_HALF_PAGE),
            Movement::HalfPageUp => accounts::step_selection(selected, len, -ACCOUNTS_HALF_PAGE),
            Movement::Enter => {
                if let Some(id) = self
                    .selected_account_index()
                    .and_then(|index| self.accounts.get(index))
                    .map(|account| account.id)
                {
                    self.open_account_ledger(id);
                }
                selected
            }
        };
    }

    /// The Accounts page's own `n`/`e`/`d` (only while it is the active noun and the view has
    /// focus, in `Normal` mode -- `route_key` hands back `NoOp` for these bare keys). Each goes
    /// through the same handler its button does.
    fn handle_accounts_key(&mut self, keystroke: &Keystroke) -> bool {
        if !self.accounts_page_has_focus() {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        let selected_id = self
            .selected_account_index()
            .and_then(|index| self.accounts.get(index))
            .map(|account| account.id);
        match keystroke.key.as_str() {
            "n" => self.open_add_account_dialog(""),
            "e" => {
                if let Some(id) = selected_id {
                    self.open_edit_account_dialog(id);
                }
            }
            "d" => {
                if let Some(id) = selected_id {
                    self.open_delete_account_dialog(id);
                }
            }
            _ => return false,
        }
        true
    }

    /// Keyboard input while on the Categories page: `n` adds a top-level category, `N` (shift+n)
    /// adds a sub-category to the selected one, `e` edits the selected category, `d` deletes it,
    /// `enter` opens Transactions filtered to the selected category.
    fn handle_categories_key(&mut self, keystroke: &Keystroke) -> bool {
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
                        self.open_add_categories_dialog(Some(category.id));
                    }
                } else {
                    // n: add top-level category
                    self.open_add_categories_dialog(None);
                }
                true
            }
            "e" => {
                if let Some(category) = selected_category {
                    self.open_edit_categories_dialog(category.id);
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
                    self.open_category_transactions(category.id);
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
    ) -> Option<categories::form::BudgetLock> {
        if category_id.is_some_and(|id| !categories::is_leaf(&self.categories, id)) {
            return Some(categories::form::BudgetLock::Parent);
        }
        self.budgets
            .get(budgets::PERSONAL_SPENDING_ID)
            .filter(|budget| budget.is_archived())
            .map(|budget| categories::form::BudgetLock::Archived(budget.name.clone()))
    }

    /// Opens the Add categories dialog pre-scoped to parent_id (None for top-level).
    fn open_add_categories_dialog(&mut self, parent_id: Option<u32>) {
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
            budget_lock: self.categories_budget_lock(None),
            focused: categories::form::CategoryField::Name,
        };
        self.open_dialog(OpenDialog::Categories(
            categories::form::CategoriesDialog::Add { parent_id, form },
        ));
    }

    /// Opens the Edit categories dialog on `id`, pre-filled. Monthly budget shows the current
    /// month's amount in the Personal spending Budget (a parent's is its children's sum).
    fn open_edit_categories_dialog(&mut self, category_id: u32) {
        if let Some(category) = self.categories.iter().find(|c| c.id == category_id) {
            let budget_str = self
                .budgets
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
                budget_lock: self.categories_budget_lock(Some(category_id)),
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
        let _ = self.budgets.set_monthly_limit(
            budgets::PERSONAL_SPENDING_ID,
            &self.categories,
            category_id,
            amount,
            self.today,
        );
    }

    /// Opens Transactions pre-filtered to the account `id`: fresh defaults plus that account, the
    /// search cleared and the table back on its first row. The Accounts selection is untouched, so
    /// returning to Accounts finds the same row selected.
    fn open_account_ledger(&mut self, id: u32) {
        self.transactions_filters = TransactionFilters::for_account(self.today, id);
        self.transactions_search.clear();
        self.transactions_filter_form = None;
        self.reset_transactions_selection();
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll();
    }

    fn open_category_transactions(&mut self, id: u32) {
        self.transactions_filters = TransactionFilters::for_category(self.today, id);
        self.transactions_search.clear();
        self.transactions_filter_form = None;
        self.reset_transactions_selection();
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll();
    }

    /// "View transactions" on the Payees page: Transactions filtered to exactly this Payee's id, not
    /// a name substring that would over-match ("BP").
    fn open_payee_transactions(&mut self, id: u32) {
        self.transactions_filters = TransactionFilters::for_payee(self.today, id);
        self.transactions_search.clear();
        self.transactions_filter_form = None;
        self.reset_transactions_selection();
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll();
    }

    /// "View transactions" on the Tags page: Transactions filtered to exactly this Tag's id, not a
    /// name substring that would over-match ("trip").
    fn open_tag_transactions(&mut self, id: u32) {
        self.transactions_filters = TransactionFilters::for_tag(self.today, id);
        self.transactions_search.clear();
        self.transactions_filter_form = None;
        self.reset_transactions_selection();
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll();
    }

    /// The selected Tag: `tags_selected` indexes the page's usage order, not `self.tags`.
    fn selected_tag_id(&self) -> Option<u32> {
        if self.settings_tags_page_has_focus() {
            return self.settings_tags_selected_id();
        }
        let sorted = tags::sorted_by_usage(&self.tags, &self.transactions);
        sorted
            .get(self.tags_selected.min(sorted.len().saturating_sub(1)))
            .map(|tag| tag.id)
    }

    /// Selects the Tag with `id`, if it still exists.
    fn select_tag(&mut self, id: u32) {
        self.settings_tags_selected = Some(id);
        if let Some(index) = tags::sorted_by_usage(&self.tags, &self.transactions)
            .iter()
            .position(|tag| tag.id == id)
        {
            self.tags_selected = index;
        }
    }

    /// The Tags page's own `n`/`e`/`x`/`m` (only while it is the active noun and the view has
    /// focus, in `Normal` mode).
    fn handle_tags_key(&mut self, keystroke: &Keystroke) -> bool {
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
                if let Some(id) = self.selected_tag_id() {
                    self.open_edit_tag_dialog(id);
                }
            }
            "x" => {
                if let Some(id) = self.selected_tag_id() {
                    self.open_remove_tag_dialog(id);
                }
            }
            "m" => {
                if let Some(id) = self.selected_tag_id() {
                    self.open_merge_tags_dialog(Some(id));
                }
            }
            _ => return false,
        }
        true
    }

    /// The Schedule tab's rows for the shown period (or All), before its filters.
    fn bills_unfiltered_rows(&self) -> Vec<bills::ScheduleRow> {
        bills::schedule_rows(
            &self.bill_plans,
            &self.bill_entries,
            (!self.bills_all).then_some(self.bills_period),
            self.today,
        )
    }

    /// The Schedule tab's rows as shown: the period's (or All's), through its filters.
    fn bills_schedule_rows(&self) -> Vec<bills::ScheduleRow> {
        self.bills_filters
            .apply(&self.bills_unfiltered_rows(), &self.bill_plans)
    }

    /// The selected Schedule row, its stored position clamped to the rows now shown.
    fn selected_bill_row(&self) -> Option<bills::ScheduleRow> {
        let rows = self.bills_schedule_rows();
        rows.get(self.bills_selected.min(rows.len().saturating_sub(1)))
            .copied()
    }

    /// The Planner tab's selected Bill Plan's id, its stored position clamped to the Plans.
    fn selected_bill_plan(&self) -> Option<u32> {
        let plans = bills::planner_order(&self.bill_plans);
        plans
            .get(self.bills_selected.min(plans.len().saturating_sub(1)))
            .map(|plan| plan.id)
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the active Bills tab's row selection; `Enter` on a
    /// Paid Schedule row opens its Transaction, and on a Planner row edits its Bill Plan. While `f`
    /// has a Schedule filter select focused, they drive the select instead.
    fn apply_bills_movement(&mut self, movement: Movement) {
        if self.bills_tab == bills::BillsTab::Schedule && self.bills_filter_focus.is_some() {
            self.apply_bills_filter_select_movement(movement);
            return;
        }
        let len = match self.bills_tab {
            bills::BillsTab::Schedule => self.bills_schedule_rows().len(),
            bills::BillsTab::Planner => self.bill_plans.len(),
        };
        let selected = self.bills_selected.min(len.saturating_sub(1));
        self.bills_selected = match movement {
            Movement::Next => accounts::step_selection(selected, len, 1),
            Movement::Prev => accounts::step_selection(selected, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => accounts::step_selection(selected, len, ACCOUNTS_HALF_PAGE),
            Movement::HalfPageUp => accounts::step_selection(selected, len, -ACCOUNTS_HALF_PAGE),
            Movement::Enter => {
                match self.bills_tab {
                    bills::BillsTab::Planner => {
                        if let Some(id) = self.selected_bill_plan() {
                            self.open_edit_bill_plan_dialog(id);
                        }
                    }
                    bills::BillsTab::Schedule => {
                        if let Some(row) = self.selected_bill_row() {
                            self.open_bill_transaction(row.id);
                        }
                    }
                }
                selected
            }
        };
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

    /// `tab` on the Bills page switches its tab (the handoff's `tab switch view`) rather than
    /// cycling focus; `shift-tab` still cycles focus, so the View zone can always be left. Runs
    /// before the router, like the Colour Theme grid's own `tab`.
    fn handle_bills_tab_key(&mut self, keystroke: &Keystroke) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        let modifiers = &keystroke.modifiers;
        if keystroke.key != "tab"
            || modifiers.shift
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
            || pending_g_active
            || self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Bills
            || self.nav.focus() != FocusZone::View
        {
            return false;
        }
        self.set_bills_tab(self.bills_tab.next());
        true
    }

    fn set_bills_tab(&mut self, tab: bills::BillsTab) {
        self.bills_tab = tab;
        self.bills_selected = 0;
        self.bills_filter_focus = None;
        self.chrome.status_message = None;
        self.reset_view_scroll();
    }

    /// Steps the Schedule tab's period a calendar month; from All, returns to the month last viewed.
    fn shift_bills_period(&mut self, forward: bool) {
        if std::mem::take(&mut self.bills_all) {
            self.bills_selected = 0;
            return;
        }
        self.bills_period = if forward {
            self.bills_period.next()
        } else {
            self.bills_period.prev()
        };
        self.bills_selected = 0;
    }

    /// `0` toggles the Schedule between its month and All.
    fn toggle_bills_all(&mut self) {
        self.bills_all = !self.bills_all;
        self.bills_selected = 0;
    }

    fn toggle_bills_filter_chip(&mut self, index: usize) {
        if let Some(status) = bills::history::STATUS_CHIPS.get(index) {
            self.bills_filters.toggle(*status);
            self.bills_selected = 0;
        }
    }

    /// A Schedule filter select's options, and the index of its current value. Category and
    /// Account list only those some Bill Plan uses.
    fn bills_filter_options(
        &self,
        field: bills_view::filters::FilterField,
    ) -> (Vec<String>, usize) {
        use bills_view::filters::FilterField;
        let filters = &self.bills_filters;
        let scoped = |all: String, choices: Vec<(u32, String)>, current: Option<u32>| {
            let index = current
                .and_then(|id| choices.iter().position(|(choice, _)| *choice == id))
                .map_or(0, |position| position + 1);
            let labels = std::iter::once(all)
                .chain(choices.into_iter().map(|(_, name)| name))
                .collect();
            (labels, index)
        };
        match field {
            FilterField::Plan => scoped(
                crate::msg::desktop_bills_filter_all_bills(),
                self.bills_filter_choices(field),
                filters.plan_id,
            ),
            FilterField::Category => scoped(
                crate::msg::desktop_bills_filter_all_categories(),
                self.bills_filter_choices(field),
                filters.category_id,
            ),
            FilterField::Account => scoped(
                crate::msg::desktop_bills_filter_all_accounts(),
                self.bills_filter_choices(field),
                filters.account_id,
            ),
        }
    }

    /// The Bill Plan, Category or Account choices behind a scope select (after its "All" option),
    /// as `(id, name)`.
    fn bills_filter_choices(&self, field: bills_view::filters::FilterField) -> Vec<(u32, String)> {
        use bills_view::filters::FilterField;
        let mut choices: Vec<(u32, String)> = match field {
            FilterField::Plan => {
                return bills::planner_order(&self.bill_plans)
                    .into_iter()
                    .map(|plan| (plan.id, plan.name.clone()))
                    .collect();
            }
            FilterField::Category => self
                .categories
                .iter()
                .filter(|c| self.bill_plans.iter().any(|p| p.category_id == c.id))
                .map(|c| (c.id, c.name.clone()))
                .collect(),
            FilterField::Account => self
                .accounts
                .iter()
                .filter(|a| self.bill_plans.iter().any(|p| p.account_id == a.id))
                .map(|a| (a.id, a.name.clone()))
                .collect(),
        };
        choices.sort_by_key(|(_, name)| name.to_lowercase());
        choices
    }

    /// Sets a Schedule filter from its select's option `index`.
    fn apply_bills_filter_option(&mut self, field: bills_view::filters::FilterField, index: usize) {
        use bills_view::filters::FilterField;
        let id = index
            .checked_sub(1)
            .and_then(|i| self.bills_filter_choices(field).get(i).map(|(id, _)| *id));
        let filters = &mut self.bills_filters;
        match field {
            FilterField::Plan => filters.plan_id = id,
            FilterField::Category => filters.category_id = id,
            FilterField::Account => filters.account_id = id,
        }
        self.bills_selected = 0;
    }

    /// A closed select state on a field's current value.
    fn bills_filter_select_state(
        &self,
        field: bills_view::filters::FilterField,
    ) -> crate::form::select::SelectState {
        let (options, index) = self.bills_filter_options(field);
        crate::form::select::SelectState::new(options.get(index).cloned())
    }

    /// `f` steps focus along the filter row's selects, then off it.
    fn cycle_bills_filter_focus(&mut self) {
        use bills_view::filters::FilterField;
        let next = match self.bills_filter_focus.as_ref().map(|(field, _)| *field) {
            None => Some(FilterField::Plan),
            Some(field) => FilterField::ORDER
                .iter()
                .position(|f| *f == field)
                .and_then(|i| FilterField::ORDER.get(i + 1))
                .copied(),
        };
        self.bills_filter_focus = next.map(|field| (field, self.bills_filter_select_state(field)));
    }

    /// `j`/`k` on a focused select: step its value while closed (applying it at once), move the
    /// highlight while open; `Enter` opens it, or commits the highlight.
    fn apply_bills_filter_select_movement(&mut self, movement: Movement) {
        let Some((field, mut state)) = self.bills_filter_focus.take() else {
            return;
        };
        let (options, _) = self.bills_filter_options(field);
        let delta = match movement {
            Movement::Next => Some(1),
            Movement::Prev => Some(-1),
            _ => None,
        };
        let mut chosen = false;
        match (movement, delta) {
            (_, Some(delta)) if state.is_open() => state.move_highlight(&options, delta),
            (_, Some(delta)) => {
                state.step(&options, delta);
                chosen = true;
            }
            (Movement::Enter, _) if state.is_open() => {
                state.commit(&options);
                chosen = true;
            }
            (Movement::Enter, _) => state.open(&options),
            _ => {}
        }
        if chosen
            && let Some(index) = state
                .value()
                .and_then(|value| options.iter().position(|option| option == value))
        {
            self.apply_bills_filter_option(field, index);
        }
        self.bills_filter_focus = Some((field, state));
    }

    fn handle_bills_filter_chip_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.toggle_bills_filter_chip(index);
        cx.notify();
    }

    /// Clicking a select opens its list (closing any other), or closes its own open list.
    fn handle_bills_filter_field_click(
        &mut self,
        field: bills_view::filters::FilterField,
        cx: &mut Context<'_, Self>,
    ) {
        let open_here = self
            .bills_filter_focus
            .as_ref()
            .is_some_and(|(focused, state)| *focused == field && state.is_open());
        let mut state = self.bills_filter_select_state(field);
        if !open_here {
            let (options, _) = self.bills_filter_options(field);
            state.open(&options);
        }
        self.bills_filter_focus = Some((field, state));
        cx.notify();
    }

    fn handle_bills_filter_option_click(
        &mut self,
        field: bills_view::filters::FilterField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.apply_bills_filter_option(field, index);
        self.bills_filter_focus = Some((field, self.bills_filter_select_state(field)));
        cx.notify();
    }

    /// The Bills page's own `p`/`s`/`e`/`n`/`f`/`[`/`]`/`0`/`1`–`5` (only while it is the active noun and the view has
    /// focus, in `Normal` mode).
    fn handle_bills_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.noun() != Noun::Bills || self.nav.focus() != FocusZone::View {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        let schedule = self.bills_tab == bills::BillsTab::Schedule;
        let planner = self.bills_tab == bills::BillsTab::Planner;
        if schedule
            && let Some(chip) = keystroke
                .key
                .parse::<usize>()
                .ok()
                .and_then(|digit| digit.checked_sub(1))
                .filter(|index| *index < bills::history::STATUS_CHIPS.len())
        {
            self.toggle_bills_filter_chip(chip);
            return true;
        }
        match keystroke.key.as_str() {
            "n" if !modifiers.shift => self.open_add_bill_plan_dialog(),
            "f" if schedule && !modifiers.shift => self.cycle_bills_filter_focus(),
            "0" if schedule => self.toggle_bills_all(),
            "e" if planner && !modifiers.shift => {
                if let Some(id) = self.selected_bill_plan() {
                    self.open_edit_bill_plan_dialog(id);
                }
            }
            "p" if schedule && !modifiers.shift => {
                if let Some(row) = self.selected_bill_row() {
                    self.open_pay_bill_dialog(row);
                }
            }
            "s" if schedule && !modifiers.shift => {
                if let Some(row) = self.selected_bill_row() {
                    self.open_skip_bill_dialog(row);
                }
            }
            "[" if schedule => self.shift_bills_period(false),
            "]" if schedule => self.shift_bills_period(true),
            _ => return false,
        }
        true
    }

    /// What the Add and Edit bill plan selects choose from, copied into the form as it opens. On
    /// Edit the Plan's own Payee stays listed even if it has since been deactivated.
    fn bill_plan_source(&self, keep_payee: Option<u32>) -> bills::form::BillPlanSource {
        bills::form::BillPlanSource::new(
            &self.categories,
            &self.accounts,
            &self.payees,
            keep_payee,
            crate::msg::desktop_payees_category_none(),
            bills_view::planner::recurrence_label,
            self.today,
            self.settings_date_style,
        )
    }

    fn open_add_bill_plan_dialog(&mut self) {
        let form = bills::form::BillPlanForm::new(self.bill_plan_source(None));
        self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Add(form))));
    }

    /// Opens the Edit dialog pre-filled from Bill Plan `id`.
    fn open_edit_bill_plan_dialog(&mut self, id: u32) {
        let Some(plan) = bills::get(&self.bill_plans, id) else {
            return;
        };
        let form = bills::form::BillPlanForm::from_plan(plan, self.bill_plan_source(plan.payee_id));
        self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Edit(
            id, form,
        ))));
    }

    /// The form behind the open Add or Edit bill plan dialog, if that is what's open.
    fn bill_plan_form_mut(&mut self) -> Option<&mut bills::form::BillPlanForm> {
        self.bills_dialog_mut()
            .and_then(bills::BillsDialog::plan_form_mut)
    }

    /// Applies a confirmed Bills dialog (`Enter` and the confirm button).
    fn apply_bills_dialog(&mut self, dialog: bills::BillsDialog) {
        match dialog {
            bills::BillsDialog::Add(form) => self.apply_bill_plan(None, form),
            bills::BillsDialog::Edit(id, form) => self.apply_bill_plan(Some(id), form),
            bills::BillsDialog::Pay(form) => self.apply_pay_bill(form),
            bills::BillsDialog::Skip(entry) => self.apply_skip_bill(entry),
        }
    }

    /// **Add bill plan** / **Save**: adds or edits the Plan (Schedule regeneration is
    /// `bills::insert_plan`'s, `edit_plan`'s and `set_active`'s) and selects it on the Planner
    /// tab. A refused Save reopens the dialog with the error shown.
    fn apply_bill_plan(&mut self, editing: Option<u32>, mut form: bills::form::BillPlanForm) {
        let Some(draft) = form.draft() else {
            return;
        };
        let (today, is_active) = (self.today, form.is_active);
        let result = match editing {
            None => bills::insert_plan(
                &mut self.bill_plans,
                &mut self.bill_entries,
                &draft,
                &self.categories,
                &self.accounts,
                today,
            ),
            Some(id) => bills::edit_plan(
                &mut self.bill_plans,
                &mut self.bill_entries,
                id,
                &draft,
                &self.categories,
                &self.accounts,
                today,
            )
            .and_then(|()| {
                bills::set_active(
                    &mut self.bill_plans,
                    &mut self.bill_entries,
                    id,
                    is_active,
                    today,
                )
            })
            .map(|()| id),
        };
        match result {
            Ok(id) => {
                if self.bills_tab == bills::BillsTab::Planner {
                    self.bills_selected = bills::planner_order(&self.bill_plans)
                        .iter()
                        .position(|plan| plan.id == id)
                        .unwrap_or(0);
                }
            }
            Err(error) => {
                form.error = Some(error);
                let dialog = match editing {
                    None => bills::BillsDialog::Add(form),
                    Some(id) => bills::BillsDialog::Edit(id, form),
                };
                self.open_dialog(OpenDialog::Bills(Box::new(dialog)));
            }
        }
    }

    fn handle_bill_plan_field_click(
        &mut self,
        field: bills::form::BillPlanField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            match field {
                bills::form::BillPlanField::Category
                | bills::form::BillPlanField::Unit
                | bills::form::BillPlanField::Account
                | bills::form::BillPlanField::Payee
                | bills::form::BillPlanField::Recurrence => form.click_select(field),
                bills::form::BillPlanField::Active => {
                    form.focus(field);
                    form.toggle();
                }
                _ => form.focus(field),
            }
        }
        cx.notify();
    }

    fn handle_bill_plan_option_click(
        &mut self,
        field: bills::form::BillPlanField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            form.choose(field, index);
        }
        cx.notify();
    }

    fn handle_bill_plan_amount_kind_click(
        &mut self,
        kind: bills::AmountKind,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            form.set_amount_kind(kind);
        }
        cx.notify();
    }

    fn handle_bills_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    fn handle_bills_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog();
        cx.notify();
    }

    /// Opens the Pay dialog (8d) on an open Schedule row, with its Match candidates worked out now;
    /// a row with nothing to pay says so instead.
    fn open_pay_bill_dialog(&mut self, row: bills::ScheduleRow) {
        let plan = bills::get(&self.bill_plans, row.id.plan_id);
        let Some(plan) = plan.filter(|_| row.is_actionable()) else {
            self.chrome.status_message = Some(crate::msg::desktop_status_bill_not_actionable());
            return;
        };
        let candidates = bills::match_candidates(
            &self.bill_plans,
            &self.bill_entries,
            &self.transactions,
            &self.accounts,
            &self.categories,
            row.id,
        );
        let preselected = bills::preselected_candidate(plan, &candidates, &self.transactions);
        let form = PayForm::new(
            row.id,
            plan,
            candidates,
            preselected,
            self.today,
            self.settings_date_style,
        );
        self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Pay(form))));
    }

    fn pay_form_mut(&mut self) -> Option<&mut PayForm> {
        match self.bills_dialog_mut() {
            Some(bills::BillsDialog::Pay(form)) => Some(form),
            _ => None,
        }
    }

    /// **Create transaction & mark paid** / **Match & mark paid**: settles the entry through
    /// `bills::pay` or `bills::match_split`. A refused settle reopens the dialog with the error
    /// shown.
    fn apply_pay_bill(&mut self, mut form: PayForm) {
        let entry = form.entry;
        let Some(action) = form.action() else {
            return;
        };
        let result = match action {
            bills::pay_form::PayAction::Pay { amount, date } => bills::pay(
                &self.bill_plans,
                &mut self.bill_entries,
                &mut self.transactions,
                entry,
                &amount,
                date,
            )
            .map(|_| ()),
            bills::pay_form::PayAction::Match(split) => bills::match_split(
                &self.bill_plans,
                &mut self.bill_entries,
                &self.transactions,
                &self.accounts,
                &self.categories,
                entry,
                split,
            ),
        };
        if let Err(error) = result {
            form.error = Some(error);
            self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Pay(form))));
        }
    }

    fn handle_pay_bill_mode_click(
        &mut self,
        mode: bills::pay_form::PayMode,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.set_mode(mode);
        }
        cx.notify();
    }

    fn handle_pay_bill_choice_click(
        &mut self,
        choice: bills::pay_form::MatchChoice,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.choose(choice);
        }
        cx.notify();
    }

    fn handle_pay_bill_field_click(
        &mut self,
        field: bills::pay_form::PayField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.focus(field);
        }
        cx.notify();
    }

    /// The Pay dialog (8d) over the Schedule, or nothing once its entry's Plan is gone.
    fn render_pay_bill_dialog(
        &self,
        form: &PayForm,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let plan = bills::get(&self.bill_plans, form.entry.plan_id)?;
        let on_mode_click: bills_view::pay_dialog::OnModeClick = {
            let entity = entity.clone();
            Rc::new(move |mode, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_pay_bill_mode_click(mode, cx));
            })
        };
        let on_choice_click: bills_view::pay_dialog::OnChoiceClick = {
            let entity = entity.clone();
            Rc::new(move |choice, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_pay_bill_choice_click(choice, cx)
                });
            })
        };
        let on_field_click: bills_view::pay_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_pay_bill_field_click(field, cx));
            })
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let account = self
            .accounts
            .iter()
            .find(|a| a.id == plan.account_id)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let payee = plan
            .payee_id
            .and_then(|id| payees::get(&self.payees, id))
            .map_or_else(crate::msg::desktop_bills_pay_no_payee, |p| p.name.clone());
        let category = categories::path(&self.categories, plan.category_id).unwrap_or_default();
        Some(bills_view::pay_dialog::render(
            bills_view::pay_dialog::PayDialogProps {
                plan_name: &plan.name,
                due: lib_locale::format::format_month_day(form.entry.due),
                form,
                candidates: self.pay_bill_candidate_rows(form),
                planned: format!("{} {}", format::amount(&plan.planned_amount).1, plan.unit),
                chips: [category, payee, account],
                amount_invalid: form.amount_invalid(),
                date_error: form.date_error(),
                error: form
                    .error
                    .as_ref()
                    .map(|_| crate::msg::desktop_bills_pay_error_gone()),
                valid: form.action().is_some(),
                handlers: bills_view::pay_dialog::PayDialogHandlers {
                    on_mode_click,
                    on_choice_click,
                    on_field_click,
                    on_cancel: plain(Shell::handle_bills_dialog_cancel),
                    on_confirm: plain(Shell::handle_bills_dialog_confirm),
                },
            },
            cx,
        ))
    }

    /// The Pay dialog's Match candidates, worded for its radio list.
    fn pay_bill_candidate_rows(&self, form: &PayForm) -> Vec<bills_view::pay_dialog::CandidateRow> {
        form.candidates
            .iter()
            .filter_map(|split_ref| {
                let transaction = self
                    .transactions
                    .iter()
                    .find(|t| t.id == split_ref.transaction_id)?;
                let split = transaction.splits.get(split_ref.split_index)?;
                let payee = split
                    .payee_id
                    .and_then(|id| payees::get(&self.payees, id))
                    .map(|p| p.name.clone())
                    .or_else(|| transaction.description.clone())
                    .unwrap_or_default();
                let account = self
                    .accounts
                    .iter()
                    .find(|a| a.id == transaction.account_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                Some(bills_view::pay_dialog::CandidateRow {
                    summary: format!(
                        "{payee} \u{b7} {} \u{b7} {}",
                        lib_locale::format::format_month_day(transaction.date),
                        format::signed_amount(&split.amount).1,
                    ),
                    account,
                })
            })
            .collect()
    }

    /// Opens the Skip dialog (8e) on an open Schedule row; a row with nothing to skip says so
    /// instead.
    fn open_skip_bill_dialog(&mut self, row: bills::ScheduleRow) {
        if !row.is_actionable() || bills::get(&self.bill_plans, row.id.plan_id).is_none() {
            self.chrome.status_message = Some(crate::msg::desktop_status_bill_not_actionable());
            return;
        }
        self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Skip(
            row.id,
        ))));
    }

    /// **Skip this cycle**: resolves the entry through `bills::skip`. The dialog carries no form
    /// to show an error on, so a refused skip (the entry resolved or superseded underneath it)
    /// closes with the reason in the status line.
    fn apply_skip_bill(&mut self, entry: bills::EntryId) {
        if bills::skip(&self.bill_plans, &mut self.bill_entries, entry).is_err() {
            self.chrome.status_message = Some(crate::msg::desktop_bills_skip_error_gone());
        }
    }

    /// The Skip dialog (8e) over the Schedule, or nothing once its entry's Plan is gone.
    fn render_skip_bill_dialog(
        &self,
        entry: bills::EntryId,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let plan = bills::get(&self.bill_plans, entry.plan_id)?;
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        Some(bills_view::skip_dialog::render(
            bills_view::skip_dialog::SkipDialogProps {
                plan_name: &plan.name,
                due: lib_locale::format::format_month_day(entry.due),
                one_shot: plan.recurrence == bills::Recurrence::OneShot,
                on_cancel: plain(Shell::handle_bills_dialog_cancel),
                on_confirm: plain(Shell::handle_bills_dialog_confirm),
            },
            cx,
        ))
    }

    /// A Dashboard Needs Attention Bill row's hand-off: the Bills Schedule tab, on the period that
    /// shows the entry, with it selected.
    fn open_bill_entry(&mut self, id: bills::EntryId) {
        let Some(entry) = bills::entry(&self.bill_entries, id) else {
            return;
        };
        self.bills_period = bills::schedule_period(entry, self.today);
        self.bills_all = false;
        // Reset so no filter hides the entry being handed off to.
        self.bills_filters = bills::history::BillFilters::default();
        self.set_bills_tab(bills::BillsTab::Schedule);
        self.bills_selected = self
            .bills_schedule_rows()
            .iter()
            .position(|row| row.id == id)
            .unwrap_or(0);
        self.nav.set_noun(Noun::Bills);
        self.reset_view_scroll();
    }

    /// A Paid Schedule row's hand-off: the Transactions page with its Matched Transaction selected,
    /// the date range widened to reach it when it falls before this year. A no-op for any other row.
    fn open_bill_transaction(&mut self, id: bills::EntryId) {
        let Some(bills::Resolution::Paid(split)) =
            bills::entry(&self.bill_entries, id).map(|entry| entry.resolution.clone())
        else {
            return;
        };
        self.open_transaction_row(split.transaction_id);
    }

    /// The Transactions page with `transaction_id` selected, the date range widened to reach it
    /// when it falls outside this year's. A no-op when the Transaction has gone.
    fn open_transaction_row(&mut self, transaction_id: u32) {
        let Some(date) = self
            .transactions
            .iter()
            .find(|t| t.id == transaction_id)
            .map(|t| t.date)
        else {
            return;
        };
        let mut filters = TransactionFilters::defaults(self.today);
        filters.from = filters.from.map(|from| from.min(date));
        filters.to = filters.to.map(|to| to.max(date));
        self.transactions_filters = filters;
        self.transactions_search.clear();
        self.transactions_filter_form = None;
        let index = transactions::query::query(
            &self.transactions_ledger(),
            &self.transactions,
            &self.transactions_filters,
            &self.transactions_search,
        )
        .rows
        .iter()
        .position(|row| row.transaction.id == transaction_id)
        .unwrap_or(0);
        self.transactions_selected = index;
        self.transactions_scroll
            .scroll_to_item_strict(index, ScrollStrategy::Center);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll();
    }

    fn handle_bills_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_bill_plan_dialog();
        cx.notify();
    }

    fn handle_bills_tab_click(&mut self, tab: bills::BillsTab, cx: &mut Context<'_, Self>) {
        self.set_bills_tab(tab);
        cx.notify();
    }

    fn handle_bills_period_prev(&mut self, cx: &mut Context<'_, Self>) {
        self.shift_bills_period(false);
        cx.notify();
    }

    fn handle_bills_period_next(&mut self, cx: &mut Context<'_, Self>) {
        self.shift_bills_period(true);
        cx.notify();
    }

    fn handle_bills_all_click(&mut self, cx: &mut Context<'_, Self>) {
        self.toggle_bills_all();
        cx.notify();
    }

    fn handle_bills_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.bills_selected = index;
        cx.notify();
    }

    fn handle_bills_edit_plan_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.bills_selected = index;
        if let Some(id) = self.selected_bill_plan() {
            self.open_edit_bill_plan_dialog(id);
        }
        cx.notify();
    }

    fn handle_bills_pay_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.bills_selected = index;
        if let Some(row) = self.selected_bill_row() {
            self.open_pay_bill_dialog(row);
        }
        cx.notify();
    }

    fn handle_bills_skip_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.bills_selected = index;
        if let Some(row) = self.selected_bill_row() {
            self.open_skip_bill_dialog(row);
        }
        cx.notify();
    }

    fn handle_bills_view_transaction_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.bills_selected = index;
        if let Some(row) = self.selected_bill_row() {
            self.open_bill_transaction(row.id);
        }
        cx.notify();
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
    fn apply_tags_dialog(&mut self, dialog: tags::form::TagsDialog) {
        match dialog {
            tags::form::TagsDialog::Add(form) => {
                let Some(draft) = form.draft() else {
                    return;
                };
                // `is_valid` ran the same name check, so a refusal can only leave the dialog open.
                match tags::insert_tag(&mut self.tags, &draft) {
                    Ok(id) => self.select_tag(id),
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
                    Ok(()) => self.select_tag(id),
                    Err(_) => {
                        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Edit(id, form)))
                    }
                }
            }
            tags::form::TagsDialog::Remove(id, _) => self.apply_remove_tag(id),
            tags::form::TagsDialog::Merge(form) => {
                if let Some((source, target)) = form.pair() {
                    self.apply_merge_tags(source, target);
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
        self.confirm_open_dialog();
        cx.notify();
    }

    fn open_edit_tag_dialog(&mut self, id: u32) {
        let Some(tag) = tags::get(&self.tags, id) else {
            return;
        };
        let form = tags::form::TagForm::for_edit(tag, self.tags.clone());
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Edit(id, form)));
    }

    fn open_remove_tag_dialog(&mut self, id: u32) {
        let Some(tag) = tags::get(&self.tags, id) else {
            return;
        };
        let form = tags::form::RemoveTagForm::new(
            &tag.name,
            tags::transaction_count(&self.transactions, id),
        );
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Remove(id, form)));
    }

    /// Untags every Split, deletes the Tag and toasts it. The selection keeps its position, so it
    /// lands on the next Tag in usage order (or the new last one).
    fn apply_remove_tag(&mut self, id: u32) {
        let Some(name) = tags::get(&self.tags, id).map(|tag| tag.name.clone()) else {
            return;
        };
        let (kind, text) = match tags::remove_tag(&mut self.tags, &mut self.transactions, id) {
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
    fn merge_tag_options(&self) -> Vec<tags::form::MergeOption> {
        tags::form::merge_options(&self.tags, &self.transactions, |name, count| {
            crate::msg::desktop_tags_merge_option(name, i64::try_from(count).unwrap_or(i64::MAX))
        })
    }

    /// Opens 7e with `source` as the source Tag and, when it is flagged as a likely duplicate, its
    /// suggested target (#354). `None` (the palette's `tags merge`, or the subline link with
    /// nothing flagged) leaves both selects empty.
    fn open_merge_tags_dialog(&mut self, source: Option<u32>) {
        let groups = tags::duplicate_groups(&self.tags, &self.transactions);
        let target = source.and_then(|id| tags::duplicate_of(&groups, id));
        let form = tags::form::MergeTagsForm::new(self.merge_tag_options(), source, target);
        self.open_dialog(OpenDialog::Tags(tags::form::TagsDialog::Merge(form)));
    }

    /// Retags the source's Splits with the target, deletes the source, toasts it and selects the
    /// target.
    fn apply_merge_tags(&mut self, source: u32, target: u32) {
        let (Some(source_name), Some(target_name)) = (
            tags::get(&self.tags, source).map(|tag| tag.name.clone()),
            tags::get(&self.tags, target).map(|tag| tag.name.clone()),
        ) else {
            return;
        };
        let transactions = tags::transaction_count(&self.transactions, source);
        let (kind, text) =
            match tags::merge_tags(&mut self.tags, &mut self.transactions, source, target) {
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
        self.select_tag(target);
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
        self.select_tag(id);
        self.focus_settings_page();
        cx.notify();
    }

    fn handle_tags_duplicate_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id);
        self.open_merge_tags_dialog(Some(id));
        cx.notify();
    }

    fn handle_tags_edit_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id);
        self.open_edit_tag_dialog(id);
        cx.notify();
    }

    fn handle_tags_remove_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_tag(id);
        self.open_remove_tag_dialog(id);
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
    fn handle_payees_key(&mut self, keystroke: &Keystroke) -> bool {
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
                    self.open_delete_payee_dialog(id);
                }
            }
            _ => return false,
        }
        true
    }

    /// The Default category select's options: "none", then the leaf Categories.
    /// `:import`: opens the stubbed 6e step on the seeded statement, in place of the Transactions
    /// page (#284).
    fn open_import(&mut self) {
        self.import = Some(ImportState::new(&self.payees, self.today));
        self.transactions_filter_form = None;
        self.nav.set_noun(Noun::Transactions);
        self.nav.set_focus(FocusZone::View);
        self.reset_view_scroll();
    }

    /// The Category select's options on 6e: "choose category…", then every leaf.
    fn import_category_options(&self) -> payees::form::PayeeOptions {
        payees::form::PayeeOptions::new(
            &self.categories,
            crate::msg::desktop_import_choose_category(),
        )
    }

    /// 6e's keys, while it shows with the view focused in `Normal` mode: `j`/`k` move the row, `p`
    /// and `c` open its Payee and Category selects, `n` creates a new Payee from its cleaned name,
    /// `r` toggles "remember", `enter` continues and `esc` goes back. While a select is open it
    /// owns `j`/`k`/arrows, `enter`/`space` and `esc`. Any other key falls through to the router.
    fn handle_import_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.import.is_none()
            || self.nav.noun() != Noun::Transactions
            || self.nav.mode() != InputMode::Normal
            || self.nav.focus() != FocusZone::View
        {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        let categories = self.import_category_options();
        let Some(state) = self.import.as_mut() else {
            return false;
        };
        let key = keystroke.key.as_str();
        let selected = state.selected.min(state.rows.len().saturating_sub(1));
        let choices = state
            .rows
            .get(selected)
            .map(|row| import_view::payee_choices(&self.payees, &row.raw));
        let Some(choices) = choices else {
            return false;
        };

        if state.open_select.is_some() {
            let select_key = match key {
                "j" | "down" => import::SelectKey::Down,
                "k" | "up" => import::SelectKey::Up,
                "enter" | "space" => import::SelectKey::Commit,
                "escape" => import::SelectKey::Cancel,
                // An open list swallows everything else, like the dialogs' selects.
                _ => return true,
            };
            state.handle_select_key(select_key, &self.payees, &choices, &categories);
            return true;
        }

        let len = state.rows.len();
        match key {
            "j" | "down" => state.selected = accounts::step_selection(selected, len, 1),
            "k" | "up" => state.selected = accounts::step_selection(selected, len, -1),
            "p" => state.open(selected, RowSelect::Payee, &choices, &categories),
            "c" => state.open(selected, RowSelect::Category, &choices, &categories),
            "n" => state.create_new_payee(&self.payees),
            "r" => state.remember = !state.remember,
            "enter" => {
                self.continue_import();
                return true;
            }
            "escape" => {
                self.import = None;
                return true;
            }
            _ => return false,
        }
        let selected = state.selected;
        self.view_scroll_handle.scroll_to_item(selected);
        true
    }

    /// **continue** / `enter`: commits the import to the stubs and lands on Transactions with a
    /// Toast. Does nothing while a row needs review (the button is disabled then).
    fn continue_import(&mut self) {
        let Some(state) = self.import.as_ref() else {
            return;
        };
        match import::commit(
            &state.rows,
            state.remember,
            import::EVERYDAY_ACCOUNT_ID,
            &mut self.payees,
            &mut self.transactions,
        ) {
            Ok(committed) => {
                if let Some(account) = self
                    .accounts
                    .iter_mut()
                    .find(|account| account.id == import::EVERYDAY_ACCOUNT_ID)
                {
                    account.transaction_count = account
                        .transaction_count
                        .saturating_add(u32::try_from(committed.transactions).unwrap_or(u32::MAX));
                }
                self.import = None;
                self.reset_transactions_selection();
                self.reset_view_scroll();
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_import_done(
                        i64::try_from(committed.transactions).unwrap_or(i64::MAX),
                    ),
                );
            }
            Err(import::ImportError::NeedsReview(_)) => {}
            Err(import::ImportError::Payee(error)) => {
                self.raise_toast(ToastKind::Error, error.to_string());
            }
        }
    }

    fn handle_import_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(state) = self.import.as_mut() {
            state.selected = index;
            if state
                .open_select
                .as_ref()
                .is_some_and(|(row, _, _)| *row != index)
            {
                state.open_select = None;
            }
        }
        self.nav.set_focus(FocusZone::View);
        cx.notify();
    }

    /// A click on a row's select: opens it, or closes it when it is already the open one.
    fn handle_import_select_click(
        &mut self,
        index: usize,
        select: RowSelect,
        cx: &mut Context<'_, Self>,
    ) {
        let categories = self.import_category_options();
        if let Some(state) = self.import.as_mut() {
            if state
                .open_select
                .as_ref()
                .is_some_and(|(row, open, _)| *row == index && *open == select)
            {
                state.open_select = None;
            } else if let Some(row) = state.rows.get(index) {
                let choices = import_view::payee_choices(&self.payees, &row.raw);
                state.open(index, select, &choices, &categories);
            }
        }
        self.nav.set_focus(FocusZone::View);
        cx.notify();
    }

    fn handle_import_option_click(&mut self, option: usize, cx: &mut Context<'_, Self>) {
        let categories = self.import_category_options();
        if let Some(state) = self.import.as_mut()
            && let Some((index, _, _)) = state.open_select.as_ref()
            && let Some(row) = state.rows.get(*index)
        {
            let choices = import_view::payee_choices(&self.payees, &row.raw);
            state.choose(option, &self.payees, &choices, &categories);
        }
        cx.notify();
    }

    fn handle_import_remember_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(state) = self.import.as_mut() {
            state.remember = !state.remember;
        }
        cx.notify();
    }

    fn handle_import_back_click(&mut self, cx: &mut Context<'_, Self>) {
        self.import = None;
        cx.notify();
    }

    fn handle_import_continue_click(&mut self, cx: &mut Context<'_, Self>) {
        self.continue_import();
        cx.notify();
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
    fn apply_payees_dialog(&mut self, dialog: payees::form::PayeesDialog) {
        match dialog {
            payees::form::PayeesDialog::Delete(id, form) => {
                let Some(name) = payees::get(&self.payees, id).map(|p| p.name.clone()) else {
                    return;
                };
                let action = form.action();
                let (kind, text) = match payees::apply_delete_action(
                    &mut self.payees,
                    &self.transactions,
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
        self.confirm_open_dialog();
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
    fn open_delete_payee_dialog(&mut self, id: u32) {
        let Some(payee) = payees::get(&self.payees, id) else {
            return;
        };
        let action = payees::form::DeleteAction::for_payee(payee, &self.transactions);
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
        self.open_edit_document_type_dialog(id);
        cx.notify();
    }

    /// **remove** on a Documents row: selects it and opens the Remove dialog.
    fn handle_settings_documents_remove_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.settings_documents_selected = Some(id);
        self.focus_settings_page();
        self.open_remove_document_type_dialog(id);
        cx.notify();
    }

    /// **+ Add document type**: opens the Add dialog.
    fn handle_settings_documents_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.focus_settings_page();
        self.open_add_document_type_dialog();
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
        self.open_remove_inventory_row(row);
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
        self.open_delete_payee_dialog(id);
        cx.notify();
    }

    /// Selects the account with `id`, if it still exists.
    fn select_account(&mut self, id: u32) {
        let Some(index) = self.accounts.iter().position(|account| account.id == id) else {
            return;
        };
        if let Some(position) = accounts::display_order(&self.accounts)
            .iter()
            .position(|&i| i == index)
        {
            self.accounts_selected = position;
        }
    }

    /// A click on an account row: selects it and, as `enter` does, tries to open its ledger.
    fn handle_accounts_row_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_account(id);
        self.open_account_ledger(id);
        cx.notify();
    }

    /// The page's **+ Add account** button: the same handler `n` reaches.
    fn handle_accounts_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_account_dialog("");
        cx.notify();
    }

    /// The three selects' option lists, read live from Settings (so an institution added there
    /// appears here) and the fixed type order.
    fn account_dialog_options(&self) -> AccountOptions {
        AccountOptions::new(
            self.settings_institutions
                .iter()
                .map(|institution| institution.name.clone())
                .collect(),
            self.settings_units
                .iter()
                .map(|unit| unit.code.clone())
                .collect(),
        )
    }

    /// Opens the Add account dialog on a fresh form (Unit starting on Settings' default Unit),
    /// with Name pre-filled from `name` -- empty for `n` and the button, the typed argument for
    /// `accounts new <account name>`.
    fn open_add_account_dialog(&mut self, name: &str) {
        let options = self.account_dialog_options();
        let default_unit = self
            .settings_units
            .iter()
            .find(|unit| unit.is_default)
            .map(|unit| unit.code.as_str());
        let mut form = AccountForm::new(&options, default_unit);
        form.name = TextField::new(name.trim());
        self.open_dialog(OpenDialog::Accounts(AccountsDialog::Add(form)));
    }

    /// A click on a field of the Add account dialog: focuses a text field, or focuses a select
    /// and toggles its list.
    fn handle_accounts_dialog_field_click(
        &mut self,
        field: AccountField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self
            .accounts_dialog_mut()
            .and_then(AccountsDialog::form_mut)
        {
            if field.is_select() {
                form.click_select(field);
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    /// A click on a row of an open dropdown list.
    fn handle_accounts_dialog_option_click(
        &mut self,
        field: AccountField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self
            .accounts_dialog_mut()
            .and_then(AccountsDialog::form_mut)
        {
            form.choose_option(field, index);
        }
        cx.notify();
    }

    fn handle_accounts_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    fn handle_accounts_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog();
        cx.notify();
    }

    /// Applies a confirmed Accounts dialog (reached through [`Self::confirm_open_dialog`] from
    /// the Add account button, the Edit dialog's **Save**, the Delete dialog's **Delete account**
    /// or `Enter`): Add builds the account, appends it and selects it; Edit writes the changes
    /// onto the existing row (which regroups if its Type changed) and keeps it selected; Delete
    /// removes it. The form has already validated.
    fn apply_accounts_dialog(&mut self, dialog: AccountsDialog) {
        let changed_id = match dialog {
            AccountsDialog::Add(form) => {
                let is_currency = form
                    .unit
                    .value()
                    .and_then(|code| self.settings_units.iter().find(|unit| unit.code == code))
                    .is_none_or(|unit| unit.kind == "currency");
                let id = accounts::next_account_id(&self.accounts);
                let opened_at = Local::now().date_naive();
                form.into_account(id, opened_at, is_currency)
                    .map(|account| {
                        self.accounts.push(account);
                        id
                    })
            }
            AccountsDialog::Edit(id, form) => self
                .accounts
                .iter_mut()
                .find(|account| account.id == id)
                .and_then(|account| form.apply_to(account).then_some(id)),
            AccountsDialog::Delete(id, _) => {
                let (kind, text) = delete_account(&mut self.accounts, &mut self.transactions, id);
                self.raise_toast(kind, text);
                // The selection is a position in display order: keep it in range, so it lands on
                // the account that slid into the deleted row's place (or the last one).
                self.accounts_selected = self
                    .accounts_selected
                    .min(self.accounts.len().saturating_sub(1));
                None
            }
        };
        if let Some(id) = changed_id {
            self.select_account(id);
        }
    }

    /// Opens the Delete account dialog on `id`. A no-op if the account is gone.
    fn open_delete_account_dialog(&mut self, id: u32) {
        let Some(account) = self.accounts.iter().find(|account| account.id == id) else {
            return;
        };
        let form = DeleteAccountForm::new(account.name.as_str());
        self.open_dialog(OpenDialog::Accounts(AccountsDialog::Delete(id, form)));
    }

    /// Opens the Edit account dialog on `id`, pre-filled. A no-op if the account is gone.
    fn open_edit_account_dialog(&mut self, id: u32) {
        let options = self.account_dialog_options();
        let Some(account) = self.accounts.iter().find(|account| account.id == id) else {
            return;
        };
        let form = AccountForm::from_account(account, &options);
        self.open_dialog(OpenDialog::Accounts(AccountsDialog::Edit(id, form)));
    }

    fn handle_accounts_edit_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_account(id);
        self.open_edit_account_dialog(id);
        cx.notify();
    }

    fn handle_accounts_delete_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_account(id);
        self.open_delete_account_dialog(id);
        cx.notify();
    }

    fn handle_categories_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_categories_dialog(None);
        cx.notify();
    }

    fn handle_categories_add_sub_click(&mut self, parent_id: u32, cx: &mut Context<'_, Self>) {
        self.categories_selected_id = Some(parent_id);
        self.open_add_categories_dialog(Some(parent_id));
        cx.notify();
    }

    fn handle_categories_edit_click(&mut self, category_id: u32, cx: &mut Context<'_, Self>) {
        if self.categories.iter().any(|c| c.id == category_id) {
            self.categories_selected_id = Some(category_id);
            self.open_edit_categories_dialog(category_id);
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
        self.confirm_open_dialog();
        cx.notify();
    }

    /// Applies a confirmed Categories dialog (reached through [`Self::confirm_open_dialog`] from
    /// the Add/Save/Delete button or `Enter`): Add inserts the category and saves its Monthly
    /// budget; Edit renames, re-parents and saves the budget; Delete removes it and keeps the
    /// selection in range. The form has already validated.
    fn apply_categories_dialog(&mut self, dialog: categories::form::CategoriesDialog) {
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
                    self.save_category_budget(category_id, &form, false);
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
                self.save_category_budget(id, &form, true);
            }
            categories::form::CategoriesDialog::Delete(category_id, _) => {
                let (kind, text) = delete_category(
                    &mut self.categories,
                    &mut self.transactions,
                    &mut self.budgets,
                    category_id,
                );
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
        self.confirm_open_dialog();
        cx.notify();
    }

    /// `accounts new|edit|delete [<account name>]`: jumps to the Accounts page, then opens the
    /// same dialog the page's `n`/`e`/`d` and buttons do. `new` pre-fills Name with the argument.
    /// `edit` and `delete` resolve the typed name ([`accounts::find_by_name`]), or use the
    /// selected row when none is given; a name that fits nothing or several accounts flashes a
    /// status-line message naming the problem rather than guessing.
    fn run_accounts_command(&mut self, command_name: &str, verb: AccountsVerb, argument: &str) {
        self.open_settings_page(SettingsSection::Accounts);
        if verb == AccountsVerb::New {
            self.open_add_account_dialog(argument);
            return;
        }

        let index = if argument.is_empty() {
            match self.selected_account_index() {
                Some(index) => index,
                None => {
                    self.chrome.status_message = Some(crate::msg::desktop_status_no_accounts(
                        &format!(":{command_name}"),
                    ));
                    return;
                }
            }
        } else {
            match accounts::find_by_name(&self.accounts, argument) {
                NameLookup::Found(index) => index,
                NameLookup::NotFound => {
                    self.chrome.status_message = Some(crate::msg::desktop_status_no_account_named(
                        &format!(":{command_name}"),
                        argument,
                    ));
                    return;
                }
                NameLookup::Ambiguous(names) => {
                    self.chrome.status_message =
                        Some(crate::msg::desktop_status_account_ambiguous(
                            &format!(":{command_name}"),
                            argument,
                            &names.join(", "),
                        ));
                    return;
                }
            }
        };
        let id = self.accounts[index].id;
        self.select_account(id);
        match verb {
            AccountsVerb::Edit => self.open_edit_account_dialog(id),
            AccountsVerb::Delete => self.open_delete_account_dialog(id),
            AccountsVerb::New => {}
        }
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
        self.transactions_scroll
            .scroll_to_item_strict(self.transactions_selected, ScrollStrategy::Center);
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
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Deletes account `id` and, as the Delete dialog says, the transactions booked to it,
/// returning the Success Toast that counts them.
fn delete_account(
    accounts: &mut Vec<Account>,
    transactions: &mut Vec<Transaction>,
    id: u32,
) -> (ToastKind, String) {
    let name = accounts
        .iter()
        .find(|account| account.id == id)
        .map(|account| account.name.clone())
        .unwrap_or_default();
    accounts.retain(|account| account.id != id);
    let before = transactions.len();
    transactions.retain(|transaction| transaction.account_id != id);
    let deleted = i64::try_from(before - transactions.len()).unwrap_or(i64::MAX);
    (
        ToastKind::Success,
        lib_locale::msg::toast_account_deleted(&name, deleted),
    )
}

/// Deletes category `id`, re-pointing its splits to Uncategorised and dropping its budget,
/// and returns the Toast: Success counting the moved splits, or the `toast-save-failed` Error
/// if the store refuses (the dialog only opens on a leaf, so that means the tree changed
/// underneath it). Nothing is touched on a refusal.
fn delete_category(
    categories: &mut Vec<Category>,
    transactions: &mut [Transaction],
    budgets: &mut budgets::Budgets,
    id: u32,
) -> (ToastKind, String) {
    let refused = |error: categories::CategoryError| {
        (
            ToastKind::Error,
            lib_locale::msg::toast_save_failed(
                &lib_locale::msg::toast_entity_category(),
                &error.to_string(),
            ),
        )
    };
    let Some(category) = categories.iter().find(|c| c.id == id) else {
        return refused(categories::CategoryError::NotFound);
    };
    if !categories::is_leaf(categories, id) {
        return refused(categories::CategoryError::NonLeafDeletion);
    }
    let name = category.name.clone();
    let category_type = category.category_type.clone();
    let uncategorised_id = categories::get_or_create_uncategorised(categories, category_type);
    let mut moved = 0_i64;
    for split in transactions.iter_mut().flat_map(|t| t.splits.iter_mut()) {
        if split.category_id == id {
            split.category_id = uncategorised_id;
            moved += 1;
        }
    }
    budgets.remove_category(id);
    match categories::delete_category(categories, id) {
        Ok(()) => (
            ToastKind::Success,
            lib_locale::msg::toast_category_deleted(&name, moved),
        ),
        Err(error) => refused(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::transactions::hints::{filter_hints, transactions_hints};

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
        let (mut accounts, _, mut transactions) = seeded_ledger();
        let account = accounts[0].clone();
        let booked = transactions
            .iter()
            .filter(|t| t.account_id == account.id)
            .count();
        let (kind, text) = delete_account(&mut accounts, &mut transactions, account.id);
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
