//! In-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly. [`RecordingRouter`] is a Router that only
//! records what it was asked.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lib_core::TagId;
use lib_toast::{Notice, Notify};

use crate::protocols::{
    SharedPorts, TagMerged, TagReassign, TagRemoved, TagRepository, TagUsage, TagUse, TagsDialog,
    TagsOutcome, TagsRouter,
};
use crate::{Tag, TagError, default_tags};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryTags {
    pub tags: Vec<Tag>,
}

impl InMemoryTags {
    /// The stub seed: the stub Ledger's Tags (see [`default_tags`]). Ids match `lib-ledger`'s
    /// stub, so its Splits still resolve their Tags.
    pub fn stub() -> Self {
        Self {
            tags: default_tags(),
        }
    }
}

impl TagRepository for InMemoryTags {
    fn tags(&self) -> Vec<Tag> {
        self.tags.clone()
    }

    fn upsert_tag(&mut self, tag: Tag) {
        match self.tags.iter_mut().find(|row| row.id == tag.id) {
            Some(row) => *row = tag,
            None => self.tags.push(tag),
        }
    }
}

/// Usage figures a test sets per Tag; a Tag not listed has none.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryTagUsage {
    pub uses: HashMap<TagId, TagUse>,
}

impl TagUsage for InMemoryTagUsage {
    fn tag_usage(&self, id: TagId) -> TagUse {
        self.uses.get(&id).cloned().unwrap_or_default()
    }
}

/// Every Tags port in memory, together, so a test can hand a write the ones it takes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryPorts {
    pub tags: InMemoryTags,
    pub usage: InMemoryTagUsage,
}

impl InMemoryPorts {
    /// The stub Tags with nothing carrying any of them.
    pub fn stub() -> Self {
        Self {
            tags: InMemoryTags::stub(),
            usage: InMemoryTagUsage::default(),
        }
    }
}

impl InMemoryPorts {
    /// These ports shared, as the Presenters hold them, with a handle a test reads them back
    /// through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl TagRepository for InMemoryPorts {
    fn tags(&self) -> Vec<Tag> {
        self.tags.tags()
    }

    fn upsert_tag(&mut self, tag: Tag) {
        self.tags.upsert_tag(tag);
    }
}

impl TagUsage for InMemoryPorts {
    fn tag_usage(&self, id: TagId) -> TagUse {
        self.usage.tag_usage(id)
    }
}

/// The reassign port over in-memory ports: it deletes the Tag, and counts the Transactions it
/// reports from the usage the test set. It rewrites no Splits, since none are held here.
#[derive(Debug, Clone)]
pub struct InMemoryReassign {
    pub ports: Rc<RefCell<InMemoryPorts>>,
}

impl InMemoryReassign {
    pub fn new(ports: Rc<RefCell<InMemoryPorts>>) -> Self {
        Self { ports }
    }
}

impl TagReassign for InMemoryReassign {
    fn remove_tag(&mut self, id: TagId) -> Result<TagRemoved, TagError> {
        let mut ports = self.ports.borrow_mut();
        let tag = ports.tags.tag(id).ok_or(TagError::NotFound)?;
        let untagged = ports.usage.tag_usage(id).transactions;
        ports.tags.tags.retain(|row| row.id != id);
        Ok(TagRemoved {
            name: tag.name,
            untagged,
        })
    }

    fn merge_tags(&mut self, source: TagId, target: TagId) -> Result<TagMerged, TagError> {
        if source == target {
            return Err(TagError::MergeIntoItself);
        }
        let mut ports = self.ports.borrow_mut();
        let source_tag = ports.tags.tag(source).ok_or(TagError::NotFound)?;
        let target_tag = ports.tags.tag(target).ok_or(TagError::NotFound)?;
        let retagged = ports.usage.tag_usage(source).transactions;
        ports.tags.tags.retain(|row| row.id != source);
        Ok(TagMerged {
            source_name: source_tag.name,
            target_name: target_tag.name,
            retagged,
        })
    }
}

/// What a [`RecordingRouter`] was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterCall {
    OpenDialog(TagsDialog),
    CloseDialog(Option<TagsOutcome>),
    Notice(Notice),
}

/// A Router that does nothing but record each call, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub calls: Vec<RouterCall>,
}

impl RecordingRouter {
    /// The calls made since the last `take`.
    pub fn take(&mut self) -> Vec<RouterCall> {
        std::mem::take(&mut self.calls)
    }
}

impl TagsRouter for RecordingRouter {
    /// Always opens: the recording Router builds no Presenter that could fail.
    fn open_dialog(&mut self, dialog: TagsDialog) -> bool {
        self.calls.push(RouterCall::OpenDialog(dialog));
        true
    }

    fn close_dialog(&mut self, outcome: Option<TagsOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upserting_a_tag_replaces_the_row_with_its_id_or_appends_it() {
        let mut tags = InMemoryTags::stub();
        let seeded = tags.tags().len();
        let mut shared = tags.tag(TagId::seed(2)).expect("seeded");
        shared.name = "split".into();
        tags.upsert_tag(shared.clone());
        let fresh = Tag {
            id: TagId::new(),
            ..shared.clone()
        };
        tags.upsert_tag(fresh.clone());

        assert_eq!(tags.tags().len(), seeded + 1);
        assert_eq!(tags.tag(TagId::seed(2)), Some(shared));
        assert_eq!(tags.tags().last(), Some(&fresh));
    }

    #[test]
    fn a_tag_no_test_names_is_unused() {
        let ports = InMemoryPorts::stub();
        assert_eq!(ports.usage.tag_usage(TagId::seed(1)), TagUse::default());
    }
}
