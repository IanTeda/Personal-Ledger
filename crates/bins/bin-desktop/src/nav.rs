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
    Reconcile,
    Budgets,
    Reports,
    Categories,
    Payees,
    Units,
    Settings,
}

impl Noun {
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
    /// Every noun besides `Settings` has its own screen still unbuilt (a placeholder-views
    /// ticket, #153), so this says whether a context rail *could* exist, not that one renders
    /// real data today.
    pub fn has_context_entities(self) -> bool {
        self != Noun::Settings
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
}

/// The active entity within a noun's context rail, if any. An index rather than a real
/// per-noun entity id: no noun has real data wired up yet (each is a future ticket of its
/// own), and "the noun's first entity" (rule 1) is exactly index `0` regardless of what that
/// entity turns out to be once real data lands.
pub type ContextSelection = Option<usize>;

/// Owns the primary/context rail state machine. See the module doc and
/// `docs/ux/desktop/README.md`'s six numbered transition rules -- each is implemented as
/// exactly one method here, named for the rule it enforces.
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

    /// Rule 1: changing `noun` resets `context` to the noun's first entity (index `0`, or
    /// `None` for a noun with none) and moves focus to `View`.
    pub fn set_noun(&mut self, noun: Noun) {
        self.noun = noun;
        self.context = noun.has_context_entities().then_some(0);
        self.focus = FocusZone::View;
    }

    /// Rule 2: changing `context` never changes `noun` or `focus`.
    pub fn set_context(&mut self, context: ContextSelection) {
        self.context = context;
    }

    /// Rule 3: cycles focus forward (`PrimaryRail -> ContextRail -> View -> PrimaryRail`),
    /// skipping `ContextRail` when the active noun has no entities to show there.
    pub fn cycle_focus_forward(&mut self) {
        self.focus = match self.focus {
            FocusZone::PrimaryRail if self.noun.has_context_entities() => FocusZone::ContextRail,
            FocusZone::PrimaryRail => FocusZone::View,
            FocusZone::ContextRail => FocusZone::View,
            FocusZone::View => FocusZone::PrimaryRail,
        };
    }

    /// The reverse of [`Self::cycle_focus_forward`], same skip rule.
    pub fn cycle_focus_backward(&mut self) {
        self.focus = match self.focus {
            FocusZone::PrimaryRail => FocusZone::View,
            FocusZone::ContextRail => FocusZone::PrimaryRail,
            FocusZone::View if self.noun.has_context_entities() => FocusZone::ContextRail,
            FocusZone::View => FocusZone::PrimaryRail,
        };
    }

    /// Sets focus directly -- e.g. `Enter` on the primary rail moves focus to `View` without
    /// going through a cycle step.
    pub fn set_focus(&mut self, focus: FocusZone) {
        self.focus = focus;
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

    // Rule 1
    #[test]
    fn set_noun_resets_context_to_first_entity_and_moves_focus_to_view() {
        let mut nav = NavState::new();
        nav.set_focus(FocusZone::PrimaryRail);

        nav.set_noun(Noun::Accounts);

        assert_eq!(nav.noun(), Noun::Accounts);
        assert_eq!(nav.context(), Some(0));
        assert_eq!(nav.focus(), FocusZone::View);
    }

    #[test]
    fn set_noun_to_a_noun_with_no_entities_clears_context() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Accounts);
        assert_eq!(nav.context(), Some(0));

        nav.set_noun(Noun::Settings);

        assert_eq!(nav.context(), None);
    }

    // Rule 2
    #[test]
    fn set_context_does_not_change_noun_or_focus() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Accounts);
        nav.set_focus(FocusZone::ContextRail);

        nav.set_context(Some(3));

        assert_eq!(nav.context(), Some(3));
        assert_eq!(nav.noun(), Noun::Accounts);
        assert_eq!(nav.focus(), FocusZone::ContextRail);
    }

    // Rule 3
    #[test]
    fn focus_cycles_through_all_three_zones_when_context_exists() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Accounts); // has_context_entities() == true
        nav.set_focus(FocusZone::PrimaryRail);

        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::ContextRail);
        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::View);
        nav.cycle_focus_forward();
        assert_eq!(nav.focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn focus_skips_context_rail_when_noun_has_no_entities() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Settings); // the one noun with no context entities
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
    fn only_settings_has_no_context_entities() {
        assert!(!Noun::Settings.has_context_entities());
        for noun in [
            Noun::Dashboard,
            Noun::Transactions,
            Noun::Accounts,
            Noun::Reconcile,
            Noun::Budgets,
            Noun::Reports,
            Noun::Categories,
            Noun::Payees,
            Noun::Units,
        ] {
            assert!(noun.has_context_entities(), "{noun:?} should have some");
        }
    }

    // Rule 5
    #[test]
    fn toggling_primary_rail_preserves_focus_and_context() {
        let mut nav = NavState::new();
        nav.set_noun(Noun::Accounts);
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
}
