//! The port Tag remove and merge write through, and the reads their Dialogs show.
//!
//! [`TagRemoval`] is the module's one atomic port: a Tag goes with the rewrite of every Split that
//! carried it, written together or not at all. It is synchronous (ADR-0046): an adapter serves its
//! reads from memory and queues every write in the batch every adapter shares, so the bin's one
//! commit after the call lands them as one transaction, and a failed save is the adapter's to
//! report. `fake` implements it in memory.
//!
//! It serves the reads the Interactors check as owned values, rather than an Interactor taking the
//! Tags and Transactions ports beside it: the adapter holds stores the Tags and Transactions
//! modules share, so an Interactor given them all would borrow one twice (#779).
//!
//! What the Dialogs only show, every Tag with how many Transactions carry it, they read through
//! [`TagRemovalReads`] once, when they open (Q7): it is modal, so nothing changes it under them.
//!
//! The Dialogs close through the shared `lib_viper::dialog::DialogRouter` with `lib-tags`'
//! `TagsOutcome`, as every Tags Dialog does, so the Tags page hears of a remove or merge as it
//! would of its own.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lib_core::TagId;
use lib_tags::{Tag, sorted_by_counts};
use lib_transactions::Transaction;

/// Everything a remove or a merge writes, as one write.
#[derive(Debug, Clone, PartialEq)]
pub struct TagRewrite {
    /// The Transactions whose Splits carried the Tag, each as it is now.
    pub transactions: Vec<Transaction>,
    /// The Tag deleted once no Split carries it: the removed Tag, or the merge's source.
    pub deleted: TagId,
}

/// The rows Tag remove and merge check, and the one write that lands either.
pub trait TagRemoval {
    /// Tag `id`, which must exist to be removed or merged.
    fn tag(&self, id: TagId) -> Option<Tag>;

    /// The Transactions with a Split carrying Tag `id`, in the store's order.
    fn transactions_carrying(&self, id: TagId) -> Vec<Transaction>;

    /// Writes `rewrite` as one write: the Transactions, then the Tag's delete, since a Split's Tag
    /// has no cascade. All land or none do.
    fn rewrite(&mut self, rewrite: TagRewrite);
}

/// Those of `transactions` with a Split carrying Tag `id`, in order: what every store answers
/// [`TagRemoval::transactions_carrying`] with.
pub fn carrying(transactions: Vec<Transaction>, id: TagId) -> Vec<Transaction> {
    transactions
        .into_iter()
        .filter(|transaction| {
            transaction
                .splits
                .iter()
                .any(|split| split.tag_ids.contains(&id))
        })
        .collect()
}

/// The removal port as the Remove tag and Merge tags Dialogs hold it, shared with the Client that
/// commits its writes. Single-threaded, as both Clients' UI threads are.
pub type SharedTagRemoval = Rc<RefCell<dyn TagRemoval>>;

/// A Tag as the Dialogs offer it, with the distinct Transactions carrying it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagCount {
    pub tag: Tag,
    pub transactions: usize,
}

/// What the Remove tag and Merge tags Dialogs show, read once when they open.
pub trait TagRemovalReads {
    /// Every Tag with its count, most-used first, ties A–Z ignoring case: the Merge selects'
    /// order.
    fn tags_by_usage(&self) -> Vec<TagCount>;
}

/// `tags` with their counts from `counts`, in usage order: what every reads answers
/// [`TagRemovalReads::tags_by_usage`] with. A Tag `counts` doesn't name has none.
pub fn by_usage(tags: &[Tag], counts: &HashMap<TagId, usize>) -> Vec<TagCount> {
    let counts: HashMap<TagId, usize> = tags
        .iter()
        .map(|tag| (tag.id, counts.get(&tag.id).copied().unwrap_or(0)))
        .collect();
    sorted_by_counts(tags, &counts)
        .into_iter()
        .map(|tag| TagCount {
            tag: tag.clone(),
            transactions: counts.get(&tag.id).copied().unwrap_or(0),
        })
        .collect()
}

/// What a remove did, for its Toast and the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRemoved {
    pub id: TagId,
    /// The Tag's name, read before it went.
    pub name: String,
    /// The Transactions whose Splits no longer carry it.
    pub untagged: usize,
}

/// What a merge did, for its Toast and the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagMerged {
    pub source: TagId,
    pub target: TagId,
    /// The names of the two Tags, read before the source went.
    pub source_name: String,
    pub target_name: String,
    /// The Transactions that carried the source and so now carry the target.
    pub retagged: usize,
}
