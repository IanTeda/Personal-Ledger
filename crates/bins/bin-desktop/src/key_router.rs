//! Pure key-routing decisions for `Shell`'s `Normal`-mode keymap
//! (`docs/ux/desktop/Shell & Navigation/README.md`'s "Keyboard" section) -- `gpui`-free, the
//! same "pure state, `Shell` applies it" split `nav.rs`/`palette.rs`/`explorer.rs` already use.
//!
//! Extracted from `Shell::handle_key_down` (issue #144's own architecture review, "pull Shell's
//! key-routing tiers out from behind its render loop"): that method mixes nine sequential,
//! order-dependent tiers (`Esc`, the command palette's own keys, a pending `g` chord's
//! completion, mode-entry keys, `Tab`, arming a fresh `g`, movement) into a method that also
//! owns rendering and dialog state -- exactly the shape that let #150's own ordering bug (mode-
//! entry keys checked *before* the pending-`g` chord) slip through untested, since `Shell`
//! can't be constructed in a plain `#[test]` at all (its `FocusHandle` needs a real `gpui`
//! window). [`route_key`] is the fix: tier order is now a directly-tested, `gpui`-free
//! function, not implicit in an untested method. Scoped to the outer tiers only -- the command
//! palette's own per-keystroke handling (`Shell::handle_palette_key`) isn't broken and doesn't
//! share this risk, so it stays where it is.

use crate::nav::{InputMode, Noun};

/// A single semantic movement, parsed once from a keystroke and then dispatched against
/// whichever zone is focused -- the same physical keys mean different things per zone, but
/// the keys-to-intent mapping itself doesn't vary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    Next,
    Prev,
    First,
    Last,
    HalfPageDown,
    HalfPageUp,
    Enter,
}

/// Every outcome [`route_key`] can produce -- one closed enum, one exhaustive match at
/// `Shell::handle_key_down`'s own application site, the same shape `CommandEffect` and
/// `bin-tui`'s own `Action` already use.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyOutcome {
    /// `Esc` cleared a pending `g` -- nothing else to do. Kept distinct from the generic
    /// [`Self::NoOp`] because `Shell` must *not* clear a stale status message for this one (see
    /// [`Self::EscapeNoOp`]'s own doc for why Escape as a whole skips that).
    ClearPendingG,
    /// `Esc` left a non-`Normal` mode: close any open dialog and exit the mode.
    ClosePopupsAndExitMode,
    /// `Esc` with nothing to clear and already in `Normal` mode -- a true no-op, distinct from
    /// [`Self::NoOp`]: in the original, un-extracted `handle_key_down`, `Esc`'s own branches
    /// return before the "any keypress clears a stale status message" rule even runs, so an
    /// `Esc` that has nothing to do leaves a showing status message on screen. Preserved here
    /// rather than "fixed" as a side effect of this extraction -- if that's actually a bug, it's
    /// a separate decision, not a free side effect of a refactor.
    EscapeNoOp,
    /// `InputMode::Command` owns every keystroke -- hand off to `Shell::handle_palette_key`.
    DelegateToPalette,
    /// `InputMode::Search` owns every keystroke -- hand off to `Shell::handle_search_key`. Only
    /// meaningful on the Settings noun today (its index rail's own `/ filter`,
    /// `docs/ux/desktop/Settings/README.md`'s "Navigation" bullet); `Shell::handle_search_key`
    /// is a no-op everywhere else, the same way `Swallowed` used to be for this mode outright.
    DelegateToSearch,
    /// `InputMode::Insert` has no real input surface yet -- swallow the key.
    Swallowed,
    /// `InputMode::Dialog` owns every keystroke -- hand off to `Shell::handle_dialog_key`.
    DelegateToDialog,
    /// A pending `g` completed: jump straight to this noun.
    JumpToNoun(Noun),
    /// A pending `g` was followed by an unbound key: flash this already-formatted hint-strip
    /// message (formatting is pure string work, so it happens here rather than in `Shell`).
    PendingGUnbound(String),
    EnterCommand,
    EnterSearch,
    EnterInsert,
    ToggleRail,
    CycleFocusForward,
    CycleFocusBackward,
    /// Arms a fresh pending `g` -- `Shell` records the timestamp; this decision doesn't need to
    /// know the clock at all.
    ArmPendingG,
    Movement(Movement),
    /// No tier claimed this key. Clears a stale status message (unlike [`Self::EscapeNoOp`]).
    NoOp,
}

/// The `g`-prefix jump target for each bound completion key. Every noun has one today --
/// `Transactions` moved off `g t` onto `g l` to free `t` for the newer `Tags` noun.
fn jump_noun_for_key(key: &str) -> Option<Noun> {
    match key {
        "d" => Some(Noun::Dashboard),
        "l" => Some(Noun::Transactions),
        "a" => Some(Noun::Accounts),
        "c" => Some(Noun::Categories),
        "p" => Some(Noun::Payees),
        "t" => Some(Noun::Tags),
        "w" => Some(Noun::Bills),
        "b" => Some(Noun::Budgets),
        "r" => Some(Noun::Reports),
        "s" => Some(Noun::Settings),
        _ => None,
    }
}

/// Decides what a keystroke means, in the exact tier order `Shell::handle_key_down` used to
/// encode as nine sequential, order-dependent `if`/`match` blocks -- `Esc` first (works in any
/// mode, clears a pending `g` before leaving the current mode), then -- while a non-`Normal`
/// mode is active -- nothing else, then a pending `g`'s own completion/abort (before anything
/// else can claim the key, so `g a` reaches Accounts rather than bare `a`'s mode entry), then
/// the global mode-entry keys, `Tab` cycling, arming a fresh `g`, and finally a [`Movement`].
///
/// Deliberately deterministic: `pending_g_active` is a plain `bool`, not the `Instant` `Shell`
/// actually times it against -- reading the clock stays on `Shell`'s own impure side, so this
/// function (and its tests) never touch real time at all.
pub fn route_key(
    mode: InputMode,
    pending_g_active: bool,
    key: &str,
    ctrl: bool,
    shift: bool,
) -> KeyOutcome {
    if key == "escape" {
        if pending_g_active {
            return KeyOutcome::ClearPendingG;
        }
        return if mode != InputMode::Normal {
            KeyOutcome::ClosePopupsAndExitMode
        } else {
            KeyOutcome::EscapeNoOp
        };
    }

    if mode == InputMode::Command {
        return KeyOutcome::DelegateToPalette;
    }
    if mode == InputMode::Search {
        return KeyOutcome::DelegateToSearch;
    }
    if mode == InputMode::Dialog {
        return KeyOutcome::DelegateToDialog;
    }
    if mode != InputMode::Normal {
        return KeyOutcome::Swallowed;
    }

    if pending_g_active {
        if key == "g" && !ctrl && !shift {
            return KeyOutcome::Movement(Movement::First);
        }
        if !ctrl
            && !shift
            && let Some(noun) = jump_noun_for_key(key)
        {
            return KeyOutcome::JumpToNoun(noun);
        }
        // The handoff: "`g` + an unbound key is a no-op: clear the pending prefix and flash
        // the hint strip." `key` itself is consumed doing nothing else -- it completes (aborts)
        // the chord rather than also being processed as its own ordinary keystroke.
        return KeyOutcome::PendingGUnbound(format!("g {key} is not a jump"));
    }

    match key {
        ":" => return KeyOutcome::EnterCommand,
        "/" => return KeyOutcome::EnterSearch,
        // `g a` (above) jumps to Accounts instead -- the pending-`g` branch always runs first
        // and returns before this match is reached, so the two never collide.
        "a" if !ctrl && !shift => return KeyOutcome::EnterInsert,
        "b" if !ctrl && !shift => return KeyOutcome::ToggleRail,
        _ => {}
    }

    if key == "tab" {
        return if shift {
            KeyOutcome::CycleFocusBackward
        } else {
            KeyOutcome::CycleFocusForward
        };
    }

    if key == "g" && !ctrl && !shift {
        return KeyOutcome::ArmPendingG;
    }

    let movement = match key {
        // Bare Shift-`g` (`G`), no pending prefix -- jump to last in the focused zone.
        "g" if shift => Some(Movement::Last),
        "j" | "down" => Some(Movement::Next),
        "k" | "up" => Some(Movement::Prev),
        "d" if ctrl => Some(Movement::HalfPageDown),
        "u" if ctrl => Some(Movement::HalfPageUp),
        "enter" => Some(Movement::Enter),
        _ => None,
    };

    match movement {
        Some(movement) => KeyOutcome::Movement(movement),
        None => KeyOutcome::NoOp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Escape
    #[test]
    fn escape_with_a_pending_g_clears_it_regardless_of_mode() {
        assert_eq!(
            route_key(InputMode::Normal, true, "escape", false, false),
            KeyOutcome::ClearPendingG
        );
    }

    #[test]
    fn escape_in_a_non_normal_mode_closes_popups_and_exits() {
        for mode in [
            InputMode::Command,
            InputMode::Insert,
            InputMode::Search,
            InputMode::Dialog,
        ] {
            assert_eq!(
                route_key(mode, false, "escape", false, false),
                KeyOutcome::ClosePopupsAndExitMode
            );
        }
    }

    #[test]
    fn escape_with_nothing_to_do_is_its_own_distinct_no_op() {
        assert_eq!(
            route_key(InputMode::Normal, false, "escape", false, false),
            KeyOutcome::EscapeNoOp
        );
    }

    // Mode gates
    #[test]
    fn command_mode_delegates_every_non_escape_key_to_the_palette() {
        assert_eq!(
            route_key(InputMode::Command, false, "j", false, false),
            KeyOutcome::DelegateToPalette
        );
    }

    #[test]
    fn insert_mode_swallows_every_non_escape_key() {
        assert_eq!(
            route_key(InputMode::Insert, false, "j", false, false),
            KeyOutcome::Swallowed
        );
    }

    #[test]
    fn search_mode_delegates_every_non_escape_key() {
        assert_eq!(
            route_key(InputMode::Search, false, "j", false, false),
            KeyOutcome::DelegateToSearch
        );
    }

    #[test]
    fn dialog_mode_delegates_every_non_escape_key() {
        assert_eq!(
            route_key(InputMode::Dialog, false, "j", false, false),
            KeyOutcome::DelegateToDialog
        );
    }

    // Pending g
    #[test]
    fn pending_g_then_g_jumps_to_first() {
        assert_eq!(
            route_key(InputMode::Normal, true, "g", false, false),
            KeyOutcome::Movement(Movement::First)
        );
    }

    #[test]
    fn pending_g_then_a_bound_key_jumps_to_its_noun() {
        assert_eq!(
            route_key(InputMode::Normal, true, "a", false, false),
            KeyOutcome::JumpToNoun(Noun::Accounts)
        );
    }

    #[test]
    fn pending_g_then_an_unbound_key_flashes_a_message_and_consumes_the_key() {
        assert_eq!(
            route_key(InputMode::Normal, true, "x", false, false),
            KeyOutcome::PendingGUnbound("g x is not a jump".to_string())
        );
    }

    #[test]
    fn pending_g_beats_mode_entry_keys_the_ordering_bug_this_replaces() {
        // The exact regression #150 fixed: `g a` must reach Accounts, not bare `a`'s
        // Insert-mode arm -- this is only true if the pending-`g` tier is checked first.
        assert_eq!(
            route_key(InputMode::Normal, true, "a", false, false),
            KeyOutcome::JumpToNoun(Noun::Accounts)
        );
        assert_eq!(
            route_key(InputMode::Normal, false, "a", false, false),
            KeyOutcome::EnterInsert
        );
    }

    // Mode entry
    #[test]
    fn colon_enters_command_mode() {
        assert_eq!(
            route_key(InputMode::Normal, false, ":", false, false),
            KeyOutcome::EnterCommand
        );
    }

    #[test]
    fn slash_enters_search_mode() {
        assert_eq!(
            route_key(InputMode::Normal, false, "/", false, false),
            KeyOutcome::EnterSearch
        );
    }

    #[test]
    fn bare_a_enters_insert_mode() {
        assert_eq!(
            route_key(InputMode::Normal, false, "a", false, false),
            KeyOutcome::EnterInsert
        );
    }

    #[test]
    fn bare_b_toggles_the_rail() {
        assert_eq!(
            route_key(InputMode::Normal, false, "b", false, false),
            KeyOutcome::ToggleRail
        );
    }

    // Tab
    #[test]
    fn tab_cycles_focus_forward_and_shift_tab_backward() {
        assert_eq!(
            route_key(InputMode::Normal, false, "tab", false, false),
            KeyOutcome::CycleFocusForward
        );
        assert_eq!(
            route_key(InputMode::Normal, false, "tab", false, true),
            KeyOutcome::CycleFocusBackward
        );
    }

    // Arming g
    #[test]
    fn bare_g_arms_a_pending_chord() {
        assert_eq!(
            route_key(InputMode::Normal, false, "g", false, false),
            KeyOutcome::ArmPendingG
        );
    }

    // Movement
    #[test]
    fn shift_g_with_no_pending_chord_jumps_to_last() {
        assert_eq!(
            route_key(InputMode::Normal, false, "g", false, true),
            KeyOutcome::Movement(Movement::Last)
        );
    }

    #[test]
    fn movement_keys_map_to_their_own_movement() {
        let cases = [
            ("j", false, Movement::Next),
            ("down", false, Movement::Next),
            ("k", false, Movement::Prev),
            ("up", false, Movement::Prev),
            ("enter", false, Movement::Enter),
        ];
        for (key, ctrl, movement) in cases {
            assert_eq!(
                route_key(InputMode::Normal, false, key, ctrl, false),
                KeyOutcome::Movement(movement)
            );
        }
    }

    #[test]
    fn ctrl_d_and_ctrl_u_are_half_page_movements() {
        assert_eq!(
            route_key(InputMode::Normal, false, "d", true, false),
            KeyOutcome::Movement(Movement::HalfPageDown)
        );
        assert_eq!(
            route_key(InputMode::Normal, false, "u", true, false),
            KeyOutcome::Movement(Movement::HalfPageUp)
        );
    }

    #[test]
    fn an_unrecognised_key_is_a_plain_no_op() {
        assert_eq!(
            route_key(InputMode::Normal, false, "z", false, false),
            KeyOutcome::NoOp
        );
    }
}
