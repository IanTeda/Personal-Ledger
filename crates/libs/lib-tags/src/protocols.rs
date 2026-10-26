//! The ports Tags reach storage and the other domains through, in the shape of `lib-accounts'`.
//! They are synchronous (ADR-0046): an adapter serves reads from memory and queues each write, so a
//! write never fails here; a failed save is the adapter's to report. `fake` implements them in
//! memory.
//!
//! `lib-database`'s Tags adapter implements the repository and the usage (#861, #862). Remove and
//! merge rewrite other domains' Splits, so they are no single-domain Interactor but the reassign
//! port ([`TagReassign`], Q6), which a Client answers through its Ledger until `lib-reassign`
//! (#813); a Presenter's confirm is lent it rather than the Presenter holding it, since the Desktop
//! writes its Ledger only with the gpui context that confirm runs in. The Presenters reach the
//! repository and the usage together, through [`TagsPorts`], and ask for navigation through
//! [`TagsRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::{TagId, Total};
use lib_toast::Notify;

use crate::{Tag, TagError};

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

/// Every Tags read and the repository write, as one value the Presenters share. Reads answer
/// owned values, so a store behind a `RefCell` can implement it by borrowing for each call.
pub trait TagsPorts: TagRepository + TagUsage {}

impl<T: TagRepository + TagUsage> TagsPorts for T {}

/// The ports as the page and its Dialogs hold them: one set, shared, so a Tag a Dialog adds is on
/// the page as soon as the Dialog closes. Single-threaded, as both Clients' UI threads are.
pub type SharedPorts = Rc<RefCell<dyn TagsPorts>>;

/// What a Tag remove did, for its Toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRemoved {
    /// The Tag's name, read before it went.
    pub name: String,
    /// The Transactions whose Splits no longer carry it.
    pub untagged: usize,
}

/// What a Tag merge did, for its Toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagMerged {
    /// The names of the two Tags, read before the source went.
    pub source_name: String,
    pub target_name: String,
    /// The Transactions that carried the source and so now carry the target.
    pub retagged: usize,
}

/// Tag remove and merge (Q6): each rewrites the Splits that carry the Tag, in the one commit of the
/// input, so neither is a `lib-tags` Interactor. The Remove and Merge Dialogs' Presenters confirm
/// through it, the pattern Categories set (`CategoryReassign`), and the port `lib-reassign` (#813)
/// answers.
pub trait TagReassign {
    /// Untags every Split carrying Tag `id` and deletes it, used or not.
    fn remove_tag(&mut self, id: TagId) -> Result<TagRemoved, TagError>;

    /// Folds `source` into `target`: each Split carrying the source carries the target instead
    /// (once, if it already had it), then the source is deleted. The target keeps its name, colour
    /// and Active flag.
    fn merge_tags(&mut self, source: TagId, target: TagId) -> Result<TagMerged, TagError>;
}

/// A Tags Dialog, as the page asks the Router to open it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagsDialog {
    AddTag,
    EditTag(TagId),
    RemoveTag(TagId),
    /// Either select is empty until chosen: the palette's `tags merge` opens with neither, the
    /// page's merge on a flagged Tag with both, on any other Tag with only the source.
    MergeTags {
        source: Option<TagId>,
        target: Option<TagId>,
    },
}

/// What a Dialog did to a Tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagChange {
    /// Added or edited.
    Saved,
    Removed,
    Merged,
}

/// What a confirmed Tags Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagsOutcome {
    pub change: TagChange,
    /// The Tag to select: the one saved, or the merge's target. A removed Tag's own id.
    pub id: TagId,
    /// The Tag's name as written, read before a remove takes it.
    pub name: String,
}

/// Navigation out of the Tags Presenters, implemented by each Client. The Client passes it into
/// each `handle` call rather than the Presenter holding it, since the Desktop can only open a
/// Dialog with the gpui context that call runs in. A Toast or status-line message goes to the
/// Shell through [`Notify`] (ADR-0048).
pub trait TagsRouter: Notify {
    /// Builds `dialog`'s Presenter and hosts its View. Returns whether it opened: a Dialog on a
    /// Tag removed since it was asked for has no Presenter to build.
    fn open_dialog(&mut self, dialog: TagsDialog) -> bool;

    /// Closes the open Tags Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`TagsPageEvent::DialogClosed`].
    ///
    /// [`TagsPageEvent::DialogClosed`]: crate::presenter::TagsPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<TagsOutcome>);
}
