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
//! `impl Root` block, not the real module of the same name; a `_ui` suffix (`transactions_ui`)
//! marks per-domain wiring. `open_ledger` is the exception: it holds [`OpenLedger`], everything
//! built on a Ledger, which `Root` keeps as an `Option` (#975).
//!
//! `Root` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

mod commands;
mod date_clock;
mod dialogs;
mod explorer;
mod focus;
mod history;
mod key_dispatch;
mod log_feed;
mod open_ledger;
mod rail;
mod render;
mod root_router;
mod snapshots;
mod status;
mod toasts;
mod transactions_ui;
mod view_events;
mod write_failures;

use crate::navigation::command;
#[cfg(test)]
use commands::record_history;
pub use date_clock::{DATE_POLL, wall_clock};
pub use key_dispatch::PENDING_G_TIMEOUT;
use lib_core::LocationId;
pub use log_feed::LOG_COALESCE;
pub use open_ledger::OpenLedger;
#[doc(hidden)]
pub use snapshots::{
    BillRowSnapshot, BillsSnapshot, BudgetRowSnapshot, BudgetsSnapshot, ChipSnapshot,
    DashboardBillSnapshot, DocumentsSnapshot, HelpGroup, ImportRowSnapshot, ImportSnapshot,
    PageSnapshot, PaletteSnapshot, SettingsAccountsSnapshot, SettingsCategoriesSnapshot,
    SettingsDisplaySnapshot, SettingsDocumentTypesSnapshot, SettingsGeneralSnapshot,
    SettingsInstitutionsSnapshot, SettingsInventorySnapshot, SettingsPayeesSnapshot,
    SettingsSnapshot, SettingsTagsSnapshot, SettingsTracingSnapshot, SettingsUnitsSnapshot,
    StatusSnapshot, ToastSnapshot, ToastsSnapshot, TransactionsSnapshot, UnitPricesSnapshot,
};
pub use write_failures::WRITE_FAILURE_POLL;

use std::time::Duration;

use crate::persistence::PersistedState;
use lib_database::Stores;

/// The Shell's Presenter over the Desktop's Settings pages.
type ShellPresenter = lib_shell::ShellPresenter<SettingsPage>;

use gpui::{App, Context, FocusHandle, Focusable, ScrollHandle, Task};

use lib_shell::{NavInput, ToastInput};

use crate::view::active::ActiveViewEntity;
use crate::view::event::ViewMode;
use crate::{
    documents::{DocumentsMode, LibraryScope, LibrarySort},
    navigation::active_view::ActiveView,
    navigation::explorer::{ExplorerFilters, FileExplorer},
    navigation::nav::{NavState, RailMode},
    settings::{SettingsPage, display::DisplayPreferences},
    shell::state::ShellState,
    shell::view::{rail as rail_view, window_size::WindowSize},
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
/// off. `stores` is the Ledger file start-up opened (the app's SQLite adapters or a test's
/// in-memory ports), which the Shell opens as its Open Ledger through [`OpenLedger::build`].
pub fn build(
    persisted: PersistedState,
    bindings: ShellBindings,
    logs: ShellLogs,
    stores: Stores,
    clock: lib_core::Clock,
    focus_handle: FocusHandle,
    cx: &mut Context<'_, Root>,
) -> Root {
    crate::input::install(cx.weak_entity(), cx);
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

    let mut shell = Root::with_clock(nav, focus_handle, clock);
    shell.open_ledger(stores, &bindings.keybindings, cx);
    shell.set_start_sidebar_minimised(persisted.start_sidebar_minimised, cx);
    shell.set_client_preferences(&persisted.preferences, cx);
    shell.set_settings_page(settings_page, cx);
    shell.set_explorer_filters(persisted.explorer_filters);
    shell.set_documents_state(documents_state.0, documents_state.1, documents_state.2, cx);
    shell.set_keymaps(&bindings.keybindings, cx);
    shell.start_toast_clock(cx);
    shell.start_date_clock(cx);
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
    /// The palette's rows under the `[keybindings]` in effect, rebuilt by [`Self::set_keymaps`]
    /// so opening the palette builds no keymap.
    palette_entries: Vec<command::Entry>,
    /// The "1e" file explorer's own state -- `Some` only while `:open`'s dialog is on screen,
    /// mirroring `Option<Palette>`. `NavState::mode` stays `InputMode::Command` for as long as
    /// this is `Some` (see [`Self::run_command`]'s own doc), so the two together -- rather than
    /// a third `InputMode` variant -- are what "the file explorer is open" means.
    file_explorer: Option<FileExplorer>,
    /// The `:open` explorer's footer checkboxes, kept here so they outlive each dialog and reach
    /// `persistence::PersistedState` at quit.
    explorer_filters: ExplorerFilters,
    /// Everything built on the Ledger file, `None` with no Ledger open: the one source of truth
    /// for whether one is (#975).
    open: Option<OpenLedger>,
    /// "Today" for the seed data and, later, the `this year` filter -- read at construction and
    /// moved on only when [`Self::presenter`] sees the Clock cross into a new calendar month, so
    /// everything derived from it (the seeded dates, the default range) agrees within a month.
    today: chrono::NaiveDate,
    /// The Shell as a `lib-shell` module: the status line, the Toasts, where the person is (the
    /// noun, focus and mode), the global keymap, and the Clock (the local date, or a test's pinned
    /// day; every View is built with it), whose month turn it notices.
    presenter: ShellPresenter,
}

impl Root {
    /// Builds the Shell on `clock`, which dates the seeded stub data from the day it reads now, so
    /// tests pin it rather than live at the mercy of the wall clock. Builds no Ledger: that is
    /// [`Self::open_ledger`]'s.
    pub fn with_clock(nav: NavState, focus_handle: FocusHandle, clock: lib_core::Clock) -> Self {
        let today = clock.today();
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
            palette_entries: command::entries(&lib_config::KeyBindingConfig::default()),
            file_explorer: None,
            explorer_filters: ExplorerFilters::default(),
            open: None,
            today,
            presenter: ShellPresenter::new(clock, nav),
        }
    }

    /// Opens `stores` as the Open Ledger, building everything on it under `keybindings`. Start-up
    /// calls it on the file it opened; `:open` will on the file picked.
    pub fn open_ledger(
        &mut self,
        stores: Stores,
        keybindings: &lib_config::KeyBindingConfig,
        cx: &mut Context<'_, Self>,
    ) {
        let clock = self.presenter.clock().clone();
        self.open = Some(OpenLedger::build(stores, keybindings, &clock, cx));
        // The Views that read their keys after construction take `[keybindings]` here, so the
        // Open Ledger comes up under them whoever opens it.
        self.set_keymaps(keybindings, cx);
    }

    /// Whether a Ledger is open: whether `Root` holds an Open Ledger.
    pub fn ledger_open(&self) -> bool {
        self.open.is_some()
    }

    /// The Open Ledger. Start-up always opens one and nothing closes it until the no Ledger open
    /// start (#977), so its absence is a bug for now.
    #[expect(
        clippy::expect_used,
        reason = "halfway state (#975): start-up always opens a Ledger and nothing drops it until #977"
    )]
    fn open(&self) -> &OpenLedger {
        self.open
            .as_ref()
            .expect("start-up opens a Ledger and nothing closes it until #977")
    }

    /// The Categories tree, read straight from the Ledger (ADR-0038).
    fn categories<'a>(&self, cx: &'a App) -> &'a [Category] {
        self.open().ledger.read(cx).ledger().categories()
    }

    /// The Bill Plans, from the Ledger's read copy of the Bills ports.
    fn bill_plans<'a>(&self, cx: &'a App) -> &'a [crate::bills::BillPlan] {
        self.open().ledger.read(cx).ledger().bill_plans()
    }

    /// The Bill Schedule entries, from the Ledger's read copy of the Bills ports.
    fn bill_entries<'a>(&self, cx: &'a App) -> &'a [crate::bills::BillScheduleEntry] {
        self.open().ledger.read(cx).ledger().bill_entries()
    }

    /// Restores the Documents mode, Library scope and sort from the last run.
    pub fn set_documents_state(
        &self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
        cx: &mut App,
    ) {
        self.open()
            .views
            .documents
            .update(cx, |page, cx| page.set_persisted(mode, scope, sort, cx));
    }

    /// The Documents mode, Library scope and sort, for persistence.
    pub fn documents_persisted(&self, cx: &App) -> (DocumentsMode, LibraryScope, LibrarySort) {
        self.open().views.documents.read(cx).persisted()
    }

    /// Files dropped anywhere on the window land in the Inbox like `Import…`, but the page stays
    /// where it is: the Toast and the rail badge say what arrived.
    pub fn drop_documents(&mut self, paths: &[std::path::PathBuf], cx: &mut App) {
        self.open()
            .views
            .documents
            .update(cx, |page, cx| page.drop_paths(paths, cx));
    }

    /// The last-visited Settings page, for persistence.
    pub fn settings_page(&self, cx: &App) -> SettingsPage {
        self.open().views.settings.read(cx).selected_section()
    }

    pub fn set_settings_page(&mut self, section: SettingsPage, cx: &mut App) {
        self.navigate(NavInput::RestoreSettingsPage(section));
        self.open()
            .views
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
        self.open()
            .views
            .settings
            .update(cx, |view, cx| view.set_display(display, cx));
    }

    /// Applies the date style and Toasts kept on this device (ADR-0051) to the Settings View and
    /// the Views that draw with them, as the app starts.
    pub fn set_client_preferences(
        &mut self,
        preferences: &lib_core::ClientPreferences,
        cx: &mut App,
    ) {
        let display = DisplayPreferences {
            date_style: preferences.date_style,
            toasts_on: preferences.toasts_on,
            ..self.display(cx)
        };
        self.open()
            .views
            .settings
            .update(cx, |view, cx| view.set_display(display, cx));
        self.push_display_preferences(display, cx);
    }

    /// The Client-scoped Preferences as they stand, for the state file at quit. On start has no
    /// control yet (#911), so it is kept as it was read.
    pub fn client_preferences(
        &self,
        on_start: lib_core::OnStart,
        cx: &App,
    ) -> lib_core::ClientPreferences {
        let display = self.display(cx);
        let colours = cx.global::<crate::theme::colours::Colours>();
        lib_core::ClientPreferences {
            toasts_on: display.toasts_on,
            colour_theme: colours.colour_theme().map(str::to_string),
            colour_appearance: colours
                .colour_appearance()
                .map(|appearance| appearance.key().to_string()),
            date_style: display.date_style,
            on_start,
        }
    }

    /// The Display Preferences, which the Settings View owns (ADR-0037).
    fn display(&self, cx: &App) -> DisplayPreferences {
        self.open().views.settings.read(cx).display(cx)
    }

    /// The global keys and the keys every module Dialog shares, under `config`'s overrides, which
    /// the palette's key column reads too.
    /// The Transactions filter popover reads its keys through the Dialog keymap too, until it is a
    /// module Dialog (#923).
    pub fn set_keymaps(&mut self, config: &lib_config::KeyBindingConfig, cx: &mut App) {
        self.palette_entries = command::entries(config);
        self.presenter.set_keymap(config);
        self.state.dialog_keymap = lib_shell::dialog_keymap(config);
        self.open()
            .views
            .transactions
            .update(cx, |view, _| view.set_keymaps(config));
        self.open()
            .views
            .import
            .update(cx, |view, _| view.set_keymaps(config));
        self.open()
            .views
            .budgets
            .update(cx, |page, _| page.set_keymaps(config));
        self.open()
            .views
            .documents
            .update(cx, |page, _| page.set_keymaps(config));
    }

    /// Empties the Inventory, standing in for removing every Location until the Remove dialog
    /// lands, so a test can reach the Inventory page's empty state.
    #[doc(hidden)]
    pub fn empty_inventory_for_test(&mut self, cx: &mut App) {
        let ids: Vec<LocationId> = self
            .open()
            .ledger
            .read(cx)
            .ledger()
            .inventory()
            .locations
            .iter()
            .map(|location| location.id)
            .collect();
        self.open().ledger.update(cx, |store, cx| {
            for id in ids {
                assert!(
                    store.apply(cx, lib_ledger::RemoveLocation(id)).is_ok(),
                    "a Location listed a moment ago must still be removable"
                );
            }
        });
    }

    /// Removes every Tag, so a test can reach the Tags page with nothing to select.
    #[doc(hidden)]
    pub fn empty_tags_for_test(&mut self, cx: &mut App) {
        let ids: Vec<_> = self
            .open()
            .ledger
            .read(cx)
            .ledger()
            .tags()
            .iter()
            .map(|tag| tag.id)
            .collect();
        self.open().ledger.update(cx, |store, cx| {
            for id in ids {
                assert!(
                    store.apply(cx, lib_ledger::RemoveTag(id)).is_ok(),
                    "a Tag listed a moment ago must still be removable"
                );
            }
        });
    }

    /// Leaves no Unit in use by an Account, a Bill Plan or a Budget, so a test can delete a
    /// seeded Unit instead of meeting the in-use refusal.
    #[doc(hidden)]
    pub fn drop_unit_users_for_test(&mut self, cx: &mut App) {
        self.open()
            .ledger
            .update(cx, |store, cx| store.drop_unit_users_for_test(cx));
    }

    /// Adds a Location priced in `unit`, which then names it.
    #[doc(hidden)]
    pub fn add_location_in_for_test(&mut self, unit: lib_core::UnitId, cx: &mut App) {
        self.open()
            .ledger
            .update(cx, |store, cx| store.add_location_in_for_test(unit, cx));
    }

    /// Adds a Budget in `unit`, which then names it.
    #[doc(hidden)]
    pub fn add_budget_in_for_test(&mut self, unit: lib_core::UnitId, cx: &mut App) {
        self.open()
            .ledger
            .update(cx, |store, cx| store.add_budget_in_for_test(unit, cx));
    }

    /// Adds a Payee called `name`, so an Import that creates it is refused.
    #[doc(hidden)]
    pub fn add_payee_for_test(&mut self, name: &str, cx: &mut App) {
        self.open()
            .ledger
            .update(cx, |store, cx| store.add_payee_for_test(name, cx));
    }

    /// Moves every Bill Plan to pay from Account `to`, so a test can delete the Account another Plan
    /// paid from instead of meeting the Bill Plans refusal.
    #[doc(hidden)]
    pub fn move_bill_plans_for_test(&mut self, to: lib_core::AccountId, cx: &mut App) {
        {
            let mut ports = self.open().bills.borrow_mut();
            for mut plan in lib_bills::BillsPorts::bills(&*ports).bill_plans() {
                plan.account_id = to;
                lib_bills::BillsPorts::split_mut(&mut *ports)
                    .bills
                    .upsert_bill_plan(plan);
            }
        }
        let open = self.open();
        crate::bills::ports::refresh(&open.bills, &open.ledger, cx);
    }

    /// Leaves no Unit quoted in another, so a test can delete a seeded Unit instead of meeting the
    /// quoted refusal.
    #[doc(hidden)]
    pub fn unquote_units_for_test(&mut self, cx: &mut App) {
        crate::units::ports::unquote_for_test(&self.open().units, &self.open().ledger, cx);
    }

    /// Gives Institution `id` Unit `unit` as its Usual unit, so a test can delete a Unit an
    /// Institution names.
    #[doc(hidden)]
    pub fn name_usual_unit_for_test(
        &mut self,
        id: lib_core::InstitutionId,
        unit: lib_core::UnitId,
        cx: &mut App,
    ) {
        self.open()
            .ledger
            .update(cx, |store, cx| store.name_usual_unit_for_test(id, unit, cx));
    }

    /// The code of Institution `id`'s Usual unit, as the Ledger read copy has it: `None` for none.
    #[doc(hidden)]
    pub fn usual_unit_code(&self, id: lib_core::InstitutionId, cx: &App) -> Option<String> {
        let ledger = self.open().ledger.read(cx).ledger();
        let unit = ledger
            .institutions()
            .iter()
            .find(|institution| institution.id == id)?
            .usual_unit?;
        Some(ledger.unit_code(unit).to_string())
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
            self.open().views.settings.read(cx).selected_section(),
            self.open().views.import.read(cx).is_open(),
        )
    }

    /// The active View as `Root` forwards to it (ADR-0037).
    fn active_view_entity(&self, cx: &App) -> ActiveViewEntity {
        ActiveViewEntity::new(self.active_view(cx), &self.open().views)
    }

    /// Hands the Display Preferences to the Views that read them and the Toasts Preference to
    /// the Toasts, after a Settings change.
    fn push_display_preferences(&mut self, display: DisplayPreferences, cx: &mut App) {
        self.presenter
            .toast(ToastInput::Display(lib_toast::Display {
                toasts_on: display.toasts_on,
            }));
        let (date_style, glyphs, density) = (
            display.date_style,
            display.status_glyphs,
            display.row_density,
        );
        self.open()
            .views
            .dashboard
            .update(cx, |view, cx| view.set_display(date_style, glyphs, cx));
        self.open()
            .views
            .bills
            .update(cx, |view, cx| view.set_display(date_style, glyphs, cx));
        self.open().views.transactions.update(cx, |view, cx| {
            view.set_display(date_style, glyphs, density, cx)
        });
        self.open()
            .views
            .documents
            .update(cx, |view, cx| view.set_display(date_style, cx));
        self.open()
            .views
            .budgets
            .update(cx, |page, cx| page.set_display(date_style, cx));
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
        let mut ledger = lib_ledger::Ledger::stub();
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
            for (_, action) in settings_plain_page_hints() {
                assert!(action.starts_with('['), "{action}");
            }
        });
    }
}
