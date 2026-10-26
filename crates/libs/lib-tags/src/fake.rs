//! In-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly.

use std::collections::HashMap;

use lib_core::TagId;

use crate::protocols::{TagRepository, TagUsage, TagUse};
use crate::{Tag, default_tags};

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
