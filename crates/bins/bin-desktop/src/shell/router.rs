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
//! built on a Ledger, which `Root` keeps as an `Option` (#975). With no Ledger open `Root` holds
//! none, draws the Dashboard's no Ledger open content and Settings' This device pages, and refuses
//! every input that needs a Ledger on the status line (#977).
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
pub use log_feed::LOG_COALESCE;
pub use open_ledger::{LedgerFile, OpenLedger};
#[doc(hidden)]
pub use snapshots::{
    BillRowSnapshot, BillsSnapshot, BudgetRowSnapshot, BudgetsSnapshot, ChipSnapshot,
    DashboardBillSnapshot, DocumentsSnapshot, HelpGroup, ImportRowSnapshot, ImportSnapshot,
    PageSnapshot, PaletteSnapshot, SettingsAccountsSnapshot, SettingsCategoriesSnapshot,
    SettingsDisplaySnapshot, SettingsDocumentTypesSnapshot, SettingsGeneralSnapshot,
    SettingsInstitutionsSnapshot, SettingsInventorySnapshot, SettingsLedgerSnapshot,
    SettingsPayeesSnapshot, SettingsSnapshot, SettingsTagsSnapshot, SettingsTracingSnapshot,
    SettingsUnitsSnapshot, StatusSnapshot, ToastSnapshot, ToastsSnapshot, TransactionsSnapshot,
    UnitPricesSnapshot,
};
pub use write_failures::WRITE_FAILURE_POLL;

use std::time::Duration;

use crate::persistence::PersistedState;

/// The Shell's Presenter over the Desktop's Settings pages.
type ShellPresenter = lib_shell::ShellPresenter<SettingsPage>;

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, ScrollHandle, Subscription, Task,
};

use lib_shell::{NavInput, ToastInput};

use crate::view::active::ActiveViewEntity;
use crate::view::event::ViewMode;
use crate::{
    documents::{DocumentsMode, LibraryScope, LibrarySort},
    navigation::active_view::ActiveView,
    navigation::explorer::{ExplorerFilters, FileExplorer},
    navigation::nav::{NavState, Noun, RailMode},
    settings::{SettingsPage, device::DeviceConfigFile, router::PageChanged, state::SettingsView},
    shell::state::ShellState,
    shell::view::{rail as rail_view, window_size::WindowSize},
};
use lib_settings::DisplayOutput;
use std::rc::Rc;

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
/// off. `ledger` is the Ledger file start-up opened (the app's SQLite adapters or a test's
/// in-memory ports), which the Shell opens as its Open Ledger through [`OpenLedger::build`]; `None`
/// starts with no Ledger open. `start_view` is where it lands (ADR-0053,
/// [`crate::start_view::landing`]): `last` reads the noun the state file remembers, and a noun that
/// needs a Ledger lands on the Dashboard with none open.
#[expect(
    clippy::too_many_arguments,
    reason = "each is a separate thing start-up read; bundling them would only rename the list"
)]
pub fn build(
    persisted: PersistedState,
    start_view: &lib_core::StartView,
    bindings: ShellBindings,
    logs: ShellLogs,
    ledger: Option<LedgerFile>,
    clock: lib_core::Clock,
    focus_handle: FocusHandle,
    cx: &mut Context<'_, Root>,
) -> Root {
    crate::input::install(cx.weak_entity(), cx);
    let today = clock.today();
    let noun = crate::start_view::landing(start_view, persisted.noun, ledger.is_some());
    let nav = NavState::starting_at(noun);
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

    let mut shell = Root::with_clock(nav, focus_handle, clock, cx);
    shell.set_keymaps(&bindings.keybindings, cx);
    if let Some(ledger) = ledger {
        shell.open_ledger(ledger, cx);
    }
    shell.set_start_sidebar_minimised(persisted.start_sidebar_minimised, cx);
    shell.restore_settings_page(settings_page, cx);
    shell.set_explorer_filters(persisted.explorer_filters);
    shell.set_documents_state(documents_state.0, documents_state.1, documents_state.2, cx);
    shell.start_toast_clock(cx);
    shell.start_date_clock(cx);
    shell.set_log_capture(
        logs.buffer,
        crate::settings::tracing::log::opening_level(logs.configured_level),
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
    /// The Ledger folder (`data`), where `:open` and `:new` start browsing; `None` until start-up
    /// sets this device's settings.
    ledger_folder: Option<std::path::PathBuf>,
    /// The `[keybindings]` in effect, which a Ledger opened at run time builds its keymaps from.
    keybindings: lib_config::KeyBindingConfig,
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
    /// Set by `:close` and cleared when a Ledger is next opened: the state file then remembers no
    /// last Ledger, rather than the one remembered before this run.
    last_ledger_forgotten: bool,
    /// The Documents mode, Library scope and sort the state file remembers, or the last Ledger
    /// closed left: put on each Ledger's Documents page as it opens, so one opened while the app
    /// runs gets them back too.
    documents_state: Option<(DocumentsMode, LibraryScope, LibrarySort)>,
    /// The Settings View, which stands with no Ledger open for its This device pages; the Open
    /// Ledger gives it its This Ledger pages (#977).
    settings: Entity<SettingsView>,
    /// Keeps the Settings View's events reaching `Root` and its changes repainting it.
    _settings_subscriptions: Vec<Subscription>,
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
    /// [`Self::open_ledger`]'s. Settings comes up with its This device pages.
    pub fn with_clock(
        nav: NavState,
        focus_handle: FocusHandle,
        clock: lib_core::Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let today = clock.today();
        let settings = cx.new(SettingsView::new);
        let _settings_subscriptions = vec![
            Self::view_event_subscription(&settings, cx),
            // The frame's Router says which page the person moved to, for Back and Forward.
            cx.subscribe(&settings, |shell, _, PageChanged(page), cx| {
                shell.navigate(NavInput::SettingsPage(*page));
                cx.notify();
            }),
            // The status line reads the page's state, so a change in it repaints `Root` too.
            cx.observe(&settings, |_, _, cx| cx.notify()),
        ];
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
                hosted: None,
                dialog_keymap: lib_shell::dialog_keymap(&lib_config::KeyBindingConfig::default()),
                undo_menu_open: false,
            },
            #[cfg(debug_assertions)]
            debug_toast_kind: 0,
            command_history: Vec::new(),
            keybindings: lib_config::KeyBindingConfig::default(),
            ledger_folder: None,
            palette_entries: command::entries(&lib_config::KeyBindingConfig::default()),
            file_explorer: None,
            last_ledger_forgotten: false,
            documents_state: None,
            explorer_filters: ExplorerFilters::default(),
            open: None,
            settings,
            _settings_subscriptions,
            today,
            presenter: ShellPresenter::new(clock, nav),
        }
    }

    /// Opens `ledger` as the Open Ledger, building everything on it under the `[keybindings]` in
    /// effect, in place of any Ledger open before, which is dropped once the new one is built.
    /// Start-up calls it on the file it opened; `:open` on the file picked, once it opened.
    pub fn open_ledger(&mut self, ledger: LedgerFile, cx: &mut Context<'_, Self>) {
        let clock = self.presenter.clock().clone();
        let keybindings = self.keybindings.clone();
        let opened = OpenLedger::build(ledger, &keybindings, &clock, &self.settings, cx);
        self.remember_documents_state(cx);
        if let Some(replaced) = self.open.replace(opened) {
            // Reported before it drops, as `:close` does, so a write it lost is never silent.
            self.report_failures_on_close(&replaced);
        }
        if let Some((mode, scope, sort)) = self.documents_state {
            self.set_documents_state(mode, scope, sort, cx);
        }
        self.last_ledger_forgotten = false;
        // The Views that read their keys and Display Preferences after construction take
        // `[keybindings]` and the Preferences here, so the Open Ledger comes up under them whoever
        // opens it.
        self.set_keymaps(&keybindings, cx);
        let display = self.display_output(cx);
        self.push_display_preferences(display, cx);
        self.refresh_rail_counts(true, cx);
    }

    /// Reads the rail's badge counts into the Shell's Presenter from the Open Ledger's stores, or
    /// drops them with none open. `send` calls it once each input is committed, and the Ledger
    /// opening and closing do, `changed` because the Ledger itself is new or gone. The counts are
    /// read again only when the Ledger announced a change since the last read (or the day moved),
    /// so a cursor move costs no read.
    pub(crate) fn refresh_rail_counts(&mut self, changed: bool, cx: &mut App) {
        let announced = self
            .open
            .as_ref()
            .is_some_and(|open| open.ledger.update(cx, |store, _| store.take_announced()));
        let reads = self
            .open
            .as_ref()
            .map(|open| &open.rail_reads as &dyn lib_shell::ShellRailCounts);
        self.presenter
            .refresh_rail_counts(reads, changed || announced);
    }

    /// Closes the Open Ledger (`:close`): flushes its writes, drops everything built on it and
    /// lands on the Dashboard's no Ledger open content. A write that failed since the last poll
    /// raises one Error Toast, since there is nothing left to put back. The remembered last Ledger
    /// is cleared, so the next run starts with none.
    pub(super) fn close_ledger(&mut self, cx: &mut Context<'_, Self>) {
        let Some(open) = self.open.take() else {
            return;
        };
        self.documents_state = Some(open.views.documents.read(cx).persisted());
        self.report_failures_on_close(&open);
        crate::input::close_ledger(cx);
        // Dropped here, once the failures are read: the writer flushes again, which finds nothing.
        drop(open);
        self.last_ledger_forgotten = true;
        self.settings.update(cx, |view, cx| view.close_ledger(cx));
        self.set_last_ledger(None, cx);
        self.show_noun(Noun::Dashboard, cx);
        // Every place in the history was on the closed Ledger, so Back and Forward must not reach
        // one with none open.
        self.presenter.forget_history();
        self.refresh_rail_counts(true, cx);
        cx.notify();
    }

    /// Flushes the closing Ledger `open` and reports a write that failed since the last poll: each
    /// is logged, and one Error Toast names the file, since there is nothing left to put back.
    /// `:close` calls it, and `:open` or `:new` on the Ledger they replace.
    fn report_failures_on_close(&mut self, open: &OpenLedger) {
        let failures = open.flush_and_take_failures();
        for failure in &failures {
            tracing::error!(
                error = %failure.error,
                domains = ?failure.domains,
                "A write failed as the Ledger closed"
            );
        }
        tracing::info!(path = ?open.path(), "Closed the Ledger file");
        if !failures.is_empty() {
            let name = open.path().map(explorer::file_name).unwrap_or_default();
            self.raise_toast(
                lib_toast::ToastKind::Error,
                crate::msg::desktop_toast_write_failed_on_close(&name),
            );
        }
    }

    /// Whether a Ledger is open: whether `Root` holds an Open Ledger.
    pub fn ledger_open(&self) -> bool {
        self.open.is_some()
    }

    /// The Open Ledger's file, which the status line shows and the state file remembers at quit;
    /// `None` with no Ledger open, or on a test's in-memory stores.
    pub fn ledger_path(&self) -> Option<&std::path::Path> {
        self.open.as_ref()?.path()
    }

    /// Whether `:close` has left no Ledger to remember: the state file then keeps no last Ledger.
    pub fn last_ledger_forgotten(&self) -> bool {
        self.last_ledger_forgotten
    }

    /// Waits for the Open Ledger's writes to land in its file, as the app quits or before another
    /// file is opened in its place.
    pub fn flush_ledger(&self) {
        if let Some(open) = &self.open {
            open.flush();
        }
    }

    /// Refuses an input that needs a Ledger while none is open, saying so on the status line.
    /// Whether it refused.
    fn refuse_without_ledger(&mut self) -> bool {
        if self.ledger_open() {
            return false;
        }
        self.set_status(crate::msg::desktop_status_refused_no_ledger(
            ":open", ":new",
        ));
        true
    }

    /// Refuses showing `noun` while no Ledger is open, unless it stands without one
    /// ([`crate::navigation::nav::needs_ledger`]). Whether it refused.
    fn refuse_noun_without_ledger(&mut self, noun: Noun) -> bool {
        crate::navigation::nav::needs_ledger(noun) && self.refuse_without_ledger()
    }

    /// The Ledger's read copy (ADR-0038), `None` with no Ledger open.
    fn ledger<'a>(&self, cx: &'a App) -> Option<&'a lib_ledger::Ledger> {
        Some(self.open.as_ref()?.ledger.read(cx).ledger())
    }

    /// The Open Ledger, for the test support and snapshots, which a test reads only on a Shell
    /// built on a Ledger. Everything else asks `self.open` and has a no Ledger open path.
    #[expect(
        clippy::expect_used,
        reason = "test support only: a test reading Ledger state builds its Shell on a Ledger"
    )]
    fn open(&self) -> &OpenLedger {
        self.open
            .as_ref()
            .expect("a test reading Ledger state builds its Shell on a Ledger")
    }

    /// Restores the Documents mode, Library scope and sort from the last run, onto the Open
    /// Ledger's Documents page, and keeps them for any Ledger opened later.
    pub fn set_documents_state(
        &mut self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
        cx: &mut App,
    ) {
        self.documents_state = Some((mode, scope, sort));
        if let Some(open) = &self.open {
            open.views
                .documents
                .update(cx, |page, cx| page.set_persisted(mode, scope, sort, cx));
        }
    }

    /// Keeps the Open Ledger's Documents mode, scope and sort for the Ledger opened next.
    fn remember_documents_state(&mut self, cx: &App) {
        if let Some(open) = &self.open {
            self.documents_state = Some(open.views.documents.read(cx).persisted());
        }
    }

    /// The Documents mode, Library scope and sort, for persistence; `None` with no Ledger open,
    /// which leaves the state file's as they were.
    pub fn documents_persisted(
        &self,
        cx: &App,
    ) -> Option<(DocumentsMode, LibraryScope, LibrarySort)> {
        let open = self.open.as_ref()?;
        Some(open.views.documents.read(cx).persisted())
    }

    /// Files dropped anywhere on the window land in the Inbox like `Import…`, but the page stays
    /// where it is: the Toast and the rail badge say what arrived. With no Ledger open there is no
    /// Inbox, so the drop is refused.
    pub fn drop_documents(&mut self, paths: &[std::path::PathBuf], cx: &mut App) {
        let Some(open) = &self.open else {
            self.refuse_without_ledger();
            return;
        };
        open.views
            .documents
            .update(cx, |page, cx| page.drop_paths(paths, cx));
    }

    /// The last-visited Settings page, for persistence.
    pub fn settings_page(&self, cx: &App) -> SettingsPage {
        self.settings.read(cx).selected_section()
    }

    /// Restores the last-visited Settings page at launch, through the frame's Presenter. A This
    /// Ledger page with no Ledger open restores General instead.
    fn restore_settings_page(&mut self, section: SettingsPage, cx: &mut App) {
        let restored = self.settings.update(cx, |view, cx| {
            view.restore(section, cx);
            view.selected_section()
        });
        self.navigate(NavInput::RestoreSettingsPage(restored));
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
        self.display_output(cx).start_sidebar_minimised
    }

    pub fn set_start_sidebar_minimised(&mut self, minimised: bool, cx: &mut App) {
        self.settings.update(cx, |view, cx| {
            view.set_start_sidebar_minimised(minimised, cx)
        });
    }

    /// Applies a device's settings as start-up read them from Configuration (ADR-0053), and hands
    /// Settings › General and Display the configuration file to write them to: the date style, Row
    /// density and Status glyphs to the Views that draw with them, and Toasts to the Toasts. The Colour Theme and Appearance are
    /// the colours', set before the window opens ([`crate::theme::colours::set_device_colours`]).
    pub fn set_configuration(
        &mut self,
        config: &lib_config::PersonalLedgerConfig,
        writer: lib_config::ConfigWriter,
        cx: &mut App,
    ) {
        let device: Rc<dyn lib_settings::DeviceConfiguration> =
            Rc::new(DeviceConfigFile::new(writer));
        let general = lib_settings::GeneralSettings {
            on_start: config.on_start(),
            start_view: config.start_view().clone(),
            toasts_on: config.toasts(),
        };
        let display = lib_settings::DisplaySettings {
            date_style: config.date_style(),
            row_density: config.row_density(),
            status_glyphs: config.status_glyphs(),
            colour_theme: config.colour_theme().map(str::to_string),
            colour_appearance: config.colour_appearance(),
        };
        self.settings.update(cx, |view, cx| {
            view.configure_device(device, general, display, cx);
        });
        let output = self.display_output(cx);
        self.push_display_preferences(output, cx);
        self.push_toasts_on(config.toasts());
    }

    /// Shows `device` on Settings › General: what start-up knows of this device.
    pub fn set_this_device(&mut self, device: lib_settings::ThisDevice, cx: &mut App) {
        self.ledger_folder = Some(device.ledger_folder.clone());
        self.settings
            .update(cx, |view, cx| view.set_this_device(device, cx));
    }

    /// How many times the rail's counts were read from the stores, for a test.
    #[doc(hidden)]
    pub fn rail_count_reads(&self) -> usize {
        self.open.as_ref().map_or(0, |open| open.rail_reads.reads())
    }

    /// How many changes the Ledger announced, for a test.
    #[doc(hidden)]
    pub fn ledger_announcements(&self, cx: &App) -> usize {
        self.open
            .as_ref()
            .map_or(0, |open| open.ledger.read(cx).announcements())
    }

    /// The rail's badge counts the Shell's Presenter holds, which the rail draws; `None` with no
    /// Ledger open.
    #[doc(hidden)]
    pub fn rail_counts(&self) -> Option<lib_shell::RailCounts> {
        self.presenter.view_model().rail_counts
    }

    /// Points `:open`'s and `:new`'s file explorer at `folder`, as start-up's Ledger folder would,
    /// so a test browses a temporary folder.
    #[doc(hidden)]
    pub fn browse_from_for_test(&mut self, folder: std::path::PathBuf) {
        self.ledger_folder = Some(folder);
    }

    /// Tells the General page which Ledger file the state file will remember at quit.
    fn set_last_ledger(&mut self, last_ledger: Option<std::path::PathBuf>, cx: &mut App) {
        self.settings
            .update(cx, |view, cx| view.set_last_ledger(last_ledger, cx));
    }

    /// The Display output, which Settings › Display's Presenter holds.
    fn display_output(&self, cx: &App) -> DisplayOutput {
        self.settings.read(cx).display_output(cx)
    }

    /// The global keys and the keys every module Dialog shares, under `config`'s overrides, which
    /// the palette's key column reads too.
    /// The Transactions filter popover reads its keys through the Dialog keymap too, until it is a
    /// module Dialog (#923).
    pub fn set_keymaps(&mut self, config: &lib_config::KeyBindingConfig, cx: &mut App) {
        self.keybindings = config.clone();
        self.palette_entries = command::entries(config);
        self.presenter.set_keymap(config);
        self.state.dialog_keymap = lib_shell::dialog_keymap(config);
        self.settings
            .update(cx, |view, cx| view.set_keymaps(config, cx));
        let Some(open) = &self.open else {
            return;
        };
        open.views
            .transactions
            .update(cx, |view, _| view.set_keymaps(config));
        open.views
            .import
            .update(cx, |view, _| view.set_keymaps(config));
        open.views
            .budgets
            .update(cx, |page, _| page.set_keymaps(config));
        open.views
            .documents
            .update(cx, |page, _| page.set_keymaps(config));
    }

    /// Empties the Inventory by removing every Location through `lib-inventory`'s Interactor, so
    /// a test can reach the Inventory page's empty state.
    #[doc(hidden)]
    pub fn empty_inventory_for_test(&mut self, cx: &mut App) {
        self.open()
            .ledger
            .update(cx, |store, cx| store.empty_inventory_for_test(cx));
    }

    /// Removes every Tag through `lib-tag-removal`'s Interactor over the Tags and Transactions
    /// stores, so a test can reach the Tags page with nothing to select.
    #[doc(hidden)]
    pub fn empty_tags_for_test(&mut self, cx: &mut App) {
        let ledger = self.open().ledger.clone();
        let (tags, transactions) = {
            let store = ledger.read(cx);
            (
                store.tags_store().clone(),
                store.transactions_store().clone(),
            )
        };
        let ids: Vec<_> = tags
            .borrow()
            .tags()
            .tags()
            .iter()
            .map(|tag| tag.id)
            .collect();
        let mut removal = lib_database::TagRemovalAdapter::new(tags, transactions);
        for id in ids {
            assert!(
                lib_tag_removal::interactor::RemoveTag { id }
                    .run(&mut removal)
                    .is_ok(),
                "a Tag listed a moment ago must still be removable"
            );
        }
        ledger.update(cx, |store, cx| {
            store.announce_transactions(cx);
            store.announce_tags(cx);
        });
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
        crate::bills::ports::announce(&open.ledger, cx);
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

    /// The code of Institution `id`'s Usual unit, as the Institutions store names it and the Units
    /// store codes it: `None` for none.
    #[doc(hidden)]
    pub fn usual_unit_code(&self, id: lib_core::InstitutionId, cx: &App) -> Option<String> {
        let unit = lib_institutions::InstitutionsPorts::institutions(
            &*self.open().ledger.read(cx).institutions_ports().borrow(),
        )
        .institution(id)?
        .usual_unit?;
        Some(
            lib_database::UnitsStore::unit_code(&*self.open().units.borrow(), unit)
                .unwrap_or_default(),
        )
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
            self.settings.read(cx).selected_section(),
            self.open
                .as_ref()
                .is_some_and(|open| open.views.import.read(cx).is_open()),
        )
    }

    /// The active View as `Root` forwards to it (ADR-0037): with no Ledger open, Settings or the
    /// no Ledger open content.
    fn active_view_entity(&self, cx: &App) -> ActiveViewEntity {
        ActiveViewEntity::new(
            self.active_view(cx),
            self.open.as_ref().map(|open| &open.views),
            &self.settings,
        )
    }

    /// Hands the Toasts Preference to the Toasts, after a change on Settings › General or
    /// `:toasts`.
    fn push_toasts_on(&mut self, toasts_on: bool) {
        self.presenter
            .toast(ToastInput::Display(lib_toast::Display { toasts_on }));
    }

    /// Hands the Display output to the Views that read it, after a Settings change; with no Ledger
    /// open there are none, and opening one hands it over.
    fn push_display_preferences(&mut self, display: DisplayOutput, cx: &mut App) {
        let Some(open) = &self.open else {
            return;
        };
        let (date_style, glyphs, density) = (
            display.date_style,
            display.status_glyphs,
            display.row_density,
        );
        open.views
            .dashboard
            .update(cx, |view, cx| view.set_display(date_style, glyphs, cx));
        open.views
            .bills
            .update(cx, |view, cx| view.set_display(date_style, glyphs, cx));
        open.views.transactions.update(cx, |view, cx| {
            view.set_display(date_style, glyphs, density, cx)
        });
        open.views
            .documents
            .update(cx, |view, cx| view.set_display(date_style, cx));
        open.views
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
    use crate::settings::hints::settings_plain_page_hints;
    use crate::transactions;
    use chrono::Local;
    use lib_accounts::Account;
    use lib_categories::Category;
    use lib_settings::keymap::settings_frame_keymap;
    use lib_transactions::Transaction;
    use lib_viper::keymap::KeyBindingConfig;

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
    fn a_plain_settings_page_has_its_own_legend() {
        crate::locale::init_for_tests();
        let keymap = settings_frame_keymap(&KeyBindingConfig::default());
        let hints = settings_plain_page_hints(&keymap);
        let keys: Vec<&str> = hints.iter().map(|hint| hint.chord.as_str()).collect();
        assert_eq!(keys, ["j/k", "h"]);
        assert!(hints.iter().all(|hint| !hint.action.is_empty()));
    }

    #[test]
    fn the_category_deleted_toast_names_the_category() {
        crate::locale::init_for_tests();
        let text = lib_locale::msg::toast_category_deleted("Rent", 3);
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
    fn the_no_ledger_hint_tags_both_commands_as_clickable_spans() {
        crate::locale::init_for_tests();
        let tagged: Vec<(Option<String>, String)> =
            crate::msg::desktop_no_ledger_hint(":open", ":new")
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
    fn the_no_ledger_content_and_hint_strips_render_in_the_pseudo_locale() {
        crate::locale::init_for_tests();
        lib_locale::with_locale(lib_locale::Locale::EnXa, || {
            assert!(crate::msg::desktop_no_ledger_title().starts_with('['));
            let hint = crate::msg::desktop_no_ledger_hint(":open", ":new");
            assert!(hint.iter().any(|s| s.tag.as_deref() == Some("open")));
            assert!(hint.iter().any(|s| s.tag.as_deref() == Some("new")));
            let keymap = settings_frame_keymap(&KeyBindingConfig::default());
            for hint in settings_plain_page_hints(&keymap) {
                assert!(hint.action.starts_with('['), "{}", hint.action);
            }
        });
    }
}
