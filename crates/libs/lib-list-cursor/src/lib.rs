//! The List Cursor shared by the Clients: moving a selection through a list of rows and keeping
//! it on a row that still exists.
//!
//! Its own crate rather than a domain crate or `lib-core`: it is a UI rule, not a domain one, and
//! every list in every domain needs it, so housing it in one domain (it used to be
//! `lib_accounts::step_selection`) made the other domains depend on Accounts for it. Like
//! `lib-toast`, it is pure and `gpui`-free so the TUI can adopt it as well.
//!
//! Positions are `usize` row indexes; ids are whatever the caller keys rows by. Every function
//! clamps rather than wraps, and an empty list never panics.

/// `Ctrl-d`/`Ctrl-u`: rows moved by a half page in a list that does not measure its viewport.
pub const HALF_PAGE: usize = 5;

/// One step through a list. The Desktop parses keys into its own `Movement`, which also carries
/// `Enter`; a cursor has nothing to do for `Enter`, so it has no variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Next,
    Prev,
    First,
    Last,
    HalfPageDown,
    HalfPageUp,
}

/// Moves position `current` by `step` in a list of `len` rows, a half page being [`HALF_PAGE`].
/// An empty list always gives `0`.
pub fn step(current: usize, len: usize, step: Step) -> usize {
    step_by(current, len, step, HALF_PAGE)
}

/// [`step`] with the caller's own half page, for a list that sizes it from its viewport.
pub fn step_by(current: usize, len: usize, step: Step, half_page: usize) -> usize {
    let Some(last) = len.checked_sub(1) else {
        return 0;
    };
    match step {
        Step::Next => current.saturating_add(1).min(last),
        Step::Prev => current.saturating_sub(1).min(last),
        Step::First => 0,
        Step::Last => last,
        Step::HalfPageDown => current.saturating_add(half_page).min(last),
        Step::HalfPageUp => current.saturating_sub(half_page).min(last),
    }
}

/// Moves the selected id by `step` through `ids`. With nothing selected, or a selected id no
/// longer in `ids`, `Last` lands on the last row and every other step on the first, so the first
/// press always shows a cursor. `None` only for an empty list.
pub fn step_id<T: Copy + PartialEq>(ids: &[T], selected: Option<T>, step: Step) -> Option<T> {
    let last = ids.len().checked_sub(1)?;
    let position = match selected.and_then(|id| ids.iter().position(|row| *row == id)) {
        Some(current) => self::step(current, ids.len(), step),
        None if step == Step::Last => last,
        None => 0,
    };
    ids.get(position).copied()
}

/// The selection to show: `selected` while it is still in `ids`, else the first row, so a deleted
/// row never leaves the list with nothing under the cursor. `None` only for an empty list.
pub fn resolve<T: PartialEq>(ids: impl IntoIterator<Item = T>, selected: Option<T>) -> Option<T> {
    let mut first = None;
    for id in ids {
        if selected.as_ref() == Some(&id) {
            return Some(id);
        }
        first.get_or_insert(id);
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Step; 6] = [
        Step::Next,
        Step::Prev,
        Step::First,
        Step::Last,
        Step::HalfPageDown,
        Step::HalfPageUp,
    ];

    #[test]
    fn each_step_moves_from_the_middle() {
        assert_eq!(step(6, 20, Step::Next), 7);
        assert_eq!(step(6, 20, Step::Prev), 5);
        assert_eq!(step(6, 20, Step::First), 0);
        assert_eq!(step(6, 20, Step::Last), 19);
        assert_eq!(step(6, 20, Step::HalfPageDown), 6 + HALF_PAGE);
        assert_eq!(step(6, 20, Step::HalfPageUp), 6 - HALF_PAGE);
    }

    #[test]
    fn steps_clamp_at_both_ends() {
        assert_eq!(step(0, 7, Step::Prev), 0);
        assert_eq!(step(0, 7, Step::HalfPageUp), 0);
        assert_eq!(step(6, 7, Step::Next), 6);
        assert_eq!(step(4, 7, Step::HalfPageDown), 6);
    }

    #[test]
    fn a_position_past_the_end_is_pulled_back_into_the_list() {
        assert_eq!(step(10, 7, Step::Prev), 6);
        assert_eq!(step(10, 7, Step::HalfPageUp), 5);
        assert_eq!(step(10, 7, Step::Next), 6);
    }

    #[test]
    fn an_empty_list_always_gives_zero() {
        for each in ALL {
            assert_eq!(step(3, 0, each), 0, "{each:?}");
        }
    }

    #[test]
    fn step_by_uses_the_callers_half_page() {
        assert_eq!(step_by(10, 40, Step::HalfPageDown, 12), 22);
        assert_eq!(step_by(10, 40, Step::HalfPageUp, 12), 0);
        assert_eq!(step_by(10, 40, Step::Next, 12), 11);
    }

    #[test]
    fn step_id_moves_through_the_ids() {
        let ids = [30, 10, 20];
        assert_eq!(step_id(&ids, Some(10), Step::Next), Some(20));
        assert_eq!(step_id(&ids, Some(10), Step::Prev), Some(30));
        assert_eq!(step_id(&ids, Some(20), Step::Next), Some(20));
        assert_eq!(step_id(&ids, Some(20), Step::First), Some(30));
    }

    #[test]
    fn step_id_with_no_selection_lands_on_the_first_row_or_the_last() {
        let ids = [30, 10, 20];
        for each in ALL {
            let expected = if each == Step::Last { 20 } else { 30 };
            assert_eq!(step_id(&ids, None, each), Some(expected), "{each:?}");
        }
    }

    #[test]
    fn step_id_treats_a_vanished_id_as_no_selection() {
        assert_eq!(step_id(&[30, 10, 20], Some(99), Step::Next), Some(30));
    }

    #[test]
    fn step_id_on_an_empty_list_selects_nothing() {
        for each in ALL {
            assert_eq!(step_id::<u32>(&[], Some(1), each), None, "{each:?}");
        }
    }

    #[test]
    fn resolve_keeps_an_id_that_still_exists() {
        assert_eq!(resolve([30, 10, 20], Some(10)), Some(10));
    }

    #[test]
    fn resolve_falls_back_to_the_first_row_when_the_id_disappears() {
        assert_eq!(resolve([30, 10, 20], Some(99)), Some(30));
        assert_eq!(resolve([30, 10, 20], None), Some(30));
    }

    #[test]
    fn resolve_on_an_empty_list_selects_nothing() {
        assert_eq!(resolve(Vec::<u32>::new(), Some(1)), None);
    }
}
