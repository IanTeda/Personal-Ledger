use std::collections::VecDeque;

use crate::ToastKind;

/// Entries kept in the session Toast history before the oldest is dropped (#339).
pub const HISTORY_LIMIT: usize = 100;

/// One past Toast. A merged duplicate stays one entry, carrying the time it was last raised and
/// its `×N` count. `S` is the wall-clock stamp the bin passes in, so this crate reads no clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry<S> {
    id: u64,
    kind: ToastKind,
    text: String,
    count: u32,
    last_raised: S,
}

impl<S> HistoryEntry<S> {
    pub fn kind(&self) -> ToastKind {
        self.kind
    }

    /// The full Message text; the history is where a truncated Toast can be read in full.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn last_raised(&self) -> &S {
        &self.last_raised
    }
}

/// The session Toast history: every Toast raised this run, whatever the Toasts Preference,
/// held in memory only and never cleared.
#[derive(Debug, Clone)]
pub struct History<S> {
    // Newest at the front, the order the history popup lists them in.
    entries: VecDeque<HistoryEntry<S>>,
    next_id: u64,
}

impl<S> Default for History<S> {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            next_id: 0,
        }
    }
}

impl<S> History<S> {
    /// Newest first.
    pub fn iter(&self) -> impl Iterator<Item = &HistoryEntry<S>> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Adds a new entry and returns its id, dropping the oldest past [`HISTORY_LIMIT`].
    pub(crate) fn record(&mut self, kind: ToastKind, text: String, count: u32, stamp: S) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push_front(HistoryEntry {
            id,
            kind,
            text,
            count,
            last_raised: stamp,
        });
        self.entries.truncate(HISTORY_LIMIT);
        id
    }

    /// Updates the entry for a merged duplicate and moves it to the front. Hands the stamp back
    /// when the entry has already dropped off the end, so the caller records a fresh one.
    pub(crate) fn bump(&mut self, id: u64, count: u32, stamp: S) -> Result<(), S> {
        let Some(mut entry) = self
            .entries
            .iter()
            .position(|e| e.id == id)
            .and_then(|i| self.entries.remove(i))
        else {
            return Err(stamp);
        };
        entry.count = count;
        entry.last_raised = stamp;
        self.entries.push_front(entry);
        Ok(())
    }
}
