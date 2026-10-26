use lib_core::TagId;

use crate::protocols::TagRepository;
use crate::{Tag, TagDraft, TagError, edit_tag};

/// Write a [`TagDraft`]'s name and colour onto Tag `id`, leaving its Active flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTag {
    pub id: TagId,
    pub draft: TagDraft,
}

impl EditTag {
    /// Returns the Tag as written, or refuses a Tag that isn't there or a name another Tag has
    /// once normalised.
    pub fn run(self, tags: &mut dyn TagRepository) -> Result<Tag, TagError> {
        let mut rows = tags.tags();
        let outcome = edit_tag(&mut rows, self.id, &self.draft).and_then(|()| {
            rows.into_iter()
                .find(|tag| tag.id == self.id)
                .ok_or(TagError::NotFound)
        });
        match &outcome {
            Ok(tag) => {
                tags.upsert_tag(tag.clone());
                tracing::info!(tag_id = %tag.id, "Edited a Tag");
            }
            Err(refusal) => tracing::info!(tag_id = %self.id, %refusal, "Refused to edit a Tag"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryTags;

    fn draft(name: &str) -> TagDraft {
        TagDraft {
            name: name.into(),
            color: None,
        }
    }

    #[test]
    fn editing_a_tag_writes_the_name_and_colour_and_keeps_active() {
        let mut tags = InMemoryTags::stub();
        let before = tags.tag(TagId::seed(8)).expect("Bali 2025 is seeded");
        assert!(!before.is_active);

        let edited = EditTag {
            id: before.id,
            draft: draft("Bali 2024"),
        }
        .run(&mut tags)
        .expect("the Tag is there");

        assert_eq!(tags.tag(before.id), Some(edited.clone()));
        assert_eq!(
            (edited.name.as_str(), edited.is_active),
            ("Bali 2024", false)
        );
    }

    #[test]
    fn a_missing_tag_or_a_taken_name_is_refused_and_nothing_is_written() {
        let mut tags = InMemoryTags::stub();
        let before = tags.clone();

        assert_eq!(
            EditTag {
                id: TagId::seed(999),
                draft: draft("x"),
            }
            .run(&mut tags),
            Err(TagError::NotFound)
        );
        assert_eq!(
            EditTag {
                id: TagId::seed(2),
                draft: draft("gift!"),
            }
            .run(&mut tags),
            Err(TagError::DuplicateName("gift".into()))
        );
        assert_eq!(tags, before);
    }
}
