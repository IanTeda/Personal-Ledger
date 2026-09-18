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
    settings::{self, BudgetPeriod, DefaultUnit, InstitutionRow, SettingsSection, UnitRow},
    statusline::StatusLine,
    theme::{color, type_scale},
    topbar::{self, TopBar},
    view::{
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
    /// The Ledger & units section's "Default unit for new entries" (issue #176) -- unlike
    /// [`Self::settings_filter`]/[`Self::settings_selected_section`], this is *not* reset by
    /// [`Self::reset_view_scroll`]: it represents a saved-in-memory preference, not navigational
    /// UI state, so it must survive leaving and re-entering Settings the way a real saved
    /// setting would.
    settings_default_unit: DefaultUnit,
    /// The same section's "Budget period" -- see [`Self::settings_default_unit`]'s own doc for
    /// why this also isn't reset on noun change.
    settings_budget_period: BudgetPeriod,
    /// The Units section's own table rows (issue #177), seeded from `settings::DEFAULT_UNITS`.
    /// A real, mutable `Vec` (not a `const` slice) so the Add/Edit/Delete unit dialogs (issues
    /// #184-#186) can push/update/remove rows once they land -- not reset on noun change, same
    /// reasoning as [`Self::settings_default_unit`].
    settings_units: Vec<UnitRow>,
    /// The Institutions section's own table rows (issue #178), seeded from
    /// `settings::DEFAULT_INSTITUTIONS` -- same reasoning as [`Self::settings_units`].
    settings_institutions: Vec<InstitutionRow>,
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
            settings_default_unit: DefaultUnit::default(),
            settings_budget_period: BudgetPeriod::default(),
            settings_units: settings::DEFAULT_UNITS.to_vec(),
            settings_institutions: settings::DEFAULT_INSTITUTIONS.to_vec(),
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
            KeyOutcome::NoOp => had_status_message,
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

    /// The Ledger & units section's "Default unit for new entries" segmented control
    /// (`view::settings::ledger_units::OnDefaultUnitClick`).
    fn handle_default_unit_click(&mut self, unit: DefaultUnit, cx: &mut Context<Self>) {
        self.settings_default_unit = unit;
        cx.notify();
    }

    /// The same section's "Budget period" segmented control
    /// (`view::settings::ledger_units::OnBudgetPeriodClick`).
    fn handle_budget_period_click(&mut self, period: BudgetPeriod, cx: &mut Context<Self>) {
        self.settings_budget_period = period;
        cx.notify();
    }

    /// The Units table's own row "edit"/"delete" buttons and its "+ Add unit" button (issue
    /// #177): all three open a dialog this map hasn't built yet (issues #184-#186), so each is a
    /// clearly-marked stub -- flashing the same "not yet built" status-line message
    /// `CommandEffect::NotYetBuilt` already uses, naming the specific ticket that owes the real
    /// behaviour, rather than silently doing nothing.
    fn handle_unit_edit_click(&mut self, index: usize, cx: &mut Context<Self>) {
        let _ = index; // no row-scoped state until #185 actually opens a dialog on it
        self.status_message = Some("edit unit -- not yet built (see issue #185)".to_string());
        cx.notify();
    }

    fn handle_unit_delete_click(&mut self, index: usize, cx: &mut Context<Self>) {
        let _ = index; // no row-scoped state until #186 actually opens a dialog on it
        self.status_message = Some("delete unit -- not yet built (see issue #186)".to_string());
        cx.notify();
    }

    fn handle_add_unit_click(&mut self, cx: &mut Context<Self>) {
        self.status_message = Some("add unit -- not yet built (see issue #184)".to_string());
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

    /// The Institutions table's own "+ Add institution" button (issue #178): opens the Add
    /// institution dialog (issue #187), not yet built -- same stub shape as
    /// [`Self::handle_add_unit_click`].
    fn handle_add_institution_click(&mut self, cx: &mut Context<Self>) {
        self.status_message = Some("add institution -- not yet built (see issue #187)".to_string());
        cx.notify();
    }

    /// The Sync server section's own "Sync now" button (issue #180): unlike the "+ Add"
    /// buttons above, this has no future ticket that will give it real behaviour -- the map's
    /// own Out-of-scope names it a permanent stand-in -- so the stub message names no issue.
    fn handle_sync_now_click(&mut self, cx: &mut Context<Self>) {
        self.status_message = Some("sync now -- not implemented".to_string());
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
        let on_default_unit_click: settings_view::ledger_units::OnDefaultUnitClick = {
            let entity = entity.clone();
            Rc::new(move |unit, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_default_unit_click(unit, cx));
            })
        };
        let on_budget_period_click: settings_view::ledger_units::OnBudgetPeriodClick = {
            let entity = entity.clone();
            Rc::new(move |period, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_budget_period_click(period, cx));
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
                            .child(PrimaryRail::new(
                                self.nav.primary_highlight(),
                                focus == FocusZone::PrimaryRail,
                                self.nav.primary_rail(),
                                self.collapsed_rail_tooltip,
                                on_row_hover,
                                on_row_click,
                            ))
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
                                SettingsPanelProps {
                                    filter: &self.settings_filter,
                                    selected: self.settings_selected_section,
                                    on_index_click: on_settings_index_click,
                                    default_unit: self.settings_default_unit,
                                    budget_period: self.settings_budget_period,
                                    on_default_unit_click,
                                    on_budget_period_click,
                                    units: &self.settings_units,
                                    on_unit_edit_click,
                                    on_unit_delete_click,
                                    on_add_unit_click,
                                    institutions: &self.settings_institutions,
                                    on_institution_edit_click,
                                    on_institution_delete_click,
                                    on_add_institution_click,
                                    on_sync_now_click,
                                },
                            )),
                    ),
            )
            .child(StatusLine::new(
                self.nav.mode(),
                self.status_message.clone(),
                self.command_echo(),
            ))
            .children(self.palette.as_ref().map(Palette::render))
            .children(self.file_explorer.as_ref().map(|explorer| {
                explorer.render(
                    on_explorer_entry_click,
                    on_explorer_breadcrumb_click,
                    on_explorer_cancel,
                    on_explorer_open,
                )
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
    default_unit: DefaultUnit,
    budget_period: BudgetPeriod,
    on_default_unit_click: settings_view::ledger_units::OnDefaultUnitClick,
    on_budget_period_click: settings_view::ledger_units::OnBudgetPeriodClick,
    units: &'a [UnitRow],
    on_unit_edit_click: settings_view::units::OnRowIndexClick,
    on_unit_delete_click: settings_view::units::OnRowIndexClick,
    on_add_unit_click: settings_view::units::OnAddClick,
    institutions: &'a [InstitutionRow],
    on_institution_edit_click: settings_view::institutions::OnRowIndexClick,
    on_institution_delete_click: settings_view::institutions::OnRowIndexClick,
    on_add_institution_click: settings_view::institutions::OnAddClick,
    on_sync_now_click: settings_view::sync_server::OnSyncNowClick,
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
    settings: SettingsPanelProps<'_>,
) -> gpui::AnyElement {
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
                    default_unit: settings.default_unit,
                    budget_period: settings.budget_period,
                    on_default_unit_click: settings.on_default_unit_click,
                    on_budget_period_click: settings.on_budget_period_click,
                    units: settings.units,
                    on_unit_edit_click: settings.on_unit_edit_click,
                    on_unit_delete_click: settings.on_unit_delete_click,
                    on_add_unit_click: settings.on_add_unit_click,
                    institutions: settings.institutions,
                    on_institution_edit_click: settings.on_institution_edit_click,
                    on_institution_delete_click: settings.on_institution_delete_click,
                    on_add_institution_click: settings.on_add_institution_click,
                    on_sync_now_click: settings.on_sync_now_click,
                },
            ))
            .into_any_element();
    }

    let content = match noun {
        Noun::Dashboard if ledger_open => Dashboard::new().into_any_element(),
        Noun::Dashboard => empty_state(on_empty_state_command_click),
        Noun::Settings => unreachable!("handled above"),
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
