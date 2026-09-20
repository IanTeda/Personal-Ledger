//! `NavState` -- the primary/context rail state machine
//! (`docs/ux/desktop/README.md`'s "State machine"), ported from the handoff's own Rust
//! snippet. Deliberately free of any `gpui` dependency: every rule here is a pure state
//! transition, unit-tested without a window, the same "pure, unit-testable" pattern the
//! desktop feasibility map used for `feasibility_demo::divergent_bar_bounds`.
//!
//! `Shell` (and the chrome tickets that follow) own the one live `NavState` and re-render
//! from it; this module only knows how the state itself changes, never how it's drawn.

use serde::{Deserialize, Serialize};

/// The ten places the primary rail can select. `Noun::default()` is `Dashboard`, the app's
/// one home noun. Serializable -- `noun` is one of the three fields that survive restart
/// (see `crate::persistence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Noun {
    #[default]
    Dashboard,
    Transactions,
    Accounts,
    Categories,
    Payees,
    Tags,
    Bills,
    Budgets,
    Reports,
    Settings,
}

impl Noun {
    /// Every noun, in the primary rail's own row order (top to bottom, skipping group
    /// headings and the divider, which aren't selectable rows).
    pub const ALL: [Noun; 10] = [
        Noun::Dashboard,
        Noun::Transactions,
        Noun::Accounts,
        Noun::Categories,
        Noun::Payees,
        Noun::Tags,
        Noun::Bills,
        Noun::Budgets,
        Noun::Reports,
        Noun::Settings,
    ];

    fn row_index(self) -> usize {
        Self::ALL
            .iter()
            .position(|noun| *noun == self)
            .expect("Noun::ALL must list every Noun")
    }

    /// The next row down. Clamps at the last row rather than wrapping -- `gg`/`G` are the
    /// handoff's own way to jump straight to the first/last row, which would be redundant if
    /// `j`/`k` also wrapped.
    pub fn next_row(self) -> Noun {
        Self::ALL[(self.row_index() + 1).min(Self::ALL.len() - 1)]
    }

    /// The next row up. See [`Self::next_row`] on why this clamps instead of wrapping.
    pub fn prev_row(self) -> Noun {
        Self::ALL[self.row_index().saturating_sub(1)]
    }

    pub fn first_row() -> Noun {
        Self::ALL[0]
    }

    pub fn last_row() -> Noun {
        Self::ALL[Self::ALL.len() - 1]
    }

    /// Whether this noun's context rail has any entities at all. Only `Settings` doesn't.
    ///
    /// The handoff's own state-machine section (rule 1's parenthetical) lists `Dashboard`
    /// alongside `Settings` as having none, but its "Context rail" component spec and its
    /// own accepted mockup both show Dashboard's context rail populated with the account
    /// roll-call ("On Dashboard it shows accounts") -- a real, visible, load-bearing context
    /// rail, not an absent one. Built against the concrete mockup, not the inconsistent
    /// prose: `Dashboard`'s "first entity" (rule 1) is the first account, same shape as any
    /// other noun with entities. See `docs/ux/desktop/README.md`'s "Where this differs from
    /// the handoff".
    ///
    /// `Transactions` has none either (its table fills the pane, as its mockup shows), and neither
    /// does `Accounts`: its page (`docs/ux/desktop/Accounts/README.md`'s 3a) is a
    /// full-width management table with no rail beside it, the same shape as `Settings`.
    ///
    /// Every other noun besides `Dashboard` has its own screen still unbuilt (a placeholder-views
    /// ticket, #153), so this says whether a context rail *could* exist, not that one renders
    /// real data today.
    pub fn has_context_entities(self) -> bool {
        !matches!(self, Noun::Settings | Noun::Accounts | Noun::Transactions)
    }
}

/// Which of the three zones currently has keyboard focus. `FocusZone::default()` is `View` --
/// every launch starts with focus there, per the handoff's "Persistence" note. Not
/// serializable -- `focus` is one of the three fields that deliberately don't survive
/// restart (see `crate::persistence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusZone {
    PrimaryRail,
    ContextRail,
    #[default]
    View,
}

/// The primary rail's own two drawn states (option 1a vs. its collapsed 1c state).
/// `RailMode::default()` is `Expanded`, the accepted design's default. Serializable -- see
/// `Noun`'s own note above.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RailMode {
    #[default]
    Expanded,
    Collapsed,
}

/// The shell's modal input state. `InputMode::default()` is `Normal` -- every launch starts
/// here, per the handoff's "Persistence" note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Insert,
    Command,
    Search,
    /// A Settings dialog (issue #184's own "Add unit" the first) owns every keystroke -- the
    /// modal-dialog analogue of `Command`'s palette ownership, entered on open and exited on
    /// close (Cancel/Confirm/`Esc`), never by a bare keybinding the way `Insert`/`Command`/
    /// `Search` are. Distinct from `Insert` even though both are "a modal surface captures
    /// typed text": `Insert` is `a`'s own vi-style mode, reserved for a future direct-list-edit
    /// feature unrelated to a floating dialog, and conflating the two would make either one
    /// harder to reason about once its own real feature lands.
    Dialog,
    /// The Transactions filter popover (`docs/ux/desktop/Transactions/README.md`'s 4b) owns every
    /// keystroke while it is open. Modal like `Dialog`, but its own mode so the status line reads
    /// `FILTER` (the mockup's own chip) and its legend can differ.
    Filter,
}

/// The active entity within a noun's context rail, if any. An index rather than a real
/// per-noun entity id: no noun has real data wired up yet (each is a future ticket of its
/// own), and "the noun's first entity" (rule 1) is exactly index `0` regardless of what that
/// entity turns out to be once real data lands.
pub type ContextSelection = Option<usize>;

/// Owns the primary/context rail state machine. See the module doc and
/// `docs/ux/desktop/README.md`'s six numbered transition rules -- each is implemented as
/// exactly one method here, named for the rule it enforces.
///
/// **Primary rail highlight vs. selection.** `set_noun` (rule 1) unconditionally moves focus
/// to `View` -- which would make repeated `j`/`k`/`gg`/`G` browsing on the primary rail
/// impossible if movement called `set_noun` directly (the first keypress would bounce focus
/// away). So movement only updates `primary_highlight`, a separate "currently browsed" row;
/// `Enter` (`commit_primary_highlight`) is the sole path that promotes it into `noun`. This
/// mirrors what the handoff's "Movement" bullet already implies by describing `Enter` as a
/// distinct "activate" step, not a side effect of mere navigation. `primary_highlight` is
/// kept equal to `noun` at every other time (any `set_noun` call, or whenever focus
/// (re-)enters `PrimaryRail`), so rendering can use it unconditionally as "the row to draw
/// selected" without needing to know whether a browse is in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct NavState {
    noun: Noun,
    context: ContextSelection,
    focus: FocusZone,
    primary_rail: RailMode,
    mode: InputMode,
    /// Rule 6: the focus zone `exit_mode` restores -- `Some` only while `mode` isn't
    /// `Normal`, set once on the transition away from `Normal` and cleared on the way back.
    pre_mode_focus: Option<FocusZone>,
    primary_highlight: Noun,
    /// Whether a ledger is loaded (`docs/ux/desktop/Shell & Navigation/README.md`'s "State"
    /// block: `shell.ledgerOpen`). `false` on every launch (the "1a" cold-start state) --
    /// deliberately absent from `crate::persistence::PersistedState`, since the "Empty state"
    /// spec section reaches this state "on cold start and after `:close`", never by restoring
    /// a previous session. Rails render identically either way (Implementation note 2); only
    /// the main pane's `Dashboard` arm branches on it (`Shell::render_view`).
    ledger_open: bool,
}

impl Default for NavState {
    /// Every launch's starting state: `Dashboard`, context at its first entity (rule 1 --
    /// `Dashboard` has entities, see `Noun::has_context_entities`), focus in the view, the
    /// primary rail expanded, `Normal` mode. Hand-written rather than derived: `context`'s
    /// own default (`None`) would be wrong for `Dashboard`, which does have entities.
    fn default() -> Self {
        Self {
            noun: Noun::default(),
            context: Noun::default().has_context_entities().then_some(0),
            focus: FocusZone::default(),
            primary_rail: RailMode::default(),
            mode: InputMode::default(),
            pre_mode_focus: None,
            primary_highlight: Noun::default(),
            ledger_open: false,
        }
    }
}

impl NavState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn noun(&self) -> Noun {
        self.noun
    }

    pub fn context(&self) -> ContextSelection {
        self.context
    }

    pub fn focus(&self) -> FocusZone {
        self.focus
    }

    pub fn primary_rail(&self) -> RailMode {
        self.primary_rail
    }

    pub fn mode(&self) -> InputMode {
        self.mode
    }

    pub fn primary_highlight(&self) -> Noun {
        self.primary_highlight
    }

    pub fn ledger_open(&self) -> bool {
        self.ledger_open
    }

    /// Rule 1: changing `noun` resets `context` to the noun's first entity (index `0`, or
    /// `None` for a noun with none) and moves focus to `View`. Also syncs `primary_highlight`
    /// -- see the struct doc's "Primary rail highlight vs. selection" note.
    pub fn set_noun(&mut self, noun: Noun) {
        self.noun = noun;
        self.primary_highlight = noun;
        self.context = noun.has_context_entities().then_some(0);
        self.focus = FocusZone::View;
    }

    /// `j`/`k`/`Down`/`Up`/`gg`/`G` while the primary rail is focused: moves the browsed row
    /// without touching `noun` (see the struct doc). Callers should guard these on
    /// `focus() == FocusZone::PrimaryRail` -- `NavState` doesn't enforce that itself, the same
    /// way `set_context` doesn't enforce being called only while `ContextRail` is focused.
    pub fn move_primary_highlight_next(&mut self) {
        self.primary_highlight = self.primary_highlight.next_row();
    }

    pub fn move_primary_highlight_prev(&mut self) {
        self.primary_highlight = self.primary_highlight.prev_row();
    }

    pub fn move_primary_highlight_first(&mut self) {
        self.primary_highlight = Noun::first_row();
    }

    pub fn move_primary_highlight_last(&mut self) {
        self.primary_highlight = Noun::last_row();
    }

    /// `Enter` on the primary rail: promotes the browsed row into the committed `noun`.
    pub fn commit_primary_highlight(&mut self) {
        self.set_noun(self.primary_highlight);
    }

    /// Rule 2: changing `context` never changes `noun` or `focus`.
    pub fn set_context(&mut self, context: ContextSelection) {
        self.context = context;
    }

    /// Whether the context rail is currently on screen at all: the active noun has entities to
    /// show there (issue #148) *and* a ledger is actually open to source them from (issue
    /// #166) -- `Shell::render` gates `ContextRail` on exactly this, and focus cycling below
    /// must agree, or `Tab` could park focus on a rail that isn't rendered.
    fn context_rail_visible(&self) -> bool {
        self.noun.has_context_entities() && self.ledger_open
    }

    /// Rule 3: cycles focus forward (`PrimaryRail -> ContextRail -> View -> PrimaryRail`),
    /// skipping `ContextRail` when it isn't currently visible (see
    /// [`Self::context_rail_visible`]).
    pub fn cycle_focus_forward(&mut self) {
        let next = match self.focus {
            FocusZone::PrimaryRail if self.context_rail_visible() => FocusZone::ContextRail,
            FocusZone::PrimaryRail => FocusZone::View,
            FocusZone::ContextRail => FocusZone::View,
            FocusZone::View => FocusZone::PrimaryRail,
        };
        self.set_focus(next);
    }

    /// The reverse of [`Self::cycle_focus_forward`], same skip rule.
    pub fn cycle_focus_backward(&mut self) {
        let next = match self.focus {
            FocusZone::PrimaryRail => FocusZone::View,
            FocusZone::ContextRail => FocusZone::PrimaryRail,
            FocusZone::View if self.context_rail_visible() => FocusZone::ContextRail,
            FocusZone::View => FocusZone::PrimaryRail,
        };
        self.set_focus(next);
    }

    /// Sets focus directly -- e.g. `Enter` on the primary rail moves focus to `View` without
    /// going through a cycle step. Resets `primary_highlight` to `noun` whenever focus
    /// (re)enters `PrimaryRail`, so a fresh browse always starts from the committed selection
    /// (see the struct doc's "Primary rail highlight vs. selection" note).
    pub fn set_focus(&mut self, focus: FocusZone) {
        self.focus = focus;
        if focus == FocusZone::PrimaryRail {
            self.primary_highlight = self.noun;
        }
    }

    /// Rule 5: collapsing/expanding the primary rail doesn't touch focus or context -- true
    /// here simply because this method never mutates either field.
    pub fn set_primary_rail(&mut self, mode: RailMode) {
        self.primary_rail = mode;
    }

    pub fn toggle_primary_rail(&mut self) {
        self.primary_rail = match self.primary_rail {
            RailMode::Expanded => RailMode::Collapsed,
            RailMode::Collapsed => RailMode::Expanded,
        };
    }

    /// Rule 6 (entry half): mode transitions pre-empt zone key handling. Entering a
    /// non-`Normal` mode from `Normal` remembers the current focus zone so `exit_mode` can
    /// restore it; entering a mode while already in a (different) non-`Normal` mode leaves
    /// the remembered zone alone, since it already reflects the focus from before any mode
    /// was active.
    pub fn enter_mode(&mut self, mode: InputMode) {
        if self.mode == InputMode::Normal && mode != InputMode::Normal {
            self.pre_mode_focus = Some(self.focus);
        }
        self.mode = mode;
    }

    /// Rule 6 (exit half): `Esc`'s effect -- returns to `Normal` and restores the focus zone
    /// from before the mode was entered. A no-op (besides being idempotently `Normal`) if
    /// already in `Normal`.
    pub fn exit_mode(&mut self) {
        self.mode = InputMode::Normal;
        if let Some(focus) = self.pre_mode_focus.take() {
            self.focus = focus;
        }
    }

    /// The `:open`/`:new` command handlers' stand-in effect (`crate::command`): flips the "1a"
    /// empty state off. Real file I/O (a native file picker, `lib_database` wiring) is separate
    /// future work -- see issue #144's "Out of scope" -- so this is the whole of what either
    /// command does today, just enough to demonstrate the empty-state -> populated-Dashboard
    /// transition.
    pub fn open_ledger(&mut self) {
        self.ledger_open = true;
    }

    /// The `:close` command's effect: returns to the "1a" empty state, same as cold start.
    /// Moves focus off `ContextRail` first if it's there -- that rail (issue #166) is about to
    /// stop rendering, and leaving focus on a zone with nothing visible in it would be a
    /// dead-end `Tab` couldn't even cycle out of consistently (see [`Self::context_rail_visible`]).
    pub fn close_ledger(&mut self) {
        if self.focus == FocusZone::ContextRail {
            self.set_focus(FocusZone::View);
        }
        self.ledger_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_dashboard_normal_focus_in_view() {
        let nav = NavState::new();
        assert_eq!(nav.noun(), Noun::Dashboard);
        // Dashboard has context entities (its account roll-call) -- rule 1's "first entity".
        assert_eq!(nav.context(), Some(0));
        assert_eq!(nav.focus(), FocusZone::View);
        assert_eq!(nav.primary_rail(), RailMode::Expanded);
        assert_eq!(nav.mode(), InputMode::Normal);
    }

    #[test]
    fn default_state_has_no_ledger_open() {
        // The "1a" cold-start state -- see docs/ux/desktop/Shell & Navigation/README.md's
        // "Empty state" section.
        assert!(!NavState::new().ledger_open());
    }

    #[test]
    fn open_ledger_and_close_ledger_toggle_ledger_open() {
        let mut nav = NavState::new();

        nav.open_ledger();
        assert!(nav.ledger_open());

        nav.close_ledger();
        assert!(!nav.ledger_open());
    }

    // Rule 1
    #[test]
    fn set_noun_resets_context_to_first_entity_and_moves_focus_to_view() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::PrimaryRail);

        nav.set_noun(Noun::Categories);

        assert_eq!(nav.noun(), Noun::Categories);
        assert_eq!(nav.context(), Some(0));
        assert_eq!(nav.focus(), FocusZone::View);
    }

    #[test]
    fn set_noun_to_a_noun_with_no_entities_clears_context() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Categories);
        assert_eq!(nav.context(), Some(0));

        nav.set_noun(Noun::Settings);

        assert_eq!(nav.context(), None);
    }

    // Rule 2
    #[test]
    fn set_context_does_not_change_noun_or_focus() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Categories);
        nav.set_focus(FocusZone::ContextRail);

        nav.set_context(Some(3));

        assert_eq!(nav.context(), Some(3));
        assert_eq!(nav.noun(), Noun::Categories);
        assert_eq!(nav.focus(), FocusZone::ContextRail);
    }

    // Rule 3
    #[test]
    fn focus_cycles_through_all_three_zones_when_context_exists() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Categories); // has_context_entities() == true
        nav.open_ledger(); // the context rail also needs a ledger open (issue #166)
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::ContextRail);
        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    // Issue #166
    #[test]
    fn focus_skips_context_rail_when_no_ledger_is_open_even_with_entities() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Categories); // has_context_entities() == true, but...
        assert!(!nav.ledger_open()); // ...no ledger is open by default.
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn closing_the_ledger_moves_focus_off_the_context_rail() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Categories);
        nav.open_ledger();
        nav.set_focus(FocusZone::ContextRail);

        nav.close_ledger();

        assert_eq!(nav.focus(), FocusZone::View);
    }

    #[test]
    fn closing_the_ledger_leaves_other_focus_zones_alone() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Categories);
        nav.open_ledger();
        nav.set_focus(FocusZone::PrimaryRail);

        nav.close_ledger();

        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn accounts_has_no_context_rail_so_focus_skips_it() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Accounts);
        nav.open_ledger();
        assert!(!Noun::Accounts.has_context_entities());
        assert_eq!(nav.context(), None);
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::View);
    }

    #[test]
    fn focus_skips_context_rail_when_noun_has_no_entities() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Settings); // a noun with no context entities
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn focus_cycles_backward_skipping_empty_context_rail() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Settings); // the one noun with no context entities
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_backward();
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_backward();
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    // Rule 4
    #[test]
    fn only_settings_accounts_and_transactions_have_no_context_entities() {
        assert!(!Noun::Settings.has_context_entities());
        assert!(!Noun::Accounts.has_context_entities());
        assert!(!Noun::Transactions.has_context_entities());
        for noun in [
            Noun::Dashboard,
            Noun::Categories,
            Noun::Payees,
            Noun::Tags,
            Noun::Bills,
            Noun::Budgets,
            Noun::Reports,
        ] {
            assert!(noun.has_context_entities(), "{noun:?} should have some");
        }
    }

    // Rule 5
    #[test]
    fn toggling_primary_rail_preserves_focus_and_context() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Categories);
        nav.set_focus(FocusZone::ContextRail);
        nav.set_context(Some(2));

        nav.toggle_primary_rail();

        assert_eq!(nav.primary_rail(), RailMode::Collapsed);
        assert_eq!(nav.focus(), FocusZone::ContextRail);
        assert_eq!(nav.context(), Some(2));

        nav.toggle_primary_rail();
        assert_eq!(nav.primary_rail(), RailMode::Expanded);
    }

    // Rule 6
    #[test]
    fn exit_mode_restores_the_focus_zone_from_before_entry() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::PrimaryRail);

        nav.enter_mode(InputMode::Command);
        assert_eq!(nav.mode(), InputMode::Command);

        // Command mode pre-empts zone focus conceptually, but nothing here forces `focus`
        // itself to change -- only `exit_mode`'s restoration is under test.
        nav.exit_mode();

        assert_eq!(nav.mode(), InputMode::Normal);
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn entering_a_second_mode_does_not_overwrite_the_remembered_focus() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::ContextRail);

        nav.enter_mode(InputMode::Insert);
        nav.set_focus(FocusZone::View); // e.g. a dialog moved focus while modal
        nav.enter_mode(InputMode::Command);
        nav.exit_mode();

        assert_eq!(nav.focus(), FocusZone::ContextRail);
    }

    #[test]
    fn exit_mode_while_already_normal_is_a_no_op() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::PrimaryRail);

        nav.exit_mode();

        assert_eq!(nav.mode(), InputMode::Normal);
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn noun_row_order_clamps_at_both_ends() {
        assert_eq!(Noun::Dashboard.prev_row(), Noun::Dashboard);
        assert_eq!(Noun::Settings.next_row(), Noun::Settings);
        assert_eq!(Noun::Dashboard.next_row(), Noun::Transactions);
        assert_eq!(Noun::Settings.prev_row(), Noun::Reports);
    }

    #[test]
    fn noun_first_and_last_row() {
        assert_eq!(Noun::first_row(), Noun::Dashboard);
        assert_eq!(Noun::last_row(), Noun::Settings);
    }

    #[test]
    fn primary_highlight_moves_independently_of_noun_until_committed() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::PrimaryRail);

        nav.move_primary_highlight_next();
        nav.move_primary_highlight_next();

        assert_eq!(nav.primary_highlight(), Noun::Accounts);
        // Browsing never touches the committed noun or moves focus away.
        assert_eq!(nav.noun(), Noun::Dashboard);
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);

        nav.commit_primary_highlight();

        assert_eq!(nav.noun(), Noun::Accounts);
        assert_eq!(nav.focus(), FocusZone::View);
    }

    #[test]
    fn primary_highlight_resets_to_noun_when_focus_enters_primary_rail() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::PrimaryRail);
        nav.move_primary_highlight_last();
        assert_eq!(nav.primary_highlight(), Noun::Settings);

        // Leaving without committing, then coming back, starts the browse over.
        nav.set_focus(FocusZone::View);
        nav.set_focus(FocusZone::PrimaryRail);

        assert_eq!(nav.primary_highlight(), Noun::Dashboard);
    }

    #[test]
    fn set_noun_syncs_primary_highlight() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Reports);
        assert_eq!(nav.primary_highlight(), Noun::Reports);
    }
}
