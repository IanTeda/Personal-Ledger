//! `lib-tags`' ports answered over the stores: the Tags from the Tags store, and what carries each
//! from the Transactions, Accounts and Units stores through [`StoredTagReads`] (#994, #1082).

use std::collections::HashMap;

use lib_core::TagId;
use lib_tags::{Tag, TagRepository, TagUsage, TagUse};

use crate::adapter::store::{SharedTagsStore, StoredTagReads};

/// [`lib_tags::TagsPorts`] over the shared Tags store, borrowed for each call, and the reads over
/// the stores of the domains that carry a Tag. A write reaches the store's recorder, so the commit
/// names the Tags in what the input wrote.
pub struct StoredTagsPorts {
    tags: SharedTagsStore,
    usage: StoredTagReads,
}

impl StoredTagsPorts {
    pub fn new(tags: SharedTagsStore, usage: StoredTagReads) -> Self {
        Self { tags, usage }
    }
}

impl TagRepository for StoredTagsPorts {
    fn tags(&self) -> Vec<Tag> {
        self.tags.borrow().tags().tags()
    }

    fn upsert_tag(&mut self, tag: Tag) {
        self.tags.borrow_mut().tags_mut().upsert_tag(tag);
    }
}

impl TagUsage for StoredTagsPorts {
    fn tag_usage(&self, id: TagId) -> TagUse {
        self.usage.tag_usage(id)
    }

    fn transactions_by_tag(&self, ids: &[TagId]) -> HashMap<TagId, usize> {
        self.usage.transactions_by_tag(ids)
    }
}

#[cfg(test)]
mod tests {
    use lib_tags::TagDraft;
    use lib_tags::interactor::{AddTag, DeactivateTag, EditTag, ReactivateTag};

    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    fn draft(name: &str) -> TagDraft {
        TagDraft {
            name: name.into(),
            color: None,
        }
    }

    #[test]
    fn each_tags_write_reports_the_tags_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts { tags, commit, .. } = ledger;
            let wrote = |write: &dyn Fn(&mut dyn lib_tags::TagRepository)| {
                write(&mut *tags.borrow_mut());
                commit.commit().wrote
            };

            let id = std::cell::Cell::new(None);
            assert_eq!(
                wrote(&|ports| id.set(Some(
                    AddTag(draft("holiday")).run(ports).expect("a new name").id
                ))),
                [Domain::Tags],
                "an add"
            );
            let id = id.get().expect("the add's Tag");
            assert_eq!(
                wrote(&|ports| {
                    EditTag {
                        id,
                        draft: draft("holidays"),
                    }
                    .run(ports)
                    .expect("the Tag exists");
                }),
                [Domain::Tags],
                "an edit"
            );
            assert_eq!(
                wrote(&|ports| {
                    DeactivateTag(id).run(ports).expect("an active Tag");
                }),
                [Domain::Tags],
                "a deactivate"
            );
            assert_eq!(
                wrote(&|ports| {
                    ReactivateTag(id).run(ports).expect("an inactive Tag");
                }),
                [Domain::Tags],
                "a reactivate"
            );
        }
    }

    #[test]
    fn a_tags_read_reports_nothing_written() {
        let ledger = fakes();

        let _ = ledger.tags.borrow().tags();
        let _ = ledger.tags.borrow().tag_usage(lib_core::TagId::seed(1));

        assert!(ledger.commit.commit().is_empty());
    }
}
