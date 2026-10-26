//! The ports Tags reach storage and the other domains through, in the shape of `lib-accounts'`.
//! They are synchronous (ADR-0046): an adapter serves reads from memory and queues each write, so a
//! write never fails here; a failed save is the adapter's to report. `fake` implements them in
//! memory.
//!
//! Delete and merge are not here: they rewrite Splits, so they stay `lib-ledger`'s (Q6). Nothing
//! persists Tags yet, so only the fake answers the repository, and `lib-ledger` answers
//! [`TagUsage`] from its Transactions.

use chrono::NaiveDate;
use lib_core::{TagId, Total};

use crate::Tag;

/// The Tags on the Ledger.
pub trait TagRepository {
    /// Every Tag, in the order they were added.
    fn tags(&self) -> Vec<Tag>;

    fn tag(&self, id: TagId) -> Option<Tag> {
        self.tags().into_iter().find(|tag| tag.id == id)
    }

    /// Writes `tag` over the row with its id, or as a new row after every row already there.
    fn upsert_tag(&mut self, tag: Tag);
}

/// A Tag's live figures for the TRANSACTIONS, TOTAL and LAST USED columns.
#[derive(Debug, Clone, PartialEq)]
pub struct TagUse {
    /// Distinct Transactions: two Splits on one Transaction count once.
    pub transactions: usize,
    /// The sum of the Splits carrying the Tag, when they share one Unit; [`Total::Mixed`] when
    /// they don't, [`Total::Empty`] for an unused Tag.
    pub total: Total,
    /// The newest carrying Transaction's date, future-dated ones included.
    pub last_used: Option<NaiveDate>,
}

impl Default for TagUse {
    /// A Tag nothing carries.
    fn default() -> Self {
        Self {
            transactions: 0,
            total: Total::Empty,
            last_used: None,
        }
    }
}

/// The read the Tags page, its Dialogs and the sort count a Tag's Transactions through, so
/// `lib-tags` needn't depend on the Transactions or Accounts crates. Only figures the page shows,
/// so a read that can't be made is the answerer's to report, not a refusal here.
pub trait TagUsage {
    /// What carries Tag `id`; a Tag nothing carries, or one that doesn't exist, has none.
    fn tag_usage(&self, id: TagId) -> TagUse;
}
