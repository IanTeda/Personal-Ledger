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
//! `b`-key/click toggle, and its hover tooltip (issue #152). `?`'s help overlay is a separate
//! build ticket still to land on the
//! [Desktop Shell & Navigation](https://github.com/IanTeda/Personal-Ledger/issues/144) map. A
//! `View` trait mirroring `bin-tui`'s is still deliberately deferred (see ADR-0016):
//! `Dashboard` is the only real view, and `render_view` below is a plain match rather than a
//! trait object because there's still only one concrete implementor to dispatch to.
//!
//! `Shell` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    Context, FocusHandle, Focusable, KeyDownEvent, Keystroke, ScrollHandle, SharedString, Timer,
    Window, div, point, prelude::*, px,
};

use crate::{
    accounts::{self, Account, AccountsDialog},
    command::{self, Command, CommandEffect},
    explorer::{self, ExplorerMode, FileExplorer},
    key_router::{KeyOutcome, Movement, route_key},
    nav::{FocusZone, InputMode, NavState, Noun},
    palette::Palette,
    rail::{
        self,
        context::ContextRail,
        primary::PrimaryRail,
        settings_index::{self, SettingsIndexRail},
    },
    settings::{
        self, AccountType, AddInstitutionForm, AddUnitField, DateFormat, DecimalSeparator,
        DeleteUnitForm, InstitutionRow, PriceSourceRow, RowDensity, SettingsDialog,
        SettingsSection, StatusGlyphs, TracingLevel, UnitForm, UnitKind, UnitRow,
    },
    statusline::{PageStatus, StatusLine},
    theme::{color, type_scale},
    topbar::{self, TopBar},
    view::{
        accounts as accounts_view,
        dashboard::Dashboard,
        settings::{self as settings_view, SettingsBodyProps},
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

/// The Accounts page's status-line legend (`docs/ux/desktop/Accounts/README.md`'s 3a).
const ACCOUNTS_HINTS: &[(&str, &str)] = &[
    ("j/k", "row"),
    ("enter", "open ledger"),
    ("e", "edit"),
    ("d", "delete"),
    ("n", "new"),
];

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
    /// The Settings index rail's own `/ filter` query (issue #173) -- live while
    /// `NavState::mode` is `InputMode::Search` and the active noun is `Settings` (see
    /// [`Self::handle_search_key`]). Cleared whenever a fresh noun is entered
    /// ([`Self::reset_view_scroll`]) or `Esc` leaves the mode, so a stale filter never survives
    /// past the session that typed it.
    settings_filter: String,
    /// The settings index rail's own active/highlighted entry -- updated only by an explicit
    /// row click ([`Self::handle_settings_index_click`]), not by scroll position (no scrollspy
    /// yet). Reset to `SettingsSection::default()` alongside the view scroll whenever a fresh
    /// noun is entered, so re-opening Settings always starts on General again.
    settings_selected_section: SettingsSection,
    /// The Display section's own "Date format" segmented control (issue #179) -- a stored
    /// preference, not reset on noun change (same reasoning as [`Self::settings_tracing_level`]).
    settings_date_format: DateFormat,
    /// The same section's "Decimal & thousands separator" segmented control.
    settings_decimal_separator: DecimalSeparator,
    /// The same section's "Row density" segmented control -- also drives the PREVIEW table's own
    /// row padding (`view::settings::display`'s own doc), unlike a purely-cosmetic preference.
    settings_row_density: RowDensity,
    /// The same section's "Status glyphs" radio group.
    settings_status_glyphs: StatusGlyphs,
    /// The Units section's own table rows (issue #177), seeded from `settings::default_units()`.
    /// A real, mutable `Vec` so the Add/Edit/Delete unit dialogs (issues #184-#186) can
    /// push/update/remove rows once they land -- unlike
    /// [`Self::settings_filter`]/[`Self::settings_selected_section`], not reset by
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
}

impl Shell {
    pub fn new(nav: NavState, focus_handle: FocusHandle) -> Self {
        Self {
            nav,
            focus_handle,
            view_scroll_handle: ScrollHandle::new(),
            pending_g: None,
            status_message: None,
            palette: None,
            command_history: Vec::new(),
            collapsed_rail_tooltip: None,
            hover_generation: 0,
            file_explorer: None,
            settings_filter: String::new(),
            settings_selected_section: SettingsSection::default(),
            settings_date_format: DateFormat::default(),
            settings_decimal_separator: DecimalSeparator::default(),
            settings_row_density: RowDensity::default(),
            settings_status_glyphs: StatusGlyphs::default(),
            settings_units: settings::default_units(),
            settings_price_sources: settings::default_price_sources(),
            settings_dialog: None,
            settings_institutions: settings::default_institutions(),
            settings_tracing_level: TracingLevel::default(),
            settings_log_lines: settings::DEFAULT_LOG_LINES.to_vec(),
            accounts: accounts::default_accounts(),
            accounts_selected: 0,
            accounts_dialog: None,
        }
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
    fn command_echo(&self) -> Option<(String, &'static str)> {
        if let Some(palette) = self.palette.as_ref() {
            Some((palette.input().to_string(), "esc close command window"))
        } else if let Some(explorer) = self.file_explorer.as_ref() {
            let name = match explorer.mode() {
                ExplorerMode::Open => "open",
                ExplorerMode::New => "new",
            };
            Some((name.to_string(), "esc close file explorer"))
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

        let outcome = route_key(self.nav.mode(), pending_g_active, key, ctrl, shift);

        // `Esc`'s three shapes short-circuit before the hint-strip-clearing precedent below --
        // mirrors the original tier order exactly, including the quirk that a bare `Esc` with
        // nothing to do (`EscapeNoOp`) does *not* clear a stale status message.
        match outcome {
            KeyOutcome::ClearPendingG => return true,
            KeyOutcome::ClosePopupsAndExitMode => {
                self.palette = None;
                self.file_explorer = None;
                // The README's own Dialog lifecycle table: "`esc` closes any dialog without
                // saving" -- discards whatever was typed, same as Cancel.
                self.settings_dialog = None;
                self.accounts_dialog = None;
                // README's "Interactions" > "Navigation": `esc` clears the settings index
                // rail's own filter, the same as it closes the palette/file explorer above.
                self.settings_filter.clear();
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
                self.nav.enter_mode(InputMode::Command);
                self.palette = Some(Palette::with_history(self.command_history.clone()));
                true
            }
            KeyOutcome::EnterSearch => {
                self.nav.enter_mode(InputMode::Search);
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
            KeyOutcome::NoOp => self.handle_accounts_key(keystroke) || had_status_message,
            KeyOutcome::ClearPendingG
            | KeyOutcome::ClosePopupsAndExitMode
            | KeyOutcome::EscapeNoOp => {
                unreachable!("handled above")
            }
        }
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
    /// fresh noun starts scrolled to the top. Also resets the Settings index rail's own
    /// highlight and filter (harmless for every other noun, and means re-entering Settings
    /// always starts back on General with a clean filter, matching the fresh scroll position).
    fn reset_view_scroll(&mut self) {
        self.view_scroll_handle.set_offset(gpui::Point::default());
        self.settings_selected_section = SettingsSection::default();
        self.settings_filter.clear();
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

    fn apply_view_movement(&mut self, movement: Movement) {
        if self.nav.noun() == Noun::Accounts {
            self.apply_accounts_movement(movement);
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
                self.palette = None;
                match command {
                    Some(command) => self.run_command(command),
                    // No result to run (an empty registry match) -- there's nothing left for
                    // `run_command` to do, so leave Command mode directly instead.
                    None => self.nav.exit_mode(),
                }
                true
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                match keystroke.key_char.as_deref() {
                    Some(text) if text.chars().count() == 1 => {
                        palette.push_char(text.chars().next().expect("checked above"));
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// Routes a keystroke while `InputMode::Search` is active (tier 2, mirroring
    /// [`Self::handle_palette_key`]'s shape): only meaningful on the Settings noun today, where
    /// it drives the settings index rail's own `/ filter`
    /// (`docs/ux/desktop/Settings/README.md`'s "Navigation" bullet). A no-op everywhere else --
    /// Search mode still has no other real input surface (`key_router::KeyOutcome::DelegateToSearch`'s
    /// own doc).
    fn handle_search_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.noun() != Noun::Settings {
            return false;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.settings_filter.pop();
                true
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                match keystroke.key_char.as_deref() {
                    Some(text) if text.chars().count() == 1 => {
                        self.settings_filter
                            .push(text.chars().next().expect("checked above"));
                        true
                    }
                    _ => false,
                }
            }
        }
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
                    _ => {
                        let modifiers = &keystroke.modifiers;
                        if modifiers.control
                            || modifiers.alt
                            || modifiers.platform
                            || modifiers.function
                        {
                            return false;
                        }
                        match keystroke.key_char.as_deref() {
                            Some(text) if text.chars().count() == 1 => {
                                form.push_char(text.chars().next().expect("checked above"));
                                true
                            }
                            _ => false,
                        }
                    }
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
                    _ => {
                        let modifiers = &keystroke.modifiers;
                        if modifiers.control
                            || modifiers.alt
                            || modifiers.platform
                            || modifiers.function
                        {
                            return false;
                        }
                        match keystroke.key_char.as_deref() {
                            Some(text) if text.chars().count() == 1 => {
                                form.push_char(text.chars().next().expect("checked above"));
                                true
                            }
                            _ => false,
                        }
                    }
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
                _ => {
                    let modifiers = &keystroke.modifiers;
                    if modifiers.control
                        || modifiers.alt
                        || modifiers.platform
                        || modifiers.function
                    {
                        return false;
                    }
                    match keystroke.key_char.as_deref() {
                        Some(text) if text.chars().count() == 1 => {
                            form.push_char(text.chars().next().expect("checked above"));
                            true
                        }
                        _ => false,
                    }
                }
            },
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
    /// scrolling it; `Enter` opens the account's ledger, which no screen exists for yet.
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
                self.flash_open_ledger_stub();
                selected
            }
        };
        self.scroll_selected_account_into_view();
    }

    /// Scrolls the selected account's group block into view (`ScrollHandle::scroll_to_item`
    /// addresses the page's direct children; see `view::accounts::GROUP_CHILD_OFFSET`).
    fn scroll_selected_account_into_view(&self) {
        if let Some(position) = self
            .selected_account_index()
            .and_then(|index| accounts::group_position(&self.accounts, index))
        {
            self.view_scroll_handle
                .scroll_to_item(accounts_view::GROUP_CHILD_OFFSET + position);
        }
    }

    /// The Accounts page's own `n`/`e`/`d` (only while it is the active noun and the view has
    /// focus, in `Normal` mode -- `route_key` hands back `NoOp` for these bare keys). Each goes
    /// through the same handler its button does.
    fn handle_accounts_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.noun() != Noun::Accounts || self.nav.focus() != FocusZone::View {
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
            "n" => self.flash_accounts_stub("add account"),
            "e" => {
                if selected_id.is_some() {
                    self.flash_accounts_stub("edit account");
                }
            }
            "d" => {
                if selected_id.is_some() {
                    self.flash_accounts_stub("delete account");
                }
            }
            _ => return false,
        }
        true
    }

    fn flash_accounts_stub(&mut self, what: &str) {
        self.status_message = Some(format!("{what} -- not yet built"));
    }

    fn flash_open_ledger_stub(&mut self) {
        self.status_message = Some("open ledger -- not yet built".to_string());
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
    fn handle_accounts_row_click(&mut self, id: u32, cx: &mut Context<Self>) {
        self.select_account(id);
        self.flash_open_ledger_stub();
        cx.notify();
    }

    /// The page's **+ Add account** button, sharing `n`'s stub until the Add dialog lands.
    fn handle_accounts_add_click(&mut self, cx: &mut Context<Self>) {
        self.flash_accounts_stub("add account");
        cx.notify();
    }

    fn handle_accounts_edit_click(&mut self, id: u32, cx: &mut Context<Self>) {
        self.select_account(id);
        self.flash_accounts_stub("edit account");
        cx.notify();
    }

    fn handle_accounts_delete_click(&mut self, id: u32, cx: &mut Context<Self>) {
        self.select_account(id);
        self.flash_accounts_stub("delete account");
        cx.notify();
    }

    /// The Units section's own "+ Add unit" button (issue #184, replacing the stub #177 left
    /// behind): opens the Add unit dialog rather than flashing a status message.
    fn handle_add_unit_click(&mut self, cx: &mut Context<Self>) {
        self.settings_dialog = Some(SettingsDialog::AddUnit(UnitForm::default()));
        self.nav.enter_mode(InputMode::Dialog);
        cx.notify();
    }

    /// The Units table's own row "edit" button (issue #185, replacing the stub #177 left
    /// behind): opens the Edit unit dialog pre-filled from the clicked row
    /// (`UnitForm::from_row`) rather than flashing a status message. A no-op if `index` is
    /// somehow out of bounds (defensive only -- every caller is a row's own click handler, so
    /// this should never actually happen).
    fn handle_unit_edit_click(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(row) = self.settings_units.get(index) else {
            return;
        };
        self.settings_dialog = Some(SettingsDialog::EditUnit(index, UnitForm::from_row(row)));
        self.nav.enter_mode(InputMode::Dialog);
        cx.notify();
    }

    /// Shared by the Add/Edit unit dialogs' own field-focus clicks (issues #184/#185) -- which
    /// field a click targets doesn't depend on which dialog variant is open.
    fn handle_unit_dialog_field_click(&mut self, field: AddUnitField, cx: &mut Context<Self>) {
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
    fn handle_unit_dialog_kind_click(&mut self, kind: UnitKind, cx: &mut Context<Self>) {
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
    fn handle_settings_dialog_cancel(&mut self, cx: &mut Context<Self>) {
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
                    self.settings_units.remove(index);
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

    fn handle_settings_dialog_confirm(&mut self, cx: &mut Context<Self>) {
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
    fn run_command(&mut self, command: &'static Command) {
        record_history(&mut self.command_history, command.name);
        match command.effect {
            CommandEffect::OpenDialog(mode) => {
                self.file_explorer = Some(FileExplorer::open_at(mode, explorer_start_dir()));
            }
            CommandEffect::Navigate(noun) => {
                self.nav.exit_mode();
                let noun_before = self.nav.noun();
                self.nav.set_noun(noun);
                if self.nav.noun() != noun_before {
                    self.reset_view_scroll();
                }
            }
            CommandEffect::CloseLedger => {
                self.nav.exit_mode();
                self.nav.close_ledger();
            }
            CommandEffect::NotYetBuilt => {
                self.nav.exit_mode();
                self.status_message = Some(format!(":{} — not yet built", command.name));
            }
        }
    }

    /// The TopBar's own rail-toggle button (`Shell::render`'s `on_rail_toggle` closure) --
    /// same action as the `b` key, see [`Self::handle_key_down`].
    fn handle_toggle_rail(&mut self, cx: &mut Context<Self>) {
        self.nav.toggle_primary_rail();
        self.collapsed_rail_tooltip = None;
        cx.notify();
    }

    /// A collapsed primary-rail row's raw hover transition (`rail::primary::PrimaryRail`'s
    /// `on_row_hover`). Leaving a row clears any settled tooltip immediately; entering one
    /// only reveals its tooltip after [`TOOLTIP_REVEAL_DELAY`], and only if hover hasn't since
    /// moved elsewhere -- `hover_generation` is the guard: a stale timer whose captured
    /// generation no longer matches the current one simply does nothing.
    fn handle_rail_hover(&mut self, noun: Noun, hovered: bool, cx: &mut Context<Self>) {
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
    fn handle_rail_click(&mut self, noun: Noun, cx: &mut Context<Self>) {
        let noun_before = self.nav.noun();
        self.nav.set_noun(noun);
        if self.nav.noun() != noun_before {
            self.reset_view_scroll();
        }
        cx.notify();
    }

    /// The settings index rail's own row click (`rail::settings_index::OnEntryClick`):
    /// highlights the clicked section and scrolls the settings body so its heading becomes the
    /// top visible child (`docs/ux/desktop/Settings/README.md`'s "Navigation" bullet: "Index
    /// entry click -> scroll the body to that section's heading; the entry takes the active
    /// dark treatment"). `scroll_to_top_of_item` addresses the body's *direct* children, so this
    /// goes through `SettingsSection::body_child_index`, not `SettingsSection::index`, to
    /// account for the page heading occupying child `0` (see `view::settings::render`).
    fn handle_settings_index_click(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        self.settings_selected_section = section;
        self.view_scroll_handle
            .scroll_to_top_of_item(section.body_child_index());
        cx.notify();
    }

    /// The Price Sources table's own row "test"/"edit"/"delete" buttons and its own "+ Add price
    /// source" button (issue #189): no dialog exists for any of these anywhere on this map (same
    /// reasoning as Institutions' own row edit/delete, `Self::handle_institution_edit_click`'s
    /// own doc), so each flashes a plain "not yet built" status message naming no issue.
    fn handle_price_source_test_click(&mut self, index: usize, cx: &mut Context<Self>) {
        let _ = index;
        self.status_message = Some("test price source -- not yet built".to_string());
        cx.notify();
    }

    fn handle_price_source_edit_click(&mut self, index: usize, cx: &mut Context<Self>) {
        let _ = index;
        self.status_message = Some("edit price source -- not yet built".to_string());
        cx.notify();
    }

    fn handle_price_source_delete_click(&mut self, index: usize, cx: &mut Context<Self>) {
        let _ = index;
        self.status_message = Some("delete price source -- not yet built".to_string());
        cx.notify();
    }

    fn handle_add_price_source_click(&mut self, cx: &mut Context<Self>) {
        self.status_message = Some("add price source -- not yet built".to_string());
        cx.notify();
    }

    /// The Units table's own row "delete" button (issue #186, replacing the stub #177 left
    /// behind): opens the destructive Delete unit confirm dialog rather than flashing a status
    /// message. A no-op if `index` is somehow out of bounds (defensive only, same reasoning as
    /// [`Self::handle_unit_edit_click`]).
    fn handle_unit_delete_click(&mut self, index: usize, cx: &mut Context<Self>) {
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
    fn handle_institution_edit_click(&mut self, index: usize, cx: &mut Context<Self>) {
        let _ = index; // no row-scoped state until a future ticket specifies this dialog
        self.status_message = Some("edit institution -- not yet built".to_string());
        cx.notify();
    }

    fn handle_institution_delete_click(&mut self, index: usize, cx: &mut Context<Self>) {
        let _ = index; // no row-scoped state until a future ticket specifies this dialog
        self.status_message = Some("delete institution -- not yet built".to_string());
        cx.notify();
    }

    /// The Institutions table's own "+ Add institution" button (issue #187, replacing the stub
    /// #178 left behind): opens the real Add institution dialog rather than flashing a status
    /// message. `AddInstitutionForm::new` seeds Default unit from `self.settings_units`' own
    /// first entry, so this dialog reads Units' live state even though the two sections are
    /// otherwise independent.
    fn handle_add_institution_click(&mut self, cx: &mut Context<Self>) {
        self.settings_dialog = Some(SettingsDialog::AddInstitution(AddInstitutionForm::new(
            &self.settings_units,
        )));
        self.nav.enter_mode(InputMode::Dialog);
        cx.notify();
    }

    fn handle_add_institution_account_type_click(
        &mut self,
        account_type: AccountType,
        cx: &mut Context<Self>,
    ) {
        if let Some(SettingsDialog::AddInstitution(form)) = self.settings_dialog.as_mut() {
            form.toggle_account_type(account_type);
            cx.notify();
        }
    }

    fn handle_add_institution_unit_click(&mut self, code: String, cx: &mut Context<Self>) {
        if let Some(SettingsDialog::AddInstitution(form)) = self.settings_dialog.as_mut() {
            form.default_unit_code = Some(code);
            cx.notify();
        }
    }

    /// The Sync server section's own "Sync now" button (issue #180): unlike the "+ Add"
    /// buttons above, this has no future ticket that will give it real behaviour -- the map's
    /// own Out-of-scope names it a permanent stand-in -- so the stub message names no issue.
    fn handle_sync_now_click(&mut self, cx: &mut Context<Self>) {
        self.status_message = Some("sync now -- not implemented".to_string());
        cx.notify();
    }

    /// The Data & backup section's own "Backup now"/"Export ledger (CSV)" buttons (issue #181)
    /// -- same permanently-out-of-scope reasoning as [`Self::handle_sync_now_click`].
    fn handle_backup_now_click(&mut self, cx: &mut Context<Self>) {
        self.status_message = Some("backup now -- not implemented".to_string());
        cx.notify();
    }

    fn handle_export_ledger_click(&mut self, cx: &mut Context<Self>) {
        self.status_message = Some("export ledger -- not implemented".to_string());
        cx.notify();
    }

    /// The Tracing (Logs) section's own level radios (issue #182): a stored preference, same
    /// shape as [`Self::handle_row_density_click`] -- there are no real log lines to filter by
    /// level yet.
    fn handle_tracing_level_click(&mut self, level: TracingLevel, cx: &mut Context<Self>) {
        self.settings_tracing_level = level;
        cx.notify();
    }

    /// The Display section's own "Date format" segmented control (issue #179) -- a stored
    /// preference that also re-renders the PREVIEW table's own DATE column.
    fn handle_date_format_click(&mut self, format: DateFormat, cx: &mut Context<Self>) {
        self.settings_date_format = format;
        cx.notify();
    }

    /// The same section's "Decimal & thousands separator" segmented control -- also re-renders
    /// the PREVIEW table's own AMOUNT column.
    fn handle_decimal_separator_click(
        &mut self,
        separator: DecimalSeparator,
        cx: &mut Context<Self>,
    ) {
        self.settings_decimal_separator = separator;
        cx.notify();
    }

    /// The same section's "Row density" segmented control -- also re-renders the PREVIEW table's
    /// own row padding (`view::settings::display`'s own doc: the one field this map gives a real
    /// visual effect to, not just a stored preference).
    fn handle_row_density_click(&mut self, density: RowDensity, cx: &mut Context<Self>) {
        self.settings_row_density = density;
        cx.notify();
    }

    /// The same section's "Status glyphs" radio group -- also re-renders the PREVIEW table's own
    /// leftmost glyph column.
    fn handle_status_glyphs_click(&mut self, glyphs: StatusGlyphs, cx: &mut Context<Self>) {
        self.settings_status_glyphs = glyphs;
        cx.notify();
    }

    /// The same section's "Clear logs" button: unlike every other button this map has built,
    /// this one has a real effect -- the ticket's own body asks for the viewport's in-memory
    /// contents to actually empty, not a stubbed status-line message.
    fn handle_clear_logs_click(&mut self, cx: &mut Context<Self>) {
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
        cx: &mut Context<Self>,
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
    fn handle_explorer_breadcrumb_click(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Some(explorer) = self.file_explorer.as_mut() {
            explorer.navigate_to(path);
            cx.notify();
        }
    }

    /// The explorer dialog's own Cancel button: closes without opening anything, leaving
    /// Command mode the same way the palette's own `esc` does.
    fn handle_explorer_cancel(&mut self, cx: &mut Context<Self>) {
        self.file_explorer = None;
        self.nav.exit_mode();
        cx.notify();
    }

    /// The explorer dialog's own Open button.
    fn handle_explorer_open(&mut self, cx: &mut Context<Self>) {
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
    fn confirm_explorer_open(&mut self, cx: &mut Context<Self>) {
        let Some(explorer) = self.file_explorer.as_ref() else {
            return;
        };
        if !explorer.can_open() {
            return;
        }
        if explorer.mode() == ExplorerMode::Open {
            self.nav.open_ledger();
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
        cx: &mut Context<Self>,
    ) {
        let Some(command) = command::all().find(|command| command.name == command_name) else {
            return;
        };
        self.nav.enter_mode(InputMode::Command);
        self.run_command(command);
        cx.notify();
    }
}

impl Focusable for Shell {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.nav.focus();
        // The "1d" spec: "The shell behind the palette drops to 30% opacity" -- the "1e" file
        // explorer reuses the same dimming pattern (Implementation note 10). Applied to the top
        // bar and content row only, not the status line -- that same spec separately describes
        // the status line's own COMMAND-mode content (the live query, "esc close command
        // window"), which stays meaningful precisely because it stays legible; only the
        // navigational chrome the palette/explorer visually floats over goes dim.
        let content_opacity = if self.palette.is_some() || self.file_explorer.is_some() {
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
        let on_date_format_click: settings_view::display::OnDateFormatClick = {
            let entity = entity.clone();
            Rc::new(move |format, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_date_format_click(format, cx));
            })
        };
        let on_decimal_separator_click: settings_view::display::OnDecimalSeparatorClick = {
            let entity = entity.clone();
            Rc::new(move |separator, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_decimal_separator_click(separator, cx)
                });
            })
        };
        let on_row_density_click: settings_view::display::OnRowDensityClick = {
            let entity = entity.clone();
            Rc::new(move |density, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_row_density_click(density, cx));
            })
        };
        let on_status_glyphs_click: settings_view::display::OnStatusGlyphsClick = {
            let entity = entity.clone();
            Rc::new(move |glyphs, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_status_glyphs_click(glyphs, cx));
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
        let page_status = (self.nav.noun() == Noun::Accounts).then(|| PageStatus {
            hints: ACCOUNTS_HINTS,
            right: match self.accounts.len() {
                1 => "1 account".to_string(),
                count => format!("{count} accounts"),
            },
        });

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color::GROUND)
            .text_color(color::INK)
            .font_family(type_scale::FONT_FAMILY)
            .text_size(type_scale::BODY)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                if this.handle_key_down(event) {
                    cx.notify();
                }
            }))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .opacity(content_opacity)
                    .child(TopBar::new(on_rail_toggle, self.nav.noun()))
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
                                .account_count(self.accounts.len()),
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
                                accounts_page,
                                SettingsPanelProps {
                                    filter: &self.settings_filter,
                                    selected: self.settings_selected_section,
                                    on_index_click: on_settings_index_click,
                                    date_format: self.settings_date_format,
                                    decimal_separator: self.settings_decimal_separator,
                                    row_density: self.settings_row_density,
                                    status_glyphs: self.settings_status_glyphs,
                                    on_date_format_click,
                                    on_decimal_separator_click,
                                    on_row_density_click,
                                    on_status_glyphs_click,
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
                            )),
                    ),
            )
            .child(
                StatusLine::new(
                    self.nav.mode(),
                    self.status_message.clone(),
                    self.command_echo(),
                )
                .page(page_status),
            )
            .children(self.palette.as_ref().map(Palette::render))
            .children(self.file_explorer.as_ref().map(|explorer| {
                explorer.render(
                    on_explorer_entry_click,
                    on_explorer_breadcrumb_click,
                    on_explorer_cancel,
                    on_explorer_open,
                )
            }))
            .children(self.settings_dialog.as_ref().map(|dialog| match dialog {
                SettingsDialog::AddUnit(form) => settings_view::add_unit_dialog::render(
                    form,
                    on_unit_dialog_field_click.clone(),
                    on_unit_dialog_kind_click.clone(),
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                ),
                SettingsDialog::EditUnit(_, form) => settings_view::edit_unit_dialog::render(
                    form,
                    on_unit_dialog_field_click,
                    on_unit_dialog_kind_click,
                    on_settings_dialog_cancel.clone(),
                    on_settings_dialog_confirm.clone(),
                ),
                SettingsDialog::DeleteUnit(index, form) => {
                    match self.settings_units.get(*index) {
                        Some(row) => settings_view::delete_unit_dialog::render(
                            row,
                            form,
                            on_settings_dialog_cancel.clone(),
                            on_settings_dialog_confirm.clone(),
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
                    )
                }
            }))
    }
}

/// Bundles `render_view`'s Settings-only parameters (keeps the function under Clippy's
/// `too_many_arguments` threshold) -- ignored entirely for every noun besides `Settings`. Covers
/// both the index rail's own state and the body's per-section interactive state
/// (`view::settings::SettingsBodyProps`); `render_view` splits it back apart when it builds
/// each half's own component.
struct SettingsPanelProps<'a> {
    filter: &'a str,
    selected: SettingsSection,
    on_index_click: settings_index::OnEntryClick,
    date_format: DateFormat,
    decimal_separator: DecimalSeparator,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    on_date_format_click: settings_view::display::OnDateFormatClick,
    on_decimal_separator_click: settings_view::display::OnDecimalSeparatorClick,
    on_row_density_click: settings_view::display::OnRowDensityClick,
    on_status_glyphs_click: settings_view::display::OnStatusGlyphsClick,
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
fn render_view(
    noun: Noun,
    ledger_open: bool,
    focused: bool,
    scroll_handle: &ScrollHandle,
    on_empty_state_command_click: OnEmptyStateCommandClick,
    accounts: accounts_view::AccountsPageProps<'_>,
    settings: SettingsPanelProps<'_>,
) -> gpui::AnyElement {
    if noun == Noun::Accounts {
        return accounts_view::render(focused, scroll_handle, accounts);
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
                settings.filter.to_string(),
                settings.on_index_click,
            ))
            .child(settings_view::render(
                focused,
                scroll_handle,
                SettingsBodyProps {
                    date_format: settings.date_format,
                    decimal_separator: settings.decimal_separator,
                    row_density: settings.row_density,
                    status_glyphs: settings.status_glyphs,
                    on_date_format_click: settings.on_date_format_click,
                    on_decimal_separator_click: settings.on_decimal_separator_click,
                    on_row_density_click: settings.on_row_density_click,
                    on_status_glyphs_click: settings.on_status_glyphs_click,
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
            ))
            .into_any_element();
    }

    let content = match noun {
        Noun::Dashboard if ledger_open => Dashboard::new().into_any_element(),
        Noun::Dashboard => empty_state(on_empty_state_command_click),
        Noun::Settings | Noun::Accounts => unreachable!("handled above"),
        other => div()
            .p(px(24.0))
            .text_color(color::INK_TERTIARY)
            .child(format!("{other:?} -- not yet built (see issue #153)"))
            .into_any_element(),
    };

    div()
        .id("view")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::INK)
        })
        .child(content)
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
fn empty_state(on_command_click: OnEmptyStateCommandClick) -> gpui::AnyElement {
    let command = |name: &'static str, on_command_click: OnEmptyStateCommandClick| {
        div()
            .id(SharedString::from(format!("empty-state-{name}")))
            .cursor_pointer()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::INK)
            .on_click(move |_event, window, cx| on_command_click(name, window, cx))
            .child(format!(":{name}"))
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
                .child("No ledger open"),
        )
        .child(
            div()
                .max_w(px(380.0))
                .flex()
                .flex_wrap()
                .justify_center()
                .text_align(gpui::TextAlign::Center)
                .text_size(px(13.0))
                .text_color(color::INK_SECONDARY)
                .child("Run ")
                .child(command("open", on_command_click.clone()))
                .child(" to load a ledger file, or ")
                .child(command("new", on_command_click))
                .child(" to start one."),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
