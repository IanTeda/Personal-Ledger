//! History: what Undo and Redo step through (#1238). It is generic over the reversal payload `R`,
//! since `lib-database` depends on `lib-shell` and so the Shell can't name what reverses a Ledger
//! change; the Client's root Router steps it either way ([`crate::ShellRouter::step`], [`Step`]).
//!
//! Two lists: `undo`, newest first and capped at [`DEPTH`], and `redo`. Undo moves an entry to
//! `redo` and Redo moves it back, so neither ever loses one; a new entry clears `redo`, since what
//! was undone no longer follows on from it. A step whose reversal fails leaves the entry where it
//! was, for the person to try again. A multi-step Undo or Redo ([`History::undo_through`]) stops
//! at the first step that fails, keeping the ones already stepped.
//!
//! Each entry keeps the moment it was recorded, which the header's dropdown shows as a relative
//! time ([`HistoryViewModel`]).

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use lib_locale::msg;

/// How many entries Undo reaches back through; a new entry past it drops the oldest.
pub const DEPTH: usize = 50;

/// Which way History steps an entry, the one direction every layer below the Shell takes too:
/// Undo puts back what the entry changed, the newest change first, and Redo makes it again, in
/// the order first made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Undo,
    Redo,
}

/// One input's change: what the person is told was undone or redone, and what reverses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<R> {
    pub label: String,
    pub reversal: R,
}

/// An entry with the moment it was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamped<R> {
    entry: Entry<R>,
    at: DateTime<Utc>,
}

/// What an Undo or Redo came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stepped {
    /// The list was empty: nothing changed.
    Nothing,
    /// The entry labelled so was reversed or replayed, and moved to the other list.
    Done(String),
    /// The entry labelled so failed to reverse or replay, and stays where it was.
    Failed(String),
}

/// What a multi-step Undo or Redo came to: the labels stepped through, in the order stepped, and
/// the label of the step that failed and stopped it, if one did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Steps {
    pub done: Vec<String>,
    pub failed: Option<String>,
}

/// The undo and redo lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History<R> {
    /// Newest first.
    undo: VecDeque<Stamped<R>>,
    /// The entry undone last at the end.
    redo: Vec<Stamped<R>>,
}

impl<R> Default for History<R> {
    fn default() -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
        }
    }
}

impl<R> History<R> {
    /// Records a new change made `at`, the newest to undo. It clears Redo, and past [`DEPTH`]
    /// drops the oldest entry.
    pub fn push(&mut self, entry: Entry<R>, at: DateTime<Utc>) {
        self.redo.clear();
        self.undo.push_front(Stamped { entry, at });
        self.undo.truncate(DEPTH);
    }

    /// Undoes the newest entry through `reverse`, which says whether it applied: one that did
    /// moves to Redo, one that didn't stays newest.
    pub fn undo(&mut self, reverse: impl FnOnce(&R) -> bool) -> Stepped {
        let Some(entry) = self.undo.pop_front() else {
            return Stepped::Nothing;
        };
        let label = entry.entry.label.clone();
        if reverse(&entry.entry.reversal) {
            self.redo.push(entry);
            Stepped::Done(label)
        } else {
            self.undo.push_front(entry);
            Stepped::Failed(label)
        }
    }

    /// Redoes the entry undone last through `replay`, which says whether it applied: one that did
    /// moves back to Undo, one that didn't stays next to redo.
    pub fn redo(&mut self, replay: impl FnOnce(&R) -> bool) -> Stepped {
        let Some(entry) = self.redo.pop() else {
            return Stepped::Nothing;
        };
        let label = entry.entry.label.clone();
        if replay(&entry.entry.reversal) {
            self.undo.push_front(entry);
            Stepped::Done(label)
        } else {
            self.redo.push(entry);
            Stepped::Failed(label)
        }
    }

    /// Undoes up to `count` entries, newest first, as one Undo each; it stops at the first that
    /// fails, or when nothing is left to undo.
    pub fn undo_through(&mut self, count: usize, mut reverse: impl FnMut(&R) -> bool) -> Steps {
        Self::through(count, || self.undo(&mut reverse))
    }

    /// Redoes up to `count` entries, the next to redo first, as one Redo each; it stops at the
    /// first that fails, or when nothing is left to redo.
    pub fn redo_through(&mut self, count: usize, mut replay: impl FnMut(&R) -> bool) -> Steps {
        Self::through(count, || self.redo(&mut replay))
    }

    fn through(count: usize, mut step: impl FnMut() -> Stepped) -> Steps {
        let mut steps = Steps::default();
        for _ in 0..count {
            match step() {
                Stepped::Nothing => break,
                Stepped::Done(label) => steps.done.push(label),
                Stepped::Failed(label) => {
                    steps.failed = Some(label);
                    break;
                }
            }
        }
        steps
    }

    /// Keeps what `keep` leaves of each entry on both lists: it trims an entry's reversal in place
    /// and says whether anything is left to step, an entry with nothing left being removed. A
    /// closed Ledger's part leaves this way (#1247).
    pub fn retain(&mut self, mut keep: impl FnMut(&mut R) -> bool) {
        self.undo
            .retain_mut(|stamped| keep(&mut stamped.entry.reversal));
        self.redo
            .retain_mut(|stamped| keep(&mut stamped.entry.reversal));
    }

    /// The labels Undo would step through, newest first.
    pub fn undo_labels(&self) -> impl Iterator<Item = &str> {
        self.undo.iter().map(|entry| entry.entry.label.as_str())
    }

    /// The labels Redo would step through, the next to redo first.
    pub fn redo_labels(&self) -> impl Iterator<Item = &str> {
        self.redo
            .iter()
            .rev()
            .map(|entry| entry.entry.label.as_str())
    }

    /// What the header's dropdown draws, its relative times measured back from `now`.
    pub fn view_model(&self, now: DateTime<Utc>) -> HistoryViewModel {
        let row = |steps: usize, stamped: &Stamped<R>| HistoryRow {
            label: stamped.entry.label.clone(),
            when: relative_time(stamped.at, now),
            steps,
        };
        let mut redo: Vec<HistoryRow> = self
            .redo
            .iter()
            .rev()
            .take(REDO_ROWS)
            .enumerate()
            .map(|(index, stamped)| row(index + 1, stamped))
            .collect();
        redo.reverse();
        HistoryViewModel {
            redo,
            undo: self
                .undo
                .iter()
                .take(UNDO_ROWS)
                .enumerate()
                .map(|(index, stamped)| row(index + 1, stamped))
                .collect(),
            can_undo: self.can_undo(),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

/// How many entries to redo the dropdown lists above NOW.
pub const REDO_ROWS: usize = 3;

/// How many entries to undo the dropdown lists below NOW.
pub const UNDO_ROWS: usize = 6;

/// The header's Undo dropdown: one timeline, the entries to redo above NOW and those to undo below.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryViewModel {
    /// Up to [`REDO_ROWS`] entries to redo, the furthest from now on top.
    pub redo: Vec<HistoryRow>,
    /// Up to [`UNDO_ROWS`] entries to undo, newest first.
    pub undo: Vec<HistoryRow>,
    /// Whether Undo has anything to step; the header draws it faint and inert when not.
    pub can_undo: bool,
}

/// One row of the dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub label: String,
    /// When it was recorded, relative to now ("just now", "3 min ago").
    pub when: String,
    /// How many entries a click on the row steps through, it included.
    pub steps: usize,
}

/// `at` as a time back from `now`, in the largest whole unit; a clock that reads earlier than `at`
/// is "just now".
fn relative_time(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let ago = now.signed_duration_since(at);
    if ago.num_minutes() < 1 {
        msg::history_just_now()
    } else if ago.num_hours() < 1 {
        msg::history_minutes_ago(ago.num_minutes())
    } else if ago.num_days() < 1 {
        msg::history_hours_ago(ago.num_hours())
    } else {
        msg::history_days_ago(ago.num_days())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minute: u32) -> DateTime<Utc> {
        chrono::NaiveDate::from_ymd_opt(2026, 6, 15)
            .and_then(|day| day.and_hms_opt(12, minute, 0))
            .map(|moment| moment.and_utc())
            .expect("a valid moment")
    }

    fn entry(label: &str) -> Entry<u32> {
        Entry {
            label: label.to_string(),
            reversal: 0,
        }
    }

    fn history(labels: &[&str]) -> History<u32> {
        let mut history = History::default();
        for label in labels {
            history.push(entry(label), at(0));
        }
        history
    }

    fn undo_labels(history: &History<u32>) -> Vec<&str> {
        history.undo_labels().collect()
    }

    fn redo_labels(history: &History<u32>) -> Vec<&str> {
        history.redo_labels().collect()
    }

    #[test]
    fn an_empty_history_has_nothing_to_undo_or_redo() {
        let mut history = History::<u32>::default();

        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(
            history.undo(|_| panic!("nothing to reverse")),
            Stepped::Nothing
        );
        assert_eq!(
            history.redo(|_| panic!("nothing to replay")),
            Stepped::Nothing
        );
    }

    #[test]
    fn a_pushed_entry_is_the_newest_to_undo() {
        let history = history(&["Rename Groceries", "Set Locale"]);

        assert_eq!(undo_labels(&history), ["Set Locale", "Rename Groceries"]);
        assert!(history.can_undo());
    }

    #[test]
    fn past_fifty_the_oldest_entry_is_dropped() {
        let labels: Vec<String> = (1..=51).map(|n| format!("change {n}")).collect();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();

        let history = history(&refs);

        let kept = undo_labels(&history);
        assert_eq!(kept.len(), 50);
        assert_eq!(kept.first(), Some(&"change 51"));
        assert_eq!(kept.last(), Some(&"change 2"), "change 1 dropped");
    }

    #[test]
    fn undo_reverses_the_newest_and_moves_it_to_redo() {
        let mut history = History::default();
        history.push(
            Entry {
                label: "first".to_string(),
                reversal: 1,
            },
            at(0),
        );
        history.push(
            Entry {
                label: "second".to_string(),
                reversal: 2,
            },
            at(0),
        );
        let mut reversed = None;

        let stepped = history.undo(|reversal| {
            reversed = Some(*reversal);
            true
        });

        assert_eq!(stepped, Stepped::Done("second".to_string()));
        assert_eq!(reversed, Some(2));
        assert_eq!(undo_labels(&history), ["first"]);
        assert_eq!(redo_labels(&history), ["second"]);
    }

    #[test]
    fn redo_replays_the_last_undone_and_moves_it_back() {
        let mut history = history(&["first", "second"]);
        history.undo(|_| true);
        history.undo(|_| true);
        assert_eq!(redo_labels(&history), ["first", "second"]);

        let stepped = history.redo(|_| true);

        assert_eq!(stepped, Stepped::Done("first".to_string()));
        assert_eq!(undo_labels(&history), ["first"]);
        assert_eq!(redo_labels(&history), ["second"]);
    }

    #[test]
    fn a_new_entry_clears_redo() {
        let mut history = history(&["first", "second"]);
        history.undo(|_| true);

        history.push(entry("third"), at(0));

        assert!(!history.can_redo());
        assert_eq!(undo_labels(&history), ["third", "first"]);
    }

    #[test]
    fn a_failed_undo_leaves_the_entry_the_newest_to_undo() {
        let mut history = history(&["first", "second"]);

        let stepped = history.undo(|_| false);

        assert_eq!(stepped, Stepped::Failed("second".to_string()));
        assert_eq!(undo_labels(&history), ["second", "first"]);
        assert!(!history.can_redo());
    }

    #[test]
    fn a_failed_redo_leaves_the_entry_next_to_redo() {
        let mut history = history(&["first", "second"]);
        history.undo(|_| true);
        history.undo(|_| true);

        let stepped = history.redo(|_| false);

        assert_eq!(stepped, Stepped::Failed("first".to_string()));
        assert_eq!(redo_labels(&history), ["first", "second"]);
        assert!(!history.can_undo());
    }

    #[test]
    fn retain_trims_each_entry_on_both_lists_and_drops_those_left_empty() {
        let mut history = History::default();
        for (label, reversal) in [("first", 1), ("second", 2), ("third", 3)] {
            history.push(
                Entry {
                    label: label.to_string(),
                    reversal,
                },
                at(0),
            );
        }
        history.undo(|_| true);

        history.retain(|reversal| {
            *reversal -= 1;
            *reversal > 0
        });

        assert_eq!(undo_labels(&history), ["second"]);
        assert_eq!(redo_labels(&history), ["third"]);
        history.redo(|reversal| {
            assert_eq!(*reversal, 2, "trimmed in place");
            true
        });
    }

    #[test]
    fn undo_through_steps_back_that_many_in_one_go() {
        let mut history = history(&["first", "second", "third"]);
        let mut reversed = Vec::new();

        let steps = history.undo_through(3, |reversal| {
            reversed.push(*reversal);
            true
        });

        assert_eq!(
            steps,
            Steps {
                done: vec!["third".into(), "second".into(), "first".into()],
                failed: None,
            }
        );
        assert_eq!(reversed.len(), 3);
        assert!(!history.can_undo());
        assert_eq!(redo_labels(&history), ["first", "second", "third"]);
    }

    #[test]
    fn redo_through_replays_that_many_back_in_order() {
        let mut history = history(&["first", "second", "third"]);
        history.undo_through(3, |_| true);

        let steps = history.redo_through(2, |_| true);

        assert_eq!(steps.done, ["first", "second"]);
        assert_eq!(undo_labels(&history), ["second", "first"]);
        assert_eq!(redo_labels(&history), ["third"]);
    }

    #[test]
    fn a_multi_step_that_fails_partway_stops_there_keeping_what_it_stepped() {
        let mut history = history(&["first", "second", "third"]);
        let mut calls = 0;

        let steps = history.undo_through(3, |_| {
            calls += 1;
            calls < 2
        });

        assert_eq!(
            steps,
            Steps {
                done: vec!["third".into()],
                failed: Some("second".into()),
            }
        );
        assert_eq!(undo_labels(&history), ["second", "first"]);
        assert_eq!(redo_labels(&history), ["third"]);
    }

    #[test]
    fn a_multi_step_past_the_end_stops_at_the_last_entry() {
        let mut history = history(&["first"]);

        let steps = history.undo_through(4, |_| true);

        assert_eq!(steps.done, ["first"]);
        assert_eq!(steps.failed, None);
    }

    fn row(label: &str, when: &str, steps: usize) -> HistoryRow {
        HistoryRow {
            label: label.to_string(),
            when: when.to_string(),
            steps,
        }
    }

    #[test]
    fn the_view_model_lists_undo_newest_first_with_relative_times() {
        let mut history = History::default();
        history.push(entry("first"), at(0));
        history.push(entry("second"), at(57));
        history.push(entry("third"), at(59));

        let view = history.view_model(at(59) + chrono::Duration::seconds(30));

        assert_eq!(
            view,
            HistoryViewModel {
                redo: Vec::new(),
                undo: vec![
                    row("third", "just now", 1),
                    row("second", "2 min ago", 2),
                    row("first", "59 min ago", 3),
                ],
                can_undo: true,
            }
        );
    }

    #[test]
    fn hours_and_days_back_read_in_whole_units() {
        assert_eq!(
            relative_time(at(0), at(0) + chrono::Duration::minutes(130)),
            "2 h ago"
        );
        assert_eq!(
            relative_time(at(0), at(0) + chrono::Duration::hours(49)),
            "2 d ago"
        );
        assert_eq!(relative_time(at(5), at(0)), "just now", "a clock set back");
    }

    #[test]
    fn the_view_model_lists_up_to_three_to_redo_the_furthest_on_top() {
        let labels: Vec<String> = (1..=8).map(|n| format!("change {n}")).collect();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let mut history = history(&refs);
        history.undo_through(4, |_| true);

        let view = history.view_model(at(0));

        let redo: Vec<(&str, usize)> = view
            .redo
            .iter()
            .map(|row| (row.label.as_str(), row.steps))
            .collect();
        assert_eq!(redo, [("change 7", 3), ("change 6", 2), ("change 5", 1)]);
        let undo: Vec<&str> = view.undo.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(undo, ["change 4", "change 3", "change 2", "change 1"]);
    }

    #[test]
    fn the_view_model_lists_the_last_six_to_undo() {
        let labels: Vec<String> = (1..=8).map(|n| format!("change {n}")).collect();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();

        let view = history(&refs).view_model(at(0));

        assert_eq!(view.undo.len(), 6);
        assert_eq!(view.undo[0].label, "change 8");
        assert_eq!(view.undo[5].steps, 6);
    }

    #[test]
    fn an_empty_history_has_nothing_to_undo_in_its_view_model() {
        let view = History::<u32>::default().view_model(at(0));

        assert_eq!(view, HistoryViewModel::default());
        assert!(!view.can_undo);
    }
}
