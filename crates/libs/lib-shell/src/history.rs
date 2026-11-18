//! History: what Undo and Redo step through (#1238). It is generic over the reversal payload `R`,
//! since `lib-database` depends on `lib-shell` and so the Shell can't name what reverses a Ledger
//! change; the Client's root Router reverses and replays it ([`crate::ShellRouter::reverse`],
//! [`crate::ShellRouter::replay`]).
//!
//! Two lists: `undo`, newest first and capped at [`DEPTH`], and `redo`. Undo moves an entry to
//! `redo` and Redo moves it back, so neither ever loses one; a new entry clears `redo`, since what
//! was undone no longer follows on from it. A step whose reversal fails leaves the entry where it
//! was, for the person to try again.

use std::collections::VecDeque;

/// How many entries Undo reaches back through; a new entry past it drops the oldest.
pub const DEPTH: usize = 50;

/// One input's change: what the person is told was undone or redone, and what reverses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<R> {
    pub label: String,
    pub reversal: R,
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

/// The undo and redo lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History<R> {
    /// Newest first.
    undo: VecDeque<Entry<R>>,
    /// The entry undone last first.
    redo: Vec<Entry<R>>,
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
    /// Records a new change, the newest to undo. It clears Redo, and past [`DEPTH`] drops the
    /// oldest entry.
    pub fn push(&mut self, entry: Entry<R>) {
        self.redo.clear();
        self.undo.push_front(entry);
        self.undo.truncate(DEPTH);
    }

    /// Undoes the newest entry through `reverse`, which says whether it applied: one that did
    /// moves to Redo, one that didn't stays newest.
    pub fn undo(&mut self, reverse: impl FnOnce(&R) -> bool) -> Stepped {
        let Some(entry) = self.undo.pop_front() else {
            return Stepped::Nothing;
        };
        let label = entry.label.clone();
        if reverse(&entry.reversal) {
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
        let label = entry.label.clone();
        if replay(&entry.reversal) {
            self.undo.push_front(entry);
            Stepped::Done(label)
        } else {
            self.redo.push(entry);
            Stepped::Failed(label)
        }
    }

    /// The labels Undo would step through, newest first.
    pub fn undo_labels(&self) -> impl Iterator<Item = &str> {
        self.undo.iter().map(|entry| entry.label.as_str())
    }

    /// The labels Redo would step through, the next to redo first.
    pub fn redo_labels(&self) -> impl Iterator<Item = &str> {
        self.redo.iter().rev().map(|entry| entry.label.as_str())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(label: &str) -> Entry<u32> {
        Entry {
            label: label.to_string(),
            reversal: 0,
        }
    }

    fn history(labels: &[&str]) -> History<u32> {
        let mut history = History::default();
        for label in labels {
            history.push(entry(label));
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
        history.push(Entry {
            label: "first".to_string(),
            reversal: 1,
        });
        history.push(Entry {
            label: "second".to_string(),
            reversal: 2,
        });
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

        history.push(entry("third"));

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
}
