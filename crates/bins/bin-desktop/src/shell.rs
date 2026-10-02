//! `Shell` -- the desktop window's root render tree (`docs/ux/desktop/README.md`'s
//! "Component tree"), replacing `feasibility_demo::DesktopApp`'s `TabBar`-driven screen
//! cycling as the real navigation entry point (ADR-0016).
//!
//! Assembles the static chrome from `docs/ux/desktop/Shell & Navigation/README.md`'s "1a"
//! spec (issue #148) and drives it with real keyboard interaction: `Tab`/`Shift-Tab`
//! focus-zone cycling and `j`/`k`/`Down`/`Up`/`gg`/`G`/`Ctrl-d`/`Ctrl-u`/`Enter` movement,
//! scoped strictly to whichever zone (`NavState::focus`) currently has it (issue #149); the
//! `g`-prefix jump chords (`g d`, `g t`, ...) and the `Normal`/`Insert`/`Command`/`Search`
//! mode transitions (issue #150); the command palette (issue #151); the collapsed rail, its
//! `b`-key/click toggle, and its hover tooltip (issue #152). `?`'s help overlay (issue #169,
//! `InputMode::Help`). A
//! `View` trait mirroring `bin-tui`'s is still deliberately deferred (see ADR-0016):
//! `Dashboard` is the only real view, and `render_view` below is a plain match rather than a
//! trait object because there's still only one concrete implementor to dispatch to.
//!
//! `Shell` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

mod bills_ui;
mod budgets_ui;
mod documents_ui;
mod settings_ui;
mod transactions_ui;
#[doc(hidden)]
pub use bills_ui::{BillRowSnapshot, BillsSnapshot};
#[doc(hidden)]
pub use budgets_ui::{BudgetRowSnapshot, BudgetsSnapshot};
#[doc(hidden)]
pub use documents_ui::DocumentsSnapshot;
#[doc(hidden)]
pub use settings_ui::SettingsSnapshot;
#[doc(hidden)]
pub use transactions_ui::{ChipSnapshot, TransactionsSnapshot};

/// What a test can see of an open command palette.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteSnapshot {
    pub input: String,
    pub matches: Vec<&'static str>,
    pub selected: Option<&'static str>,
}

use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    Context, ExternalPaths, FocusHandle, Focusable, KeyDownEvent, Keystroke, ScrollHandle,
    ScrollStrategy, SharedString, Timer, UniformListScrollHandle, Window, div, point, prelude::*,
    px,
};

use chrono::{DateTime, Local};
use lib_core::{CategoryTypes, DateStyle};
use lib_toast::{ToastKind, Toasts};

use crate::{
    accounts::{
        self, Account, AccountField, AccountForm, AccountOptions, AccountsDialog,
        DeleteAccountForm, NameLookup, SelectKey,
    },
    bill_form, bill_history, bills, budget_form, budgets,
    categories::{self, Category},
    colours::ColourChange,
    command::{self, AccountsVerb, BudgetsVerb, Command, CommandEffect},
    documents::{self, DocumentsMode, LibraryScope, LibrarySort},
    documents_form::DocumentsDialog,
    explorer::{self, ExplorerFilter, ExplorerFilters, ExplorerMode, FileExplorer},
    format,
    import::{self, ImportState, RowSelect},
    key_router::{self, KeyOutcome, Movement, route_key},
    limit_form,
    nav::{FocusZone, InputMode, NavState, Noun},
    palette::Palette,
    pay_form::{self, PayForm},
    payees::{self, Payee},
    rail::{
        self,
        context::ContextRail,
        primary::PrimaryRail,
        settings_index::{self, SettingsIndexRail},
    },
    settings::{
        self, AccountType, AddInstitutionForm, AddUnitField, DATE_STYLE_CHOICES,
        DISPLAY_FIELD_COUNT, DISPLAY_FIELD_SIDEBAR, DeleteUnitForm, InstitutionRow, PriceSourceRow,
        RowDensity, SettingsDialog, SettingsFocus, SettingsSection, StatusGlyphs, TracingLevel,
        UnitForm, UnitKind, UnitRow, step_choice,
    },
    statusline::{self, HintAction, PageStatus, StatusLine},
    tags::{self, Tag},
    theme::{color, type_scale},
    topbar::{self, TopBar},
    transaction_chips::{self, FilterField},
    transaction_filter_form::{FilterForm, FormField, FormOptions, SelectKey as FilterSelectKey},
    transaction_query::{self, Ledger, TransactionFilters},
    transaction_rows::{self, DisplayPrefs},
    transactions::{self, Transaction},
    view::{
        accounts as accounts_view, bills as bills_view,
        budgets::{self as budgets_view, manage_dialog::ManageAction},
        categories as categories_view,
        dashboard::{self, Dashboard},
        documents::{self as documents_view, DocumentsFocus},
        help as help_view, import as import_view, payees as payees_view,
        settings::{self as settings_view, SettingsBodyProps},
        tags as tags_view, toast_history as toast_history_view, transactions as transactions_view,
    },
};

/// The collapsed rail's own hover-reveal delay (`docs/ux/desktop/Shell & Navigation/README.md`'s
/// "1c" tooltip spec) -- deliberately the same 500ms `gpui`'s own built-in `.tooltip()` uses,
/// even though this tooltip is hand-rolled (row-anchored, not cursor-anchored -- see
/// `rail::primary::collapsed_tooltip`'s doc) rather than that builtin.
const TOOLTIP_REVEAL_DELAY: Duration = Duration::from_millis(500);

/// The handoff's own "Jumps" timeout: a `g` with no completing chord within this window is
/// abandoned rather than left waiting indefinitely.
const PENDING_G_TIMEOUT: Duration = Duration::from_millis(1000);

/// A click on the empty state's own `:open`/`:new` text (issue #167): the clicked command's
/// name (`"open"` or `"new"`), looked up in `command::COMMANDS` and run exactly as the palette's
/// own `enter` key would (see [`Shell::handle_empty_state_command_click`]).
type OnEmptyStateCommandClick = Rc<dyn Fn(&'static str, &mut Window, &mut gpui::App)>;

/// Where `:open`'s file explorer starts browsing -- the handoff names no default starting
/// directory of its own, so the platform home directory is the reasonable stand-in, falling
/// back to the current directory on a platform/sandbox with no resolvable home.
fn explorer_start_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Records `name` as just-run in `history`, most-recent-first: drops any earlier occurrence
/// first so re-running a command moves it to the top rather than piling up a duplicate. Free
/// (rather than a `Palette` method) since `Shell::command_history` outlives any one `Palette`
/// instance -- see that field's own doc.
fn record_history(history: &mut Vec<String>, name: &str) {
    history.retain(|entry| entry != name);
    history.insert(0, name.to_string());
}

/// The primary rail has a fixed 10-row list, not a real "page" of variable-height content --
/// half of that is a reasonable stand-in for `Ctrl-d`/`Ctrl-u` there.
const PRIMARY_RAIL_HALF_PAGE: usize = 5;

/// A single `j`/`k`/`Down`/`Up` step in the `View` zone's own scroll, in logical pixels --
/// there's no literal "row height" for a dashboard of charts and figures, so this is a plain
/// reading-sized increment, not a computed value.
const VIEW_LINE_STEP: f32 = 40.0;

/// `Ctrl-d`/`Ctrl-u` on the Accounts page: half of a typical screenful of rows.
const ACCOUNTS_HALF_PAGE: isize = 5;
/// `Ctrl-d`/`Ctrl-u` on the Settings Categories page: rows per half page.
const CATEGORIES_HALF_PAGE: usize = 5;
/// `Ctrl-d`/`Ctrl-u` on the Settings Tags page: rows per half page.
const SETTINGS_TAGS_HALF_PAGE: isize = 5;
/// `Ctrl-d`/`Ctrl-u` on the Settings Payees page: rows per half page.
const SETTINGS_PAYEES_HALF_PAGE: isize = 5;

/// The Settings Payees page's status-line legend: the Payees page's keys without the Transactions
/// hand-off, which stays on the old page.
fn settings_payees_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_delete()),
        ("n", crate::msg::desktop_hint_new()),
    ]
}

/// The Settings Tags page's status-line legend (2j's keys), as `(key, action)`.
fn settings_tags_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit()),
        ("x", crate::msg::desktop_hint_remove()),
        ("m", crate::msg::desktop_hint_merge()),
        ("n", crate::msg::desktop_hint_new()),
    ]
}

/// The Bills Schedule tab's status-line legend (`docs/ux/desktop/Bills/README.md`'s 8a), with
/// `[/]` and `0` added for the period nav and its All toggle, and 8f's `1–5` and `f` for the
/// filter row it absorbed (#381).
fn bills_schedule_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("p", crate::msg::desktop_hint_pay()),
        ("s", crate::msg::desktop_hint_skip()),
        ("[/]", crate::msg::desktop_hint_period()),
        ("0", crate::msg::desktop_hint_all()),
        ("1\u{2013}5", crate::msg::desktop_hint_status_chips()),
        ("f", crate::msg::desktop_hint_filters()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// Where the Switcher's card starts below the window's top: under the top bar, the page's top
/// padding and the title line.
const BUDGETS_SWITCHER_TOP: gpui::Pixels = px(122.0);

/// The file name the History export's save dialog suggests.
const BUDGET_HISTORY_FILE: &str = "budget-history.csv";

/// The Budgets Progress tab's status-line legend (`docs/ux/desktop/Budgets_v2/limits-9a-9g.md`'s
/// 9a).
fn budgets_progress_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("e", crate::msg::desktop_hint_edit_budget()),
        ("c", crate::msg::desktop_hint_budget_category()),
        ("s", crate::msg::desktop_hint_stop_budgeting()),
        ("B", crate::msg::desktop_hint_switch_budget()),
        ("[/]", crate::msg::desktop_hint_period()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The Budgets Plan tab's status-line legend in Normal mode (9b).
fn budgets_plan_hints() -> Vec<(&'static str, String)> {
    vec![
        ("h/j/k/l", crate::msg::desktop_hint_cell()),
        ("enter", crate::msg::desktop_hint_edit()),
        ("r", crate::msg::desktop_hint_rollover()),
        ("x", crate::msg::desktop_hint_clear()),
        ("f", crate::msg::desktop_hint_fill()),
        ("c", crate::msg::desktop_hint_budget_category()),
        ("[/]", crate::msg::desktop_hint_range()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The Plan grid's legend while a cell is being typed into.
fn budgets_plan_insert_hints() -> Vec<(&'static str, String)> {
    vec![
        ("enter", crate::msg::desktop_hint_save_cell()),
        ("esc", crate::msg::desktop_hint_cancel()),
        ("tab", crate::msg::desktop_hint_next_month()),
        ("shift+enter", crate::msg::desktop_hint_this_month_only()),
    ]
}

/// The status-line legend while the Category detail dialog (9d) is open.
fn budgets_detail_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_close()),
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit_budget()),
        ("t", crate::msg::desktop_hint_transactions()),
    ]
}

/// The status-line legend while Edit budget (9e) is open.
fn budgets_limit_hints() -> Vec<(&'static str, String)> {
    vec![
        ("tab", crate::msg::desktop_hint_next_field()),
        ("\u{2190}/\u{2192}", crate::msg::desktop_hint_choose()),
        ("enter", crate::msg::desktop_hint_save()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}

/// The Budgets History tab's status-line legend (9c).
fn budgets_history_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("h/l", crate::msg::desktop_hint_month()),
        ("enter", crate::msg::desktop_hint_open()),
        ("x", crate::msg::desktop_hint_export()),
        ("[/]", crate::msg::desktop_hint_range()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The status-line legend while the Switcher (11b) is open.
fn budgets_switcher_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("/", crate::msg::desktop_hint_search()),
        ("n", crate::msg::desktop_hint_new_budget()),
        ("esc", crate::msg::desktop_hint_close()),
    ]
}

/// The status-line legend while Manage budgets (11f) is open.
fn budgets_manage_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("n", crate::msg::desktop_hint_new_budget()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_duplicate()),
        ("*", crate::msg::desktop_hint_set_default()),
        ("x", crate::msg::desktop_hint_archive()),
    ]
}

/// The status-line legend while Fill (9f) is open.
fn budgets_fill_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_source()),
        ("enter", crate::msg::desktop_hint_fill()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}

/// The status-line legend while Stop budgeting (9g) is open.
fn budgets_stop_hints() -> Vec<(&'static str, String)> {
    vec![
        ("\u{2191}/\u{2193}", crate::msg::desktop_hint_choose()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}

/// The Bills Planner tab's status-line legend (`docs/ux/desktop/Bills/README.md`'s 8b).
fn bills_planner_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit()),
        ("n", crate::msg::desktop_hint_new()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The Settings index rail's keys (the keyboard model's index scope): `j`/`k` swap the page live,
/// `l`/`enter` step into it.
fn settings_index_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_page()),
        ("l/enter", crate::msg::desktop_hint_open()),
    ]
}

/// The Display page's keys while a control above the Colour Theme grid has focus.
fn settings_display_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_field()),
        ("h/l", crate::msg::desktop_hint_change()),
        ("enter", crate::msg::desktop_hint_toggle()),
        ("esc", crate::msg::desktop_hint_index()),
    ]
}

/// The Display page's keys while the Colour Theme grid has focus.
fn settings_colour_grid_hints() -> Vec<(&'static str, String)> {
    vec![
        ("h/j/k/l", crate::msg::desktop_hint_colour()),
        ("enter", crate::msg::desktop_hint_choose()),
        ("esc", crate::msg::desktop_hint_field()),
    ]
}

/// The keys of a Settings page with no row or field cursor of its own (the info pages, General,
/// and the Units and Institutions tables, which are mouse-driven for now).
fn settings_plain_page_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_scroll()),
        ("h", crate::msg::desktop_hint_index()),
    ]
}

/// The status-line legend while the 7e Merge dialog is open.
fn merge_tags_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("tab", crate::msg::desktop_hint_next_field()),
        ("\u{2191}/\u{2193}", crate::msg::desktop_hint_choose()),
    ]
}

/// The status-line legend while the Add or Edit tag dialog is open (the handoff's 7b/7c), with
/// `←/→ colour` added: the swatch row has no other key. Edit adds `space` for its Active checkbox.
fn tag_dialog_hints(editing: bool) -> Vec<(&'static str, String)> {
    let mut hints = vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("tab", crate::msg::desktop_hint_next_field()),
        ("\u{2190}/\u{2192}", crate::msg::desktop_hint_colour()),
    ];
    if editing {
        hints.push(("space", crate::msg::desktop_hint_toggle_active()));
    }
    hints
}

/// The 6e Import step's status-line legend. The handoff's `enter accept suggestion` has no
/// separate key here: a suggestion is pre-selected, so `enter` continues (#290).
fn import_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("p", crate::msg::desktop_hint_payee()),
        ("c", crate::msg::desktop_hint_category()),
        ("n", crate::msg::desktop_hint_create_new_payee()),
        ("r", crate::msg::desktop_hint_remember_rules()),
        ("enter", crate::msg::desktop_hint_continue()),
        ("esc", crate::msg::desktop_hint_back()),
    ]
}

/// The status-line legend while the Pay dialog is open.
fn pay_bill_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("j/k", crate::msg::desktop_hint_choose()),
        ("\u{2190}/\u{2192}", crate::msg::desktop_hint_switch_panel()),
    ]
}

/// The status-line legend while the Skip dialog is open (the handoff's 8e).
fn skip_bill_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ]
}

/// The status-line legend while an Add or Edit payee dialog is open (the handoff's 6b).
fn payee_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("tab", crate::msg::desktop_hint_next_field()),
    ]
}

/// The status-line legend while the Delete payee or Remove tag dialog is open: each has at most
/// the one confirm field, so no `tab`.
fn delete_payee_dialog_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ]
}

/// The Accounts page's status-line legend (`docs/ux/desktop/Accounts/README.md`'s 3a), as
/// `(key, action)`.
fn accounts_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open_ledger()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_delete()),
        ("n", crate::msg::desktop_hint_new()),
    ]
}

/// The Settings Categories page's status-line legend (2i's keys), as `(key, action)`.
fn settings_categories_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("\u{2192}/\u{2190}", crate::msg::desktop_hint_expand()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_delete()),
        ("n", crate::msg::desktop_hint_new()),
        ("N", crate::msg::desktop_hint_sub()),
    ]
}

/// The status-line legend while the filter popover is open (`docs/ux/desktop/Transactions/
/// README.md`'s 4b), with `^r reset` added: the bundle's `reset` is a button, and this shell is
/// keyboard-first.
fn filter_hints() -> Vec<(&'static str, String)> {
    vec![
        ("tab", crate::msg::desktop_hint_next_field()),
        ("enter", crate::msg::desktop_hint_apply()),
        ("esc", crate::msg::desktop_hint_cancel()),
        ("^r", crate::msg::desktop_hint_reset()),
    ]
}

/// The Transactions page's status-line legend (`docs/ux/desktop/Transactions/README.md`'s 4a),
/// without the mockup's `R reconcile` (an Accounts action) and with `/` reading `search` beside a
/// separate `f filter`, since here `/` searches and `f` opens the filter popover.
fn transactions_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("e", crate::msg::desktop_hint_edit()),
        ("n", crate::msg::desktop_hint_add()),
        ("/", crate::msg::desktop_hint_search()),
        ("f", crate::msg::desktop_hint_filter()),
    ]
}

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
    /// Replaces the status line's hint strip until the next keypress -- the `g`-prefix's own
    /// "flash the hint strip" abort message, and the command palette's "not yet built" message
    /// once it closes back to `Normal` (see [`Self::run_command`]).
    status_message: Option<String>,
    /// The Toasts raised this session (`crate::toast` draws them), stamped with the local time
    /// for the history. Advanced by [`Self::start_toast_clock`]'s timer.
    toasts: Toasts<DateTime<Local>>,
    /// The pointer is over the Toast stack, which pauses the timers.
    toasts_hovered: bool,
    /// The `[keybindings] dismiss_toasts` spec, `ctrl+l` by default.
    dismiss_toasts_binding: String,
    /// The `[keybindings] toast_history` spec; unbound by default.
    toast_history_binding: Option<String>,
    /// The session Toast history is open (in `InputMode::Dialog`). Showing Toasts hide behind it.
    toast_history_open: bool,
    /// Debug builds only: the Kind `F9` raises next, so each can be eyeballed.
    #[cfg(debug_assertions)]
    debug_toast_kind: usize,
    /// The command palette's own input/selection state -- `Some` only while
    /// `NavState::mode` is `InputMode::Command`, mirroring `bin-tui`'s own
    /// `Shell`'s `Option<popup::command::CommandPopup>` (`docs/ux/desktop/README.md`'s Notes).
    palette: Option<Palette>,
    /// Previously run palette command names, most-recent-first, deduplicated -- outlives any
    /// one `Palette` (see [`record_history`]), cloned into a fresh `Palette` on every `:` open
    /// so `^r` can reach commands run in an earlier palette session.
    command_history: Vec<String>,
    /// The collapsed primary rail's row whose hover has settled past
    /// [`TOOLTIP_REVEAL_DELAY`] -- `None` while nothing's hovered, the delay hasn't elapsed
    /// yet, or the rail isn't collapsed (see `rail::primary::PrimaryRail`, which only wires
    /// hover at all in its collapsed rendering).
    collapsed_rail_tooltip: Option<Noun>,
    /// Bumped on every hover transition; a pending reveal timer checks this against the value
    /// it captured before applying, so hovering a second row (or leaving the rail entirely)
    /// before the first row's delay elapses can't reveal the wrong tooltip.
    hover_generation: u64,
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
    /// preference, not reset on noun change (same reasoning as [`Self::settings_tracing_level`]).
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
    /// The Units section's own table rows (issue #177), seeded from `settings::default_units()`.
    /// A real, mutable `Vec` so the Add/Edit/Delete unit dialogs (issues #184-#186) can
    /// push/update/remove rows once they land -- unlike
    /// [`Self::settings_selected_section`], not reset by
    /// [`Self::reset_view_scroll`]: it represents saved-in-memory state, not navigational UI
    /// state, so it must survive leaving and re-entering Settings the way real saved data would.
    settings_units: Vec<UnitRow>,
    /// The same section's own "Price Sources" subsection rows (issue #189), seeded from
    /// `settings::default_price_sources()` -- same reasoning as [`Self::settings_units`], though
    /// nothing on this map's own dialog tickets mutates this `Vec` yet (test/edit/delete/add are
    /// all clearly-marked stubs, see `view::settings::units`'s own doc).
    settings_price_sources: Vec<PriceSourceRow>,
    /// The currently open Settings dialog, if any (issue #184's own "Add unit" the first
    /// variant) -- `NavState::mode` is `InputMode::Dialog` for exactly as long as this is
    /// `Some`, the same "`Option<T>` + a matching mode" shape `Self::palette`/
    /// `Self::file_explorer` already use with `InputMode::Command`.
    settings_dialog: Option<SettingsDialog>,
    /// The Institutions section's own table rows (issue #178), seeded from
    /// `settings::default_institutions()` -- same reasoning as [`Self::settings_units`].
    settings_institutions: Vec<InstitutionRow>,
    /// The Tracing (Logs) section's own selected level (issue #182) -- a stored preference, not
    /// reset on noun change (same reasoning as [`Self::settings_units`]).
    settings_tracing_level: TracingLevel,
    /// The same section's log viewport contents, seeded from `settings::DEFAULT_LOG_LINES`.
    /// Real, mutable state -- "Clear logs" empties this `Vec`, the one button in this map with a
    /// real effect rather than a permanently-out-of-scope stub.
    settings_log_lines: Vec<&'static str>,
    /// The Accounts page's rows, seeded from `accounts::default_accounts()`. A real, mutable
    /// `Vec` the Add/Edit/Delete dialogs push to, update and remove from -- saved-in-memory
    /// state like [`Self::settings_units`], so it survives leaving and re-entering Accounts.
    accounts: Vec<Account>,
    /// The selected row as a position in `accounts::display_order(&self.accounts)` -- what
    /// `j`/`k` move -- not an index into [`Self::accounts`], since the page shows accounts
    /// grouped by type rather than in insertion order.
    accounts_selected: usize,
    /// The currently open Accounts dialog, if any -- same shape as [`Self::settings_dialog`],
    /// with `NavState::mode` being `InputMode::Dialog` for exactly as long as it is `Some`.
    accounts_dialog: Option<AccountsDialog>,
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
    /// The Budget the Budgets surface shows. The last one opened is a Client-scoped Preference, so
    /// this starts on the default Budget; the stub doesn't persist it.
    budgets_current: u32,
    budgets_tab: budgets::BudgetsTab,
    /// The Progress tab's calendar month; starts at today's.
    budgets_period: bills::Period,
    /// The selected Progress row (a position in `PeriodFigures::rows`).
    budgets_selected: usize,
    /// The open Budgets dialog, if any -- `NavState::mode` is `InputMode::Dialog` for exactly as
    /// long as this is `Some`, following `bills_dialog`.
    budgets_dialog: Option<budgets::BudgetsDialog>,
    /// The `j`/`k` cursor over the Category detail's listed Transactions.
    budgets_detail_selected: usize,
    /// The Plan grid's first month; it shows six from here.
    budgets_plan_start: bills::Period,
    /// The Plan grid's cursor: a row across sections, and a column (the last is ROLLOVER).
    budgets_plan_cursor: (usize, usize),
    /// The Plan cell being typed into; `InputMode::Insert` is on for exactly as long as this is set.
    budgets_plan_edit: Option<budgets::PlanEdit>,
    /// The last month of the History tab's range (9c).
    budgets_history_end: bills::Period,
    /// The History cursor: a row and a month column.
    budgets_history_cursor: (usize, usize),
    /// `x` on the History tab asked for the export; the key-down listener runs it, since the save
    /// dialog needs the `Context` a key handler doesn't have.
    pending_budgets_export: bool,
    /// The selected category row in the tree view (the position in a depth-first enumeration).
    categories_selected: usize,
    /// The selected category ID for keyboard navigation, if any.
    categories_selected_id: Option<u32>,
    /// Which category nodes are expanded in the tree view.
    categories_expanded: Vec<u32>,
    /// The currently open Categories dialog, if any -- `NavState::mode` is `InputMode::Dialog`
    /// for exactly as long as this is `Some`, following the pattern of `accounts_dialog`.
    categories_dialog: Option<categories::CategoriesDialog>,
    payees: Vec<Payee>,
    /// The selected row on the Payees page.
    payees_selected: usize,
    /// The selected row on Settings' Payees page, by Payee id: that page lists A–Z.
    settings_payees_selected: Option<u32>,
    /// The currently open Payees dialog, if any -- `NavState::mode` is `InputMode::Dialog` for
    /// exactly as long as this is `Some`, following the pattern of `accounts_dialog`.
    payees_dialog: Option<payees::PayeesDialog>,
    /// The stubbed 6e Import "match payees" step, `Some` while it shows in place of the
    /// Transactions page (`:import`). Dropped on leaving Transactions.
    import: Option<ImportState>,
    tags: Vec<Tag>,
    /// The selected row on the Tags page, a position in `tags::sorted_by_usage`'s order.
    tags_selected: usize,
    /// The selected row on Settings' Tags page, by Tag id: that page lists A–Z, not by usage.
    settings_tags_selected: Option<u32>,
    /// The currently open Tags dialog, if any -- `NavState::mode` is `InputMode::Dialog` for
    /// exactly as long as this is `Some`, following the pattern of `payees_dialog`.
    tags_dialog: Option<tags::TagsDialog>,
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
    bills_period: bills::Period,
    bills_all: bool,
    /// The Schedule tab's filters, and the filter-row select `f` has focused (open or closed).
    bills_filters: bill_history::BillFilters,
    bills_filter_focus: Option<(bills_view::filters::FilterField, crate::select::SelectState)>,
    /// The currently open Bills dialog, if any -- `NavState::mode` is `InputMode::Dialog` for
    /// exactly as long as this is `Some`, following the pattern of `tags_dialog`.
    bills_dialog: Option<bills::BillsDialog>,
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
    /// The Documents surface's stub Documents, and the Inventory Items they link to.
    documents: Vec<documents::Document>,
    inventory: Vec<documents::InventoryItem>,
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
    /// The open Documents dialog -- `NavState::mode` is `InputMode::Dialog` while it is `Some`.
    documents_dialog: Option<DocumentsDialog>,
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
        let documents_seed = documents::default_documents(
            &seeded_accounts,
            &categories,
            &payees,
            &bills_seed.plans,
            &mut transactions,
            today,
        );
        let seeded_budgets = budgets::default_budgets(&seeded_accounts, &categories, today);
        let budgets_current = seeded_budgets
            .default_budget()
            .map_or(budgets::PERSONAL_SPENDING_ID, |budget| budget.id);
        Self {
            nav,
            focus_handle,
            view_scroll_handle: ScrollHandle::new(),
            pending_g: None,
            status_message: None,
            toasts: Toasts::default(),
            toasts_hovered: false,
            dismiss_toasts_binding: key_router::DEFAULT_DISMISS_TOASTS.to_string(),
            toast_history_binding: None,
            toast_history_open: false,
            #[cfg(debug_assertions)]
            debug_toast_kind: 0,
            palette: None,
            command_history: Vec::new(),
            collapsed_rail_tooltip: None,
            hover_generation: 0,
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
            settings_units: settings::default_units(),
            settings_price_sources: settings::default_price_sources(),
            settings_dialog: None,
            settings_institutions: settings::default_institutions(),
            settings_tracing_level: TracingLevel::default(),
            settings_log_lines: settings::DEFAULT_LOG_LINES.to_vec(),
            accounts: accounts::default_accounts(),
            accounts_selected: 0,
            accounts_dialog: None,
            today,
            categories,
            budgets: seeded_budgets,
            budgets_current,
            budgets_tab: budgets::BudgetsTab::default(),
            budgets_period: bills::Period::of(today),
            budgets_selected: 0,
            budgets_dialog: None,
            budgets_detail_selected: 0,
            budgets_plan_start: budgets::default_plan_start(today),
            budgets_plan_cursor: (0, 0),
            budgets_plan_edit: None,
            budgets_history_end: bills::Period::of(today),
            budgets_history_cursor: (0, 0),
            pending_budgets_export: false,
            categories_selected: 0,
            categories_selected_id: None,
            categories_expanded: vec![1, 3, 6], // Housing, Utilities, Food expanded by default
            categories_dialog: None,
            payees,
            payees_selected: 0,
            settings_payees_selected: None,
            payees_dialog: None,
            import: None,
            tags,
            tags_selected: 0,
            settings_tags_selected: None,
            tags_dialog: None,
            transactions,
            bill_plans: bills_seed.plans,
            bill_entries: bills_seed.entries,
            bills_tab: bills::BillsTab::default(),
            bills_selected: 0,
            bills_period: bills::Period::of(today),
            bills_all: false,
            bills_filters: bill_history::BillFilters::default(),
            bills_filter_focus: None,
            bills_dialog: None,
            transactions_selected: 0,
            transactions_scroll: UniformListScrollHandle::new(),
            transactions_filters: TransactionFilters::defaults(today),
            transactions_search: String::new(),
            transactions_filter_form: None,
            transactions_filter_anchor: FilterField::Account,
            transactions_chip_bounds: Default::default(),
            documents: documents_seed.documents,
            inventory: documents_seed.inventory,
            documents_mode: DocumentsMode::default(),
            documents_scope: LibraryScope::default(),
            documents_sort: LibrarySort::default(),
            documents_query: String::new(),
            documents_focus: DocumentsFocus::default(),
            documents_selected: 0,
            documents_scroll: documents_ui::new_scroll(),
            documents_inbox_selected: 0,
            documents_undo: None,
            documents_dialog: None,
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
        self.dismiss_toasts_binding = spec;
    }

    pub fn set_toast_history_binding(&mut self, spec: Option<String>) {
        self.toast_history_binding = spec;
    }

    /// Opens the session Toast history as a modal, which pauses the Toast timers.
    fn open_toast_history(&mut self) {
        self.palette = None;
        self.toast_history_open = true;
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Sets the Client-scoped Toasts Preference (ADR-0027), held in memory like the Colour Theme
    /// Preferences. The Desktop can always draw a Toast, so only `toasts_on` ever changes.
    pub fn set_toasts_on(&mut self, on: bool) {
        self.toasts.set_display(lib_toast::Display {
            toasts_on: on,
            ..self.toasts.display()
        });
    }

    /// Raises a Toast whose Message the caller has already resolved to text.
    pub fn raise_toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.toasts.raise(kind, text, Local::now());
    }

    /// Starts the Toast clock for the window's life: every [`crate::toast::TICK`] it advances
    /// the model by the real time elapsed, paused while the pointer is over the stack or a modal
    /// surface is open, and redraws only when a Toast has gone.
    pub fn start_toast_clock(&self, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            let mut last = Instant::now();
            loop {
                Timer::after(crate::toast::TICK).await;
                let now = Instant::now();
                let elapsed = now - last;
                last = now;
                // Stops once the window, and with it the Shell, has gone.
                if this
                    .update(cx, |shell, cx| {
                        if shell.advance_toasts(elapsed) {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// One clock tick; `true` when the stack changed and needs a redraw.
    fn advance_toasts(&mut self, elapsed: Duration) -> bool {
        let before = (
            self.toasts.visible().len(),
            self.toasts.more_count(),
            self.toasts.echo().is_some(),
        );
        if before.0 == 0 {
            // A dismissed stack never reports the pointer leaving it.
            self.toasts_hovered = false;
            return false;
        }
        if self.toasts_hovered || self.modal_open() {
            self.toasts.pause();
        } else {
            self.toasts.resume();
        }
        self.toasts.advance(elapsed);
        before
            != (
                self.toasts.visible().len(),
                self.toasts.more_count(),
                self.toasts.echo().is_some(),
            )
    }

    /// Whether a modal surface is open -- the palette, the file explorer, a dialog, the filter
    /// popover or the help overlay -- which pauses the Toast timers.
    fn modal_open(&self) -> bool {
        self.palette.is_some()
            || self.file_explorer.is_some()
            || matches!(
                self.nav.mode(),
                InputMode::Command | InputMode::Dialog | InputMode::Filter | InputMode::Help
            )
    }

    /// Stands in for opening a ledger, so a test can reach the context rail without the file
    /// explorer's filesystem walk.
    #[doc(hidden)]
    pub fn open_ledger_for_test(&mut self) {
        self.nav.open_ledger();
    }

    /// The command palette's state for a test: `None` while it is closed.
    #[doc(hidden)]
    pub fn palette_snapshot(&self) -> Option<PaletteSnapshot> {
        self.palette.as_ref().map(|palette| PaletteSnapshot {
            input: palette.input().to_string(),
            matches: palette.match_names(),
            selected: palette.selected_command().map(|command| command.name),
        })
    }

    /// The status line's flash message (e.g. "not yet built"), if one is showing.
    #[doc(hidden)]
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
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

    /// The status line's own COMMAND-mode echo (`crate::statusline::StatusLine`'s
    /// `command_echo`): the palette's live input while it's open, or -- once `:open` has been
    /// confirmed and the palette has closed in its favour -- the file explorer's frozen
    /// `"open"` echo, each paired with its own "esc closes ..." hint text.
    fn command_echo(&self) -> Option<(String, String)> {
        if let Some(palette) = self.palette.as_ref() {
            Some((
                palette.input().to_string(),
                crate::msg::desktop_status_close_command_window("esc"),
            ))
        } else if let Some(explorer) = self.file_explorer.as_ref() {
            let name = match explorer.mode() {
                ExplorerMode::Open => "open",
                ExplorerMode::New => "new",
            };
            Some((
                name.to_string(),
                crate::msg::desktop_status_close_file_explorer("esc"),
            ))
        } else {
            None
        }
    }

    /// Routes a keystroke through the pure [`key_router::route_key`] decision function, then
    /// applies whatever [`KeyOutcome`] it returns. The routing logic itself -- `Esc`'s
    /// any-mode precedence, the `Command`-mode/other-non-`Normal` gates, a pending `g`'s
    /// completion/abort, the global mode-entry keys, `Tab` cycling, arming a fresh `g`, and
    /// [`Movement`] dispatch -- lives entirely in that `gpui`-free module now; this method is
    /// just the impure shell that owns `Shell`'s own state (`pending_g`, `status_message`,
    /// `palette`, `file_explorer`, `nav`) and applies the outcome to it. Returns `false` for a
    /// keystroke that changed nothing (nothing to redraw).
    fn handle_key_down(&mut self, event: &KeyDownEvent) -> bool {
        let keystroke = &event.keystroke;
        let ctrl = keystroke.modifiers.control;
        let shift = keystroke.modifiers.shift;
        let key = keystroke.key.as_str();

        let pending_g_active = self
            .pending_g
            .take()
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);

        let modifiers = key_router::Modifiers {
            ctrl,
            alt: keystroke.modifiers.alt,
            shift,
        };
        if key_router::dismisses_toasts(
            self.nav.mode(),
            pending_g_active,
            &self.dismiss_toasts_binding,
            key,
            modifiers,
        ) {
            self.toasts.dismiss_all();
            self.status_message = None;
            return true;
        }

        if key_router::opens_toast_history(
            self.nav.mode(),
            pending_g_active,
            self.toast_history_binding.as_deref(),
            key,
            modifiers,
        ) {
            self.open_toast_history();
            self.status_message = None;
            return true;
        }

        // Debug builds only: F9 raises each Kind in turn, to eyeball the layer before real
        // call sites exist (#346).
        #[cfg(debug_assertions)]
        if key == "f9" && self.nav.mode() == InputMode::Normal {
            let kind = ToastKind::ALL[self.debug_toast_kind % ToastKind::ALL.len()];
            self.debug_toast_kind += 1;
            self.raise_toast(kind, format!("{kind:?} Toast raised from F9"));
            return true;
        }

        if !pending_g_active && self.handle_budgets_plan_edit_key(keystroke) {
            self.status_message = None;
            return true;
        }

        if !pending_g_active && self.handle_import_key(keystroke) {
            self.status_message = None;
            return true;
        }

        if !pending_g_active && self.handle_documents_key(keystroke) {
            self.status_message = None;
            return true;
        }

        let outcome = route_key(self.nav.mode(), pending_g_active, key, ctrl, shift);

        // `Esc`'s three shapes short-circuit before the hint-strip-clearing precedent below --
        // mirrors the original tier order exactly, including the quirk that a bare `Esc` with
        // nothing to do (`EscapeNoOp`) does *not* clear a stale status message.
        match outcome {
            KeyOutcome::ClearPendingG => return true,
            KeyOutcome::ClosePopupsAndExitMode => {
                // The Accounts dialogs' dropdowns: the first `Esc` closes an open list only, the
                // next one cancels the dialog (the Desktop Accounts map's select-control decision).
                if let Some(form) = self
                    .accounts_dialog
                    .as_mut()
                    .and_then(AccountsDialog::form_mut)
                    && form.close_open_select()
                {
                    return true;
                }
                if let Some(form) = self
                    .payees_dialog
                    .as_mut()
                    .and_then(payees::PayeesDialog::form_mut)
                    && form.close_open_select()
                {
                    return true;
                }
                if let Some(tags::TagsDialog::Merge(form)) = self.tags_dialog.as_mut()
                    && form.close_open_select()
                {
                    return true;
                }
                if let Some(form) = self
                    .bills_dialog
                    .as_mut()
                    .and_then(bills::BillsDialog::plan_form_mut)
                    && form.close_open_select()
                {
                    return true;
                }
                // The Switcher's search gives the keys back to the list before the popover closes.
                if let Some(budgets::BudgetsDialog::Switcher(switcher)) =
                    self.budgets_dialog.as_mut()
                    && switcher.searching
                {
                    switcher.searching = false;
                    return true;
                }
                let closed_budgets_list = match self.budgets_dialog.as_mut() {
                    Some(budgets::BudgetsDialog::EditLimit(form)) => form.close_open_select(),
                    Some(budgets::BudgetsDialog::Budget(form)) => form.close_open_select(),
                    Some(budgets::BudgetsDialog::Stop(form)) => {
                        let open = form.from.is_open();
                        form.from.cancel();
                        open
                    }
                    _ => false,
                };
                if closed_budgets_list {
                    return true;
                }
                // The Schedule tab's filter selects: the first `Esc` closes an open list, the next
                // leaves the filter row.
                if let Some((_, state)) = self.bills_filter_focus.as_mut() {
                    if state.is_open() {
                        state.cancel();
                    } else {
                        self.bills_filter_focus = None;
                    }
                    return true;
                }
                if self.documents_dialog_close_select() {
                    return true;
                }
                self.palette = None;
                self.file_explorer = None;
                // The README's own Dialog lifecycle table: "`esc` closes any dialog without
                // saving" -- discards whatever was typed, same as Cancel.
                // The filter popover's dropdowns work the same way: the first `Esc` closes an open
                // list only, the next discards the draft and closes the popover.
                if let Some(form) = self.transactions_filter_form.as_mut()
                    && form.close_open_select()
                {
                    return true;
                }
                self.transactions_filter_form = None;
                self.settings_dialog = None;
                self.accounts_dialog = None;
                self.categories_dialog = None;
                self.payees_dialog = None;
                self.tags_dialog = None;
                self.bills_dialog = None;
                self.budgets_dialog = None;
                self.budgets_plan_edit = None;
                self.documents_dialog = None;
                self.toast_history_open = false;
                // `Esc` while searching Transactions clears the search text as well as leaving the
                // mode (the map's decision: search is cleared by `Esc` or by emptying the box).
                if self.nav.mode() == InputMode::Search && self.nav.noun() == Noun::Transactions {
                    self.transactions_search.clear();
                    self.reset_transactions_selection();
                }
                if self.nav.mode() == InputMode::Search && self.nav.noun() == Noun::Documents {
                    self.documents_cancel_search();
                }
                self.nav.exit_mode();
                return true;
            }
            KeyOutcome::EscapeNoOp => return false,
            _ => {}
        }

        // The handoff's own precedent for hint-strip messages (`docs/ux/desktop/README.md`'s
        // "Loading and error states"): any keypress clears one, not just a timer.
        let had_status_message = self.status_message.take().is_some();

        match outcome {
            KeyOutcome::DelegateToPalette => {
                self.handle_palette_key(keystroke) || had_status_message
            }
            KeyOutcome::DelegateToSearch => self.handle_search_key(keystroke) || had_status_message,
            KeyOutcome::DelegateToDialog => self.handle_dialog_key(keystroke) || had_status_message,
            KeyOutcome::DelegateToFilter => self.handle_filter_key(keystroke) || had_status_message,
            KeyOutcome::Swallowed => had_status_message,
            KeyOutcome::JumpToNoun(noun) => {
                self.nav.set_noun(noun);
                self.reset_view_scroll();
                true
            }
            KeyOutcome::PendingGUnbound(message) => {
                self.status_message = Some(message);
                true
            }
            KeyOutcome::EnterCommand => {
                self.open_palette();
                true
            }
            KeyOutcome::EnterSearch => {
                // Settings has no Search: `/` is inert there rather than opening a mode with no box.
                if self.nav.noun() == Noun::Documents {
                    self.documents_start_search();
                } else if self.nav.noun() != Noun::Settings {
                    self.nav.enter_mode(InputMode::Search);
                }
                true
            }
            KeyOutcome::EnterHelp => {
                self.nav.enter_mode(InputMode::Help);
                true
            }
            KeyOutcome::CloseHelp => {
                self.nav.exit_mode();
                true
            }
            KeyOutcome::EnterInsert => {
                self.nav.enter_mode(InputMode::Insert);
                true
            }
            // Mirrors the TopBar's own rail-toggle button (`Shell::handle_toggle_rail`) --
            // same action, two entry points. Clears any settled collapsed-rail tooltip: it's
            // meaningless once the rail that anchors it changes shape.
            KeyOutcome::ToggleRail => {
                self.nav.toggle_primary_rail();
                self.collapsed_rail_tooltip = None;
                true
            }
            KeyOutcome::CycleFocusForward => {
                self.nav.cycle_focus_forward();
                true
            }
            KeyOutcome::CycleFocusBackward => {
                self.nav.cycle_focus_backward();
                true
            }
            KeyOutcome::ArmPendingG => {
                self.pending_g = Some(Instant::now());
                had_status_message
            }
            KeyOutcome::Movement(movement) => {
                self.apply_movement(movement);
                true
            }
            KeyOutcome::NoOp => {
                self.handle_accounts_key(keystroke)
                    || self.handle_categories_key(keystroke)
                    || self.handle_payees_key(keystroke)
                    || self.handle_tags_key(keystroke)
                    || self.handle_bills_key(keystroke)
                    || self.handle_budgets_key(keystroke)
                    || self.handle_transactions_key(keystroke)
                    || had_status_message
            }
            KeyOutcome::ClearPendingG
            | KeyOutcome::ClosePopupsAndExitMode
            | KeyOutcome::EscapeNoOp => {
                unreachable!("handled above")
            }
        }
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
                self.status_message = None;
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
            SettingsSection::Display => {
                let mut keys = settings_display_hints();
                keys.extend(settings_colour_grid_hints().into_iter().take(2));
                keys
            }
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

    fn open_palette(&mut self) {
        self.nav.enter_mode(InputMode::Command);
        self.palette = Some(Palette::with_history(self.command_history.clone()));
    }

    /// A click on the status line's hint strip: the same action as the matching `Normal`-mode key,
    /// and ignored in any other mode (where those keys aren't live either).
    fn handle_hint_click(&mut self, action: HintAction, cx: &mut Context<'_, Self>) {
        if self.nav.mode() != InputMode::Normal {
            return;
        }
        self.status_message = None;
        self.pending_g = None;
        match action {
            HintAction::Command => self.open_palette(),
            HintAction::Search => self.nav.enter_mode(InputMode::Search),
            HintAction::Help => self.nav.enter_mode(InputMode::Help),
            HintAction::ToggleRail => {
                self.nav.toggle_primary_rail();
                self.collapsed_rail_tooltip = None;
            }
        }
        cx.notify();
    }

    fn apply_movement(&mut self, movement: Movement) {
        let noun_before = self.nav.noun();
        match self.nav.focus() {
            FocusZone::PrimaryRail => self.apply_primary_rail_movement(movement),
            FocusZone::ContextRail => self.apply_context_rail_movement(movement),
            FocusZone::View => self.apply_view_movement(movement),
        }
        if self.nav.noun() != noun_before {
            self.reset_view_scroll();
        }
    }

    /// A new noun's view is a different (usually much shorter) length -- carrying over the
    /// old scroll offset could leave it scrolled past all its content, rendering blank. Every
    /// fresh noun starts scrolled to the top. Also puts
    /// Settings' focus back on the index (`g s` "lands on the index"); the page on show is the
    /// last-visited one and stays.
    fn reset_view_scroll(&mut self) {
        self.view_scroll_handle.set_offset(gpui::Point::default());
        self.settings_focus = SettingsFocus::default();
        self.colour_theme_focus = None;
        // `g f` and the palette land on the Documents list, unlike Settings' index.
        self.documents_focus = DocumentsFocus::List;
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
                self.status_message = None;
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
                self.status_message = None;
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
                self.status_message = None;
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
            (SettingsFocus::Page, "h" | "left") if self.colour_theme_focus.is_none() => {
                self.status_message = None;
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

    fn apply_primary_rail_movement(&mut self, movement: Movement) {
        match movement {
            Movement::Next => self.nav.move_primary_highlight_next(),
            Movement::Prev => self.nav.move_primary_highlight_prev(),
            Movement::First => self.nav.move_primary_highlight_first(),
            Movement::Last => self.nav.move_primary_highlight_last(),
            Movement::HalfPageDown => {
                for _ in 0..PRIMARY_RAIL_HALF_PAGE {
                    self.nav.move_primary_highlight_next();
                }
            }
            Movement::HalfPageUp => {
                for _ in 0..PRIMARY_RAIL_HALF_PAGE {
                    self.nav.move_primary_highlight_prev();
                }
            }
            Movement::Enter => self.nav.commit_primary_highlight(),
        }
    }

    fn apply_context_rail_movement(&mut self, movement: Movement) {
        let count = rail::context::entity_count(self.nav.noun());
        // Every noun besides Dashboard has a placeholder context rail with nothing in it yet
        // (see `rail::context::entity_count`'s own doc) -- movement is a no-op there, not an
        // out-of-bounds index.
        if count == 0 {
            return;
        }
        let half_page = (count / 2).max(1);
        let current = self.nav.context().unwrap_or(0);
        let next = match movement {
            Movement::Next => (current + 1).min(count - 1),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            Movement::Last => count - 1,
            Movement::HalfPageDown => (current + half_page).min(count - 1),
            Movement::HalfPageUp => current.saturating_sub(half_page),
            // The handoff's own "Movement" bullet: `Enter` "selects the entity and leaves
            // focus where it is" -- but movement here already updates `context` directly
            // (rule 2 guarantees that's side-effect-free), so there's nothing left for
            // `Enter` to additionally commit until a real per-entity detail view exists
            // (out of scope for this map, per issue #144).
            Movement::Enter => current,
        };
        self.nav.set_context(Some(next));
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

    fn apply_view_movement(&mut self, movement: Movement) {
        if self.accounts_page_has_focus() {
            self.apply_accounts_movement(movement);
            return;
        }
        if self.settings_categories_page_has_focus() {
            self.apply_settings_categories_movement(movement);
            return;
        }
        if self.settings_tags_page_has_focus() {
            self.apply_settings_tags_movement(movement);
            return;
        }
        if self.settings_payees_page_has_focus() {
            self.apply_settings_payees_movement(movement);
            return;
        }
        if self.nav.noun() == Noun::Transactions {
            self.apply_transactions_movement(movement);
            return;
        }
        if self.nav.noun() == Noun::Documents {
            self.apply_documents_movement(movement);
            return;
        }
        if self.nav.noun() == Noun::Bills {
            self.apply_bills_movement(movement);
            return;
        }
        if self.nav.noun() == Noun::Budgets {
            self.apply_budgets_movement(movement);
            return;
        }
        if self.nav.noun() == Noun::Settings
            && self.settings_focus == SettingsFocus::Index
            && matches!(
                movement,
                Movement::Next | Movement::Prev | Movement::First | Movement::Last
            )
        {
            self.apply_settings_section_movement(movement);
            return;
        }
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

    /// Routes a keystroke while the palette is open (tier 2, "popup-owned keys" -- mirroring
    /// `docs/ux/tui/navigation.md`): `Backspace` mutates the input buffer, `Up`/`Down` move the
    /// selection, `Tab` completes to the selected result's full name, `Ctrl-r` cycles backward
    /// through previously run commands, `Enter` runs the selected command (see
    /// [`Self::run_command`]), and any other unmodified, printable key is typed into the query.
    /// Everything else is swallowed here rather than falling through to the zone/movement
    /// handling below -- keeping "the popup owns every keystroke" true even for a modified key
    /// (e.g. a bare `Ctrl`) this palette gives no meaning to.
    fn handle_palette_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(palette) = self.palette.as_mut() else {
            return false;
        };

        match keystroke.key.as_str() {
            "backspace" => {
                palette.backspace();
                true
            }
            "up" => {
                palette.move_up();
                true
            }
            "down" => {
                palette.move_down();
                true
            }
            "tab" => {
                palette.complete_selected();
                true
            }
            "r" if keystroke.modifiers.control => {
                palette.cycle_history_back();
                true
            }
            "enter" => {
                let command = palette.selected_command();
                let argument = palette.argument().to_string();
                self.palette = None;
                match command {
                    Some(command) => self.run_command(command, &argument),
                    // No result to run (an empty registry match) -- there's nothing left for
                    // `run_command` to do, so leave Command mode directly instead.
                    None => self.nav.exit_mode(),
                }
                true
            }
            _ => match typed_char(keystroke) {
                Some(ch) => {
                    palette.push_char(ch);
                    true
                }
                None => false,
            },
        }
    }

    /// Routes a keystroke while `InputMode::Search` is active (tier 2, mirroring
    /// [`Self::handle_palette_key`]'s shape): only Transactions has a Search box today. A no-op
    /// everywhere else.
    fn handle_search_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.noun() == Noun::Transactions {
            return self.handle_transactions_search_key(keystroke);
        }
        if self.nav.noun() == Noun::Documents {
            return self.handle_documents_search_key(keystroke);
        }
        false
    }

    /// Routes a keystroke while `InputMode::Dialog` is active (tier 2, mirroring
    /// [`Self::handle_search_key`]'s shape): extracts the open [`SettingsDialog`]'s own
    /// [`UnitForm`] regardless of which variant it is (`AddUnit`/`EditUnit` share one form type,
    /// so their keystroke handling is identical). `Tab` cycles the open dialog's own field focus
    /// rather than reaching `NavState::cycle_focus_forward` -- this tier returns before
    /// `route_key`'s `Tab` tier is ever checked, so the shell-wide zones stay untouched while a
    /// dialog is up. `Enter` submits only when the form validates, mirroring
    /// `dialog::confirm_button`'s own `enabled`-gated `on_click`.
    fn handle_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        // Read-only: `Esc` (handled before this) is its only key; the list scrolls by pointer.
        if self.toast_history_open {
            return false;
        }
        if self.accounts_dialog.is_some() {
            return self.handle_accounts_dialog_key(keystroke);
        }
        if self.categories_dialog.is_some() {
            return self.handle_categories_dialog_key(keystroke);
        }
        if self.payees_dialog.is_some() {
            return self.handle_payees_dialog_key(keystroke);
        }
        if self.tags_dialog.is_some() {
            return self.handle_tags_dialog_key(keystroke);
        }
        if self.bills_dialog.is_some() {
            return self.handle_bills_dialog_key(keystroke);
        }
        if self.budgets_dialog.is_some() {
            return self.handle_budgets_dialog_key(keystroke);
        }
        if self.documents_dialog.is_some() {
            return self.handle_documents_dialog_key(keystroke);
        }
        let Some(dialog) = self.settings_dialog.as_mut() else {
            return false;
        };

        match dialog {
            SettingsDialog::AddUnit(form) | SettingsDialog::EditUnit(_, form) => {
                match keystroke.key.as_str() {
                    "backspace" => {
                        form.backspace();
                        true
                    }
                    "tab" => {
                        form.cycle_field();
                        true
                    }
                    "enter" => {
                        let valid = form.is_valid();
                        if valid {
                            self.confirm_settings_dialog();
                        }
                        true
                    }
                    _ => match typed_char(keystroke) {
                        Some(ch) => {
                            form.push_char(ch);
                            true
                        }
                        None => false,
                    },
                }
            }
            // No `Tab` field to cycle -- the confirmation input is the dialog's only field, so
            // `Tab` is swallowed as a no-op rather than reaching the shell-wide zones.
            SettingsDialog::DeleteUnit(index, form) => {
                let index = *index;
                match keystroke.key.as_str() {
                    "backspace" => {
                        form.backspace();
                        true
                    }
                    "tab" => true,
                    "enter" => {
                        let matches = self
                            .settings_units
                            .get(index)
                            .is_some_and(|row| form.matches(&row.code));
                        if matches {
                            self.confirm_settings_dialog();
                        }
                        true
                    }
                    _ => match typed_char(keystroke) {
                        Some(ch) => {
                            form.push_char(ch);
                            true
                        }
                        None => false,
                    },
                }
            }
            // Institution name is the dialog's only text field, same shape as `DeleteUnit`'s
            // own confirm input above -- `Tab` is swallowed, and Account types/Default unit are
            // click-only (`Shell::handle_add_institution_account_type_click`/
            // `handle_add_institution_unit_click`), never typed into.
            SettingsDialog::AddInstitution(form) => match keystroke.key.as_str() {
                "backspace" => {
                    form.backspace();
                    true
                }
                "tab" => true,
                "enter" => {
                    let valid = form.is_valid();
                    if valid {
                        self.confirm_settings_dialog();
                    }
                    true
                }
                _ => match typed_char(keystroke) {
                    Some(ch) => {
                        form.push_char(ch);
                        true
                    }
                    None => false,
                },
            },
        }
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
        transaction_query::query(
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
        let selected = transaction_rows::clamp_selection(self.transactions_selected, len);
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
        let half = transaction_rows::half_page_rows(
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
                self.status_message =
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
        self.status_message = Some(message);
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
        status: transaction_query::StatusFilter,
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
        transaction_chips::clear_field(&mut self.transactions_filters, field, self.today);
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
        self.status_message = Some(crate::msg::desktop_status_add_transaction_not_yet_built());
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
                        self.status_message =
                            Some(crate::msg::desktop_status_delete_children_first());
                    } else {
                        let form = categories::DeleteCategoryForm::default();
                        self.categories_dialog =
                            Some(categories::CategoriesDialog::Delete(category.id, form));
                        self.nav.enter_mode(InputMode::Dialog);
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
    fn categories_budget_lock(&self, category_id: Option<u32>) -> Option<categories::BudgetLock> {
        if category_id.is_some_and(|id| !categories::is_leaf(&self.categories, id)) {
            return Some(categories::BudgetLock::Parent);
        }
        self.budgets
            .get(budgets::PERSONAL_SPENDING_ID)
            .filter(|budget| budget.is_archived())
            .map(|budget| categories::BudgetLock::Archived(budget.name.clone()))
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
        let form = categories::CategoryForm {
            name: String::new(),
            parent_id,
            category_type,
            budget: String::new(),
            budget_lock: self.categories_budget_lock(None),
            focused: categories::CategoryField::Name,
        };
        self.categories_dialog = Some(categories::CategoriesDialog::Add { parent_id, form });
        self.nav.enter_mode(InputMode::Dialog);
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
            let form = categories::CategoryForm {
                name: category.name.clone(),
                parent_id: category.parent,
                category_type: Some(category.category_type.clone()),
                budget: budget_str,
                budget_lock: self.categories_budget_lock(Some(category_id)),
                focused: categories::CategoryField::Name,
            };
            self.categories_dialog = Some(categories::CategoriesDialog::Edit(category_id, form));
            self.nav.enter_mode(InputMode::Dialog);
        }
    }

    /// Categories 5c's write on a saved dialog: the Monthly budget text as an Onward amount from
    /// the current month in the Personal spending Budget, or a Stop when `clears` and it is
    /// blank. A locked field (a parent's rollup, an archived Budget) writes nothing.
    fn save_category_budget(
        &mut self,
        category_id: u32,
        form: &categories::CategoryForm,
        clears: bool,
    ) {
        if form.budget_lock.is_some() {
            return;
        }
        let amount = if form.budget.trim().is_empty() {
            if !clears {
                return;
            }
            None
        } else {
            match form.budget.parse::<lib_core::Money>() {
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

    /// The Budget the Budgets surface shows and its figures for `budgets_period`, as of today.
    fn budgets_figures(&self) -> Option<(&budgets::Budget, budgets::PeriodFigures)> {
        let budget = self.budgets.get(self.budgets_current)?;
        let figures = budgets::period_figures(
            budget,
            &self.budgets_ledger(),
            self.budgets_period,
            self.today,
        );
        Some((budget, figures))
    }

    fn budgets_ledger(&self) -> budgets::Ledger<'_> {
        budgets::Ledger {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &self.bill_plans,
            entries: &self.bill_entries,
        }
    }

    /// The Plan tab's grid for the range in view.
    fn budgets_plan_data(&self) -> Option<budgets::Plan> {
        let budget = self.budgets.get(self.budgets_current)?;
        Some(budgets::plan(
            budget,
            &self.budgets_ledger(),
            self.budgets_plan_start,
            self.today,
        ))
    }

    /// The cursor held inside the grid, so a row that vanished can't leave it dangling.
    fn budgets_plan_cursor_in(&self, plan: &budgets::Plan) -> (usize, usize) {
        (
            self.budgets_plan_cursor
                .0
                .min(plan.row_count().saturating_sub(1)),
            self.budgets_plan_cursor
                .1
                .min(budgets::PLAN_ROLLOVER_COLUMN),
        )
    }

    /// Steps the Plan range a month, inside the Budget's bounds.
    fn shift_budgets_plan_range(&mut self, forward: bool) -> bool {
        let Some(budget) = self.budgets.get(self.budgets_current) else {
            return false;
        };
        let (earliest, latest) = budgets::plan_start_bounds(budget, self.today);
        let target = if forward {
            self.budgets_plan_start.next()
        } else {
            self.budgets_plan_start.prev()
        };
        if target < earliest || target > latest {
            return false;
        }
        self.budgets_plan_start = target;
        true
    }

    /// `enter`/`i`/a click on the cursor cell: types into an open month, or cycles Rollover.
    fn start_budgets_plan_edit(&mut self) {
        let Some(plan) = self.budgets_plan_data() else {
            return;
        };
        let (row_index, column) = self.budgets_plan_cursor_in(&plan);
        let Some(row) = plan.row(row_index) else {
            return;
        };
        if column == budgets::PLAN_ROLLOVER_COLUMN {
            let _ = self
                .budgets
                .cycle_rollover(self.budgets_current, row.category_id, self.today);
            return;
        }
        let Some(cell) = row.cells.get(column) else {
            return;
        };
        let archived = self
            .budgets
            .get(self.budgets_current)
            .is_none_or(budgets::Budget::is_archived);
        if cell.closed || archived {
            return;
        }
        self.budgets_plan_edit = Some(budgets::PlanEdit::new(
            row.category_id,
            cell.month,
            cell.amount.as_ref(),
        ));
        self.nav.enter_mode(InputMode::Insert);
    }

    /// Saves the typed cell and leaves Insert; `step` moves on to the next (1) or previous (-1)
    /// month's cell and edits it, as `tab`/`shift+tab` do. Text that isn't an amount keeps the
    /// cell open.
    fn commit_budgets_plan_edit(&mut self, span: budgets::Span, step: i32) {
        let Some(edit) = self.budgets_plan_edit.take() else {
            return;
        };
        let saved = self.budgets.save_cell(
            self.budgets_current,
            &self.categories,
            edit.category_id,
            edit.month,
            span,
            &edit.text,
            self.today,
        );
        if saved == Err(budgets::BudgetError::InvalidAmount) {
            self.budgets_plan_edit = Some(edit);
            return;
        }
        self.nav.exit_mode();
        if step == 0 {
            return;
        }
        let column = self.budgets_plan_cursor.1;
        if step > 0 {
            if column + 1 < budgets::PLAN_MONTHS {
                self.budgets_plan_cursor.1 += 1;
            } else if !self.shift_budgets_plan_range(true) {
                return;
            }
        } else if column > 0 {
            self.budgets_plan_cursor.1 -= 1;
        } else if !self.shift_budgets_plan_range(false) {
            return;
        }
        self.start_budgets_plan_edit();
    }

    /// Writes straight to the cursor's month cell in Normal mode: `x`/`backspace` clear it (a
    /// Stop from that month) and `0` writes an explicit 0.00 onward.
    fn write_budgets_plan_cell(&mut self, text: &str) {
        let Some(plan) = self.budgets_plan_data() else {
            return;
        };
        let (row_index, column) = self.budgets_plan_cursor_in(&plan);
        let Some(row) = plan.row(row_index) else {
            return;
        };
        let Some(cell) = row.cells.get(column).filter(|cell| !cell.closed) else {
            return;
        };
        let _ = self.budgets.save_cell(
            self.budgets_current,
            &self.categories,
            row.category_id,
            cell.month,
            budgets::Span::Onward,
            text,
            self.today,
        );
    }

    /// Keys typed into a Plan cell (`docs/ux/desktop/Budgets_v2/limits-9a-9g.md`'s 9b, Insert
    /// mode). `esc` is left to the router, which cancels the edit and leaves the mode.
    fn handle_budgets_plan_edit_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.mode() != InputMode::Insert
            || self.nav.noun() != Noun::Budgets
            || self.budgets_plan_edit.is_none()
        {
            return false;
        }
        let shift = keystroke.modifiers.shift;
        match keystroke.key.as_str() {
            "escape" => return false,
            "enter" => self.commit_budgets_plan_edit(
                if shift {
                    budgets::Span::MonthOnly
                } else {
                    budgets::Span::Onward
                },
                0,
            ),
            "tab" => {
                self.commit_budgets_plan_edit(budgets::Span::Onward, if shift { -1 } else { 1 })
            }
            "backspace" => {
                if let Some(edit) = self.budgets_plan_edit.as_mut() {
                    edit.backspace();
                }
            }
            _ => {
                if let (Some(edit), Some(text), false) = (
                    self.budgets_plan_edit.as_mut(),
                    keystroke.key_char.as_deref(),
                    keystroke.modifiers.control,
                ) {
                    for c in text.chars() {
                        edit.type_char(c);
                    }
                }
            }
        }
        true
    }

    /// The Plan grid's Normal-mode keys. `false` for any it doesn't take.
    fn handle_budgets_plan_key(&mut self, key: &str) -> bool {
        match key {
            "h" | "left" => {
                self.budgets_plan_cursor.1 = self.budgets_plan_cursor.1.saturating_sub(1);
            }
            "l" | "right" => {
                self.budgets_plan_cursor.1 =
                    (self.budgets_plan_cursor.1 + 1).min(budgets::PLAN_ROLLOVER_COLUMN);
            }
            "i" => self.start_budgets_plan_edit(),
            "r" => {
                if let Some(plan) = self.budgets_plan_data()
                    && let Some(row) = plan.row(self.budgets_plan_cursor_in(&plan).0)
                {
                    let _ = self.budgets.cycle_rollover(
                        self.budgets_current,
                        row.category_id,
                        self.today,
                    );
                }
            }
            "f" => self.open_budgets_fill(),
            "x" | "backspace" => self.write_budgets_plan_cell(""),
            "0" => self.write_budgets_plan_cell("0"),
            "[" => {
                self.shift_budgets_plan_range(false);
            }
            "]" => {
                self.shift_budgets_plan_range(true);
            }
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the Progress rows. `enter` opens the Category detail (9d).
    fn apply_budgets_movement(&mut self, movement: Movement) {
        if self.budgets_tab == budgets::BudgetsTab::Plan {
            let Some(plan) = self.budgets_plan_data() else {
                return;
            };
            let len = plan.row_count();
            let selected = self.budgets_plan_cursor_in(&plan).0;
            self.budgets_plan_cursor.0 = match movement {
                Movement::Next => accounts::step_selection(selected, len, 1),
                Movement::Prev => accounts::step_selection(selected, len, -1),
                Movement::First => 0,
                Movement::Last => len.saturating_sub(1),
                Movement::HalfPageDown => {
                    accounts::step_selection(selected, len, ACCOUNTS_HALF_PAGE)
                }
                Movement::HalfPageUp => {
                    accounts::step_selection(selected, len, -ACCOUNTS_HALF_PAGE)
                }
                Movement::Enter => {
                    self.budgets_plan_cursor.0 = selected;
                    self.start_budgets_plan_edit();
                    selected
                }
            };
            return;
        }
        if self.budgets_tab == budgets::BudgetsTab::History {
            let Some(history) = self.budgets_history_data() else {
                return;
            };
            let len = history.rows.len();
            let selected = self.budgets_history_cursor_in(&history).0;
            self.budgets_history_cursor.0 = match movement {
                Movement::Next => accounts::step_selection(selected, len, 1),
                Movement::Prev => accounts::step_selection(selected, len, -1),
                Movement::First => 0,
                Movement::Last => len.saturating_sub(1),
                Movement::HalfPageDown => {
                    accounts::step_selection(selected, len, ACCOUNTS_HALF_PAGE)
                }
                Movement::HalfPageUp => {
                    accounts::step_selection(selected, len, -ACCOUNTS_HALF_PAGE)
                }
                Movement::Enter => {
                    self.open_budgets_history_detail();
                    selected
                }
            };
            return;
        }
        let len = self
            .budgets_figures()
            .map_or(0, |(_, figures)| figures.rows.len());
        let selected = self.budgets_selected.min(len.saturating_sub(1));
        self.budgets_selected = match movement {
            Movement::Next => accounts::step_selection(selected, len, 1),
            Movement::Prev => accounts::step_selection(selected, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => accounts::step_selection(selected, len, ACCOUNTS_HALF_PAGE),
            Movement::HalfPageUp => accounts::step_selection(selected, len, -ACCOUNTS_HALF_PAGE),
            Movement::Enter => {
                self.open_budgets_detail_at(selected);
                selected
            }
        };
    }

    /// The History tab's chart and table for the range in view.
    fn budgets_history_data(&self) -> Option<budgets::History> {
        let budget = self.budgets.get(self.budgets_current)?;
        let (first, last) = budgets::history_range(budget, self.budgets_history_end, self.today)?;
        Some(budgets::history(
            budget,
            &self.budgets_ledger(),
            first,
            last,
            self.today,
        ))
    }

    /// The cursor held inside the table, so a shorter range can't leave it dangling.
    fn budgets_history_cursor_in(&self, history: &budgets::History) -> (usize, usize) {
        (
            self.budgets_history_cursor
                .0
                .min(history.rows.len().saturating_sub(1)),
            self.budgets_history_cursor
                .1
                .min(history.months.len().saturating_sub(1)),
        )
    }

    /// Steps the History range a month, inside the Budget's bounds.
    fn shift_budgets_history_range(&mut self, forward: bool) {
        let Some(budget) = self.budgets.get(self.budgets_current) else {
            return;
        };
        // Step from where the range really ends, not from a stale out-of-bounds end.
        let Some((_, last)) = budgets::history_range(budget, self.budgets_history_end, self.today)
        else {
            return;
        };
        let target = if forward { last.next() } else { last.prev() };
        if let Some((_, held)) = budgets::history_range(budget, target, self.today) {
            self.budgets_history_end = held;
        }
    }

    /// `h`/`l` on the History tab: the month cursor, stopping at either end of the range.
    fn step_budgets_history_month(&mut self, forward: bool) {
        let Some(history) = self.budgets_history_data() else {
            return;
        };
        let (row, column) = self.budgets_history_cursor_in(&history);
        let column = if forward {
            (column + 1).min(history.months.len().saturating_sub(1))
        } else {
            column.saturating_sub(1)
        };
        self.budgets_history_cursor = (row, column);
    }

    /// `enter` on a History cell: 9d for that Category and month (a parent shows its rollup).
    fn open_budgets_history_detail(&mut self) {
        let Some(history) = self.budgets_history_data() else {
            return;
        };
        let (row, column) = self.budgets_history_cursor_in(&history);
        let (Some(row), Some(month)) = (history.rows.get(row), history.months.get(column)) else {
            return;
        };
        self.budgets_detail_selected = 0;
        self.budgets_dialog = Some(budgets::BudgetsDialog::CategoryDetail {
            category_id: row.category_id,
            month: month.month,
        });
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// **Export CSV**: asks where to save through the platform's save dialog, writes the visible
    /// History table there and raises a Toast naming the path. A cancelled dialog does nothing.
    fn export_budgets_history(&mut self, cx: &mut Context<'_, Self>) {
        let Some(history) = self.budgets_history_data() else {
            return;
        };
        let labels = budgets::HistoryCsvLabels {
            category: lib_locale::msg::column_category(),
            budget: crate::msg::desktop_budgets_column_budget(),
            spent: crate::msg::desktop_budgets_column_spent(),
            average: crate::msg::desktop_budgets_history_column_avg(),
            over: crate::msg::desktop_budgets_history_column_over(),
        };
        let csv = budgets::history_csv(
            &history,
            &self.categories,
            &labels,
            budgets_view::period_label,
        );
        let directory = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_default();
        let chosen = cx.prompt_for_new_path(&directory, Some(BUDGET_HISTORY_FILE));
        cx.spawn(async move |this, cx| {
            let outcome = match chosen.await {
                Ok(Ok(Some(path))) => std::fs::write(&path, csv)
                    .map(|()| path.display().to_string())
                    .map_err(|error| error.to_string()),
                Ok(Err(error)) => Err(error.to_string()),
                // Cancelled, or the dialog went away.
                Ok(Ok(None)) | Err(_) => return,
            };
            this.update(cx, |shell, cx| {
                match outcome {
                    Ok(path) => shell.raise_toast(
                        ToastKind::Success,
                        crate::msg::desktop_budgets_history_exported(&path),
                    ),
                    Err(error) => shell.raise_toast(
                        ToastKind::Error,
                        crate::msg::desktop_budgets_history_export_failed(&error),
                    ),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn handle_budgets_export_click(&mut self, cx: &mut Context<'_, Self>) {
        self.export_budgets_history(cx);
        cx.notify();
    }

    /// A click on a History cell moves the cursor there; a second click opens its detail.
    fn handle_budgets_history_cell_click(
        &mut self,
        row: usize,
        column: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let was_here = self.budgets_history_cursor == (row, column);
        self.budgets_history_cursor = (row, column);
        if was_here {
            self.open_budgets_history_detail();
        }
        cx.notify();
    }

    /// Opens 9d on the Progress row at `index`, for the month being shown.
    fn open_budgets_detail_at(&mut self, index: usize) {
        let Some(category_id) = self
            .budgets_figures()
            .and_then(|(_, figures)| figures.rows.get(index).map(|row| row.category_id))
        else {
            return;
        };
        self.budgets_detail_selected = 0;
        self.budgets_dialog = Some(budgets::BudgetsDialog::CategoryDetail {
            category_id,
            month: self.budgets_period,
        });
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// The open Category detail's figures, or `None` once its Budget or Category is gone.
    fn budgets_detail(&self) -> Option<budgets::CategoryDetail> {
        let Some(budgets::BudgetsDialog::CategoryDetail { category_id, month }) =
            self.budgets_dialog
        else {
            return None;
        };
        let budget = self.budgets.get(self.budgets_current)?;
        let ledger = budgets::Ledger {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &self.bill_plans,
            entries: &self.bill_entries,
        };
        budgets::category_detail(budget, &ledger, category_id, month, self.today)
    }

    /// 9d's `open in Transactions →`: Transactions filtered to the Category, the month and the
    /// Budget's on-budget Accounts, following [`Self::open_payee_transactions`].
    fn open_budgets_detail_transactions(&mut self) {
        let Some(budgets::BudgetsDialog::CategoryDetail { category_id, month }) =
            self.budgets_dialog.take()
        else {
            return;
        };
        self.nav.exit_mode();
        let account_ids = self
            .budgets
            .get(self.budgets_current)
            .map(|budget| budget.account_ids.clone())
            .unwrap_or_default();
        self.transactions_filters =
            TransactionFilters::for_budget_category(category_id, month, &account_ids);
        self.transactions_search.clear();
        self.transactions_filter_form = None;
        self.reset_transactions_selection();
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll();
    }

    /// The Starting / From options for the Budget on show: `count` months from the current one.
    fn budgets_limit_options(&self, count: usize) -> Option<limit_form::LimitOptions> {
        let budget = self.budgets.get(self.budgets_current)?;
        Some(limit_form::LimitOptions::new(
            budget,
            &self.categories,
            bills::Period::of(self.today),
            count,
            |month, current| {
                let label = budgets_view::period_label(month);
                if current {
                    crate::msg::desktop_budgets_limit_month_current(&label)
                } else {
                    label
                }
            },
        ))
    }

    /// The Budget on show when it takes edits: an archived one is read-only.
    fn budgets_editable(&self) -> Option<&budgets::Budget> {
        self.budgets
            .get(self.budgets_current)
            .filter(|budget| !budget.is_archived())
    }

    /// Opens 9e on a leaf Category, starting in `month` when that is offered. A Category with no
    /// Budget Amount this month opens the picker with it preselected (9a's `set`); a parent only
    /// rolls up, so it opens nothing.
    fn open_budgets_limit(&mut self, category_id: u32, month: bills::Period) {
        let (Some(budget), Some(options)) = (
            self.budgets_editable(),
            self.budgets_limit_options(limit_form::START_MONTHS),
        ) else {
            return;
        };
        let current = bills::Period::of(self.today);
        let form = if budgets::applied(budget.chain(category_id), current).is_some() {
            if !categories::is_leaf(&self.categories, category_id) {
                return;
            }
            limit_form::LimitForm::edit(budget, category_id, month, &options)
        } else if budgets::unbudgeted_leaves(budget, &self.categories, current)
            .contains(&category_id)
        {
            limit_form::LimitForm::pick(Some(category_id), &options)
        } else {
            return;
        };
        self.budgets_dialog = Some(budgets::BudgetsDialog::EditLimit(form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// **+ Budget a category**: 9e with its Category picker.
    fn open_budgets_limit_picker(&mut self) {
        if self.budgets_editable().is_none() {
            return;
        }
        let Some(options) = self.budgets_limit_options(limit_form::START_MONTHS) else {
            return;
        };
        self.budgets_dialog = Some(budgets::BudgetsDialog::EditLimit(
            limit_form::LimitForm::pick(None, &options),
        ));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Opens 9g on a leaf Category that has a Budget Amount this month or in `month`.
    fn open_budgets_stop(&mut self, category_id: u32, month: bills::Period) {
        let (Some(budget), Some(options)) = (
            self.budgets_editable(),
            self.budgets_limit_options(limit_form::STOP_MONTHS),
        ) else {
            return;
        };
        let chain = budget.chain(category_id);
        let current = bills::Period::of(self.today);
        if budgets::applied(chain, current).is_none() && budgets::applied(chain, month).is_none() {
            return;
        }
        self.budgets_dialog = Some(budgets::BudgetsDialog::Stop(limit_form::StopForm::new(
            category_id,
            month,
            &options,
        )));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// The Category under the cursor on Progress or Plan, with the month its dialog starts in.
    fn budgets_cursor_category(&self) -> Option<(u32, bills::Period)> {
        match self.budgets_tab {
            budgets::BudgetsTab::Progress => {
                let (_, figures) = self.budgets_figures()?;
                let row = figures.rows.get(self.budgets_selected)?;
                (!row.is_parent).then_some((row.category_id, self.budgets_period))
            }
            budgets::BudgetsTab::Plan => {
                let plan = self.budgets_plan_data()?;
                let (row_index, column) = self.budgets_plan_cursor_in(&plan);
                let month = plan
                    .months
                    .get(column)
                    .copied()
                    .unwrap_or_else(|| bills::Period::of(self.today));
                Some((plan.row(row_index)?.category_id, month))
            }
            budgets::BudgetsTab::History => None,
        }
    }

    /// Runs `change` on the open Edit budget form with its options.
    fn with_budgets_limit_form(
        &mut self,
        change: impl FnOnce(&mut limit_form::LimitForm, &limit_form::LimitOptions),
    ) {
        let Some(options) = self.budgets_limit_options(limit_form::START_MONTHS) else {
            return;
        };
        if let Some(budgets::BudgetsDialog::EditLimit(form)) = self.budgets_dialog.as_mut() {
            change(form, &options);
        }
    }

    /// Runs `change` on the open Stop budgeting form with its options.
    fn with_budgets_stop_form(
        &mut self,
        change: impl FnOnce(&mut limit_form::StopForm, &limit_form::LimitOptions),
    ) {
        let Some(options) = self.budgets_limit_options(limit_form::STOP_MONTHS) else {
            return;
        };
        if let Some(budgets::BudgetsDialog::Stop(form)) = self.budgets_dialog.as_mut() {
            change(form, &options);
        }
    }

    /// **Save budget** and `enter`: writes the record and closes the dialog. A no-op while the
    /// form is incomplete; a refused save keeps the dialog open with the error shown.
    fn confirm_budgets_limit(&mut self) {
        let Some(options) = self.budgets_limit_options(limit_form::START_MONTHS) else {
            return;
        };
        let Some(budgets::BudgetsDialog::EditLimit(form)) = self.budgets_dialog.as_ref() else {
            return;
        };
        let Some(draft) = form.draft(&options) else {
            return;
        };
        let saved = limit_form::save(
            &mut self.budgets,
            self.budgets_current,
            &self.categories,
            &draft,
            self.today,
        );
        match saved {
            Ok(()) => {
                self.budgets_dialog = None;
                self.nav.exit_mode();
            }
            Err(error) => {
                if let Some(budgets::BudgetsDialog::EditLimit(form)) = self.budgets_dialog.as_mut()
                {
                    form.error = Some(error);
                }
            }
        }
    }

    /// **Stop budgeting** and `enter`: writes the Stop from the chosen month and closes.
    fn confirm_budgets_stop(&mut self) {
        let Some(options) = self.budgets_limit_options(limit_form::STOP_MONTHS) else {
            return;
        };
        let Some(budgets::BudgetsDialog::Stop(form)) = self.budgets_dialog.as_ref() else {
            return;
        };
        let Some(month) = form.month(&options) else {
            return;
        };
        let stopped = self.budgets.stop(
            self.budgets_current,
            &self.categories,
            form.category_id,
            month,
            self.today,
        );
        match stopped {
            Ok(()) => {
                self.budgets_dialog = None;
                self.nav.exit_mode();
            }
            Err(error) => {
                if let Some(budgets::BudgetsDialog::Stop(form)) = self.budgets_dialog.as_mut() {
                    form.error = Some(error);
                }
            }
        }
    }

    /// 9e's footer link: swaps Edit budget for Stop budgeting on the same Category and month.
    fn open_budgets_stop_from_limit(&mut self) {
        let Some(options) = self.budgets_limit_options(limit_form::START_MONTHS) else {
            return;
        };
        let Some(budgets::BudgetsDialog::EditLimit(form)) = self.budgets_dialog.as_ref() else {
            return;
        };
        let (Some(category_id), Some(month)) = (form.fixed_category, form.month(&options)) else {
            return;
        };
        self.open_budgets_stop(category_id, month);
    }

    /// Keys while Edit budget (9e) is open. `Esc` never reaches here.
    fn handle_budgets_limit_key(&mut self, keystroke: &Keystroke) -> bool {
        let modifiers = keystroke.modifiers;
        let Some(budgets::BudgetsDialog::EditLimit(form)) = self.budgets_dialog.as_ref() else {
            return false;
        };
        let on_select = matches!(
            form.focused,
            limit_form::LimitField::Category | limit_form::LimitField::Starting
        );
        let list_open = form.category.is_open() || form.starting.is_open();
        match keystroke.key.as_str() {
            "tab" => self.with_budgets_limit_form(|form, _| form.cycle_focus(modifiers.shift)),
            "up" | "down" if on_select => {
                let key = if keystroke.key == "up" {
                    SelectKey::Up
                } else {
                    SelectKey::Down
                };
                self.with_budgets_limit_form(|form, options| {
                    form.handle_select_key(key, options);
                });
            }
            "space" if on_select => self.with_budgets_limit_form(|form, options| {
                form.handle_select_key(SelectKey::Activate, options);
            }),
            "enter" if list_open => self.with_budgets_limit_form(|form, options| {
                form.handle_select_key(SelectKey::Activate, options);
            }),
            "left" | "right" => {
                let forward = keystroke.key == "right";
                self.with_budgets_limit_form(|form, _| {
                    form.step_segment(forward);
                });
            }
            "enter" => self.confirm_budgets_limit(),
            "backspace" => self.with_budgets_limit_form(|form, _| form.backspace()),
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(ch) = typed_char(keystroke) {
                    self.with_budgets_limit_form(|form, _| form.push_char(ch));
                }
            }
        }
        true
    }

    /// Keys while Stop budgeting (9g) is open: `up`/`down` pick the month, `space` opens its
    /// list and `enter` confirms.
    fn handle_budgets_stop_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(budgets::BudgetsDialog::Stop(form)) = self.budgets_dialog.as_ref() else {
            return false;
        };
        let list_open = form.from.is_open();
        let key = match keystroke.key.as_str() {
            "up" | "k" => SelectKey::Up,
            "down" | "j" => SelectKey::Down,
            "space" => SelectKey::Activate,
            "enter" if list_open => SelectKey::Activate,
            "enter" => {
                self.confirm_budgets_stop();
                return true;
            }
            _ => return false,
        };
        self.with_budgets_stop_form(|form, options| form.handle_select_key(key, options));
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

    /// The Budgets page's status-line legend: the open dialog's keys, else the tab's.
    fn budgets_hints(&self) -> Vec<(&'static str, String)> {
        use budgets::{BudgetsDialog, BudgetsTab};
        match (&self.budgets_dialog, self.budgets_tab) {
            (Some(BudgetsDialog::Switcher(_)), _) => budgets_switcher_hints(),
            (Some(BudgetsDialog::EditLimit(_) | BudgetsDialog::Budget(_)), _) => {
                budgets_limit_hints()
            }
            (Some(BudgetsDialog::Stop(_)), _) => budgets_stop_hints(),
            (Some(BudgetsDialog::Manage { .. }), _) => budgets_manage_hints(),
            (Some(BudgetsDialog::Fill { .. }), _) => budgets_fill_hints(),
            (Some(_), _) => budgets_detail_hints(),
            (None, BudgetsTab::Progress) => budgets_progress_hints(),
            (None, BudgetsTab::Plan) if self.budgets_plan_edit.is_some() => {
                budgets_plan_insert_hints()
            }
            (None, BudgetsTab::Plan) => budgets_plan_hints(),
            (None, BudgetsTab::History) => budgets_history_hints(),
        }
    }

    /// `B` or a click on the title: opens 11b with the Budget on show highlighted.
    fn open_budgets_switcher(&mut self) {
        self.budgets_dialog = Some(budgets::BudgetsDialog::Switcher(budgets::Switcher::new(
            &self.budgets,
            self.budgets_current,
        )));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Shows Budget `id` on the surface, keeping the active tab (every method built has all
    /// three) and putting each tab's cursor and range back at its start.
    fn switch_budget(&mut self, id: u32) {
        if self.budgets.get(id).is_none() {
            return;
        }
        self.budgets_current = id;
        self.budgets_selected = 0;
        self.budgets_plan_cursor = (0, 0);
        self.budgets_plan_edit = None;
        self.budgets_plan_start = budgets::default_plan_start(self.today);
        self.budgets_history_cursor = (0, 0);
        self.budgets_history_end = bills::Period::of(self.today);
        self.reset_view_scroll();
    }

    /// `enter` or a click on a Switcher row: switches to it and closes the popover.
    fn choose_budgets_switcher(&mut self, id: u32) {
        self.budgets_dialog = None;
        self.nav.exit_mode();
        self.switch_budget(id);
    }

    /// `n` on the Budgets page and the Switcher's `+ New budget`: 11c, with the Budget on show as
    /// what "Copy categories from" names.
    fn open_budgets_new(&mut self) {
        let form =
            budget_form::BudgetForm::new(&self.accounts, self.budgets.get(self.budgets_current));
        self.budgets_dialog = Some(budgets::BudgetsDialog::Budget(form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// 11c's edit mode on Budget `id`: rename it or change its Accounts. An archived Budget is
    /// read-only, so it opens nothing.
    fn open_budgets_edit(&mut self, id: u32) {
        let Some(budget) = self.budgets.get(id).filter(|budget| !budget.is_archived()) else {
            return;
        };
        self.budgets_dialog = Some(budgets::BudgetsDialog::Budget(
            budget_form::BudgetForm::from_budget(budget),
        ));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Runs `change` on the open New budget form with the options for its Unit, then keeps its
    /// Accounts inside the Unit it ends up in.
    fn with_budgets_form(
        &mut self,
        change: impl FnOnce(&mut budget_form::BudgetForm, &budget_form::BudgetOptions),
    ) {
        let Some(budgets::BudgetsDialog::Budget(form)) = self.budgets_dialog.as_mut() else {
            return;
        };
        let options = budget_form::BudgetOptions::new(&self.accounts, form.unit_code());
        change(form, &options);
        let options = budget_form::BudgetOptions::new(&self.accounts, form.unit_code());
        form.sync_unit(&options);
    }

    /// **Create budget** / **Save** and `enter`: creates the Budget and switches to it, or saves
    /// the rename and Accounts. A no-op while the form is incomplete; a refused save keeps the
    /// dialog open with the error shown.
    fn confirm_budgets_form(&mut self) {
        let Some(budgets::BudgetsDialog::Budget(form)) = self.budgets_dialog.as_ref() else {
            return;
        };
        let Some(draft) = form.draft() else {
            return;
        };
        let saved = match draft {
            budget_form::BudgetDraft::Create(new) => {
                let ledger = budgets::Ledger {
                    categories: &self.categories,
                    accounts: &self.accounts,
                    transactions: &self.transactions,
                    plans: &self.bill_plans,
                    entries: &self.bill_entries,
                };
                self.budgets.create(&new, &ledger, self.today).map(Some)
            }
            budget_form::BudgetDraft::Edit {
                id,
                name,
                account_ids,
            } => self
                .budgets
                .edit(id, &name, account_ids, &self.accounts)
                .map(|()| None),
        };
        match saved {
            Ok(created) => {
                self.budgets_dialog = None;
                self.nav.exit_mode();
                if let Some(id) = created {
                    self.switch_budget(id);
                }
            }
            Err(error) => {
                if let Some(budgets::BudgetsDialog::Budget(form)) = self.budgets_dialog.as_mut() {
                    form.error = Some(error);
                }
            }
        }
    }

    /// Keys while New budget (11c) is open. `Esc` never reaches here.
    fn handle_budgets_form_key(&mut self, keystroke: &Keystroke) -> bool {
        let modifiers = keystroke.modifiers;
        let Some(budgets::BudgetsDialog::Budget(form)) = self.budgets_dialog.as_ref() else {
            return false;
        };
        let focused = form.focused;
        let on_unit = focused == budget_form::BudgetField::Unit;
        let on_name = focused == budget_form::BudgetField::Name;
        let list_open = form.unit.is_open();
        match keystroke.key.as_str() {
            "tab" => self.with_budgets_form(|form, _| form.cycle_focus(modifiers.shift)),
            "up" | "down" if on_unit => {
                let key = if keystroke.key == "up" {
                    SelectKey::Up
                } else {
                    SelectKey::Down
                };
                self.with_budgets_form(|form, options| {
                    form.handle_select_key(key, options);
                });
            }
            "space" if on_unit => self.with_budgets_form(|form, options| {
                form.handle_select_key(SelectKey::Activate, options);
            }),
            "enter" if list_open => self.with_budgets_form(|form, options| {
                form.handle_select_key(SelectKey::Activate, options);
            }),
            "space" if focused == budget_form::BudgetField::Accounts => {
                self.with_budgets_form(|form, options| form.toggle_cursor_account(options));
            }
            "left" | "right" if !on_name => {
                let forward = keystroke.key == "right";
                self.with_budgets_form(|form, options| {
                    form.step(forward, options);
                });
            }
            "enter" => self.confirm_budgets_form(),
            "backspace" => self.with_budgets_form(|form, _| form.backspace()),
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(ch) = typed_char(keystroke) {
                    self.with_budgets_form(|form, _| form.push_char(ch));
                }
            }
        }
        true
    }

    fn handle_budgets_form_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_budgets_form();
        cx.notify();
    }

    /// New budget (11c), or its edit mode.
    fn render_budgets_form_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(budgets::BudgetsDialog::Budget(form)) = self.budgets_dialog.as_ref() else {
            return None;
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let options = budget_form::BudgetOptions::new(&self.accounts, form.unit_code());
        let named = form
            .editing
            .or_else(|| form.copy_source.as_ref().map(|(id, _)| *id));
        let handlers = budgets_view::budget_dialog::BudgetDialogHandlers {
            on_field_click: {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.with_budgets_form(|form, options| {
                            if field == budget_form::BudgetField::Unit {
                                form.click_unit(options);
                            } else {
                                form.focus(field);
                            }
                        });
                        cx.notify();
                    });
                })
            },
            on_unit_option_click: {
                let entity = entity.clone();
                Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.with_budgets_form(|form, options| form.choose_unit(index, options));
                        cx.notify();
                    });
                })
            },
            on_account_click: {
                let entity = entity.clone();
                Rc::new(move |id, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.with_budgets_form(|form, options| form.toggle_account(id, options));
                        cx.notify();
                    });
                })
            },
            on_start_click: {
                let entity = entity.clone();
                Rc::new(move |choice, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.with_budgets_form(|form, _| form.set_start_from(choice));
                        cx.notify();
                    });
                })
            },
            on_cancel: plain(Shell::handle_budgets_dialog_cancel),
            on_confirm: plain(Shell::handle_budgets_form_confirm),
        };
        Some(budgets_view::budget_dialog::render(
            budgets_view::budget_dialog::BudgetDialogProps {
                form,
                options: &options,
                budget_name: named
                    .and_then(|id| self.budgets.get(id))
                    .map(|budget| budget.name.clone()),
                handlers,
            },
            cx,
        ))
    }

    /// The Switcher's `Manage budgets…`: 11f, with the Budget on show under the cursor.
    fn open_budgets_manage(&mut self) {
        self.budgets_dialog = Some(budgets::BudgetsDialog::Manage {
            selected: budgets::switcher_ids(&self.budgets, "")
                .iter()
                .position(|id| *id == self.budgets_current)
                .unwrap_or(0),
            error: None,
        });
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Runs one of 11f's actions on Budget `id`. Open and Edit leave Manage budgets; Duplicate
    /// opens the copy in edit mode; the rest stay, showing a refusal when there is one. Run from
    /// the palette with Manage closed, a refusal goes to the status line instead.
    fn run_budgets_manage_action(&mut self, id: u32, action: ManageAction) {
        let outcome = match action {
            ManageAction::Open => {
                self.choose_budgets_switcher(id);
                return;
            }
            ManageAction::Edit => {
                self.open_budgets_edit(id);
                return;
            }
            ManageAction::Duplicate => self.budgets.duplicate(id).map(|copy| {
                self.switch_budget(copy);
                self.open_budgets_edit(copy);
            }),
            ManageAction::SetDefault => self.budgets.set_default(id),
            ManageAction::Archive => self.budgets.archive(id, self.today),
            ManageAction::Restore => self.budgets.restore(id),
        };
        let refused = outcome.err();
        match self.budgets_dialog.as_mut() {
            Some(budgets::BudgetsDialog::Manage { selected, error }) => {
                // Archiving or restoring moves the row between the two sections: follow it.
                *selected = budgets::switcher_ids(&self.budgets, "")
                    .iter()
                    .position(|each| *each == id)
                    .unwrap_or(0);
                *error = refused;
            }
            _ => {
                if let Some(error) = refused {
                    self.status_message = Some(match error {
                        budgets::BudgetError::DefaultCannotBeArchived => {
                            crate::msg::desktop_budgets_manage_error_default()
                        }
                        other => budgets_view::limit_dialog::error_text(&other),
                    });
                }
            }
        }
    }

    /// Keys while Manage budgets (11f) is open: `j`/`k` move, `enter` opens, `n` starts a new
    /// Budget, `e` edits, `d` duplicates, `*` sets the default and `x` archives or restores.
    fn handle_budgets_manage_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(budgets::BudgetsDialog::Manage { selected, .. }) = self.budgets_dialog.as_mut()
        else {
            return false;
        };
        let ids = budgets::switcher_ids(&self.budgets, "");
        let at = (*selected).min(ids.len().saturating_sub(1));
        let Some(id) = ids.get(at).copied() else {
            return false;
        };
        let archived = self
            .budgets
            .get(id)
            .is_some_and(budgets::Budget::is_archived);
        let shift = keystroke.modifiers.shift;
        let action = match keystroke.key.as_str() {
            "j" | "down" => {
                *selected = (at + 1).min(ids.len() - 1);
                return true;
            }
            "k" | "up" => {
                *selected = at.saturating_sub(1);
                return true;
            }
            "n" => {
                self.open_budgets_new();
                return true;
            }
            "enter" => ManageAction::Open,
            "e" => ManageAction::Edit,
            "d" => ManageAction::Duplicate,
            // `*` arrives as itself or as a shifted `8`, depending on the platform.
            "*" => ManageAction::SetDefault,
            "8" if shift => ManageAction::SetDefault,
            "x" if archived => ManageAction::Restore,
            "x" => ManageAction::Archive,
            _ => return false,
        };
        self.run_budgets_manage_action(id, action);
        true
    }

    /// Manage budgets (11f).
    fn render_budgets_manage_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(budgets::BudgetsDialog::Manage { selected, error }) = self.budgets_dialog.as_ref()
        else {
            return None;
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let current = bills::Period::of(self.today);
        let rows = budgets::switcher_ids(&self.budgets, "")
            .into_iter()
            .filter_map(|id| self.budgets.get(id))
            .map(|budget| budgets_view::manage_dialog::ManageRow {
                id: budget.id,
                name: budget.name.clone(),
                method: budget.method,
                accounts: crate::msg::desktop_budgets_manage_accounts(
                    i64::try_from(budget.account_ids.len()).unwrap_or(i64::MAX),
                ),
                scope: crate::msg::desktop_budgets_manage_scope(
                    i64::try_from(
                        budget
                            .category_ids()
                            .filter(|id| budgets::applied(budget.chain(*id), current).is_some())
                            .count(),
                    )
                    .unwrap_or(i64::MAX),
                ),
                is_default: budget.is_default,
                archived: budget.is_archived(),
            })
            .collect();
        Some(budgets_view::manage_dialog::render(
            budgets_view::manage_dialog::ManageDialogProps {
                counts: crate::msg::desktop_budgets_manage_counts(
                    &self.budgets.active().count().to_string(),
                    &self.budgets.archived().count().to_string(),
                    &self
                        .budgets
                        .default_budget()
                        .map(|budget| budget.name.clone())
                        .unwrap_or_default(),
                ),
                rows,
                selected: *selected,
                error: error.clone(),
                on_row_click: {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            if let Some(budgets::BudgetsDialog::Manage { selected, .. }) =
                                shell.budgets_dialog.as_mut()
                            {
                                *selected = index;
                            }
                            cx.notify();
                        });
                    })
                },
                on_action_click: {
                    let entity = entity.clone();
                    Rc::new(move |id, action, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.run_budgets_manage_action(id, action);
                            cx.notify();
                        });
                    })
                },
                on_new: plain(Shell::handle_budgets_new_click),
                on_close: plain(Shell::handle_budgets_dialog_cancel),
            },
            cx,
        ))
    }

    /// Keys while the Switcher (11b) is open. With the list focused `j`/`k` move, `enter` opens,
    /// `n` starts a new Budget and `/` hands the keys to the search field; while searching, every
    /// character narrows the list and only the arrows move. `Esc` never reaches here.
    fn handle_budgets_switcher_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(budgets::BudgetsDialog::Switcher(switcher)) = self.budgets_dialog.as_mut() else {
            return false;
        };
        let searching = switcher.searching;
        match keystroke.key.as_str() {
            "down" => switcher.step(&self.budgets, true),
            "up" => switcher.step(&self.budgets, false),
            "enter" => {
                if let Some(id) = switcher.chosen(&self.budgets) {
                    self.choose_budgets_switcher(id);
                }
            }
            "backspace" if searching => switcher.backspace(),
            "j" if !searching => switcher.step(&self.budgets, true),
            "k" if !searching => switcher.step(&self.budgets, false),
            "/" if !searching => switcher.searching = true,
            "n" if !searching => self.open_budgets_new(),
            _ if searching => {
                if let Some(ch) = typed_char(keystroke) {
                    switcher.type_char(ch);
                }
            }
            _ => return false,
        }
        true
    }

    fn handle_budgets_title_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_switcher();
        cx.notify();
    }

    fn handle_budgets_switcher_search_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(budgets::BudgetsDialog::Switcher(switcher)) = self.budgets_dialog.as_mut() {
            switcher.searching = true;
        }
        cx.notify();
    }

    fn handle_budgets_new_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_new();
        cx.notify();
    }

    fn handle_budgets_manage_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_manage();
        cx.notify();
    }

    /// A Budget's one-line summary in the Switcher: its health this month, or when it was
    /// archived.
    fn budgets_summary(&self, budget: &budgets::Budget) -> String {
        if let Some(date) = budget.archived_at {
            return crate::msg::desktop_budgets_switcher_archived_on(&format::date(
                date,
                self.settings_date_style,
            ));
        }
        let health = budgets::health(budget, &self.budgets_ledger(), self.today);
        let left = format::amount(&health.left).1;
        if health.at_risk > 0 {
            crate::msg::desktop_budgets_switcher_health_at_risk(
                &health.over.to_string(),
                &left,
                &health.at_risk.to_string(),
            )
        } else {
            crate::msg::desktop_budgets_switcher_health(&health.over.to_string(), &left)
        }
    }

    /// The Switcher popover (11b), anchored under the page title.
    fn render_budgets_switcher(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(budgets::BudgetsDialog::Switcher(switcher)) = self.budgets_dialog.as_ref() else {
            return None;
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let rows = budgets::switcher_ids(&self.budgets, &switcher.query)
            .into_iter()
            .filter_map(|id| self.budgets.get(id))
            .map(|budget| budgets_view::switcher::SwitcherRow {
                id: budget.id,
                name: budget.name.clone(),
                is_default: budget.is_default,
                is_current: budget.id == self.budgets_current,
                archived: budget.is_archived(),
                summary: self.budgets_summary(budget),
                method: budget.method,
            })
            .collect();
        // The page's left edge: past the primary rail and the context rail, inside its gutter.
        let rail = match self.nav.primary_rail() {
            crate::nav::RailMode::Expanded => rail::primary::WIDTH,
            crate::nav::RailMode::Collapsed => rail::primary::COLLAPSED_WIDTH,
        };
        Some(budgets_view::switcher::render(
            budgets_view::switcher::SwitcherProps {
                state: switcher,
                rows,
                left: rail + rail::context::WIDTH + px(28.0),
                top: BUDGETS_SWITCHER_TOP,
                on_search_click: plain(Shell::handle_budgets_switcher_search_click),
                on_budget_click: {
                    let entity = entity.clone();
                    Rc::new(move |id, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.choose_budgets_switcher(id);
                            cx.notify();
                        });
                    })
                },
                on_new: plain(Shell::handle_budgets_new_click),
                on_manage: plain(Shell::handle_budgets_manage_click),
                on_cancel: plain(Shell::handle_budgets_dialog_cancel),
            },
            cx,
        ))
    }

    /// The month Fill would target from the Plan cursor: the first open month at or after its
    /// column.
    fn budgets_fill_target(&self) -> bills::Period {
        let cursor = self.budgets_plan_data().and_then(|plan| {
            let column = self.budgets_plan_cursor_in(&plan).1;
            plan.months.get(column).copied()
        });
        budgets::fill_target(cursor, self.today)
    }

    /// **Fill … from…** and `f` on the Plan tab: opens 9f on the cursor's target month.
    fn open_budgets_fill(&mut self) {
        if self.budgets_editable().is_none() {
            return;
        }
        self.budgets_dialog = Some(budgets::BudgetsDialog::Fill {
            month: self.budgets_fill_target(),
            source: budgets::FillSource::default(),
        });
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// **Fill** and `enter`: writes the chosen source's Month-only amounts and closes. A Fill that
    /// would change nothing stays open, as its disabled button says.
    fn confirm_budgets_fill(&mut self) {
        let Some(budgets::BudgetsDialog::Fill { month, source }) = self.budgets_dialog else {
            return;
        };
        let ledger = budgets::Ledger {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &self.bill_plans,
            entries: &self.bill_entries,
        };
        let wrote = self
            .budgets
            .fill(self.budgets_current, &ledger, month, source, self.today);
        if matches!(wrote, Ok(count) if count > 0) {
            self.budgets_dialog = None;
            self.nav.exit_mode();
        }
    }

    fn set_budgets_fill_source(&mut self, chosen: budgets::FillSource) {
        if let Some(budgets::BudgetsDialog::Fill { source, .. }) = self.budgets_dialog.as_mut() {
            *source = chosen;
        }
    }

    /// Keys while Fill (9f) is open: `j`/`k` pick the source and `enter` fills.
    fn handle_budgets_fill_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(budgets::BudgetsDialog::Fill { source, .. }) = self.budgets_dialog else {
            return false;
        };
        let all = budgets::FillSource::ALL;
        let at = all.iter().position(|s| *s == source).unwrap_or(0);
        match keystroke.key.as_str() {
            "j" | "down" => self.set_budgets_fill_source(all[(at + 1).min(all.len() - 1)]),
            "k" | "up" => self.set_budgets_fill_source(all[at.saturating_sub(1)]),
            "enter" => self.confirm_budgets_fill(),
            _ => return false,
        }
        true
    }

    fn handle_budgets_fill_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_fill();
        cx.notify();
    }

    fn handle_budgets_fill_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_budgets_fill();
        cx.notify();
    }

    /// Fill (9f), with each source's total and the chosen one's diff against the plan.
    fn render_budgets_fill_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(budgets::BudgetsDialog::Fill { month, source }) = self.budgets_dialog else {
            return None;
        };
        let budget = self.budgets.get(self.budgets_current)?;
        let ledger = self.budgets_ledger();
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let previews = budgets::FillSource::ALL
            .map(|each| budgets::fill_preview(budget, &ledger, month, each));
        let chosen = previews.iter().find(|preview| preview.source == source)?;
        let name = |category_id: &u32| {
            self.categories
                .iter()
                .find(|category| category.id == *category_id)
                .map(|category| category.name.clone())
                .unwrap_or_default()
        };
        let sources = previews
            .iter()
            .map(|preview| {
                let (title, detail) = match preview.source {
                    budgets::FillSource::PreviousMonth => (
                        crate::msg::desktop_budgets_fill_source_previous(
                            &lib_locale::format::format_month(month.prev().month),
                        ),
                        crate::msg::desktop_budgets_fill_source_previous_detail(),
                    ),
                    budgets::FillSource::Average => (
                        crate::msg::desktop_budgets_fill_source_average(),
                        crate::msg::desktop_budgets_fill_source_average_detail(
                            &lib_locale::format::format_month(month.prev().prev().prev().month),
                            &lib_locale::format::format_month(month.prev().month),
                        ),
                    ),
                };
                budgets_view::fill_dialog::SourceRow {
                    source: preview.source,
                    title,
                    detail,
                    total: format::amount(&preview.total).1,
                }
            })
            .collect();
        Some(budgets_view::fill_dialog::render(
            budgets_view::fill_dialog::FillDialogProps {
                month: budgets_view::period_label(month),
                sources,
                preview: chosen,
                change_names: chosen
                    .changes
                    .iter()
                    .map(|line| name(&line.category_id))
                    .collect(),
                kept_names: chosen.kept.iter().map(name).collect(),
                on_source_click: {
                    let entity = entity.clone();
                    Rc::new(move |source, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.set_budgets_fill_source(source);
                            cx.notify();
                        });
                    })
                },
                on_cancel: plain(Shell::handle_budgets_dialog_cancel),
                on_confirm: plain(Shell::handle_budgets_fill_confirm),
            },
            cx,
        ))
    }

    /// Keys while a Budgets dialog is open. On the Category detail `j`/`k` move the Transaction
    /// cursor, `t` or `enter` hand off to Transactions and `e` opens Edit budget. `Esc` never
    /// reaches here.
    fn handle_budgets_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        match self.budgets_dialog {
            Some(budgets::BudgetsDialog::EditLimit(_)) => {
                return self.handle_budgets_limit_key(keystroke);
            }
            Some(budgets::BudgetsDialog::Stop(_)) => {
                return self.handle_budgets_stop_key(keystroke);
            }
            Some(budgets::BudgetsDialog::Fill { .. }) => {
                return self.handle_budgets_fill_key(keystroke);
            }
            Some(budgets::BudgetsDialog::Switcher(_)) => {
                return self.handle_budgets_switcher_key(keystroke);
            }
            Some(budgets::BudgetsDialog::Budget(_)) => {
                return self.handle_budgets_form_key(keystroke);
            }
            Some(budgets::BudgetsDialog::Manage { .. }) => {
                return self.handle_budgets_manage_key(keystroke);
            }
            _ => {}
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        let listed = self.budgets_detail().map_or(0, |detail| {
            detail.lines.len().min(budgets_view::detail_dialog::LISTED)
        });
        match keystroke.key.as_str() {
            "j" | "down" => {
                self.budgets_detail_selected =
                    accounts::step_selection(self.budgets_detail_selected, listed, 1);
            }
            "k" | "up" => {
                self.budgets_detail_selected =
                    accounts::step_selection(self.budgets_detail_selected, listed, -1);
            }
            "t" | "enter" => self.open_budgets_detail_transactions(),
            "e" => self.open_budgets_detail_limit(),
            _ => return false,
        }
        true
    }

    /// 9d's **Edit budget**: 9e on the detail's Category and month. A parent's rollup has none.
    fn open_budgets_detail_limit(&mut self) {
        if let Some(budgets::BudgetsDialog::CategoryDetail { category_id, month }) =
            self.budgets_dialog
        {
            self.open_budgets_limit(category_id, month);
        }
    }

    /// Whether 9d offers Edit budget: a leaf Expense Category in a Budget that takes edits.
    fn budgets_detail_editable(&self, category_id: u32) -> bool {
        self.budgets_editable().is_some()
            && categories::is_leaf(&self.categories, category_id)
            && self.categories.iter().any(|category| {
                category.id == category_id && category.category_type == CategoryTypes::Expense
            })
    }

    fn handle_budgets_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.budgets_dialog = None;
        self.nav.exit_mode();
        cx.notify();
    }

    fn handle_budgets_limit_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_budgets_limit();
        cx.notify();
    }

    fn handle_budgets_limit_stop(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_stop_from_limit();
        cx.notify();
    }

    fn handle_budgets_stop_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_budgets_stop();
        cx.notify();
    }

    fn handle_budgets_stop_field_click(&mut self, cx: &mut Context<'_, Self>) {
        self.with_budgets_stop_form(|form, options| form.click_select(options));
        cx.notify();
    }

    fn handle_budgets_detail_edit(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_detail_limit();
        cx.notify();
    }

    fn handle_budgets_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_limit_picker();
        cx.notify();
    }

    /// A Progress row's `edit` or `set`.
    fn handle_budgets_action_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.budgets_selected = index;
        if let Some((category_id, month)) = self.budgets_cursor_category() {
            self.open_budgets_limit(category_id, month);
        }
        cx.notify();
    }

    /// Edit budget (9e) or Stop budgeting (9g), whichever is open.
    fn render_budgets_limit_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let leaf_name = |category_id: u32| {
            self.categories
                .iter()
                .find(|category| category.id == category_id)
                .map(|category| category.name.clone())
        };
        match self.budgets_dialog.as_ref()? {
            budgets::BudgetsDialog::EditLimit(form) => {
                let options = self.budgets_limit_options(limit_form::START_MONTHS)?;
                let draft = form.draft(&options);
                let preview = draft.as_ref().and_then(|draft| {
                    limit_form::preview(
                        &self.budgets,
                        self.budgets_current,
                        &self.categories,
                        draft,
                        self.today,
                    )
                });
                let handlers = budgets_view::limit_dialog::LimitDialogHandlers {
                    on_field_click: {
                        let entity = entity.clone();
                        Rc::new(move |field, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.with_budgets_limit_form(|form, options| match field {
                                    limit_form::LimitField::Category
                                    | limit_form::LimitField::Starting => {
                                        form.click_select(field, options);
                                    }
                                    _ => form.focus(field),
                                });
                                cx.notify();
                            });
                        })
                    },
                    on_option_click: {
                        let entity = entity.clone();
                        Rc::new(move |field, index, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.with_budgets_limit_form(|form, options| {
                                    form.choose(field, index, options);
                                });
                                cx.notify();
                            });
                        })
                    },
                    on_span_click: {
                        let entity = entity.clone();
                        Rc::new(move |span, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.with_budgets_limit_form(|form, _| form.set_span(span));
                                cx.notify();
                            });
                        })
                    },
                    on_rollover_click: {
                        let entity = entity.clone();
                        Rc::new(move |rollover, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.with_budgets_limit_form(|form, _| {
                                    form.set_rollover(rollover);
                                });
                                cx.notify();
                            });
                        })
                    },
                    on_stop: (!form.is_pick()).then(|| plain(Shell::handle_budgets_limit_stop)),
                    on_cancel: plain(Shell::handle_budgets_dialog_cancel),
                    on_confirm: plain(Shell::handle_budgets_limit_confirm),
                };
                Some(budgets_view::limit_dialog::render(
                    budgets_view::limit_dialog::LimitDialogProps {
                        category: form.fixed_category.and_then(leaf_name),
                        form,
                        options: &options,
                        preview: preview.as_ref(),
                        unchanged_month: preview
                            .as_ref()
                            .and_then(|preview| preview.unchanged.as_ref())
                            .map(|(month, _)| budgets_view::period_label(*month)),
                        month: preview
                            .as_ref()
                            .map(|preview| budgets_view::period_label(preview.month))
                            .unwrap_or_default(),
                        mirrors_category_field: self.budgets_current
                            == budgets::PERSONAL_SPENDING_ID,
                        valid: draft.is_some(),
                        handlers,
                    },
                    cx,
                ))
            }
            budgets::BudgetsDialog::Stop(form) => {
                let options = self.budgets_limit_options(limit_form::STOP_MONTHS)?;
                let category = leaf_name(form.category_id)?;
                let on_option_click: accounts_view::select_field::OnOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.with_budgets_stop_form(|form, options| {
                                form.choose(index, options);
                            });
                            cx.notify();
                        });
                    })
                };
                Some(budgets_view::stop_dialog::render(
                    budgets_view::stop_dialog::StopDialogProps {
                        category: &category,
                        form,
                        options: &options,
                        bill_plans: self
                            .bill_plans
                            .iter()
                            .filter(|plan| plan.category_id == form.category_id && plan.is_active)
                            .map(|plan| plan.name.clone())
                            .collect(),
                        on_field_click: plain(Shell::handle_budgets_stop_field_click),
                        on_option_click,
                        on_cancel: plain(Shell::handle_budgets_dialog_cancel),
                        on_confirm: plain(Shell::handle_budgets_stop_confirm),
                    },
                    cx,
                ))
            }
            _ => None,
        }
    }

    fn handle_budgets_detail_close(&mut self, cx: &mut Context<'_, Self>) {
        self.budgets_dialog = None;
        self.nav.exit_mode();
        cx.notify();
    }

    fn handle_budgets_detail_transactions(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_detail_transactions();
        cx.notify();
    }

    /// The Category detail (9d) over the Progress tab, or nothing once its figures are gone.
    fn render_budgets_detail(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let detail = self.budgets_detail()?;
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let category = categories::path(&self.categories, detail.category_id).unwrap_or_default();
        let name = category
            .rsplit(" / ")
            .next()
            .unwrap_or(&category)
            .to_string();
        let listed = budgets_view::detail_dialog::LISTED;
        let lines: Vec<budgets_view::detail_dialog::LineRow> = detail
            .lines
            .iter()
            .take(listed)
            .map(|line| budgets_view::detail_dialog::LineRow {
                date: lib_locale::format::format_month_day(line.date),
                payee: line
                    .payee_id
                    .and_then(|id| payees::get(&self.payees, id))
                    .map_or_else(crate::msg::desktop_budgets_detail_no_payee, |payee| {
                        payee.name.clone()
                    }),
                account: self
                    .accounts
                    .iter()
                    .find(|account| account.id == line.account_id)
                    .map(|account| account.name.clone())
                    .unwrap_or_default(),
                amount: format::amount(&line.amount).1,
            })
            .collect();
        let hidden = detail
            .lines
            .iter()
            .skip(listed)
            .fold(bigdecimal::BigDecimal::from(0), |sum, line| {
                sum + line.amount.0.clone()
            });
        let more = (detail.lines.len() > listed).then(|| {
            crate::msg::desktop_budgets_detail_more(
                &(detail.lines.len() - listed).to_string(),
                &format::amount(&lib_core::Money(hidden)).1,
            )
        });
        let track = detail.track.as_ref().map_or_else(
            crate::msg::desktop_budgets_detail_track_none,
            |track| {
                crate::msg::desktop_budgets_detail_track(
                    &track.over_months.to_string(),
                    i64::from(track.months),
                    &format::amount(&track.average).1,
                )
            },
        );
        let next = budgets_view::period_label(detail.month.next());
        let rollover_note = detail
            .figures
            .over
            .then_some(detail.rollover)
            .flatten()
            .map(|rollover| match rollover {
                budgets::Rollover::None => crate::msg::desktop_budgets_detail_rollover_off(&next),
                budgets::Rollover::CarryUnspent => {
                    crate::msg::desktop_budgets_detail_rollover_unspent(&next)
                }
                budgets::Rollover::CarryBoth => {
                    crate::msg::desktop_budgets_detail_rollover_both(&next)
                }
            });
        let bills = match i64::try_from(detail.bill_plans).unwrap_or(i64::MAX) {
            0 => crate::msg::desktop_budgets_detail_no_bills(),
            count => crate::msg::desktop_budgets_detail_bills(count),
        };
        Some(budgets_view::detail_dialog::render(
            budgets_view::detail_dialog::DetailDialogProps {
                title: crate::msg::desktop_budgets_detail_title(
                    &name,
                    &budgets_view::period_label(detail.month),
                ),
                detail: &detail,
                lines,
                more,
                selected: self.budgets_detail_selected,
                track,
                rollover_note,
                bills,
                on_close: plain(Shell::handle_budgets_detail_close),
                on_open_transactions: plain(Shell::handle_budgets_detail_transactions),
                on_edit: self
                    .budgets_detail_editable(detail.category_id)
                    .then(|| plain(Shell::handle_budgets_detail_edit)),
            },
            cx,
        ))
    }

    /// `tab` on the Budgets page switches its tab rather than cycling focus, as on Bills.
    fn handle_budgets_tab_key(&mut self, keystroke: &Keystroke) -> bool {
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
            || self.nav.noun() != Noun::Budgets
            || self.nav.focus() != FocusZone::View
        {
            return false;
        }
        self.set_budgets_tab(self.budgets_tab.next());
        true
    }

    fn set_budgets_tab(&mut self, tab: budgets::BudgetsTab) {
        self.budgets_tab = tab;
        self.budgets_selected = 0;
        self.status_message = None;
        self.reset_view_scroll();
    }

    /// Steps the period a calendar month. The Plan and History range navs are their own tickets'.
    fn shift_budgets_period(&mut self, forward: bool) {
        self.budgets_period = if forward {
            self.budgets_period.next()
        } else {
            self.budgets_period.prev()
        };
        self.budgets_selected = 0;
    }

    /// `B` opens the Switcher and `n` New budget; `[`/`]` step the period and `1`/`2`/`3` pick the
    /// tab; `c` budgets a Category, and on Progress `e` edits the row's budget and `s` stops it.
    fn handle_budgets_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.noun() != Noun::Budgets || self.nav.focus() != FocusZone::View {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        // Bare `b` stays the rail toggle, so the Switcher takes the shifted key (#400).
        if modifiers.shift {
            if keystroke.key.eq_ignore_ascii_case("b") {
                self.open_budgets_switcher();
                return true;
            }
            return false;
        }
        if let Some(tab) = keystroke
            .key
            .chars()
            .next()
            .filter(|_| keystroke.key.chars().count() == 1)
            .and_then(budgets::BudgetsTab::from_digit)
        {
            self.set_budgets_tab(tab);
            return true;
        }
        match keystroke.key.as_str() {
            "c" if self.budgets_tab != budgets::BudgetsTab::History => {
                self.open_budgets_limit_picker();
                return true;
            }
            "n" => {
                self.open_budgets_new();
                return true;
            }
            "e" | "s" if self.budgets_tab == budgets::BudgetsTab::Progress => {
                if let Some((category_id, month)) = self.budgets_cursor_category() {
                    if keystroke.key == "e" {
                        self.open_budgets_limit(category_id, month);
                    } else {
                        self.open_budgets_stop(category_id, month);
                    }
                }
                return true;
            }
            _ => {}
        }
        if self.budgets_tab == budgets::BudgetsTab::Plan {
            return self.handle_budgets_plan_key(keystroke.key.as_str());
        }
        let history = self.budgets_tab == budgets::BudgetsTab::History;
        match keystroke.key.as_str() {
            "[" if history => self.shift_budgets_history_range(false),
            "]" if history => self.shift_budgets_history_range(true),
            "h" | "left" if history => self.step_budgets_history_month(false),
            "l" | "right" if history => self.step_budgets_history_month(true),
            "x" if history => self.pending_budgets_export = true,
            "[" => self.shift_budgets_period(false),
            "]" => self.shift_budgets_period(true),
            _ => return false,
        }
        true
    }

    /// The KNOWN COSTS stat's link: the Bills Schedule on the period being shown.
    fn open_budgets_schedule(&mut self) {
        self.bills_period = self.budgets_period;
        self.bills_all = false;
        self.bills_filters = bill_history::BillFilters::default();
        self.set_bills_tab(bills::BillsTab::Schedule);
        self.nav.set_noun(Noun::Bills);
        self.reset_view_scroll();
    }

    fn handle_budgets_tab_click(&mut self, tab: budgets::BudgetsTab, cx: &mut Context<'_, Self>) {
        self.set_budgets_tab(tab);
        cx.notify();
    }

    fn handle_budgets_period_prev(&mut self, cx: &mut Context<'_, Self>) {
        self.shift_budgets_period(false);
        cx.notify();
    }

    fn handle_budgets_period_next(&mut self, cx: &mut Context<'_, Self>) {
        self.shift_budgets_period(true);
        cx.notify();
    }

    fn handle_budgets_edit_plan_click(&mut self, cx: &mut Context<'_, Self>) {
        self.set_budgets_tab(budgets::BudgetsTab::Plan);
        cx.notify();
    }

    fn handle_budgets_range_prev(&mut self, cx: &mut Context<'_, Self>) {
        if self.budgets_tab == budgets::BudgetsTab::History {
            self.shift_budgets_history_range(false);
        } else {
            self.shift_budgets_plan_range(false);
        }
        cx.notify();
    }

    fn handle_budgets_range_next(&mut self, cx: &mut Context<'_, Self>) {
        if self.budgets_tab == budgets::BudgetsTab::History {
            self.shift_budgets_history_range(true);
        } else {
            self.shift_budgets_plan_range(true);
        }
        cx.notify();
    }

    /// A click puts the cursor on the cell; on an open month it also starts typing, and on the
    /// ROLLOVER cell it cycles the mode. A click elsewhere abandons an edit in progress.
    fn handle_budgets_plan_cell_click(
        &mut self,
        row: usize,
        column: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if self.budgets_plan_edit.take().is_some() {
            self.nav.exit_mode();
        }
        self.budgets_plan_cursor = (row, column);
        self.start_budgets_plan_edit();
        cx.notify();
    }

    fn handle_budgets_known_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_schedule();
        cx.notify();
    }

    /// A click selects a row; clicking the selected row opens its detail.
    fn handle_budgets_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let was_selected = self.budgets_selected == index;
        self.budgets_selected = index;
        if was_selected {
            self.open_budgets_detail_at(index);
        }
        cx.notify();
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
        self.status_message = None;
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
        if let Some(status) = bill_history::STATUS_CHIPS.get(index) {
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
    ) -> crate::select::SelectState {
        let (options, index) = self.bills_filter_options(field);
        crate::select::SelectState::new(options.get(index).cloned())
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
                .filter(|index| *index < bill_history::STATUS_CHIPS.len())
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

    /// The Add and Edit bill plan dialogs' select options, for the open form's Unit (and, on
    /// Edit, keeping the Plan's own Payee listed even if it has since been deactivated).
    fn bill_plan_options(&self) -> bill_form::BillPlanOptions {
        let (unit, keep_payee) = match self.bills_dialog.as_ref() {
            Some(bills::BillsDialog::Add(form)) => (form.unit_code(), None),
            Some(bills::BillsDialog::Edit(id, form)) => (
                form.unit_code(),
                bills::get(&self.bill_plans, *id).and_then(|plan| plan.payee_id),
            ),
            _ => (None, None),
        };
        self.bill_plan_options_for(unit, keep_payee)
    }

    fn bill_plan_options_for(
        &self,
        unit: Option<&str>,
        keep_payee: Option<u32>,
    ) -> bill_form::BillPlanOptions {
        bill_form::BillPlanOptions::new(
            &self.categories,
            &self.accounts,
            &self.payees,
            unit,
            keep_payee,
            crate::msg::desktop_payees_category_none(),
            bills_view::planner::recurrence_label,
        )
    }

    fn open_add_bill_plan_dialog(&mut self) {
        let form = bill_form::BillPlanForm::new(
            &self.categories,
            &self.accounts,
            &self.payees,
            crate::msg::desktop_payees_category_none(),
            bills_view::planner::recurrence_label,
        );
        self.bills_dialog = Some(bills::BillsDialog::Add(form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Opens the Edit dialog pre-filled from Bill Plan `id`.
    fn open_edit_bill_plan_dialog(&mut self, id: u32) {
        let Some(plan) = bills::get(&self.bill_plans, id) else {
            return;
        };
        let options = self.bill_plan_options_for(Some(&plan.unit), plan.payee_id);
        let form = bill_form::BillPlanForm::from_plan(plan, &options, self.settings_date_style);
        self.bills_dialog = Some(bills::BillsDialog::Edit(id, form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Runs `change` on the open Add or Edit bill plan form, then keeps its Account in its Unit
    /// (a Unit change rebuilds the Account list) and clears a stale Save error.
    fn with_bill_plan_form(
        &mut self,
        change: impl FnOnce(&mut bill_form::BillPlanForm, &bill_form::BillPlanOptions),
    ) {
        let options = self.bill_plan_options();
        let Some(form) = self
            .bills_dialog
            .as_mut()
            .and_then(bills::BillsDialog::plan_form_mut)
        else {
            return;
        };
        change(form, &options);
        form.error = None;
        let options = self.bill_plan_options();
        if let Some(form) = self
            .bills_dialog
            .as_mut()
            .and_then(bills::BillsDialog::plan_form_mut)
        {
            form.sync_account(&options);
        }
    }

    /// Keys while the Add or Edit bill plan dialog is open. `enter` on a select opens or commits
    /// its list and anywhere else saves; `space` flips Fixed/Estimated and Active, as `left` /
    /// `right` do on the segmented control. `Esc` never reaches here.
    fn handle_bills_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(form) = self
            .bills_dialog
            .as_mut()
            .and_then(bills::BillsDialog::plan_form_mut)
        else {
            if matches!(self.bills_dialog, Some(bills::BillsDialog::Pay(_))) {
                return self.handle_pay_bill_key(keystroke);
            }
            if matches!(self.bills_dialog, Some(bills::BillsDialog::Skip(_)))
                && keystroke.key == "enter"
            {
                self.confirm_skip_bill_dialog();
                return true;
            }
            return false;
        };
        let modifiers = keystroke.modifiers;
        let focused = form.focused;
        let on_toggle = matches!(
            focused,
            bill_form::BillPlanField::AmountKind | bill_form::BillPlanField::Active
        );
        let on_select = matches!(
            focused,
            bill_form::BillPlanField::Category
                | bill_form::BillPlanField::Account
                | bill_form::BillPlanField::Payee
                | bill_form::BillPlanField::Recurrence
        ) || (focused == bill_form::BillPlanField::Unit && !form.is_edit);
        match keystroke.key.as_str() {
            "tab" => self.with_bill_plan_form(|form, options| {
                form.cycle_focus(modifiers.shift, options);
            }),
            "up" | "down" if on_select => {
                let key = if keystroke.key == "up" {
                    SelectKey::Up
                } else {
                    SelectKey::Down
                };
                self.with_bill_plan_form(|form, options| {
                    form.handle_select_key(key, options);
                });
            }
            "space" | "enter" if on_select => self.with_bill_plan_form(|form, options| {
                form.handle_select_key(SelectKey::Activate, options);
            }),
            "space" | "left" | "right"
                if on_toggle
                    && (keystroke.key == "space"
                        || focused == bill_form::BillPlanField::AmountKind) =>
            {
                self.with_bill_plan_form(|form, _| form.toggle());
            }
            "enter" => self.confirm_bill_plan_dialog(),
            "backspace" => self.with_bill_plan_form(|form, _| form.backspace()),
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(ch) = typed_char(keystroke) {
                    self.with_bill_plan_form(|form, _| form.push_char(ch));
                }
            }
        }
        true
    }

    /// **Add bill plan** / **Save** and `enter`: adds or edits the Plan (Schedule regeneration is
    /// `bills::insert_plan`'s, `edit_plan`'s and `set_active`'s), closes the dialog and selects
    /// the Plan on the Planner tab. A no-op while the form is incomplete or has a problem; a
    /// refused Save keeps the dialog open with the error shown.
    fn confirm_bill_plan_dialog(&mut self) {
        let options = self.bill_plan_options();
        let (today, date_style) = (self.today, self.settings_date_style);
        let Some(dialog) = self.bills_dialog.as_ref() else {
            return;
        };
        let (editing, form) = match dialog {
            bills::BillsDialog::Add(form) => (None, form),
            bills::BillsDialog::Edit(id, form) => (Some(*id), form),
            _ => return,
        };
        let Some(draft) = form.draft(&options, today, date_style) else {
            return;
        };
        let is_active = form.is_active;
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
                self.bills_dialog = None;
                self.nav.exit_mode();
                if self.bills_tab == bills::BillsTab::Planner {
                    self.bills_selected = bills::planner_order(&self.bill_plans)
                        .iter()
                        .position(|plan| plan.id == id)
                        .unwrap_or(0);
                }
            }
            Err(error) => {
                if let Some(form) = self
                    .bills_dialog
                    .as_mut()
                    .and_then(bills::BillsDialog::plan_form_mut)
                {
                    form.error = Some(error);
                }
            }
        }
    }

    fn handle_bill_plan_field_click(
        &mut self,
        field: bill_form::BillPlanField,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_bill_plan_form(|form, options| match field {
            bill_form::BillPlanField::Category
            | bill_form::BillPlanField::Unit
            | bill_form::BillPlanField::Account
            | bill_form::BillPlanField::Payee
            | bill_form::BillPlanField::Recurrence => form.click_select(field, options),
            bill_form::BillPlanField::Active => {
                form.focus(field);
                form.toggle();
            }
            _ => form.focus(field),
        });
        cx.notify();
    }

    fn handle_bill_plan_option_click(
        &mut self,
        field: bill_form::BillPlanField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_bill_plan_form(|form, options| form.choose(field, index, options));
        cx.notify();
    }

    fn handle_bill_plan_amount_kind_click(
        &mut self,
        kind: bills::AmountKind,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_bill_plan_form(|form, _| {
            form.focus(bill_form::BillPlanField::AmountKind);
            form.amount_kind = kind;
        });
        cx.notify();
    }

    fn handle_bills_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.bills_dialog = None;
        self.nav.exit_mode();
        cx.notify();
    }

    fn handle_bill_plan_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_bill_plan_dialog();
        cx.notify();
    }

    /// Opens the Pay dialog (8d) on an open Schedule row, with its Match candidates worked out now;
    /// a row with nothing to pay says so instead.
    fn open_pay_bill_dialog(&mut self, row: bills::ScheduleRow) {
        let plan = bills::get(&self.bill_plans, row.id.plan_id);
        let Some(plan) = plan.filter(|_| row.is_actionable()) else {
            self.status_message = Some(crate::msg::desktop_status_bill_not_actionable());
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
        self.bills_dialog = Some(bills::BillsDialog::Pay(form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn pay_form_mut(&mut self) -> Option<&mut PayForm> {
        match self.bills_dialog.as_mut() {
            Some(bills::BillsDialog::Pay(form)) => Some(form),
            _ => None,
        }
    }

    /// Keys while the Pay dialog is open: `left`/`right` switch panel; on Match `j`/`k` choose and
    /// `enter` confirms (on "None of these", switches panel); on Pay it directly `tab` moves
    /// between Amount and Date. `Esc` never reaches here.
    fn handle_pay_bill_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(form) = self.pay_form_mut() else {
            return false;
        };
        let modifiers = keystroke.modifiers;
        let on_match = form.mode == pay_form::PayMode::Match;
        match keystroke.key.as_str() {
            "left" | "right" => form.toggle_mode(),
            "j" | "down" if on_match => form.step_choice(true),
            "k" | "up" if on_match => form.step_choice(false),
            "enter" if on_match && form.choice == Some(pay_form::MatchChoice::NoneOfThese) => {
                form.choose(pay_form::MatchChoice::NoneOfThese);
            }
            "enter" => self.confirm_pay_bill_dialog(),
            "tab" => form.cycle_focus(),
            "backspace" => form.backspace(),
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(ch) = typed_char(keystroke) {
                    form.push_char(ch);
                }
            }
        }
        true
    }

    /// **Create transaction & mark paid** / **Match & mark paid**: settles the entry through
    /// `bills::pay` or `bills::match_split` and closes the dialog. A no-op while the active panel
    /// is incomplete; a refused settle keeps the dialog open with the error shown.
    fn confirm_pay_bill_dialog(&mut self) {
        let (today, date_style) = (self.today, self.settings_date_style);
        let Some(bills::BillsDialog::Pay(form)) = self.bills_dialog.as_ref() else {
            return;
        };
        let entry = form.entry;
        let Some(action) = form.action(today, date_style) else {
            return;
        };
        let result = match action {
            pay_form::PayAction::Pay { amount, date } => bills::pay(
                &self.bill_plans,
                &mut self.bill_entries,
                &mut self.transactions,
                entry,
                &amount,
                date,
            )
            .map(|_| ()),
            pay_form::PayAction::Match(split) => bills::match_split(
                &self.bill_plans,
                &mut self.bill_entries,
                &self.transactions,
                &self.accounts,
                &self.categories,
                entry,
                split,
            ),
        };
        match result {
            Ok(()) => {
                self.bills_dialog = None;
                self.nav.exit_mode();
            }
            Err(error) => {
                if let Some(form) = self.pay_form_mut() {
                    form.error = Some(error);
                }
            }
        }
    }

    fn handle_pay_bill_mode_click(&mut self, mode: pay_form::PayMode, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.pay_form_mut() {
            form.set_mode(mode);
        }
        cx.notify();
    }

    fn handle_pay_bill_choice_click(
        &mut self,
        choice: pay_form::MatchChoice,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.choose(choice);
        }
        cx.notify();
    }

    fn handle_pay_bill_field_click(
        &mut self,
        field: pay_form::PayField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.focus(field);
        }
        cx.notify();
    }

    fn handle_pay_bill_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_pay_bill_dialog();
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
        let (today, date_style) = (self.today, self.settings_date_style);
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
                date_error: form.date_error(today, date_style),
                error: form
                    .error
                    .as_ref()
                    .map(|_| crate::msg::desktop_bills_pay_error_gone()),
                valid: form.action(today, date_style).is_some(),
                handlers: bills_view::pay_dialog::PayDialogHandlers {
                    on_mode_click,
                    on_choice_click,
                    on_field_click,
                    on_cancel: plain(Shell::handle_bills_dialog_cancel),
                    on_confirm: plain(Shell::handle_pay_bill_confirm),
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
            self.status_message = Some(crate::msg::desktop_status_bill_not_actionable());
            return;
        }
        self.bills_dialog = Some(bills::BillsDialog::Skip(row.id));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// **Skip this cycle**: resolves the entry through `bills::skip` and closes the dialog. The
    /// dialog carries no form to show an error on, so a refused skip (the entry resolved or
    /// superseded underneath it) closes with the reason in the status line.
    fn confirm_skip_bill_dialog(&mut self) {
        let Some(bills::BillsDialog::Skip(entry)) = self.bills_dialog else {
            return;
        };
        if bills::skip(&self.bill_plans, &mut self.bill_entries, entry).is_err() {
            self.status_message = Some(crate::msg::desktop_bills_skip_error_gone());
        }
        self.bills_dialog = None;
        self.nav.exit_mode();
    }

    fn handle_skip_bill_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_skip_bill_dialog();
        cx.notify();
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
                on_confirm: plain(Shell::handle_skip_bill_confirm),
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
        self.bills_filters = bill_history::BillFilters::default();
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
        let index = transaction_query::query(
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
        self.tags_dialog = Some(tags::TagsDialog::Add(tags::TagForm::new()));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// The form behind the open Add or Edit tag dialog, if that is what's open.
    fn tag_form_mut(&mut self) -> Option<&mut tags::TagForm> {
        match self.tags_dialog.as_mut() {
            Some(tags::TagsDialog::Add(form) | tags::TagsDialog::Edit(_, form)) => Some(form),
            _ => None,
        }
    }

    /// Keys while the Add or Edit tag dialog is open: `tab` moves between Name, the swatch row, the
    /// hex box and (Edit only) Active, `←`/`→` step the swatch row, `space` toggles Active, and
    /// `enter` submits from anywhere. `Esc` never reaches here.
    fn handle_tags_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        if matches!(self.tags_dialog, Some(tags::TagsDialog::Merge(_))) {
            return self.handle_merge_tags_key(keystroke);
        }
        if let Some(tags::TagsDialog::Remove(_, form)) = self.tags_dialog.as_mut() {
            match keystroke.key.as_str() {
                "enter" => self.confirm_remove_tag_dialog(),
                "backspace" => form.backspace(),
                _ => {
                    let modifiers = &keystroke.modifiers;
                    if modifiers.control
                        || modifiers.alt
                        || modifiers.platform
                        || modifiers.function
                    {
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
            return true;
        }
        let Some(form) = self.tag_form_mut() else {
            return false;
        };
        let modifiers = &keystroke.modifiers;
        let on_swatches = form.focused == tags::TagField::Swatches;
        let on_active = form.focused == tags::TagField::Active;
        match keystroke.key.as_str() {
            "tab" => form.cycle_focus(modifiers.shift),
            "space" if on_active => form.toggle_active(),
            "left" if on_swatches => form.step_pick(false),
            "right" if on_swatches => form.step_pick(true),
            "enter" => self.confirm_tags_dialog(),
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
        true
    }

    /// **Add tag** / **Save** and `enter`: adds or saves the Tag and selects it, closing the
    /// dialog. A no-op while the name or the hex box is invalid.
    fn confirm_tags_dialog(&mut self) {
        let (own_id, form) = match self.tags_dialog.as_ref() {
            Some(tags::TagsDialog::Add(form)) => (None, form),
            Some(tags::TagsDialog::Edit(id, form)) => (Some(*id), form),
            _ => return,
        };
        if !form.is_valid(&self.tags, own_id) {
            return;
        }
        let Some(draft) = form.draft() else {
            return;
        };
        let is_active = form.is_active;
        // `is_valid` ran the same name check, so a refusal here can only leave the dialog open.
        let saved = match own_id {
            None => tags::insert_tag(&mut self.tags, &draft),
            Some(id) => tags::edit_tag(&mut self.tags, id, &draft)
                .and_then(|()| tags::set_active(&mut self.tags, id, is_active))
                .map(|()| id),
        };
        let Ok(id) = saved else {
            return;
        };
        self.tags_dialog = None;
        self.nav.exit_mode();
        self.select_tag(id);
    }

    fn handle_tags_dialog_field_click(
        &mut self,
        field: tags::TagField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.tag_form_mut() {
            form.focus(field);
        }
        cx.notify();
    }

    fn handle_tags_dialog_pick(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.tag_form_mut() {
            form.focus(tags::TagField::Swatches);
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
        self.tags_dialog = None;
        self.nav.exit_mode();
        cx.notify();
    }

    fn handle_tags_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        if matches!(self.tags_dialog, Some(tags::TagsDialog::Remove(..))) {
            self.confirm_remove_tag_dialog();
        } else if matches!(self.tags_dialog, Some(tags::TagsDialog::Merge(_))) {
            self.confirm_merge_tags_dialog();
        } else {
            self.confirm_tags_dialog();
        }
        cx.notify();
    }

    fn open_edit_tag_dialog(&mut self, id: u32) {
        let Some(tag) = tags::get(&self.tags, id) else {
            return;
        };
        self.tags_dialog = Some(tags::TagsDialog::Edit(id, tags::TagForm::for_edit(tag)));
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn open_remove_tag_dialog(&mut self, id: u32) {
        if tags::get(&self.tags, id).is_none() {
            return;
        }
        self.tags_dialog = Some(tags::TagsDialog::Remove(id, tags::RemoveTagForm::default()));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// **Remove tag** and `enter`: a no-op until a used Tag's name is typed, then untags every
    /// Split, deletes the Tag, toasts it and closes the dialog. The selection keeps its position,
    /// so it lands on the next Tag in usage order (or the new last one).
    fn confirm_remove_tag_dialog(&mut self) {
        let Some(tags::TagsDialog::Remove(id, form)) = self.tags_dialog.as_ref() else {
            return;
        };
        let id = *id;
        let Some(tag) = tags::get(&self.tags, id) else {
            return;
        };
        if !form.allows(&tag.name, tags::transaction_count(&self.transactions, id)) {
            return;
        }
        let name = tag.name.clone();
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
        self.tags_dialog = None;
        self.nav.exit_mode();
        self.tags_selected = self.tags_selected.min(self.tags.len().saturating_sub(1));
    }

    /// The 7e selects' options, labelled `Shared (9 txns)`.
    fn merge_tag_options(&self) -> Vec<tags::MergeOption> {
        tags::merge_options(&self.tags, &self.transactions, |name, count| {
            crate::msg::desktop_tags_merge_option(name, i64::try_from(count).unwrap_or(i64::MAX))
        })
    }

    /// Opens 7e with `source` as the source Tag and, when it is flagged as a likely duplicate, its
    /// suggested target (#354). `None` (the palette's `tags merge`, or the subline link with
    /// nothing flagged) leaves both selects empty.
    fn open_merge_tags_dialog(&mut self, source: Option<u32>) {
        let groups = tags::duplicate_groups(&self.tags, &self.transactions);
        let target = source.and_then(|id| tags::duplicate_of(&groups, id));
        let options = self.merge_tag_options();
        self.tags_dialog = Some(tags::TagsDialog::Merge(tags::MergeTagsForm::new(
            &options, source, target,
        )));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Keys while 7e is open: `tab` commits an open list and moves to the other select, `↑`/`↓`
    /// step the value (or an open list's highlight), `space` opens or commits the list, and
    /// `enter` commits an open list or else merges. `Esc` never reaches here.
    fn handle_merge_tags_key(&mut self, keystroke: &Keystroke) -> bool {
        let options = self.merge_tag_options();
        let Some(tags::TagsDialog::Merge(form)) = self.tags_dialog.as_mut() else {
            return false;
        };
        match keystroke.key.as_str() {
            "tab" => form.cycle_focus(&options),
            "up" => form.step(&options, -1),
            "down" => form.step(&options, 1),
            "space" => form.toggle(&options, form.focused),
            "enter" if form.is_open() => form.toggle(&options, form.focused),
            "enter" => self.confirm_merge_tags_dialog(),
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
            }
        }
        true
    }

    /// **Merge into "…"** and `enter`: a no-op until both Tags are chosen, then retags the
    /// source's Splits with the target, deletes the source, toasts it, closes the dialog and
    /// selects the target.
    fn confirm_merge_tags_dialog(&mut self) {
        let options = self.merge_tag_options();
        let Some(tags::TagsDialog::Merge(form)) = self.tags_dialog.as_ref() else {
            return;
        };
        let Some((source, target)) = form.pair(&options) else {
            return;
        };
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
        self.tags_dialog = None;
        self.nav.exit_mode();
        self.select_tag(target);
    }

    fn handle_merge_tags_field_click(
        &mut self,
        field: tags::MergeField,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.merge_tag_options();
        if let Some(tags::TagsDialog::Merge(form)) = self.tags_dialog.as_mut() {
            form.toggle(&options, field);
        }
        cx.notify();
    }

    fn handle_merge_tags_option_click(
        &mut self,
        field: tags::MergeField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.merge_tag_options();
        if let Some(tags::TagsDialog::Merge(form)) = self.tags_dialog.as_mut() {
            form.choose(&options, field, index);
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
    fn import_category_options(&self) -> payees::PayeeOptions {
        payees::PayeeOptions::new(
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

    fn payee_dialog_options(&self) -> payees::PayeeOptions {
        payees::PayeeOptions::new(&self.categories, crate::msg::desktop_payees_category_none())
    }

    fn open_add_payee_dialog(&mut self) {
        let form = payees::PayeeForm::new(&self.payee_dialog_options());
        self.payees_dialog = Some(payees::PayeesDialog::Add(form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// The Payee a form dialog is editing (`None` for Add), so its own name and rules don't count
    /// as taken.
    fn payee_dialog_own_id(&self) -> Option<u32> {
        match self.payees_dialog {
            Some(payees::PayeesDialog::Edit(id, _)) => Some(id),
            _ => None,
        }
    }

    /// Keys while an Add or Edit payee dialog is open. `enter` in the rule input adds a chip; on a
    /// closed select it opens the list; anywhere else it submits. `Esc` never reaches here.
    fn handle_payees_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        if let Some(payees::PayeesDialog::Delete(_, form)) = self.payees_dialog.as_mut() {
            match keystroke.key.as_str() {
                "enter" => self.confirm_delete_payee_dialog(),
                "backspace" => form.backspace(),
                _ => {
                    let modifiers = &keystroke.modifiers;
                    if modifiers.control
                        || modifiers.alt
                        || modifiers.platform
                        || modifiers.function
                    {
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
            return true;
        }
        let options = self.payee_dialog_options();
        let own_id = self.payee_dialog_own_id();
        let Some(form) = self
            .payees_dialog
            .as_mut()
            .and_then(payees::PayeesDialog::form_mut)
        else {
            return false;
        };
        let modifiers = &keystroke.modifiers;
        let on_select = form.focused == payees::PayeeField::DefaultCategory;
        match keystroke.key.as_str() {
            "tab" => form.cycle_focus(modifiers.shift, &options),
            "up" => {
                form.handle_select_key(SelectKey::Up, &options);
            }
            "down" => {
                form.handle_select_key(SelectKey::Down, &options);
            }
            "space" | "enter" if on_select => {
                form.handle_select_key(SelectKey::Activate, &options);
            }
            "enter" if form.focused == payees::PayeeField::Rule && !form.rule_input.is_empty() => {
                form.add_rule(&self.payees, own_id);
            }
            "enter" => self.confirm_payees_dialog(),
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
        true
    }

    /// **Add payee** / **Save** and `enter`: adds or edits the Payee and selects it, closing the
    /// dialog. A no-op while the name is invalid; a refused rule keeps the dialog open with the
    /// error shown.
    fn confirm_payees_dialog(&mut self) {
        let options = self.payee_dialog_options();
        let own_id = self.payee_dialog_own_id();
        let Some(dialog) = self.payees_dialog.as_mut() else {
            return;
        };
        let Some(form) = dialog.form_mut() else {
            return;
        };
        if !form.is_valid(&self.payees, own_id) {
            return;
        }
        let draft = form.draft(&options);
        let result = match dialog {
            payees::PayeesDialog::Add(_) => payees::insert_payee(&mut self.payees, &draft),
            payees::PayeesDialog::Edit(id, _) => {
                let id = *id;
                payees::edit_payee(&mut self.payees, id, &draft).map(|()| id)
            }
            payees::PayeesDialog::Delete(..) => return,
        };
        match result {
            Ok(id) => {
                self.payees_dialog = None;
                self.nav.exit_mode();
                self.select_payee(id);
            }
            Err(error) => {
                if let Some(form) = self
                    .payees_dialog
                    .as_mut()
                    .and_then(payees::PayeesDialog::form_mut)
                {
                    form.error = Some(error);
                }
            }
        }
    }

    fn with_payee_form(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut payees::PayeeForm, &payees::PayeeOptions, &[Payee], Option<u32>),
    ) {
        let options = self.payee_dialog_options();
        let own_id = self.payee_dialog_own_id();
        if let Some(form) = self
            .payees_dialog
            .as_mut()
            .and_then(payees::PayeesDialog::form_mut)
        {
            change(form, &options, &self.payees, own_id);
        }
        cx.notify();
    }

    fn handle_payees_dialog_field_click(
        &mut self,
        field: payees::PayeeField,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_payee_form(cx, |form, options, _, _| {
            if field == payees::PayeeField::DefaultCategory {
                form.click_select(options);
            } else {
                form.focus(field);
            }
        });
    }

    fn handle_payees_dialog_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.with_payee_form(cx, |form, options, _, _| {
            form.default_category.choose(&options.labels, index);
        });
    }

    fn handle_payees_dialog_add_rule(&mut self, cx: &mut Context<'_, Self>) {
        self.with_payee_form(cx, |form, _, payees, own_id| {
            form.focus(payees::PayeeField::Rule);
            form.add_rule(payees, own_id);
        });
    }

    fn handle_payees_dialog_remove_rule(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.with_payee_form(cx, |form, _, _, _| form.remove_rule(index));
    }

    fn handle_payees_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.payees_dialog = None;
        self.nav.exit_mode();
        cx.notify();
    }

    fn handle_payees_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_payees_dialog();
        cx.notify();
    }

    /// Opens the Edit dialog pre-filled from Payee `id`.
    fn open_edit_payee_dialog(&mut self, id: u32) {
        let Some(payee) = payees::get(&self.payees, id) else {
            return;
        };
        let form = payees::PayeeForm::from_payee(payee, &self.payee_dialog_options());
        self.payees_dialog = Some(payees::PayeesDialog::Edit(id, form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn open_delete_payee_dialog(&mut self, id: u32) {
        if payees::get(&self.payees, id).is_none() {
            return;
        }
        self.payees_dialog = Some(payees::PayeesDialog::Delete(
            id,
            payees::DeletePayeeForm::default(),
        ));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// The Payee the Delete dialog is open on and what confirming it would do (#283).
    fn delete_payee_target(&self) -> Option<(&Payee, payees::DeleteAction)> {
        let Some(payees::PayeesDialog::Delete(id, _)) = self.payees_dialog.as_ref() else {
            return None;
        };
        let payee = payees::get(&self.payees, *id)?;
        Some((
            payee,
            payees::DeleteAction::for_payee(payee, &self.transactions),
        ))
    }

    /// **Delete payee** / **Deactivate payee** / **Reactivate payee** and `enter`: a no-op until the
    /// typed name matches (for the destructive two), then applies the action, toasts the outcome
    /// and closes the dialog.
    fn confirm_delete_payee_dialog(&mut self) {
        let Some((payee, action)) = self.delete_payee_target() else {
            return;
        };
        let Some(payees::PayeesDialog::Delete(_, form)) = self.payees_dialog.as_ref() else {
            return;
        };
        if !form.allows(action, &payee.name) {
            return;
        }
        let (id, name) = (payee.id, payee.name.clone());
        let (kind, text) =
            match payees::apply_delete_action(&mut self.payees, &self.transactions, id, action) {
                Ok(()) => (
                    ToastKind::Success,
                    match action {
                        payees::DeleteAction::Delete => lib_locale::msg::toast_payee_deleted(&name),
                        payees::DeleteAction::Deactivate => {
                            lib_locale::msg::toast_payee_deactivated(&name)
                        }
                        payees::DeleteAction::Reactivate => {
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
        self.payees_dialog = None;
        self.nav.exit_mode();
        self.payees_selected = self
            .payees_selected
            .min(self.payees.len().saturating_sub(1));
    }

    fn handle_delete_payee_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_delete_payee_dialog();
        cx.notify();
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
        form.name = name.trim().to_string();
        self.accounts_dialog = Some(AccountsDialog::Add(form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Routes a keystroke while an Accounts dialog with a form (Add or Edit) is open. `Esc` never
    /// reaches here (it is handled ahead of the mode gates); everything else is swallowed.
    fn handle_accounts_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        if matches!(self.accounts_dialog, Some(AccountsDialog::Delete(..))) {
            return self.handle_delete_account_key(keystroke);
        }
        let options = self.account_dialog_options();
        let Some(form) = self
            .accounts_dialog
            .as_mut()
            .and_then(AccountsDialog::form_mut)
        else {
            return false;
        };
        let modifiers = &keystroke.modifiers;
        match keystroke.key.as_str() {
            "tab" => form.cycle_focus(modifiers.shift, &options),
            "up" => {
                form.handle_select_key(SelectKey::Up, &options);
            }
            "down" => {
                form.handle_select_key(SelectKey::Down, &options);
            }
            "space" if form.focused.is_select() => {
                form.handle_select_key(SelectKey::Activate, &options);
            }
            "enter" if form.focused.is_select() => {
                form.handle_select_key(SelectKey::Activate, &options);
            }
            "enter" => {
                if form.is_valid() {
                    self.confirm_accounts_dialog();
                }
            }
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
        true
    }

    /// Keys in the Delete account dialog: type the account's name back (`Backspace` edits it),
    /// `Enter` deletes once it matches, and `Tab` is swallowed since the confirmation is the only
    /// field. `Esc` never reaches here (it cancels ahead of the mode gates).
    fn handle_categories_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        if matches!(
            self.categories_dialog,
            Some(categories::CategoriesDialog::Delete(..))
        ) {
            return self.handle_delete_categories_key(keystroke);
        }

        let Some(dialog) = self.categories_dialog.as_mut() else {
            return false;
        };

        let Some(form) = dialog.form_mut() else {
            return false;
        };

        match keystroke.key.as_str() {
            "escape" => {
                self.categories_dialog = None;
                self.nav.exit_mode();
                true
            }
            "backspace" => {
                form.backspace();
                true
            }
            "tab" => {
                form.cycle_field();
                true
            }
            "enter" => {
                if !form.name.trim().is_empty()
                    && let Some(dialog) = self.categories_dialog.take()
                {
                    match dialog {
                        categories::CategoriesDialog::Add { form, .. } => {
                            let category_type =
                                form.category_type.clone().unwrap_or(CategoryTypes::Expense);
                            if let Ok(category_id) = categories::insert_category(
                                &mut self.categories,
                                form.name.trim().to_string(),
                                form.parent_id,
                                category_type,
                            ) {
                                self.save_category_budget(category_id, &form, false);
                            }
                            self.nav.exit_mode();
                        }
                        categories::CategoriesDialog::Edit(id, form) => {
                            let _ = categories::edit_category(
                                &mut self.categories,
                                id,
                                form.name.trim().to_string(),
                            );
                            if let Some(new_parent) = form.parent_id {
                                let _ = categories::move_category(
                                    &mut self.categories,
                                    id,
                                    Some(new_parent),
                                );
                            }
                            self.save_category_budget(id, &form, true);
                            self.nav.exit_mode();
                        }
                        _ => {
                            self.nav.exit_mode();
                        }
                    }
                }
                true
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(text) = keystroke.key_char.as_deref()
                    && text.chars().count() == 1
                    && let Some(ch) = text.chars().next()
                {
                    form.push_char(ch);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn handle_delete_account_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(AccountsDialog::Delete(id, form)) = self.accounts_dialog.as_mut() else {
            return false;
        };
        match keystroke.key.as_str() {
            "backspace" => form.backspace(),
            "tab" => {}
            "enter" => {
                let matches = self
                    .accounts
                    .iter()
                    .find(|account| account.id == *id)
                    .is_some_and(|account| form.matches(&account.name));
                if matches {
                    self.confirm_accounts_dialog();
                }
            }
            _ => {
                let modifiers = &keystroke.modifiers;
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
        true
    }

    /// Keys in the Delete category dialog: type the category's name back (`Backspace` edits it),
    /// `Enter` deletes once it matches, and `Tab` is swallowed since the confirmation is the only
    /// field. `Esc` never reaches here (it cancels ahead of the mode gates).
    fn handle_delete_categories_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(categories::CategoriesDialog::Delete(id, form)) = self.categories_dialog.as_mut()
        else {
            return false;
        };
        match keystroke.key.as_str() {
            "backspace" => form.backspace(),
            "tab" => {}
            "enter" => {
                let matches = self
                    .categories
                    .iter()
                    .find(|category| category.id == *id)
                    .is_some_and(|category| form.matches(&category.name));
                if matches {
                    self.confirm_categories_dialog();
                }
            }
            _ => {
                let modifiers = &keystroke.modifiers;
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
        true
    }

    fn confirm_categories_dialog(&mut self) {
        if let Some(categories::CategoriesDialog::Delete(category_id, form)) =
            self.categories_dialog.as_ref()
        {
            let category = self.categories.iter().find(|c| c.id == *category_id);
            let matches = category.is_some_and(|c| form.matches(&c.name));
            if matches {
                let category_id = *category_id;
                let (kind, text) = delete_category(
                    &mut self.categories,
                    &mut self.transactions,
                    &mut self.budgets,
                    category_id,
                );
                self.raise_toast(kind, text);
                // Keep the selection in range
                let tree_rows = categories::tree_rows(&self.categories, &self.categories_expanded);
                let filtered_rows: Vec<_> = tree_rows
                    .iter()
                    .filter(|row| {
                        self.categories
                            .iter()
                            .find(|c| c.id == row.id)
                            .map(|c| c.category_type == CategoryTypes::Expense)
                            .unwrap_or(false)
                    })
                    .collect();
                self.categories_selected = self
                    .categories_selected
                    .min(filtered_rows.len().saturating_sub(1));
            }
            self.categories_dialog = None;
        }
    }

    /// A click on a field of the Add account dialog: focuses a text field, or focuses a select
    /// and toggles its list.
    fn handle_accounts_dialog_field_click(
        &mut self,
        field: AccountField,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.account_dialog_options();
        if let Some(form) = self
            .accounts_dialog
            .as_mut()
            .and_then(AccountsDialog::form_mut)
        {
            if field.is_select() {
                form.click_select(field, &options);
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
        let options = self.account_dialog_options();
        if let Some(form) = self
            .accounts_dialog
            .as_mut()
            .and_then(AccountsDialog::form_mut)
        {
            form.choose_option(field, index, &options);
        }
        cx.notify();
    }

    fn handle_accounts_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.accounts_dialog = None;
        self.nav.exit_mode();
        cx.notify();
    }

    fn handle_accounts_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_accounts_dialog();
        cx.notify();
    }

    /// The Add account dialog's **Add account** button, the Edit dialog's **Save**, the Delete
    /// dialog's **Delete account** (once the name matches), and `Enter` in any of them: Add builds the account, appends it and selects it; Edit writes the changes onto the
    /// existing row (which regroups if its Type changed) and keeps it selected. Either way the
    /// dialog closes. A no-op, leaving it open, while the form is invalid.
    fn confirm_accounts_dialog(&mut self) {
        let valid = match self.accounts_dialog.as_ref() {
            Some(AccountsDialog::Delete(id, form)) => self
                .accounts
                .iter()
                .find(|account| account.id == *id)
                .is_some_and(|account| form.matches(&account.name)),
            Some(dialog) => dialog.form().is_some_and(AccountForm::is_valid),
            None => false,
        };
        if !valid {
            return;
        }
        let Some(dialog) = self.accounts_dialog.take() else {
            return;
        };
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
        self.nav.exit_mode();
    }

    /// Opens the Delete account dialog on `id`. A no-op if the account is gone.
    fn open_delete_account_dialog(&mut self, id: u32) {
        if !self.accounts.iter().any(|account| account.id == id) {
            return;
        }
        self.accounts_dialog = Some(AccountsDialog::Delete(id, DeleteAccountForm::default()));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Opens the Edit account dialog on `id`, pre-filled. A no-op if the account is gone.
    fn open_edit_account_dialog(&mut self, id: u32) {
        let options = self.account_dialog_options();
        let Some(account) = self.accounts.iter().find(|account| account.id == id) else {
            return;
        };
        let form = AccountForm::from_account(account, &options);
        self.accounts_dialog = Some(AccountsDialog::Edit(id, form));
        self.nav.enter_mode(InputMode::Dialog);
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
            self.status_message = Some(crate::msg::desktop_status_delete_children_first());
            cx.notify();
            return;
        }

        let form = categories::DeleteCategoryForm::default();
        self.categories_dialog = Some(categories::CategoriesDialog::Delete(category_id, form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn handle_categories_dialog_field_click(
        &mut self,
        field: categories::CategoryField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(dialog) = self.categories_dialog.as_mut()
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
        if let Some(dialog) = self.categories_dialog.as_mut()
            && let Some(form) = dialog.form_mut()
        {
            form.parent_id = parent_id;
            // Update the category type if a parent is selected
            if let Some(parent_id) = parent_id
                && let Some(parent) = self.categories.iter().find(|c| c.id == parent_id)
            {
                form.category_type = Some(parent.category_type.clone());
            }
            cx.notify();
        }
    }

    fn handle_categories_dialog_type_change(
        &mut self,
        category_type: CategoryTypes,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(dialog) = self.categories_dialog.as_mut() {
            let can_change_type = match dialog {
                categories::CategoriesDialog::Add { form, .. } => form.parent_id.is_none(),
                categories::CategoriesDialog::Edit(id, _form) => self
                    .categories
                    .iter()
                    .find(|c| c.id == *id)
                    .map(|c| c.parent.is_none())
                    .unwrap_or(false),
                _ => false,
            };

            if can_change_type && let Some(form) = dialog.form_mut() {
                form.category_type = Some(category_type.clone());
                // For Edit dialogs, cascade the type change to descendants
                if let categories::CategoriesDialog::Edit(id, _) = dialog {
                    let _ =
                        categories::change_category_type(&mut self.categories, *id, category_type);
                }
                cx.notify();
            }
        }
    }

    fn handle_categories_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.categories_dialog = None;
        self.nav.exit_mode();
        cx.notify();
    }

    fn handle_categories_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(dialog) = self.categories_dialog.take() {
            match dialog {
                categories::CategoriesDialog::Add { form, .. } => {
                    if !form.name.trim().is_empty() {
                        let category_type =
                            form.category_type.clone().unwrap_or(CategoryTypes::Expense);
                        match categories::insert_category(
                            &mut self.categories,
                            form.name.trim().to_string(),
                            form.parent_id,
                            category_type,
                        ) {
                            Ok(category_id) => {
                                self.save_category_budget(category_id, &form, false);
                                self.nav.exit_mode();
                                cx.notify();
                            }
                            Err(_) => {
                                // TODO: Show error message
                                self.nav.exit_mode();
                            }
                        }
                    }
                }
                categories::CategoriesDialog::Edit(id, form) => {
                    if !form.name.trim().is_empty() {
                        let _ = categories::edit_category(
                            &mut self.categories,
                            id,
                            form.name.trim().to_string(),
                        );
                        // Handle parent change if necessary
                        if let Some(new_parent) = form.parent_id {
                            let _ = categories::move_category(
                                &mut self.categories,
                                id,
                                Some(new_parent),
                            );
                        } else if form.parent_id.is_none() {
                            // If parent was cleared, move to top-level
                            let _ = categories::move_category(&mut self.categories, id, None);
                        }
                        self.save_category_budget(id, &form, true);
                        self.nav.exit_mode();
                        cx.notify();
                    }
                }
                categories::CategoriesDialog::Delete(category_id, form) => {
                    self.categories_dialog =
                        Some(categories::CategoriesDialog::Delete(category_id, form));
                    self.confirm_categories_dialog();
                    self.nav.exit_mode();
                    cx.notify();
                }
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
        self.settings_dialog = Some(SettingsDialog::AddUnit(UnitForm::default()));
        self.nav.enter_mode(InputMode::Dialog);
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
        self.settings_dialog = Some(SettingsDialog::EditUnit(index, UnitForm::from_row(row)));
        self.nav.enter_mode(InputMode::Dialog);
        cx.notify();
    }

    /// Shared by the Add/Edit unit dialogs' own field-focus clicks (issues #184/#185) -- which
    /// field a click targets doesn't depend on which dialog variant is open.
    fn handle_unit_dialog_field_click(&mut self, field: AddUnitField, cx: &mut Context<'_, Self>) {
        if let Some(dialog) = self.settings_dialog.as_mut() {
            match dialog {
                SettingsDialog::AddUnit(form) | SettingsDialog::EditUnit(_, form) => {
                    form.focused_field = field;
                    cx.notify();
                }
                // Neither has more than one text field, always implicitly focused -- nothing to
                // click into.
                SettingsDialog::DeleteUnit(..) | SettingsDialog::AddInstitution(_) => {}
            }
        }
    }

    /// Shared by the Add/Edit unit dialogs' own Type segmented control (issues #184/#185).
    fn handle_unit_dialog_kind_click(&mut self, kind: UnitKind, cx: &mut Context<'_, Self>) {
        if let Some(dialog) = self.settings_dialog.as_mut() {
            match dialog {
                SettingsDialog::AddUnit(form) | SettingsDialog::EditUnit(_, form) => {
                    form.kind = kind;
                    cx.notify();
                }
                // Neither has a Type selector at all.
                SettingsDialog::DeleteUnit(..) | SettingsDialog::AddInstitution(_) => {}
            }
        }
    }

    /// Shared by the Add/Edit unit dialogs' own Cancel button -- discards whatever was typed,
    /// same as `Esc` (`Self::handle_key_down`'s `ClosePopupsAndExitMode` arm).
    fn handle_settings_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.settings_dialog = None;
        self.nav.exit_mode();
        cx.notify();
    }

    /// The Add/Edit/Delete unit dialogs' own Add/Save/Delete unit button (and `Enter`, via
    /// [`Self::handle_dialog_key`]): the README's own "Dialog lifecycle" rows -- Add "validate ->
    /// append to Units table -> close", Edit "Save -> update the in-memory row -> close" (a
    /// changed code just relabels the row here; rewriting real references is out of scope per
    /// the map's own Destination), Delete "confirm -> remove + close". A no-op if the relevant
    /// form isn't valid/matching, or (defensively) nothing is actually open -- each dialog's own
    /// confirm button is only clickable while that gate already holds, so this should only ever
    /// run on a form that's already passed it.
    fn confirm_settings_dialog(&mut self) {
        let Some(dialog) = self.settings_dialog.take() else {
            return;
        };
        match dialog {
            SettingsDialog::AddUnit(form) => {
                if !form.is_valid() {
                    return;
                }
                self.settings_units.push(UnitRow {
                    code: form.code,
                    name: form.name,
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
                if !form.is_valid() {
                    return;
                }
                if let Some(existing) = self.settings_units.get_mut(index) {
                    // `source`/`is_base`/`is_default` aren't Edit unit dialog fields either
                    // (issue #185's own body: "same form as Add unit") -- preserved from the
                    // row being edited rather than reset, unlike `code`/`name`/`kind`.
                    *existing = UnitRow {
                        code: form.code,
                        name: form.name,
                        kind: form.kind.label().to_string(),
                        source: existing.source.clone(),
                        is_base: existing.is_base,
                        is_default: existing.is_default,
                    };
                }
            }
            SettingsDialog::DeleteUnit(index, form) => {
                let matches = self
                    .settings_units
                    .get(index)
                    .is_some_and(|row| form.matches(&row.code));
                if !matches {
                    return;
                }
                if index < self.settings_units.len() {
                    let unit = self.settings_units.remove(index);
                    self.raise_toast(
                        ToastKind::Success,
                        lib_locale::msg::toast_unit_deleted(&unit.code),
                    );
                }
            }
            SettingsDialog::AddInstitution(form) => {
                if !form.is_valid() {
                    return;
                }
                let account_type = form
                    .account_types
                    .iter()
                    .map(|account_type| account_type.label())
                    .collect::<Vec<_>>()
                    .join(" \u{b7} ");
                self.settings_institutions.push(InstitutionRow {
                    name: form.name,
                    account_type,
                });
            }
        }
        self.nav.exit_mode();
    }

    fn handle_settings_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_settings_dialog();
        cx.notify();
    }

    /// Runs `command`'s effect -- one exhaustive match over [`CommandEffect`], the single
    /// source of truth for what a command does (issue #144's own architecture review, "deepen
    /// the command's interface": this replaced an `Option<fn(&mut NavState)>` handler plus a
    /// `Shell`-side `command.name == "open"` string match that couldn't express opening a
    /// `Shell`-owned dialog).
    ///
    /// [`CommandEffect::Navigate`] resets the view's scroll when it lands on a different noun,
    /// matching every other navigation entry point (`g`-jumps, rail `Enter`).
    /// [`CommandEffect::NotYetBuilt`] shows the same "not yet built" message
    /// `docs/ux/tui/navigation.md` describes for its own popup, reusing the status line's
    /// existing `status_message` slot (the "1d" spec's own COMMAND-mode status line has no
    /// message slot of its own, and the palette has already closed by the time this runs -- see
    /// the `enter` arm of [`Self::handle_palette_key`]).
    ///
    /// [`CommandEffect::OpenDialog`] (issues #165/#167) is the one variant that doesn't call
    /// `NavState::exit_mode` -- unlike every other effect, opening the dialog does *not* leave
    /// `InputMode::Command`: the "1e" file explorer's own status-line treatment (`Shell::render`'s
    /// `command_echo`) depends on staying there for as long as the dialog is on screen, exiting
    /// only when it closes (`Self::handle_explorer_cancel`/`Self::confirm_explorer_open`, or
    /// `Self::handle_key_down`'s `escape` arm).
    fn run_command(&mut self, command: &'static Command, argument: &str) {
        // History keeps what was typed, argument and all, so `^r` recalls `accounts delete Home
        // Loan` rather than just the bare command.
        if argument.is_empty() {
            record_history(&mut self.command_history, command.name);
        } else {
            record_history(
                &mut self.command_history,
                &format!("{} {argument}", command.name),
            );
        }
        match command.effect {
            CommandEffect::OpenDialog(mode) => {
                self.file_explorer = Some(FileExplorer::open_at(
                    mode,
                    explorer_start_dir(),
                    self.explorer_filters,
                ));
            }
            CommandEffect::Navigate(noun) => {
                self.nav.exit_mode();
                let noun_before = self.nav.noun();
                self.nav.set_noun(noun);
                if self.nav.noun() != noun_before {
                    self.reset_view_scroll();
                }
            }
            CommandEffect::OpenSettingsPage(section) => {
                self.nav.exit_mode();
                self.open_settings_page(section);
            }
            CommandEffect::CloseLedger => {
                self.nav.exit_mode();
                self.nav.close_ledger();
            }
            CommandEffect::Accounts(verb) => {
                self.nav.exit_mode();
                self.run_accounts_command(command.name, verb, argument);
            }
            CommandEffect::Colour(change) => {
                self.nav.exit_mode();
                self.pending_colour_change = Some(change);
            }
            CommandEffect::DismissToast => {
                self.nav.exit_mode();
                self.toasts.dismiss_newest();
            }
            CommandEffect::DismissAllToasts => {
                self.nav.exit_mode();
                self.toasts.dismiss_all();
            }
            CommandEffect::SetToasts(on) => {
                self.nav.exit_mode();
                self.set_toasts_on(on);
            }
            CommandEffect::OpenToastHistory => {
                self.nav.exit_mode();
                self.open_toast_history();
            }
            CommandEffect::MergeTags => {
                self.nav.exit_mode();
                self.open_settings_page(SettingsSection::Tags);
                self.open_merge_tags_dialog(None);
            }
            CommandEffect::Import => {
                self.nav.exit_mode();
                self.open_import();
            }
            CommandEffect::Documents(verb) => {
                self.nav.exit_mode();
                self.run_documents_command(verb);
            }
            CommandEffect::Budgets(verb) => {
                self.nav.exit_mode();
                let noun_before = self.nav.noun();
                self.nav.set_noun(Noun::Budgets);
                if noun_before != Noun::Budgets {
                    self.reset_view_scroll();
                }
                match verb {
                    BudgetsVerb::Switch => self.open_budgets_switcher(),
                    BudgetsVerb::New => self.open_budgets_new(),
                    BudgetsVerb::Edit => self.open_budgets_edit(self.budgets_current),
                    BudgetsVerb::Manage => self.open_budgets_manage(),
                    BudgetsVerb::Duplicate => {
                        self.run_budgets_manage_action(
                            self.budgets_current,
                            ManageAction::Duplicate,
                        );
                    }
                    BudgetsVerb::SetDefault => {
                        self.run_budgets_manage_action(
                            self.budgets_current,
                            ManageAction::SetDefault,
                        );
                    }
                    BudgetsVerb::Archive => {
                        self.run_budgets_manage_action(self.budgets_current, ManageAction::Archive);
                    }
                    BudgetsVerb::Restore => {
                        self.run_budgets_manage_action(self.budgets_current, ManageAction::Restore);
                    }
                }
            }
            CommandEffect::NotYetBuilt => {
                self.nav.exit_mode();
                self.status_message = Some(crate::msg::desktop_status_command_not_yet_built(
                    command.name,
                ));
            }
        }
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
                    self.status_message = Some(crate::msg::desktop_status_no_accounts(&format!(
                        ":{command_name}"
                    )));
                    return;
                }
            }
        } else {
            match accounts::find_by_name(&self.accounts, argument) {
                NameLookup::Found(index) => index,
                NameLookup::NotFound => {
                    self.status_message = Some(crate::msg::desktop_status_no_account_named(
                        &format!(":{command_name}"),
                        argument,
                    ));
                    return;
                }
                NameLookup::Ambiguous(names) => {
                    self.status_message = Some(crate::msg::desktop_status_account_ambiguous(
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

    /// The TopBar's own rail-toggle button (`Shell::render`'s `on_rail_toggle` closure) --
    /// same action as the `b` key, see [`Self::handle_key_down`].
    fn handle_toggle_rail(&mut self, cx: &mut Context<'_, Self>) {
        self.nav.toggle_primary_rail();
        self.collapsed_rail_tooltip = None;
        cx.notify();
    }

    /// A collapsed primary-rail row's raw hover transition (`rail::primary::PrimaryRail`'s
    /// `on_row_hover`). Leaving a row clears any settled tooltip immediately; entering one
    /// only reveals its tooltip after [`TOOLTIP_REVEAL_DELAY`], and only if hover hasn't since
    /// moved elsewhere -- `hover_generation` is the guard: a stale timer whose captured
    /// generation no longer matches the current one simply does nothing.
    fn handle_rail_hover(&mut self, noun: Noun, hovered: bool, cx: &mut Context<'_, Self>) {
        self.hover_generation += 1;
        if !hovered {
            self.collapsed_rail_tooltip = None;
            cx.notify();
            return;
        }

        let generation = self.hover_generation;
        cx.spawn(async move |this, cx| {
            Timer::after(TOOLTIP_REVEAL_DELAY).await;
            this.update(cx, |shell, cx| {
                if shell.hover_generation == generation {
                    shell.collapsed_rail_tooltip = Some(noun);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// A primary-rail row's click (`rail::primary::PrimaryRail`'s `on_row_click`), expanded or
    /// collapsed alike. A direct `NavState::set_noun`, not a browse-then-commit -- the same
    /// call `g`-jump (`Self::handle_key_down`'s pending-`g` arm) and the palette
    /// (`Self::run_command`'s own `CommandEffect::Navigate` arm) both make, so rail click,
    /// `g`-jump and the palette land in the same state per acceptance criterion 1.
    fn handle_rail_click(&mut self, noun: Noun, cx: &mut Context<'_, Self>) {
        let noun_before = self.nav.noun();
        self.nav.set_noun(noun);
        if self.nav.noun() != noun_before {
            self.reset_view_scroll();
        }
        cx.notify();
    }

    /// The settings index rail's own row click (`rail::settings_index::OnEntryClick`):
    /// swaps the settings body to the clicked page and takes the active dark treatment
    /// (`docs/ux/desktop/Settings/README.md`'s "Navigation" bullet).
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
        self.status_message = Some(crate::msg::desktop_status_test_price_source_not_yet_built());
        cx.notify();
    }

    fn handle_price_source_edit_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index;
        self.status_message = Some(crate::msg::desktop_status_edit_price_source_not_yet_built());
        cx.notify();
    }

    fn handle_price_source_delete_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index;
        self.status_message = Some(crate::msg::desktop_status_delete_price_source_not_yet_built());
        cx.notify();
    }

    fn handle_add_price_source_click(&mut self, cx: &mut Context<'_, Self>) {
        self.status_message = Some(crate::msg::desktop_status_add_price_source_not_yet_built());
        cx.notify();
    }

    /// The Units table's own row "delete" button (issue #186, replacing the stub #177 left
    /// behind): opens the destructive Delete unit confirm dialog rather than flashing a status
    /// message. A no-op if `index` is somehow out of bounds (defensive only, same reasoning as
    /// [`Self::handle_unit_edit_click`]).
    fn handle_unit_delete_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if index >= self.settings_units.len() {
            return;
        }
        self.settings_dialog = Some(SettingsDialog::DeleteUnit(index, DeleteUnitForm::default()));
        self.nav.enter_mode(InputMode::Dialog);
        cx.notify();
    }

    /// The Institutions table's own row "edit"/"delete" buttons (issue #178). Unlike
    /// [`Self::handle_unit_edit_click`]/[`Self::handle_unit_delete_click`], neither stub names an
    /// issue: no `EditInstitution`/`DeleteInstitution` dialog is specified anywhere on this map
    /// (the README's own "Dialog lifecycle" table and `State` block only ever mention
    /// `AddInstitution`), so there is no ticket to point at.
    fn handle_institution_edit_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index; // no row-scoped state until a future ticket specifies this dialog
        self.status_message = Some(crate::msg::desktop_status_edit_institution_not_yet_built());
        cx.notify();
    }

    fn handle_institution_delete_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let _ = index; // no row-scoped state until a future ticket specifies this dialog
        self.status_message = Some(crate::msg::desktop_status_delete_institution_not_yet_built());
        cx.notify();
    }

    /// The Institutions table's own "+ Add institution" button (issue #187, replacing the stub
    /// #178 left behind): opens the real Add institution dialog rather than flashing a status
    /// message. `AddInstitutionForm::new` seeds Default unit from `self.settings_units`' own
    /// first entry, so this dialog reads Units' live state even though the two sections are
    /// otherwise independent.
    fn handle_add_institution_click(&mut self, cx: &mut Context<'_, Self>) {
        self.settings_dialog = Some(SettingsDialog::AddInstitution(AddInstitutionForm::new(
            &self.settings_units,
        )));
        self.nav.enter_mode(InputMode::Dialog);
        cx.notify();
    }

    fn handle_add_institution_account_type_click(
        &mut self,
        account_type: AccountType,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(SettingsDialog::AddInstitution(form)) = self.settings_dialog.as_mut() {
            form.toggle_account_type(account_type);
            cx.notify();
        }
    }

    fn handle_add_institution_unit_click(&mut self, code: String, cx: &mut Context<'_, Self>) {
        if let Some(SettingsDialog::AddInstitution(form)) = self.settings_dialog.as_mut() {
            form.default_unit_code = Some(code);
            cx.notify();
        }
    }

    /// The Sync server section's own "Sync now" button (issue #180): unlike the "+ Add"
    /// buttons above, this has no future ticket that will give it real behaviour -- the map's
    /// own Out-of-scope names it a permanent stand-in -- so the stub message names no issue.
    fn handle_sync_now_click(&mut self, cx: &mut Context<'_, Self>) {
        self.status_message = Some(crate::msg::desktop_status_sync_now_not_implemented());
        cx.notify();
    }

    /// The Data & backup section's own "Backup now"/"Export ledger (CSV)" buttons (issue #181)
    /// -- same permanently-out-of-scope reasoning as [`Self::handle_sync_now_click`].
    fn handle_backup_now_click(&mut self, cx: &mut Context<'_, Self>) {
        self.status_message = Some(crate::msg::desktop_status_backup_now_not_implemented());
        cx.notify();
    }

    fn handle_export_ledger_click(&mut self, cx: &mut Context<'_, Self>) {
        self.status_message = Some(crate::msg::desktop_status_export_ledger_not_implemented());
        cx.notify();
    }

    /// The Tracing (Logs) section's own level radios (issue #182): a stored preference, same
    /// shape as [`Self::handle_row_density_click`] -- there are no real log lines to filter by
    /// level yet.
    fn handle_tracing_level_click(&mut self, level: TracingLevel, cx: &mut Context<'_, Self>) {
        self.settings_tracing_level = level;
        cx.notify();
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

    /// The same section's "Clear logs" button: unlike every other button this map has built,
    /// this one has a real effect -- the ticket's own body asks for the viewport's in-memory
    /// contents to actually empty, not a stubbed status-line message.
    fn handle_clear_logs_click(&mut self, cx: &mut Context<'_, Self>) {
        self.settings_log_lines.clear();
        cx.notify();
    }

    /// A file explorer row click (`explorer::OnEntryClick`): applies it to `FileExplorer`'s own
    /// state, then -- README's "double-click a `.pldb` row opens immediately" -- confirms the
    /// open immediately when `click_count` reports a real double-click landing on a row that
    /// (as of the resulting state) is the current selection. A single click on a not-yet-open
    /// `.pldb` row only selects it; a second, separate click completing the double-click is
    /// what actually opens it.
    fn handle_explorer_entry_click(
        &mut self,
        path: PathBuf,
        click_count: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(explorer) = self.file_explorer.as_mut() else {
            return;
        };
        explorer.click_entry(&path);
        if click_count >= 2 && explorer.selected() == Some(path.as_path()) {
            self.confirm_explorer_open(cx);
            return;
        }
        cx.notify();
    }

    /// A breadcrumb segment click (`explorer::OnBreadcrumbClick`).
    fn handle_explorer_breadcrumb_click(&mut self, path: PathBuf, cx: &mut Context<'_, Self>) {
        if let Some(explorer) = self.file_explorer.as_mut() {
            explorer.navigate_to(path);
            cx.notify();
        }
    }

    /// A footer checkbox click (`explorer::OnFilterToggle`): re-filters the open dialog and keeps
    /// the new state for the next one and for the quit-time save.
    fn handle_explorer_filter_toggle(
        &mut self,
        filter: ExplorerFilter,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(explorer) = self.file_explorer.as_mut() {
            explorer.toggle_filter(filter);
            self.explorer_filters = explorer.filters();
            cx.notify();
        }
    }

    /// The explorer dialog's own Cancel button: closes without opening anything, leaving
    /// Command mode the same way the palette's own `esc` does.
    fn handle_explorer_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.file_explorer = None;
        self.nav.exit_mode();
        cx.notify();
    }

    /// The help overlay's own Close button.
    fn handle_help_close(&mut self, cx: &mut Context<'_, Self>) {
        self.nav.exit_mode();
        cx.notify();
    }

    /// The explorer dialog's own Open button.
    fn handle_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_explorer_open(cx);
    }

    /// Confirms the explorer's current selection and closes the dialog. In `ExplorerMode::Open`
    /// this is the stand-in "opening" effect (`NavState::open_ledger`, from issue #164) -- real
    /// `.pldb` parsing stays out of scope for this map. In `ExplorerMode::New` (issue #167) it
    /// closes without touching `NavState::ledger_open` at all: the dialog is a literal copy of
    /// Open's for now, but confirming an *existing* file was never what "new" means, even as a
    /// stand-in -- the real "create a fresh `.pldb`" workflow is still fog. A no-op if nothing
    /// is selected (Open/New is only clickable once `FileExplorer::can_open` is true, but a
    /// double-click can also reach here -- see [`Self::handle_explorer_entry_click`] -- so this
    /// re-checks rather than trusting the caller).
    fn confirm_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        let Some(explorer) = self.file_explorer.as_ref() else {
            return;
        };
        if !explorer.can_open() {
            return;
        }
        let name = explorer
            .selected()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Neither mode can fail yet (no real `.pldb` I/O), so `toast-ledger-open-failed` waits
        // for the ticket that parses and creates ledger files.
        if explorer.mode() == ExplorerMode::Open {
            self.nav.open_ledger();
            self.raise_toast(
                ToastKind::Success,
                lib_locale::msg::toast_ledger_opened(&name),
            );
        } else {
            self.raise_toast(
                ToastKind::Success,
                lib_locale::msg::toast_ledger_created(&name),
            );
        }
        self.file_explorer = None;
        self.nav.exit_mode();
        cx.notify();
    }

    /// A click on the empty state's own `:open`/`:new` text (`OnEmptyStateCommandClick`, issue
    /// #167): looks `command_name` up in the registry and runs it exactly as the palette's own
    /// `enter` key would -- entering `InputMode::Command` first, same as typing `:` would, so
    /// the status line's `COMMAND` badge and `Esc` (which only acts outside `InputMode::Normal`)
    /// both behave identically regardless of which entry point opened the dialog.
    fn handle_empty_state_command_click(
        &mut self,
        command_name: &'static str,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(command) = command::all().find(|command| command.name == command_name) else {
            return;
        };
        self.nav.enter_mode(InputMode::Command);
        self.run_command(command, "");
        if let Some(change) = self.pending_colour_change.take() {
            change.apply(cx);
        }
        cx.notify();
    }
}

impl Focusable for Shell {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        // 6e shows only on Transactions: leaving the page (a jump, a rail click) abandons it.
        if self.nav.noun() != Noun::Transactions {
            self.import = None;
        }
        let focus = self.nav.focus();
        // The "1d" spec: "The shell behind the palette drops to 30% opacity" -- the "1e" file
        // explorer reuses the same dimming pattern (Implementation note 10). Applied to the top
        // bar and content row only, not the status line -- that same spec separately describes
        // the status line's own COMMAND-mode content (the live query, "esc close command
        // window"), which stays meaningful precisely because it stays legible; only the
        // navigational chrome the palette/explorer visually floats over goes dim.
        let content_opacity = if self.palette.is_some()
            || self.file_explorer.is_some()
            || self.nav.mode() == InputMode::Help
        {
            0.3
        } else {
            1.0
        };

        // Both closures go through an `Entity` handle (mirroring `feasibility_demo`'s own
        // `TabBar::on_click` wiring) rather than `cx.listener`: `on_click`/`on_hover`'s own
        // signatures are `Fn(_, &mut Window, &mut App)`, with no `&mut Shell` parameter for
        // `cx.listener` to supply, and the collapsed rail's `on_row_hover` closure additionally
        // needs to close over each row's own `Noun` -- `PrimaryRail` curries that in per-row
        // from the single `Rc` given here.
        let entity = cx.entity();
        let on_rail_toggle: topbar::OnRailToggle = {
            let entity = entity.clone();
            Rc::new(move |_event, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_toggle_rail(cx));
            })
        };
        let on_row_hover: rail::primary::OnRowHover = {
            let entity = entity.clone();
            Rc::new(move |noun, hovered, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_rail_hover(noun, hovered, cx));
            })
        };
        let on_row_click: rail::primary::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |noun, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_rail_click(noun, cx));
            })
        };
        let on_explorer_entry_click: explorer::OnEntryClick = {
            let entity = entity.clone();
            Rc::new(move |path, click_count, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_explorer_entry_click(path, click_count, cx)
                });
            })
        };
        let on_explorer_breadcrumb_click: explorer::OnBreadcrumbClick = {
            let entity = entity.clone();
            Rc::new(move |path, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_explorer_breadcrumb_click(path, cx)
                });
            })
        };
        let on_explorer_filter_toggle: explorer::OnFilterToggle = {
            let entity = entity.clone();
            Rc::new(move |filter, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_explorer_filter_toggle(filter, cx)
                });
            })
        };
        let on_hint: statusline::OnHint = {
            let entity = entity.clone();
            Rc::new(move |action, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_hint_click(action, cx));
            })
        };
        let on_toast_dismiss: crate::toast::OnDismiss = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.toasts.dismiss_visible(index);
                    cx.notify();
                });
            })
        };
        let on_toast_hover: crate::toast::OnHover = {
            let entity = entity.clone();
            Rc::new(move |hovered, _window, cx| {
                entity.update(cx, |shell, _cx| shell.toasts_hovered = hovered);
            })
        };
        // Hidden while the history is open, which lists them in full.
        let toast_layer = if self.toast_history_open {
            None
        } else {
            crate::toast::render(&self.toasts, on_toast_dismiss, on_toast_hover, cx)
        };
        let on_toast_history_close: toast_history_view::OnClose = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.toast_history_open = false;
                    shell.nav.exit_mode();
                    cx.notify();
                });
            })
        };
        let on_help_close: help_view::OnClose = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_help_close(cx));
            })
        };
        let on_explorer_cancel: explorer::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_explorer_cancel(cx));
            })
        };
        let on_explorer_open: explorer::OnOpen = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_explorer_open(cx));
            })
        };
        // Shared by the Add unit (issue #184) and Edit unit (issue #185) dialogs -- both wrap
        // the same `UnitForm`, and `Shell`'s own handlers already dispatch on whichever
        // `SettingsDialog` variant is actually open, so one set of closures serves both renders.
        let on_unit_dialog_field_click: settings_view::add_unit_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_unit_dialog_field_click(field, cx)
                });
            })
        };
        let on_unit_dialog_kind_click: settings_view::add_unit_dialog::OnKindClick = {
            let entity = entity.clone();
            Rc::new(move |kind, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_unit_dialog_kind_click(kind, cx)
                });
            })
        };
        let on_settings_dialog_cancel: settings_view::add_unit_dialog::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_settings_dialog_cancel(cx));
            })
        };
        let on_settings_dialog_confirm: settings_view::add_unit_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_settings_dialog_confirm(cx));
            })
        };
        let on_add_institution_account_type_click: settings_view::add_institution_dialog::OnAccountTypeClick = {
            let entity = entity.clone();
            Rc::new(move |account_type, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_add_institution_account_type_click(account_type, cx)
                });
            })
        };
        let on_add_institution_unit_click: settings_view::add_institution_dialog::OnUnitClick = {
            let entity = entity.clone();
            Rc::new(move |code, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_add_institution_unit_click(code, cx)
                });
            })
        };
        let on_empty_state_command_click: OnEmptyStateCommandClick = {
            let entity = entity.clone();
            Rc::new(move |command_name, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_empty_state_command_click(command_name, cx)
                });
            })
        };
        let on_settings_index_click: settings_index::OnEntryClick = {
            let entity = entity.clone();
            Rc::new(move |section, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_settings_index_click(section, cx)
                });
            })
        };
        let on_price_source_test_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_price_source_test_click(index, cx)
                });
            })
        };
        let on_price_source_edit_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_price_source_edit_click(index, cx)
                });
            })
        };
        let on_price_source_delete_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_price_source_delete_click(index, cx)
                });
            })
        };
        let on_add_price_source_click: settings_view::units::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_add_price_source_click(cx));
            })
        };
        let on_unit_edit_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_unit_edit_click(index, cx));
            })
        };
        let on_unit_delete_click: settings_view::units::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_unit_delete_click(index, cx));
            })
        };
        let on_add_unit_click: settings_view::units::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_add_unit_click(cx));
            })
        };
        let on_institution_edit_click: settings_view::institutions::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_institution_edit_click(index, cx)
                });
            })
        };
        let on_institution_delete_click: settings_view::institutions::OnRowIndexClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_institution_delete_click(index, cx)
                });
            })
        };
        let on_add_institution_click: settings_view::institutions::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_add_institution_click(cx));
            })
        };
        let on_sync_now_click: settings_view::sync_server::OnSyncNowClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_sync_now_click(cx));
            })
        };
        let on_backup_now_click: settings_view::data_backup::OnBackupNowClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_backup_now_click(cx));
            })
        };
        let on_export_ledger_click: settings_view::data_backup::OnExportLedgerClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_export_ledger_click(cx));
            })
        };
        let on_tracing_level_click: settings_view::tracing::OnLevelClick = {
            let entity = entity.clone();
            Rc::new(move |level, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_tracing_level_click(level, cx));
            })
        };
        let on_clear_logs_click: settings_view::tracing::OnClearLogsClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_clear_logs_click(cx));
            })
        };
        let on_date_style_click: settings_view::display::OnDateStyleClick = {
            let entity = entity.clone();
            Rc::new(move |style, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_date_style_click(style, cx));
            })
        };
        let on_row_density_click: settings_view::display::OnRowDensityClick = {
            let entity = entity.clone();
            Rc::new(move |density, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_row_density_click(density, cx));
            })
        };
        // A click selects the card and clears any keyboard focus left on the grid.
        let on_colour_theme_click: settings_view::colour_theme::OnColourThemeClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.colour_theme_focus = None;
                    cx.notify();
                });
                ColourChange::Theme(id).apply(cx);
            })
        };
        let on_status_glyphs_click: settings_view::display::OnStatusGlyphsClick = {
            let entity = entity.clone();
            Rc::new(move |glyphs, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_status_glyphs_click(glyphs, cx));
            })
        };

        let on_toasts_click: settings_view::display::OnToastsClick = {
            let entity = entity.clone();
            Rc::new(move |on, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.set_toasts_on(on);
                    cx.notify();
                });
            })
        };

        let on_start_sidebar_minimised_click: settings_view::display::OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_start_sidebar_minimised_click(cx)
                });
            })
        };

        let on_accounts_add_click: accounts_view::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_add_click(cx));
            })
        };
        let on_accounts_row_click: accounts_view::OnAccountClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_row_click(id, cx));
            })
        };
        let on_accounts_edit_click: accounts_view::OnAccountClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_edit_click(id, cx));
            })
        };
        let on_accounts_delete_click: accounts_view::OnAccountClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_delete_click(id, cx));
            })
        };
        let on_accounts_dialog_field_click: accounts_view::add_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_accounts_dialog_field_click(field, cx)
                });
            })
        };
        let on_accounts_dialog_option_click: accounts_view::add_dialog::OnOptionClick = {
            let entity = entity.clone();
            Rc::new(move |field, index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_accounts_dialog_option_click(field, index, cx)
                });
            })
        };
        let on_accounts_dialog_cancel: accounts_view::add_dialog::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_dialog_cancel(cx));
            })
        };
        let on_accounts_dialog_confirm: accounts_view::add_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_accounts_dialog_confirm(cx));
            })
        };
        let account_options = self.account_dialog_options();
        let selected_account = self.selected_account_index();
        let accounts_page = accounts_view::AccountsPageProps {
            accounts: &self.accounts,
            units: &self.settings_units,
            selected: selected_account,
            on_add_click: on_accounts_add_click,
            on_row_click: on_accounts_row_click,
            on_edit_click: on_accounts_edit_click,
            on_delete_click: on_accounts_delete_click,
        };
        let on_categories_add_click: categories_view::OnAddClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_categories_add_click(cx));
            })
        };
        let on_categories_add_sub_click: categories_view::OnAddSubClick = {
            let entity = entity.clone();
            Rc::new(move |parent_id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_add_sub_click(parent_id, cx)
                });
            })
        };
        let on_categories_edit_click: categories_view::OnEditClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_categories_edit_click(id, cx));
            })
        };
        let on_categories_delete_click: categories_view::OnDeleteClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_categories_delete_click(id, cx));
            })
        };
        let on_categories_disclosure_click: categories_view::OnDisclosureClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_disclosure_click(id, cx)
                });
            })
        };
        let payee_click = |handler: fn(&mut Shell, u32, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: settings_view::payees::OnPayeeClick = Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| handler(shell, id, cx));
            });
            on_click
        };
        let on_payees_add_click: crate::dialog::OnClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_payees_add_click(cx));
            })
        };
        let plain_payees = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let bills_dialog_element = self.bills_dialog.as_ref().and_then(|dialog| {
            let (editing, form) = match dialog {
                bills::BillsDialog::Add(form) => (None, form),
                bills::BillsDialog::Edit(id, form) => {
                    (Some(bills::get(&self.bill_plans, *id)?.name.as_str()), form)
                }
                bills::BillsDialog::Pay(form) => {
                    return self.render_pay_bill_dialog(form, &entity, cx);
                }
                bills::BillsDialog::Skip(entry) => {
                    return self.render_skip_bill_dialog(*entry, &entity, cx);
                }
            };
            let options = self.bill_plan_options();
            let (today, date_style) = (self.today, self.settings_date_style);
            let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                    entity.update(cx, handler);
                });
                on_click
            };
            let on_field_click: bills_view::plan_dialog::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_field_click(field, cx)
                    });
                })
            };
            let on_option_click: bills_view::plan_dialog::OnOptionClick = {
                let entity = entity.clone();
                Rc::new(move |field, index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_option_click(field, index, cx);
                    });
                })
            };
            let on_amount_kind_click: bills_view::plan_dialog::OnAmountKindClick = {
                let entity = entity.clone();
                Rc::new(move |kind, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_amount_kind_click(kind, cx);
                    });
                })
            };
            Some(bills_view::plan_dialog::render(
                bills_view::plan_dialog::PlanDialogProps {
                    editing,
                    form,
                    options: &options,
                    errors: form.errors(&options, today, date_style),
                    valid: form.draft(&options, today, date_style).is_some(),
                    handlers: bills_view::plan_dialog::PlanDialogHandlers {
                        on_field_click,
                        on_option_click,
                        on_amount_kind_click,
                        on_cancel: plain(Shell::handle_bills_dialog_cancel),
                        on_confirm: plain(Shell::handle_bill_plan_confirm),
                    },
                },
                cx,
            ))
        });
        let payees_dialog_handlers = {
            let plain = plain_payees;
            let indexed = |handler: fn(&mut Shell, usize, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: payees_view::add_dialog::OnOptionClick =
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| handler(shell, index, cx));
                    });
                on_click
            };
            let on_field_click: payees_view::add_dialog::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_payees_dialog_field_click(field, cx);
                    });
                })
            };
            payees_view::add_dialog::PayeeDialogHandlers {
                on_field_click,
                on_option_click: indexed(Shell::handle_payees_dialog_option_click),
                on_add_rule: plain(Shell::handle_payees_dialog_add_rule),
                on_remove_rule: indexed(Shell::handle_payees_dialog_remove_rule),
                on_cancel: plain(Shell::handle_payees_dialog_cancel),
                on_confirm: plain(Shell::handle_payees_dialog_confirm),
            }
        };
        let settings_payees_page = settings_view::payees::PayeesPageProps {
            payees: &self.payees,
            categories: &self.categories,
            selected: self.settings_payees_selected_id(),
            on_add_click: on_payees_add_click,
            on_row_click: payee_click(Shell::handle_settings_payees_row_click),
            on_edit_click: payee_click(Shell::handle_payees_edit_click),
            on_delete_click: payee_click(Shell::handle_payees_delete_click),
        };
        let tag_click = |handler: fn(&mut Shell, u32, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: tags_view::OnTagClick = Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| handler(shell, id, cx));
            });
            on_click
        };
        let tag_plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: tags_view::OnPlainClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let tags_dialog_handlers = {
            let on_field_click: tags_view::add_dialog::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_tags_dialog_field_click(field, cx);
                    });
                })
            };
            let on_pick: tags_view::colour_field::OnPick = {
                let entity = entity.clone();
                Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| shell.handle_tags_dialog_pick(index, cx));
                })
            };
            tags_view::add_dialog::TagDialogHandlers {
                on_field_click,
                on_pick,
                on_cancel: tag_plain(Shell::handle_tags_dialog_cancel),
                on_confirm: tag_plain(Shell::handle_tags_dialog_confirm),
            }
        };
        let tag_groups = tags::duplicate_groups(&self.tags, &self.transactions);
        let settings_tags_page = settings_view::tags::TagsPageProps {
            tags: &self.tags,
            selected: self.settings_tags_selected_id(),
            groups: &tag_groups,
            on_add_click: tag_plain(Shell::handle_tags_add_click),
            on_row_click: tag_click(Shell::handle_settings_tags_row_click),
            on_merge_click: tag_click(Shell::handle_tags_duplicate_click),
            on_edit_click: tag_click(Shell::handle_tags_edit_click),
            on_remove_click: tag_click(Shell::handle_tags_remove_click),
        };
        let bills_unfiltered = self.bills_unfiltered_rows();
        let bills_rows = self
            .bills_filters
            .apply(&bills_unfiltered, &self.bill_plans);
        let bills_summary = bills::period_summary(
            &bills_rows,
            &self.bill_plans,
            &self.bill_entries,
            &self.transactions,
        );
        let bills_planner_plans = bills::planner_order(&self.bill_plans);
        let bills_base_unit = self
            .settings_units
            .iter()
            .find(|unit| unit.is_base)
            .map(|unit| unit.code.as_str());
        let bills_plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: bills_view::OnPlainClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let bills_indexed = |handler: fn(&mut Shell, usize, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: bills_view::OnRowClick = Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| handler(shell, index, cx));
            });
            on_click
        };
        let on_dashboard_bill_click: dashboard::OnBillClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.open_bill_entry(id);
                    cx.notify();
                });
            })
        };
        // The default Budget's current month: the rail badge and the Dashboard's budget list.
        let default_budget_figures = self.budgets.default_budget().map(|budget| {
            budgets::period_figures(
                budget,
                &self.budgets_ledger(),
                bills::Period::of(self.today),
                self.today,
            )
        });
        let dashboard = Dashboard::new(
            bills::attention_entries(&self.bill_plans, &self.bill_entries, self.today)
                .into_iter()
                .filter_map(|id| {
                    let entry = bills::entry(&self.bill_entries, id)?;
                    let plan = bills::get(&self.bill_plans, id.plan_id)?;
                    let amount = bills::amount(entry, &self.bill_plans, &self.transactions)?;
                    Some(dashboard::AttentionBill {
                        id,
                        plan: plan.name.clone(),
                        overdue: bills::status(entry, self.today) == bills::BillStatus::Overdue,
                        due: format::date(id.due, self.settings_date_style),
                        amount: bills_view::schedule::with_unit(
                            format::amount(&amount).1,
                            &plan.unit,
                            bills_base_unit,
                        ),
                    })
                })
                .collect(),
            format::flag_glyph(self.settings_status_glyphs),
            on_dashboard_bill_click,
        )
        .budgets(
            default_budget_figures
                .as_ref()
                .map(|figures| self.dashboard_budget_list(figures))
                .unwrap_or_default(),
        );
        let bills_page = bills_view::BillsPageProps {
            tab: self.bills_tab,
            period: self.bills_period,
            all: self.bills_all,
            schedule: bills_view::schedule::ScheduleProps {
                rows: &bills_rows,
                total: bills_unfiltered.len(),
                all: self.bills_all,
                filters: bills_view::filters::FilterProps {
                    filters: &self.bills_filters,
                    selects: bills_view::filters::FilterField::ORDER
                        .into_iter()
                        .map(|field| {
                            let focused = self
                                .bills_filter_focus
                                .as_ref()
                                .filter(|(focused, _)| *focused == field);
                            bills_view::filters::FilterSelect {
                                field,
                                options: self.bills_filter_options(field).0,
                                state: focused.map_or_else(
                                    || self.bills_filter_select_state(field),
                                    |(_, state)| state.clone(),
                                ),
                                focused: focused.is_some(),
                            }
                        })
                        .collect(),
                    stats: self
                        .bills_filters
                        .plan_id
                        .and_then(|id| bills::get(&self.bill_plans, id))
                        .map(|plan| {
                            let stats = bill_history::plan_stats(
                                plan,
                                &self.bill_plans,
                                &self.bill_entries,
                                &self.transactions,
                                self.today,
                            );
                            (plan, stats)
                        }),
                    base_unit: bills_base_unit,
                    on_chip_click: bills_indexed(Shell::handle_bills_filter_chip_click),
                    on_field_click: {
                        let entity = entity.clone();
                        Rc::new(move |field, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.handle_bills_filter_field_click(field, cx);
                            });
                        })
                    },
                    on_option_click: {
                        let entity = entity.clone();
                        Rc::new(move |field, index, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.handle_bills_filter_option_click(field, index, cx);
                            });
                        })
                    },
                },
                summary: &bills_summary,
                plans: &self.bill_plans,
                entries: &self.bill_entries,
                accounts: &self.accounts,
                transactions: &self.transactions,
                base_unit: bills_base_unit,
                glyphs: self.settings_status_glyphs,
                selected: (!bills_rows.is_empty())
                    .then(|| self.bills_selected.min(bills_rows.len() - 1)),
                on_row_click: bills_indexed(Shell::handle_bills_row_click),
                on_pay_click: bills_indexed(Shell::handle_bills_pay_click),
                on_skip_click: bills_indexed(Shell::handle_bills_skip_click),
                on_view_transaction_click: bills_indexed(
                    Shell::handle_bills_view_transaction_click,
                ),
            },
            planner: bills_view::planner::PlannerProps {
                plans: &bills_planner_plans,
                inactive: bills::inactive_count(&self.bill_plans),
                categories: &self.categories,
                accounts: &self.accounts,
                base_unit: bills_base_unit,
                selected: (!bills_planner_plans.is_empty())
                    .then(|| self.bills_selected.min(bills_planner_plans.len() - 1)),
                on_row_click: bills_indexed(Shell::handle_bills_row_click),
                on_edit_click: bills_indexed(Shell::handle_bills_edit_plan_click),
            },
            on_add_click: bills_plain(Shell::handle_bills_add_click),
            on_tab_click: {
                let entity = entity.clone();
                Rc::new(move |tab, _window, cx| {
                    entity.update(cx, |shell, cx| shell.handle_bills_tab_click(tab, cx));
                })
            },
            on_period_prev: bills_plain(Shell::handle_bills_period_prev),
            on_period_next: bills_plain(Shell::handle_bills_period_next),
            on_all_click: bills_plain(Shell::handle_bills_all_click),
        };
        let import_categories = self.import_category_options();
        let import_page = self.import.as_ref().map(|state| {
            let entity_for = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: import_view::OnClick = Rc::new(move |_window, cx| {
                    entity.update(cx, handler);
                });
                on_click
            };
            let indexed = |handler: fn(&mut Shell, usize, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: import_view::OnRowClick = Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| handler(shell, index, cx));
                });
                on_click
            };
            let on_select_click: import_view::OnSelectClick = {
                let entity = entity.clone();
                Rc::new(move |index, select, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_import_select_click(index, select, cx);
                    });
                })
            };
            import_view::ImportPageProps {
                state,
                payees: &self.payees,
                categories: &import_categories,
                on_row_click: indexed(Shell::handle_import_row_click),
                on_select_click,
                on_option_click: indexed(Shell::handle_import_option_click),
                on_remember_click: entity_for(Shell::handle_import_remember_click),
                on_back_click: entity_for(Shell::handle_import_back_click),
                on_continue_click: entity_for(Shell::handle_import_continue_click),
            }
        });
        let budgets_figures = (self.nav.noun() == Noun::Budgets)
            .then(|| self.budgets_figures())
            .flatten();
        let budgets_plan = (self.nav.noun() == Noun::Budgets
            && self.budgets_tab == budgets::BudgetsTab::Plan)
            .then(|| self.budgets_plan_data())
            .flatten();
        let budgets_history = (self.nav.noun() == Noun::Budgets
            && self.budgets_tab == budgets::BudgetsTab::History)
            .then(|| self.budgets_history_data())
            .flatten();
        let budgets_page = budgets_figures.as_ref().map(|(budget, figures)| {
            let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: budgets_view::OnPlainClick = Rc::new(move |_window, cx| {
                    entity.update(cx, handler);
                });
                on_click
            };
            budgets_view::BudgetsPageProps {
                name: &budget.name,
                method: budget.method,
                tab: self.budgets_tab,
                period: self.budgets_period,
                figures,
                plan: budgets_plan
                    .as_ref()
                    .map(|plan| budgets_view::plan::PlanProps {
                        plan,
                        current: bills::Period::of(self.today),
                        cursor: self.budgets_plan_cursor_in(plan),
                        edit: self.budgets_plan_edit.as_ref(),
                        on_cell_click: {
                            let entity = entity.clone();
                            Rc::new(move |row, column, _window, cx| {
                                entity.update(cx, |shell, cx| {
                                    shell.handle_budgets_plan_cell_click(row, column, cx);
                                });
                            })
                        },
                    }),
                history: budgets_history.as_ref().map(|history| {
                    budgets_view::history::HistoryProps {
                        history,
                        categories: &self.categories,
                        cursor: self.budgets_history_cursor_in(history),
                        on_cell_click: {
                            let entity = entity.clone();
                            Rc::new(move |row, column, _window, cx| {
                                entity.update(cx, |shell, cx| {
                                    shell.handle_budgets_history_cell_click(row, column, cx);
                                });
                            })
                        },
                    }
                }),
                on_export_click: plain(Shell::handle_budgets_export_click),
                on_range_prev: plain(Shell::handle_budgets_range_prev),
                on_range_next: plain(Shell::handle_budgets_range_next),
                categories: &self.categories,
                selected: (!figures.rows.is_empty())
                    .then(|| self.budgets_selected.min(figures.rows.len() - 1)),
                on_tab_click: {
                    let entity = entity.clone();
                    Rc::new(move |tab, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_budgets_tab_click(tab, cx));
                    })
                },
                on_title_click: plain(Shell::handle_budgets_title_click),
                on_period_prev: plain(Shell::handle_budgets_period_prev),
                on_period_next: plain(Shell::handle_budgets_period_next),
                on_edit_plan_click: plain(Shell::handle_budgets_edit_plan_click),
                on_add_click: plain(Shell::handle_budgets_add_click),
                fill_label: crate::msg::desktop_budgets_fill_button(
                    &lib_locale::format::format_month(self.budgets_fill_target().month),
                ),
                on_fill_click: plain(Shell::handle_budgets_fill_click),
                on_action_click: {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_budgets_action_click(index, cx);
                        });
                    })
                },
                on_row_click: {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_budgets_row_click(index, cx));
                    })
                },
                on_known_click: plain(Shell::handle_budgets_known_click),
            }
        });
        let on_settings_categories_row_click: categories_view::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_settings_categories_row_click(id, cx)
                });
            })
        };
        let settings_categories_page = settings_view::categories::CategoriesPageProps {
            categories: &self.categories,
            expanded: &self.categories_expanded,
            selected: self.categories_selected_id,
            on_add_click: on_categories_add_click.clone(),
            on_add_sub_click: on_categories_add_sub_click.clone(),
            on_edit_click: on_categories_edit_click.clone(),
            on_delete_click: on_categories_delete_click.clone(),
            on_disclosure_click: on_categories_disclosure_click.clone(),
            on_row_click: on_settings_categories_row_click,
        };

        // Categories dialog closures
        let on_categories_dialog_field_click: categories_view::add_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_field_click(field, cx);
                });
            })
        };
        let on_categories_dialog_parent_change: categories_view::add_dialog::OnParentChange = {
            let entity = entity.clone();
            Rc::new(move |parent_id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_parent_change(parent_id, cx);
                });
            })
        };
        let on_categories_dialog_type_change: categories_view::add_dialog::OnTypeChange = {
            let entity = entity.clone();
            Rc::new(move |category_type, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_type_change(category_type, cx);
                });
            })
        };
        let on_categories_dialog_cancel: categories_view::add_dialog::OnCancel = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_cancel(cx);
                });
            })
        };
        let on_categories_dialog_confirm: categories_view::add_dialog::OnConfirm = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_categories_dialog_confirm(cx);
                });
            })
        };

        let on_transactions_row_click: transactions_view::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_transactions_row_click(index, cx)
                });
            })
        };
        // Built only while the page is showing: formatting every visible row is a pass over the
        // whole filtered set, which no other page needs.
        let on_transactions_add_click: transactions_view::OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_transactions_add_click(cx));
            })
        };
        let on_transactions_chip_click: transactions_view::OnChipClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_transactions_chip_click(field, cx)
                });
            })
        };
        let on_transactions_chip_clear: transactions_view::OnChipClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_transactions_chip_clear(field, cx)
                });
            })
        };
        let on_transactions_clear_all: transactions_view::OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_transactions_clear_all(cx));
            })
        };
        let on_transactions_search_click: transactions_view::OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |shell, cx| shell.handle_transactions_search_click(cx));
            })
        };
        let filter_popover = self.transactions_filter_form.as_ref().map(|form| {
            let entity_for = |shell_call: fn(&mut Shell, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                Rc::new(move |_window: &mut Window, cx: &mut gpui::App| {
                    entity.update(cx, shell_call);
                }) as Rc<dyn Fn(&mut Window, &mut gpui::App)>
            };
            let on_field_click: transactions_view::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| shell.handle_filter_field_click(field, cx));
                })
            };
            let on_option_click: transactions_view::OnOptionClick = {
                let entity = entity.clone();
                Rc::new(move |field, index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_filter_option_click(field, index, cx)
                    });
                })
            };
            let on_status_click: transactions_view::OnStatusClick = {
                let entity = entity.clone();
                Rc::new(move |status, _window, cx| {
                    entity.update(cx, |shell, cx| shell.handle_filter_status_click(status, cx));
                })
            };
            // Anchor just below the chip that opened it, left-aligned to it and kept inside the
            // window; before any chip has been painted, a fixed spot.
            let viewport = window.viewport_size();
            let (left, top) = match self
                .transactions_chip_bounds
                .borrow()
                .get(&self.transactions_filter_anchor)
                .copied()
            {
                Some(bounds) => {
                    let mut left = bounds.origin.x;
                    let max_left = viewport.width - px(416.0);
                    if left > max_left {
                        left = max_left;
                    }
                    if left < px(8.0) {
                        left = px(8.0);
                    }
                    (left, bounds.origin.y + bounds.size.height + px(6.0))
                }
                None => (px(300.0), px(200.0)),
            };
            let options = self.filter_form_options();
            transactions_view::render_popover(
                transactions_view::PopoverProps {
                    form,
                    options: &options,
                    start_hint: form.start_hint(self.today, self.settings_date_style),
                    end_hint: form.end_hint(self.today, self.settings_date_style),
                    can_apply: form.is_valid(self.today, self.settings_date_style),
                    left,
                    top,
                    on_field_click,
                    on_option_click,
                    on_status_click,
                    on_reset: entity_for(Shell::handle_filter_reset),
                    on_apply: entity_for(Shell::handle_filter_apply),
                    on_cancel: entity_for(Shell::handle_filter_cancel),
                },
                cx,
            )
        });
        let transactions_page = (self.nav.noun() == Noun::Transactions).then(|| {
            let ledger = self.transactions_ledger();
            let visible = transaction_query::query(
                &ledger,
                &self.transactions,
                &self.transactions_filters,
                &self.transactions_search,
            );
            let rows = transaction_rows::build_rows(&visible, &ledger, &self.transactions_prefs());
            let footer = transaction_chips::footer(&visible, &self.transactions_filters, &ledger);
            let chips = transaction_chips::chips(
                &self.transactions_filters,
                &ledger,
                self.today,
                self.settings_date_style,
            );
            transactions_view::TransactionsPageProps {
                dimmed: self.transactions_filter_form.is_some(),
                header: transactions_view::HeaderProps {
                    count_line: transaction_chips::count_line(&self.transactions),
                    chips,
                    show_clear: !self.transactions_filters.is_default(self.today),
                    search: self.transactions_search.clone(),
                    searching: self.nav.mode() == InputMode::Search,
                    on_add_click: on_transactions_add_click,
                    on_chip_click: on_transactions_chip_click,
                    on_chip_clear: on_transactions_chip_clear,
                    on_clear_all: on_transactions_clear_all,
                    on_search_click: on_transactions_search_click,
                    chip_bounds: self.transactions_chip_bounds.clone(),
                },
                selected: transaction_rows::clamp_selection(self.transactions_selected, rows.len()),
                rows: Rc::new(rows),
                row_height: px(format::row_height_px(self.settings_row_density)),
                scroll: self.transactions_scroll.clone(),
                on_row_click: on_transactions_row_click,
                footer,
            }
        });
        let documents_page =
            (self.nav.noun() == Noun::Documents).then(|| self.documents_page_props(&entity));
        let page_status = match self.nav.noun() {
            Noun::Documents => self.documents_page_status(),
            Noun::Settings if self.accounts_page_has_focus() => Some(PageStatus {
                hints: accounts_hints(),
                right: crate::msg::desktop_accounts_count(
                    i64::try_from(self.accounts.len()).unwrap_or(i64::MAX),
                ),
            }),
            Noun::Settings if self.settings_tags_page_has_focus() => Some(PageStatus {
                hints: match self.tags_dialog {
                    Some(tags::TagsDialog::Add(_)) => tag_dialog_hints(false),
                    Some(tags::TagsDialog::Edit(..)) => tag_dialog_hints(true),
                    Some(tags::TagsDialog::Remove(..)) => delete_payee_dialog_hints(),
                    Some(tags::TagsDialog::Merge(_)) => merge_tags_dialog_hints(),
                    None => settings_tags_hints(),
                },
                right: settings_view::tags::scope_text(
                    &self.tags,
                    &tags::duplicate_groups(&self.tags, &self.transactions),
                ),
            }),
            Noun::Settings if self.settings_payees_page_has_focus() => Some(PageStatus {
                hints: match self.payees_dialog {
                    Some(payees::PayeesDialog::Delete(..)) => delete_payee_dialog_hints(),
                    Some(_) => payee_dialog_hints(),
                    None => settings_payees_hints(),
                },
                right: settings_view::payees::scope_text(&self.payees),
            }),
            Noun::Settings if self.settings_categories_page_has_focus() => Some(PageStatus {
                hints: settings_categories_hints(),
                right: settings_view::categories::scope_note(&self.categories),
            }),
            Noun::Settings if self.nav.focus() == FocusZone::View => Some(PageStatus {
                hints: self.settings_hints(),
                right: self.settings_selected_section.scope_note(),
            }),
            Noun::Bills if matches!(self.bills_dialog, Some(bills::BillsDialog::Pay(_))) => {
                Some(PageStatus {
                    hints: pay_bill_dialog_hints(),
                    right: crate::msg::desktop_bills_status_plans(
                        i64::try_from(self.bill_plans.len()).unwrap_or(i64::MAX),
                    ),
                })
            }
            Noun::Bills if matches!(self.bills_dialog, Some(bills::BillsDialog::Skip(_))) => {
                Some(PageStatus {
                    hints: skip_bill_dialog_hints(),
                    right: crate::msg::desktop_bills_status_plans(
                        i64::try_from(self.bill_plans.len()).unwrap_or(i64::MAX),
                    ),
                })
            }
            Noun::Bills
                if matches!(
                    self.bills_dialog,
                    Some(bills::BillsDialog::Add(_) | bills::BillsDialog::Edit(..))
                ) =>
            {
                Some(PageStatus {
                    hints: payee_dialog_hints(),
                    right: crate::msg::desktop_bills_status_plans(
                        i64::try_from(self.bill_plans.len()).unwrap_or(i64::MAX),
                    ),
                })
            }
            Noun::Bills if self.bills_tab == bills::BillsTab::Schedule => Some(PageStatus {
                hints: bills_schedule_hints(),
                right: crate::msg::desktop_bills_status_period(
                    &if self.bills_all {
                        crate::msg::desktop_bills_period_all()
                    } else {
                        bills_view::period_label(self.bills_period)
                    },
                    &self.bills_schedule_rows().len().to_string(),
                    i64::try_from(self.bills_unfiltered_rows().len()).unwrap_or(i64::MAX),
                ),
            }),
            Noun::Bills if self.bills_tab == bills::BillsTab::Planner => Some(PageStatus {
                hints: bills_planner_hints(),
                right: crate::msg::desktop_bills_status_plans(
                    i64::try_from(self.bill_plans.len()).unwrap_or(i64::MAX),
                ),
            }),
            Noun::Budgets if self.budgets_tab == budgets::BudgetsTab::Progress => {
                Some(PageStatus {
                    hints: self.budgets_hints(),
                    right: crate::msg::desktop_budgets_status_period(
                        &budgets_view::period_label(self.budgets_period),
                        budgets_figures.as_ref().map_or(0, |(_, figures)| {
                            i64::try_from(figures.rows.len()).unwrap_or(i64::MAX)
                        }),
                    ),
                })
            }
            Noun::Budgets if self.budgets_tab == budgets::BudgetsTab::Plan => Some(PageStatus {
                hints: self.budgets_hints(),
                right: budgets_plan.as_ref().map_or_else(String::new, |plan| {
                    crate::msg::desktop_budgets_status_plan(
                        &budgets_view::plan::range_label(plan),
                        i64::try_from(plan.row_count()).unwrap_or(i64::MAX),
                    )
                }),
            }),
            Noun::Budgets => Some(PageStatus {
                hints: self.budgets_hints(),
                right: budgets_history
                    .as_ref()
                    .map_or_else(String::new, |history| {
                        crate::msg::desktop_budgets_status_history(
                            i64::try_from(history.leaves_ever_budgeted).unwrap_or(i64::MAX),
                            &history.leaves_shown.to_string(),
                        )
                    }),
            }),
            Noun::Transactions if self.import.is_some() => {
                let pending = self
                    .import
                    .as_ref()
                    .map_or(0, |state| import::summary(&state.rows).needs_review);
                Some(PageStatus {
                    hints: import_hints(),
                    right: if pending == 0 {
                        crate::msg::desktop_import_status_ready()
                    } else {
                        crate::msg::desktop_import_status_pending(
                            i64::try_from(pending).unwrap_or(i64::MAX),
                        )
                    },
                })
            }
            Noun::Transactions => Some(PageStatus {
                hints: if self.nav.mode() == InputMode::Filter {
                    filter_hints()
                } else {
                    transactions_hints()
                },
                right: format::status_legend(self.settings_status_glyphs),
            }),
            _ => None,
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color::background(cx))
            .text_color(color::foreground(cx))
            .font_family(type_scale::FONT_FAMILY)
            .text_size(type_scale::BODY)
            .track_focus(&self.focus_handle)
            .group(documents_view::drop_overlay::GROUP)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                this.drop_documents(paths.paths());
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                let chosen = settings_view::colour_theme::chosen_index(cx);
                if this.handle_settings_form_key(&event.keystroke, chosen)
                    || this.handle_settings_focus_key(&event.keystroke)
                    || this.handle_colour_theme_grid_key(&event.keystroke)
                    || this.handle_bills_tab_key(&event.keystroke)
                    || this.handle_budgets_tab_key(&event.keystroke)
                    || this.handle_key_down(event)
                {
                    cx.notify();
                }
                if std::mem::take(&mut this.pending_budgets_export) {
                    this.export_budgets_history(cx);
                }
                if let Some(change) = this.pending_colour_change.take() {
                    change.apply(cx);
                }
                this.run_pending_file_action(cx);
            }))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .opacity(content_opacity)
                    .child(TopBar::new(on_rail_toggle, self.nav.noun()).context(
                        if self.nav.noun() == Noun::Settings {
                            Some(self.settings_selected_section.label())
                        } else {
                            self.import
                                .as_ref()
                                .map(|_| crate::msg::desktop_import_context(import::STATEMENT_FILE))
                        },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.0))
                            .flex()
                            .child(
                                PrimaryRail::new(
                                    self.nav.primary_highlight(),
                                    focus == FocusZone::PrimaryRail,
                                    self.nav.primary_rail(),
                                    self.collapsed_rail_tooltip,
                                    on_row_hover,
                                    on_row_click,
                                )
                                .bill_attention(
                                    bills::attention_entries(
                                        &self.bill_plans,
                                        &self.bill_entries,
                                        self.today,
                                    )
                                    .len(),
                                )
                                .budget_over(
                                    default_budget_figures
                                        .as_ref()
                                        .map_or(0, |figures| figures.over_count),
                                )
                                .documents_inbox(documents::inbox_count(&self.documents)),
                            )
                            .when(
                                self.nav.noun().has_context_entities() && self.nav.ledger_open(),
                                |this| {
                                    this.child(ContextRail::new(
                                        self.nav.noun(),
                                        self.nav.context(),
                                        focus == FocusZone::ContextRail,
                                    ))
                                },
                            )
                            .child(render_view(
                                self.nav.noun(),
                                self.nav.ledger_open(),
                                focus == FocusZone::View,
                                &self.view_scroll_handle,
                                on_empty_state_command_click,
                                PageProps {
                                    dashboard,
                                    accounts: accounts_page,
                                    settings_categories: settings_categories_page,
                                    settings_tags: settings_tags_page,
                                    settings_payees: settings_payees_page,
                                    bills: bills_page,
                                    budgets: budgets_page,
                                    import: import_page,
                                    transactions: transactions_page,
                                    documents: documents_page,
                                },
                                SettingsPanelProps {
                                    selected: self.settings_selected_section,
                                    focus: self.settings_focus,
                                    on_index_click: on_settings_index_click,
                                    date_style: self.settings_date_style,
                                    row_density: self.settings_row_density,
                                    status_glyphs: self.settings_status_glyphs,
                                    start_sidebar_minimised: self.settings_start_sidebar_minimised,
                                    on_start_sidebar_minimised_click,
                                    on_date_style_click,
                                    on_row_density_click,
                                    on_status_glyphs_click,
                                    toasts_on: self.toasts.display().toasts_on,
                                    on_toasts_click,
                                    colour_theme_focus: self.colour_theme_focus,
                                    display_field: self.settings_display_field,
                                    on_colour_theme_click,
                                    units: &self.settings_units,
                                    on_unit_edit_click,
                                    on_unit_delete_click,
                                    on_add_unit_click,
                                    price_sources: &self.settings_price_sources,
                                    on_price_source_test_click,
                                    on_price_source_edit_click,
                                    on_price_source_delete_click,
                                    on_add_price_source_click,
                                    institutions: &self.settings_institutions,
                                    on_institution_edit_click,
                                    on_institution_delete_click,
                                    on_add_institution_click,
                                    on_sync_now_click,
                                    on_backup_now_click,
                                    on_export_ledger_click,
                                    tracing_level: self.settings_tracing_level,
                                    log_lines: &self.settings_log_lines,
                                    on_tracing_level_click,
                                    on_clear_logs_click,
                                },
                                cx,
                            )),
                    ),
            )
            .child(
                StatusLine::new(
                    self.nav.mode(),
                    self.status_message.clone(),
                    self.command_echo(),
                )
                .toast_echo(
                    self.toasts
                        .echo()
                        .filter(|_| !self.toast_history_open)
                        .map(|toast| (toast.kind(), toast.text().to_string())),
                )
                .page(page_status)
                .mode_label(
                    (self.import.is_some() && self.nav.mode() == InputMode::Normal)
                        .then(crate::msg::desktop_mode_import),
                )
                .on_hint(on_hint),
            )
            .children(self.palette.as_ref().map(|palette| palette.render(cx)))
            .children(self.file_explorer.as_ref().map(|explorer| {
                explorer.render(
                    on_explorer_entry_click,
                    on_explorer_breadcrumb_click,
                    on_explorer_filter_toggle,
                    on_explorer_cancel,
                    on_explorer_open,
                    cx,
                )
            }))
            .children(filter_popover)
            .children((self.nav.mode() == InputMode::Help).then(|| {
                let mut sheet = self.settings_cheat_sheet();
                sheet.extend(self.documents_cheat_sheet());
                help_view::render(on_help_close, sheet, cx)
            }))
            .children(self.toast_history_open.then(|| {
                toast_history_view::render(self.toasts.history(), on_toast_history_close, cx)
            }))
            .children(self.render_documents_dialog(&entity, cx))
            .child(documents_view::drop_overlay::render(cx))
            .children(self.accounts_dialog.as_ref().map(|dialog| match dialog {
                AccountsDialog::Add(form) => accounts_view::add_dialog::render(
                    form,
                    &account_options,
                    on_accounts_dialog_field_click,
                    on_accounts_dialog_option_click,
                    on_accounts_dialog_cancel,
                    on_accounts_dialog_confirm,
                    cx,
                ),
                AccountsDialog::Edit(id, form) => {
                    match self.accounts.iter().find(|account| account.id == *id) {
                        Some(account) => accounts_view::edit_dialog::render(
                            form,
                            account,
                            &account_options,
                            on_accounts_dialog_field_click,
                            on_accounts_dialog_option_click,
                            on_accounts_dialog_cancel,
                            on_accounts_dialog_confirm,
                            cx,
                        ),
                        // Defensive only: the id comes from a live row when the dialog opens.
                        None => div().into_any_element(),
                    }
                }
                AccountsDialog::Delete(id, form) => {
                    match self.accounts.iter().find(|account| account.id == *id) {
                        Some(account) => accounts_view::delete_dialog::render(
                            account,
                            form,
                            on_accounts_dialog_cancel,
                            on_accounts_dialog_confirm,
                            cx,
                        ),
                        // Defensive only: the id comes from a live row when the dialog opens.
                        None => div().into_any_element(),
                    }
                }
            }))
            .children(bills_dialog_element)
            .children(self.render_budgets_detail(&entity, cx))
            .children(self.render_budgets_limit_dialog(&entity, cx))
            .children(self.render_budgets_fill_dialog(&entity, cx))
            .children(self.render_budgets_switcher(&entity, cx))
            .children(self.render_budgets_form_dialog(&entity, cx))
            .children(self.render_budgets_manage_dialog(&entity, cx))
            .children(match self.payees_dialog.as_ref() {
                Some(payees::PayeesDialog::Add(form)) => Some(payees_view::add_dialog::render(
                    payees_view::add_dialog::PayeeDialogMode::Add,
                    form,
                    &self.payee_dialog_options(),
                    form.name_error(&self.payees, None),
                    form.is_valid(&self.payees, None),
                    payees_dialog_handlers,
                    cx,
                )),
                Some(payees::PayeesDialog::Edit(id, form)) => {
                    payees::get(&self.payees, *id).map(|payee| {
                        payees_view::add_dialog::render(
                            payees_view::add_dialog::PayeeDialogMode::Edit {
                                name: &payee.name,
                                splits: payees::usage(
                                    &self.transactions,
                                    &self.accounts,
                                    None,
                                    *id,
                                )
                                .splits,
                            },
                            form,
                            &self.payee_dialog_options(),
                            form.name_error(&self.payees, Some(*id)),
                            form.is_valid(&self.payees, Some(*id)),
                            payees_dialog_handlers,
                            cx,
                        )
                    })
                }
                Some(payees::PayeesDialog::Delete(id, form)) => {
                    self.delete_payee_target().map(|(payee, action)| {
                        payees_view::delete_dialog::render(
                            payees_view::delete_dialog::DeletePayeeProps {
                                payee,
                                action,
                                form,
                                splits: payees::usage(
                                    &self.transactions,
                                    &self.accounts,
                                    None,
                                    *id,
                                )
                                .splits,
                                on_cancel: plain_payees(Shell::handle_payees_dialog_cancel),
                                on_confirm: plain_payees(Shell::handle_delete_payee_confirm),
                            },
                            cx,
                        )
                    })
                }
                None => None,
            })
            .children(match self.tags_dialog.as_ref() {
                Some(tags::TagsDialog::Add(form)) => Some(tags_view::add_dialog::render(
                    form,
                    form.name_error(&self.tags, None),
                    form.is_valid(&self.tags, None),
                    tags_dialog_handlers,
                    cx,
                )),
                Some(tags::TagsDialog::Edit(id, form)) => tags::get(&self.tags, *id).map(|tag| {
                    tags_view::edit_dialog::render(
                        tags_view::edit_dialog::EditTagProps {
                            original_name: &tag.name,
                            form,
                            name_error: form.name_error(&self.tags, Some(*id)),
                            valid: form.is_valid(&self.tags, Some(*id)),
                            transactions: tags::transaction_count(&self.transactions, *id),
                            on_toggle_active: tag_plain(Shell::handle_tags_dialog_toggle_active),
                        },
                        tags_dialog_handlers,
                        cx,
                    )
                }),
                Some(tags::TagsDialog::Remove(id, form)) => tags::get(&self.tags, *id).map(|tag| {
                    tags_view::remove_dialog::render(
                        tags_view::remove_dialog::RemoveTagProps {
                            tag,
                            form,
                            transactions: tags::transaction_count(&self.transactions, *id),
                            on_cancel: tag_plain(Shell::handle_tags_dialog_cancel),
                            on_confirm: tag_plain(Shell::handle_tags_dialog_confirm),
                        },
                        cx,
                    )
                }),
                Some(tags::TagsDialog::Merge(form)) => {
                    let options = self.merge_tag_options();
                    let source = form.source_id(&options);
                    let entity = entity.clone();
                    let on_field_click: tags_view::merge_dialog::OnFieldClick = {
                        let entity = entity.clone();
                        Rc::new(move |field, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.handle_merge_tags_field_click(field, cx);
                            });
                        })
                    };
                    let on_option_click: tags_view::merge_dialog::OnOptionClick =
                        Rc::new(move |field, index, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                shell.handle_merge_tags_option_click(field, index, cx);
                            });
                        });
                    Some(tags_view::merge_dialog::render(
                        tags_view::merge_dialog::MergeTagsProps {
                            form,
                            options: &options,
                            source: source.and_then(|id| tags::get(&self.tags, id)),
                            target: form
                                .target_id(&options)
                                .and_then(|id| tags::get(&self.tags, id)),
                            transactions: source
                                .map_or(0, |id| tags::transaction_count(&self.transactions, id)),
                            on_field_click,
                            on_option_click,
                            on_cancel: tag_plain(Shell::handle_tags_dialog_cancel),
                            on_confirm: tag_plain(Shell::handle_tags_dialog_confirm),
                        },
                        cx,
                    ))
                }
                None => None,
            })
            .children(self.categories_dialog.as_ref().map(|dialog| match dialog {
                categories::CategoriesDialog::Add { form, .. } => {
                    let parent_options: Vec<_> = self
                        .categories
                        .iter()
                        .map(|c| {
                            let depth = categories::depth(&self.categories, c.id);
                            let is_available = depth < 2; // Can't add children to depth-2 categories
                            categories_view::add_dialog::ParentOption {
                                id: Some(c.id),
                                label: categories::path(&self.categories, c.id)
                                    .unwrap_or_else(|| c.name.clone()),
                                is_available,
                            }
                        })
                        .collect();

                    categories_view::add_dialog::render(
                        form,
                        &parent_options,
                        &self.categories,
                        categories_view::DialogHandlers {
                            on_field_click: on_categories_dialog_field_click,
                            on_parent_change: on_categories_dialog_parent_change,
                            on_type_change: on_categories_dialog_type_change,
                            on_cancel: on_categories_dialog_cancel,
                            on_confirm: on_categories_dialog_confirm,
                        },
                        cx,
                    )
                }
                categories::CategoriesDialog::Edit(category_id, form) => {
                    let category = self.categories.iter().find(|c| c.id == *category_id);
                    let descendants = category
                        .map(|_| categories::descendants_inclusive(&self.categories, *category_id))
                        .unwrap_or_default();

                    let parent_options: Vec<_> = self
                        .categories
                        .iter()
                        .filter(|c| {
                            // Exclude the category itself
                            if c.id == *category_id {
                                return false;
                            }
                            // Exclude descendants (to prevent cycles)
                            if descendants.contains(&c.id) {
                                return false;
                            }
                            // Check depth: can't be at depth 2 or deeper
                            let depth = categories::depth(&self.categories, c.id);
                            depth < 2
                        })
                        .map(|c| categories_view::edit_dialog::ParentOption {
                            id: Some(c.id),
                            label: categories::path(&self.categories, c.id)
                                .unwrap_or_else(|| c.name.clone()),
                            is_available: true,
                        })
                        .collect();

                    // Count splits in this category (and descendants if parent)
                    let split_count =
                        categories::descendants_inclusive(&self.categories, *category_id)
                            .iter()
                            .flat_map(|cat_id| {
                                self.transactions.iter().flat_map(move |t| {
                                    t.splits.iter().filter(move |s| s.category_id == *cat_id)
                                })
                            })
                            .count();

                    let is_parent = !categories::is_leaf(&self.categories, *category_id);

                    categories_view::edit_dialog::render(
                        *category_id,
                        form,
                        &parent_options,
                        &self.categories,
                        split_count,
                        is_parent,
                        categories_view::DialogHandlers {
                            on_field_click: on_categories_dialog_field_click,
                            on_parent_change: on_categories_dialog_parent_change,
                            on_type_change: on_categories_dialog_type_change,
                            on_cancel: on_categories_dialog_cancel,
                            on_confirm: on_categories_dialog_confirm,
                        },
                        cx,
                    )
                }
                categories::CategoriesDialog::Delete(category_id, form) => {
                    let category = self
                        .categories
                        .iter()
                        .find(|c| c.id == *category_id)
                        .cloned();

                    if let Some(category) = category {
                        // Count splits in this category (and descendants if parent)
                        let split_count =
                            categories::descendants_inclusive(&self.categories, *category_id)
                                .iter()
                                .flat_map(|cat_id| {
                                    self.transactions.iter().flat_map(move |t| {
                                        t.splits.iter().filter(move |s| s.category_id == *cat_id)
                                    })
                                })
                                .count();

                        // Count budgets attached to this category
                        let budget_count = self.budgets.limit_count(*category_id);

                        categories_view::delete_dialog::render(
                            &category,
                            form,
                            split_count,
                            budget_count,
                            on_categories_dialog_cancel.clone(),
                            on_categories_dialog_confirm.clone(),
                            cx,
                        )
                    } else {
                        div().into_any_element()
                    }
                }
            }))
            .children(self.settings_dialog.as_ref().map(|dialog| match dialog {
                SettingsDialog::AddUnit(form) => settings_view::add_unit_dialog::render(
                    form,
                    on_unit_dialog_field_click.clone(),
                    on_unit_dialog_kind_click.clone(),
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                    cx,
                ),
                SettingsDialog::EditUnit(_, form) => settings_view::edit_unit_dialog::render(
                    form,
                    on_unit_dialog_field_click,
                    on_unit_dialog_kind_click,
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                    cx,
                ),
                SettingsDialog::DeleteUnit(index, form) => {
                    match self.settings_units.get(*index) {
                        Some(row) => settings_view::delete_unit_dialog::render(
                            row,
                            form,
                            on_settings_dialog_cancel.clone(),
                            on_settings_dialog_confirm.clone(),
                            cx,
                        ),
                        // Defensive only: `index` should always be in bounds (it's only ever
                        // set from a real row's own click handler) -- an empty overlay is a
                        // safer failure than panicking mid-render.
                        None => div().into_any_element(),
                    }
                }
                SettingsDialog::AddInstitution(form) => {
                    settings_view::add_institution_dialog::render(
                        form,
                        &self.settings_units,
                        on_add_institution_account_type_click,
                        on_add_institution_unit_click,
                        on_settings_dialog_cancel,
                        on_settings_dialog_confirm,
                        cx,
                    )
                }
            }))
            // Last: above every dialog, the palette and their scrim, never over the status line.
            .children(toast_layer)
    }
}

/// The per-page props `render_view` needs for the pages that own their whole pane: Accounts, and
/// Transactions (built only while it is the active page, hence the `Option`).
struct PageProps<'a> {
    dashboard: Dashboard,
    accounts: accounts_view::AccountsPageProps<'a>,
    /// Settings' own Categories page (2i), mounted only while Settings shows it.
    settings_categories: settings_view::categories::CategoriesPageProps<'a>,
    /// Settings' own Tags page (2j), mounted only while Settings shows it.
    settings_tags: settings_view::tags::TagsPageProps<'a>,
    /// Settings' own Payees page, mounted only while Settings shows it.
    settings_payees: settings_view::payees::PayeesPageProps<'a>,
    bills: bills_view::BillsPageProps<'a>,
    /// `None` when the Budget shown has gone.
    budgets: Option<budgets_view::BudgetsPageProps<'a>>,
    /// `Some` while 6e shows in place of the Transactions page.
    import: Option<import_view::ImportPageProps<'a>>,
    transactions: Option<transactions_view::TransactionsPageProps>,
    /// `Some` while Documents is the noun on show.
    documents: Option<documents_view::DocumentsPageProps>,
}

/// Bundles `render_view`'s Settings-only parameters (keeps the function under Clippy's
/// `too_many_arguments` threshold) -- ignored entirely for every noun besides `Settings`. Covers
/// both the index rail's own state and the body's per-section interactive state
/// (`view::settings::SettingsBodyProps`); `render_view` splits it back apart when it builds
/// each half's own component.
struct SettingsPanelProps<'a> {
    selected: SettingsSection,
    focus: SettingsFocus,
    on_index_click: settings_index::OnEntryClick,
    date_style: Option<DateStyle>,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    start_sidebar_minimised: bool,
    on_start_sidebar_minimised_click: settings_view::display::OnPlainClick,
    on_date_style_click: settings_view::display::OnDateStyleClick,
    on_row_density_click: settings_view::display::OnRowDensityClick,
    on_status_glyphs_click: settings_view::display::OnStatusGlyphsClick,
    toasts_on: bool,
    on_toasts_click: settings_view::display::OnToastsClick,
    colour_theme_focus: Option<usize>,
    display_field: Option<usize>,
    on_colour_theme_click: settings_view::colour_theme::OnColourThemeClick,
    units: &'a [UnitRow],
    on_unit_edit_click: settings_view::units::OnRowIndexClick,
    on_unit_delete_click: settings_view::units::OnRowIndexClick,
    on_add_unit_click: settings_view::units::OnAddClick,
    price_sources: &'a [PriceSourceRow],
    on_price_source_test_click: settings_view::units::OnRowIndexClick,
    on_price_source_edit_click: settings_view::units::OnRowIndexClick,
    on_price_source_delete_click: settings_view::units::OnRowIndexClick,
    on_add_price_source_click: settings_view::units::OnAddClick,
    institutions: &'a [InstitutionRow],
    on_institution_edit_click: settings_view::institutions::OnRowIndexClick,
    on_institution_delete_click: settings_view::institutions::OnRowIndexClick,
    on_add_institution_click: settings_view::institutions::OnAddClick,
    on_sync_now_click: settings_view::sync_server::OnSyncNowClick,
    on_backup_now_click: settings_view::data_backup::OnBackupNowClick,
    on_export_ledger_click: settings_view::data_backup::OnExportLedgerClick,
    tracing_level: TracingLevel,
    log_lines: &'a [&'static str],
    on_tracing_level_click: settings_view::tracing::OnLevelClick,
    on_clear_logs_click: settings_view::tracing::OnClearLogsClick,
}

/// The active noun's own view interior. Only `Dashboard` and `Settings` are real; every other
/// noun is a placeholder until its own view lands (issue #153). `Dashboard` itself further
/// branches on `ledger_open` (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" empty
/// state) -- implementation note 2's "only the main pane branches on `ledgerOpen`" scopes that
/// to the one real view; the still-placeholder nouns say "not yet built" either way.
///
/// `Settings` (issue #173) is handled separately, before the generic match below: it renders
/// its own two-column [index rail][scrollable body] layout filling the whole slot, rather than
/// the single scrollable `#view` div every other noun gets -- the settings body owns
/// `scroll_handle` directly (see `view::settings::render`), so wrapping the whole thing in a
/// second scrollable container here would fight it for the same scroll state. Every other noun
/// is scrollable and focus-bordered regardless of which is active, since both are properties of
/// the `View` zone itself, not of any one noun's content.
#[expect(
    clippy::too_many_arguments,
    reason = "the view props plus the App the Colour Theme is read from; the remaining sweeps may fold them into one struct"
)]
fn render_view(
    noun: Noun,
    ledger_open: bool,
    focused: bool,
    scroll_handle: &ScrollHandle,
    on_empty_state_command_click: OnEmptyStateCommandClick,
    pages: PageProps<'_>,
    settings: SettingsPanelProps<'_>,
    cx: &gpui::App,
) -> gpui::AnyElement {
    if noun == Noun::Documents
        && let Some(documents) = pages.documents
    {
        return documents_view::render(focused, documents, cx);
    }
    if noun == Noun::Bills {
        return bills_view::render(focused, scroll_handle, pages.bills, cx);
    }
    if noun == Noun::Budgets
        && let Some(budgets) = pages.budgets
    {
        return budgets_view::render(focused, scroll_handle, budgets, cx);
    }
    if noun == Noun::Transactions
        && let Some(import) = pages.import
    {
        return import_view::render(focused, scroll_handle, import, cx);
    }
    if noun == Noun::Transactions
        && let Some(transactions) = pages.transactions
    {
        return transactions_view::render(focused, transactions, cx);
    }

    if noun == Noun::Settings {
        return div()
            .id("settings")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .child(SettingsIndexRail::new(
                settings.selected,
                focused && settings.focus == SettingsFocus::Index,
                settings.on_index_click,
            ))
            .child(settings_view::render(
                focused && settings.focus == SettingsFocus::Page,
                scroll_handle,
                SettingsBodyProps {
                    selected: settings.selected,
                    page_focused: focused && settings.focus == SettingsFocus::Page,
                    accounts: settings_view::accounts::AccountsPageProps {
                        accounts: pages.accounts.accounts,
                        units: pages.accounts.units,
                        selected: pages.accounts.selected,
                        on_add_click: pages.accounts.on_add_click,
                        on_row_click: pages.accounts.on_row_click,
                        on_edit_click: pages.accounts.on_edit_click,
                        on_delete_click: pages.accounts.on_delete_click,
                    },
                    categories: pages.settings_categories,
                    tags: pages.settings_tags,
                    payees: pages.settings_payees,
                    date_style: settings.date_style,
                    row_density: settings.row_density,
                    status_glyphs: settings.status_glyphs,
                    start_sidebar_minimised: settings.start_sidebar_minimised,
                    on_start_sidebar_minimised_click: settings.on_start_sidebar_minimised_click,
                    on_date_style_click: settings.on_date_style_click,
                    on_row_density_click: settings.on_row_density_click,
                    on_status_glyphs_click: settings.on_status_glyphs_click,
                    toasts_on: settings.toasts_on,
                    on_toasts_click: settings.on_toasts_click,
                    colour_theme_focus: settings.colour_theme_focus,
                    display_field: settings.display_field,
                    on_colour_theme_click: settings.on_colour_theme_click,
                    units: settings.units,
                    on_unit_edit_click: settings.on_unit_edit_click,
                    on_unit_delete_click: settings.on_unit_delete_click,
                    on_add_unit_click: settings.on_add_unit_click,
                    price_sources: settings.price_sources,
                    on_price_source_test_click: settings.on_price_source_test_click,
                    on_price_source_edit_click: settings.on_price_source_edit_click,
                    on_price_source_delete_click: settings.on_price_source_delete_click,
                    on_add_price_source_click: settings.on_add_price_source_click,
                    institutions: settings.institutions,
                    on_institution_edit_click: settings.on_institution_edit_click,
                    on_institution_delete_click: settings.on_institution_delete_click,
                    on_add_institution_click: settings.on_add_institution_click,
                    on_sync_now_click: settings.on_sync_now_click,
                    on_backup_now_click: settings.on_backup_now_click,
                    on_export_ledger_click: settings.on_export_ledger_click,
                    tracing_level: settings.tracing_level,
                    log_lines: settings.log_lines,
                    on_tracing_level_click: settings.on_tracing_level_click,
                    on_clear_logs_click: settings.on_clear_logs_click,
                },
                cx,
            ))
            .into_any_element();
    }

    let content = match noun {
        Noun::Dashboard if ledger_open => pages.dashboard.into_any_element(),
        Noun::Dashboard => empty_state(on_empty_state_command_click, cx),
        Noun::Settings => unreachable!("handled above"),
        other => placeholder_view(other, cx),
    };

    div()
        .id("view")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .child(content)
        .into_any_element()
}

/// The "not yet built" view of a noun whose surface is a later map's work (#420): its name and
/// one line saying so. The rail row and `g` binding are live; the surface is not.
fn placeholder_view(noun: Noun, cx: &gpui::App) -> gpui::AnyElement {
    div()
        .p(px(24.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(18.0))
                .child(crate::msg::desktop_placeholder_title(&noun.label())),
        )
        .child(
            div()
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_placeholder_body()),
        )
        .into_any_element()
}

/// The "1a" cold-start empty state: "No ledger open" centered in the main pane, `:open`/`:new`
/// named in the body copy (`docs/ux/desktop/Shell & Navigation/README.md`'s "Main pane"
/// bullet). `gpui` 0.2's `Styled` trait has no letter-spacing hook, so the title's `-.01em`
/// tracking from the spec has no equivalent here -- a real, not merely unverified, gap.
///
/// Both command names are real click targets (issue #167), not just copy: clicking one lands
/// in exactly the state running it from the palette would (this shell's own repeated invariant
/// -- rail click, `g`-jump and the palette already all call `NavState::set_noun` identically).
fn empty_state(on_command_click: OnEmptyStateCommandClick, cx: &gpui::App) -> gpui::AnyElement {
    let command = |name: &'static str, on_command_click: OnEmptyStateCommandClick, text: String| {
        div()
            .id(SharedString::from(format!("empty-state-{name}")))
            .cursor_pointer()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::foreground(cx))
            .on_click(move |_event, window, cx| on_command_click(name, window, cx))
            .child(text)
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(14.0))
        .p(px(24.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(26.0))
                .line_height(gpui::relative(1.1))
                .child(crate::msg::desktop_empty_state_title()),
        )
        .child(
            div()
                .max_w(px(380.0))
                .flex()
                .flex_wrap()
                .justify_center()
                .text_align(gpui::TextAlign::Center)
                .text_size(px(13.0))
                .text_color(color::muted(cx))
                .children(
                    crate::msg::desktop_empty_state_hint(":open", ":new")
                        .into_iter()
                        .map(|segment| match segment.tag.as_deref() {
                            Some("open") => command("open", on_command_click.clone(), segment.text)
                                .into_any_element(),
                            Some("new") => command("new", on_command_click.clone(), segment.text)
                                .into_any_element(),
                            _ => div().child(segment.text).into_any_element(),
                        }),
                ),
        )
        .into_any_element()
}

/// The one character an unmodified keystroke types into a text field, or `None` for a modified
/// key (a chord this field gives no meaning to) or anything that isn't a single character.
fn typed_char(keystroke: &Keystroke) -> Option<char> {
    let modifiers = &keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return None;
    }
    let mut chars = keystroke.key_char.as_deref()?.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => Some(ch),
        _ => None,
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
