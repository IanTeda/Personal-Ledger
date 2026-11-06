//! `NavState`, the primary/context rail state machine (`docs/ux/desktop-mockups/README.md`'s
//! "State machine"): the [`Noun`] showing, the context selection, which zone has focus, the input
//! mode, the Back/Forward history and whether a Ledger is open. Moved from the Desktop's
//! `navigation/` (#826) so the Shell's Presenter holds focus and modes; every rule is a pure state
//! transition, tested without a window. `P` is the Client's Settings page, which the history steps
//! between.

use crate::navigation::Noun;

/// Which of the three zones currently has keyboard focus. `FocusZone::default()` is `View` --
/// every launch starts with focus there, per the handoff's "Persistence" note. Not
/// serializable -- `focus` is one of the three fields that deliberately don't survive
/// restart (the Desktop's `persistence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusZone {
    PrimaryRail,
    ContextRail,
    #[default]
    View,
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
    /// The Transactions filter popover (`docs/ux/desktop-mockups/03-transactions/README.md`'s 4b) owns every
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
/// `docs/ux/desktop-mockups/README.md`'s six numbered transition rules -- each is implemented as
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
pub struct NavState<P> {
    noun: Noun,
    context: ContextSelection,
    focus: FocusZone,
    mode: InputMode,
    /// Whether the active View entered `mode` for something of its own (its `EnterMode`), so its
    /// keys are the View's. Set by `enter_view_mode`, cleared by any other entry and by leaving.
    view_owns_mode: bool,
    /// Rule 6: the focus zone `exit_mode` restores -- `Some` only while `mode` isn't
    /// `Normal`, set once on the transition away from `Normal` and cleared on the way back.
    pre_mode_focus: Option<FocusZone>,
    primary_highlight: Noun,
    /// The top bar's Back history: the nouns left by `set_noun`, most recent last, capped at
    /// `HISTORY_LIMIT`. A noun change clears `forward`, the way a browser's does.
    back: Vec<Place<P>>,
    /// The nouns `go_back` stepped away from, most recent last, for `go_forward` to return to.
    forward: Vec<Place<P>>,
    /// The Settings page showing (or last shown), so Back and Forward can step between pages.
    settings_page: P,
}

/// One stop in the Back/Forward history: a noun, and for Settings which page was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Place<P> {
    noun: Noun,
    settings_page: P,
}

/// How far Back reaches: enough for any session's wandering without growing for ever.
const HISTORY_LIMIT: usize = 50;

impl<P: Default> Default for NavState<P> {
    /// Every launch's starting state: `Dashboard`, context at its first entity (rule 1 --
    /// `Dashboard` has entities, see `Noun::has_context_entities`), focus in the view, the
    /// primary rail expanded, `Normal` mode. Hand-written rather than derived: `context`'s
    /// own default (`None`) would be wrong for `Dashboard`, which does have entities.
    fn default() -> Self {
        Self {
            noun: Noun::default(),
            context: Noun::default().has_context_entities().then_some(0),
            focus: FocusZone::default(),
            mode: InputMode::default(),
            view_owns_mode: false,
            pre_mode_focus: None,
            primary_highlight: Noun::default(),
            back: Vec::new(),
            forward: Vec::new(),
            settings_page: P::default(),
        }
    }
}

impl<P: Copy + Eq + Default> NavState<P> {
    pub fn new() -> Self {
        Self::default()
    }

    /// A launch's starting state at a restored `noun`, with no Back history: the restored View
    /// is where the session starts, not a step taken from the Dashboard.
    pub fn starting_at(noun: Noun) -> Self {
        let mut nav = Self::default();
        nav.show_noun(noun);
        nav
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

    pub fn mode(&self) -> InputMode {
        self.mode
    }

    /// Whether the current mode is one the active View entered for something of its own.
    pub fn view_owns_mode(&self) -> bool {
        self.view_owns_mode
    }

    pub fn primary_highlight(&self) -> Noun {
        self.primary_highlight
    }

    /// Rule 1: changing `noun` resets `context` to the noun's first entity (index `0`, or
    /// `None` for a noun with none) and moves focus to `View`. Also syncs `primary_highlight`
    /// -- see the struct doc's "Primary rail highlight vs. selection" note.
    pub fn set_noun(&mut self, noun: Noun) {
        if noun != self.noun {
            self.push_back();
        }
        self.show_noun(noun);
    }

    fn here(&self) -> Place<P> {
        Place {
            noun: self.noun,
            settings_page: self.settings_page,
        }
    }

    /// Records the current place for Back and drops Forward, as a browser's navigation does.
    fn push_back(&mut self) {
        if self.back.len() == HISTORY_LIMIT {
            self.back.remove(0);
        }
        self.back.push(self.here());
        self.forward.clear();
    }

    /// The Settings page showing. A page change while Settings is the noun is a step in the
    /// history; otherwise (a `:settings <page>` about to switch noun) it is just noted.
    pub fn record_settings_page(&mut self, page: P) {
        if page == self.settings_page {
            return;
        }
        if self.noun == Noun::Settings {
            self.push_back();
        }
        self.settings_page = page;
    }

    /// Sets the page without a history step: the launch restoring the last-visited page.
    pub fn restore_settings_page(&mut self, page: P) {
        self.settings_page = page;
    }

    /// The Settings page the history says should be showing.
    pub fn settings_page(&self) -> P {
        self.settings_page
    }

    /// Drops the Back and Forward history, leaving the place showing. `:close` calls it: the
    /// places visited were on the closed Ledger, so with none open Back must not reach them.
    pub fn forget_history(&mut self) {
        self.back.clear();
        self.forward.clear();
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// The top bar's Back: returns to the noun before the last `set_noun`, with rule 1's
    /// resets. `false` when there's nowhere to go back to.
    pub fn go_back(&mut self) -> bool {
        let Some(place) = self.back.pop() else {
            return false;
        };
        self.forward.push(self.here());
        self.settings_page = place.settings_page;
        self.show_noun(place.noun);
        true
    }

    /// The top bar's Forward: undoes the last `go_back`. `false` once nothing is ahead.
    pub fn go_forward(&mut self) -> bool {
        let Some(place) = self.forward.pop() else {
            return false;
        };
        self.back.push(self.here());
        self.settings_page = place.settings_page;
        self.show_noun(place.noun);
        true
    }

    /// Rule 1's resets on showing `noun`, without touching the Back/Forward history.
    fn show_noun(&mut self, noun: Noun) {
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

    /// Whether the context rail is on screen at all: the active noun has entities to show there
    /// (issue #148) *and* a Ledger is open to source them from (issue #166). Whether one is open
    /// is the Client's to know, from its Open Ledger (#975), so it is passed in rather than kept
    /// here. The Desktop gates `ContextRail` on exactly this, and focus cycling below must agree,
    /// or `Tab` could park focus on a rail that isn't rendered.
    pub fn context_rail_visible(&self, ledger_open: bool) -> bool {
        self.noun.has_context_entities() && ledger_open
    }

    /// Rule 3: cycles focus forward (`PrimaryRail -> ContextRail -> View -> PrimaryRail`),
    /// skipping `ContextRail` when it isn't visible with `ledger_open` (see
    /// [`Self::context_rail_visible`]).
    pub fn cycle_focus_forward(&mut self, ledger_open: bool) {
        let next = match self.focus {
            FocusZone::PrimaryRail if self.context_rail_visible(ledger_open) => {
                FocusZone::ContextRail
            }
            FocusZone::PrimaryRail => FocusZone::View,
            FocusZone::ContextRail => FocusZone::View,
            FocusZone::View => FocusZone::PrimaryRail,
        };
        self.set_focus(next);
    }

    /// The reverse of [`Self::cycle_focus_forward`], same skip rule.
    pub fn cycle_focus_backward(&mut self, ledger_open: bool) {
        let next = match self.focus {
            FocusZone::PrimaryRail => FocusZone::View,
            FocusZone::ContextRail => FocusZone::PrimaryRail,
            FocusZone::View if self.context_rail_visible(ledger_open) => FocusZone::ContextRail,
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
        self.view_owns_mode = false;
    }

    /// As [`Self::enter_mode`], for a mode the active View opened something of its own for: its
    /// Search box, a filter popover or an edit in place. The owner lives here rather than in the
    /// bin so the Presenter that holds the mode can say whose keys it takes.
    pub fn enter_view_mode(&mut self, mode: InputMode) {
        self.enter_mode(mode);
        self.view_owns_mode = true;
    }

    /// Rule 6 (exit half): `Esc`'s effect -- returns to `Normal` and restores the focus zone
    /// from before the mode was entered. A no-op (besides being idempotently `Normal`) if
    /// already in `Normal`.
    pub fn exit_mode(&mut self) {
        self.mode = InputMode::Normal;
        self.view_owns_mode = false;
        if let Some(focus) = self.pre_mode_focus.take() {
            self.focus = focus;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Client's Settings pages, as the history steps between them.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    enum Page {
        #[default]
        General,
        Units,
        Tags,
    }

    type NavState = super::NavState<Page>;

    #[test]
    fn default_state_is_dashboard_normal_focus_in_view() {
        let nav = NavState::new();
        assert_eq!(nav.noun(), Noun::Dashboard);
        // Dashboard has context entities (its account roll-call) -- rule 1's "first entity".
        assert_eq!(nav.context(), Some(0));
        assert_eq!(nav.focus(), FocusZone::View);
        assert_eq!(nav.mode(), InputMode::Normal);
    }

    // Rule 1
    #[test]
    fn set_noun_resets_context_to_first_entity_and_moves_focus_to_view() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::PrimaryRail);

        nav.set_noun(Noun::Budgets);

        assert_eq!(nav.noun(), Noun::Budgets);
        assert_eq!(nav.context(), Some(0));
        assert_eq!(nav.focus(), FocusZone::View);
    }

    #[test]
    fn set_noun_to_a_noun_with_no_entities_clears_context() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Budgets);
        assert_eq!(nav.context(), Some(0));

        nav.set_noun(Noun::Settings);

        assert_eq!(nav.context(), None);
    }

    // Rule 2
    #[test]
    fn set_context_does_not_change_noun_or_focus() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Budgets);
        nav.set_focus(FocusZone::ContextRail);

        nav.set_context(Some(3));

        assert_eq!(nav.context(), Some(3));
        assert_eq!(nav.noun(), Noun::Budgets);
        assert_eq!(nav.focus(), FocusZone::ContextRail);
    }

    // Rule 3
    #[test]
    fn focus_cycles_through_all_three_zones_when_context_exists() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Budgets); // has_context_entities() == true
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward(true);
        assert_eq!(nav.focus(), FocusZone::ContextRail);
        nav.cycle_focus_forward(true);
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_forward(true);
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    // Issue #166
    #[test]
    fn focus_skips_context_rail_when_no_ledger_is_open_even_with_entities() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Budgets); // has_context_entities() == true, but no Ledger is open
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward(false);
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_forward(false);
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn placeholder_nouns_have_no_context_rail_so_focus_skips_it() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Cash);
        assert!(!Noun::Cash.has_context_entities());
        assert_eq!(nav.context(), None);
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward(true);
        assert_eq!(nav.focus(), FocusZone::View);
    }

    #[test]
    fn focus_skips_context_rail_when_noun_has_no_entities() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Settings); // a noun with no context entities
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward(true);
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_forward(true);
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn focus_cycles_backward_skipping_empty_context_rail() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Settings); // the one noun with no context entities
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_backward(true);
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_backward(true);
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
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
    fn a_mode_the_view_entered_is_the_views() {
        let mut nav = NavState::new();
        assert!(!nav.view_owns_mode());

        nav.enter_view_mode(InputMode::Filter);

        assert_eq!(nav.mode(), InputMode::Filter);
        assert!(nav.view_owns_mode());
    }

    #[test]
    fn a_mode_the_shell_entered_is_not_the_views() {
        let mut nav = NavState::new();

        nav.enter_mode(InputMode::Search);

        assert!(!nav.view_owns_mode());
    }

    #[test]
    fn leaving_a_mode_clears_the_owner() {
        let mut nav = NavState::new();
        nav.enter_view_mode(InputMode::Insert);

        nav.exit_mode();

        assert_eq!(nav.mode(), InputMode::Normal);
        assert!(!nav.view_owns_mode());
    }

    #[test]
    fn a_mode_entered_over_the_views_is_the_shells() {
        let mut nav = NavState::new();
        nav.enter_view_mode(InputMode::Search);

        // A Dialog opening over the View's Search.
        nav.enter_mode(InputMode::Dialog);

        assert!(!nav.view_owns_mode());
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
    fn primary_highlight_moves_independently_of_noun_until_committed() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::PrimaryRail);

        nav.move_primary_highlight_next();
        nav.move_primary_highlight_next();

        assert_eq!(nav.primary_highlight(), Noun::Documents);
        // Browsing never touches the committed noun or moves focus away.
        assert_eq!(nav.noun(), Noun::Dashboard);
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);

        nav.commit_primary_highlight();

        assert_eq!(nav.noun(), Noun::Documents);
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

    #[test]
    fn set_noun_records_the_noun_it_left_for_back() {
        let mut nav = NavState::new();
        assert!(!nav.can_go_back());

        nav.set_noun(Noun::Budgets);
        nav.set_noun(Noun::Budgets);
        nav.set_noun(Noun::Bills);

        assert!(nav.go_back());
        assert_eq!(nav.noun(), Noun::Budgets);
        assert!(nav.go_back());
        assert_eq!(nav.noun(), Noun::Dashboard);
        assert!(!nav.go_back(), "a repeated noun is not a step");
    }

    #[test]
    fn settings_page_changes_are_history_steps() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Settings);
        nav.record_settings_page(Page::Units);
        nav.record_settings_page(Page::Tags);
        assert!(nav.go_back());
        assert_eq!(nav.settings_page(), Page::Units);
        assert!(nav.go_back());
        assert_eq!(nav.settings_page(), Page::General);
        assert!(nav.go_forward());
        assert_eq!(nav.settings_page(), Page::Units);
    }

    #[test]
    fn go_forward_retraces_back_until_a_new_noun_clears_it() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Budgets);
        nav.set_noun(Noun::Bills);
        nav.go_back();
        nav.go_back();

        assert!(nav.go_forward());
        assert_eq!(nav.noun(), Noun::Budgets);
        assert_eq!(nav.context(), Some(0), "rule 1's resets still apply");

        nav.set_noun(Noun::Settings);
        assert!(!nav.can_go_forward());
        assert!(!nav.go_forward());
    }

    #[test]
    fn back_history_is_capped() {
        let mut nav = NavState::new();
        for _ in 0..HISTORY_LIMIT {
            nav.set_noun(Noun::Budgets);
            nav.set_noun(Noun::Bills);
        }
        let mut steps = 0;
        while nav.go_back() {
            steps += 1;
        }
        assert_eq!(steps, HISTORY_LIMIT);
    }

    #[test]
    fn forgetting_history_leaves_nowhere_to_go_back_or_forward() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Transactions);
        nav.set_noun(Noun::Reports);
        nav.go_back();
        nav.forget_history();
        assert_eq!(nav.noun(), Noun::Transactions);
        assert!(!nav.can_go_back());
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn starting_at_a_restored_noun_has_no_history() {
        let nav = NavState::starting_at(Noun::Reports);
        assert_eq!(nav.noun(), Noun::Reports);
        assert!(!nav.can_go_back());
    }
}
