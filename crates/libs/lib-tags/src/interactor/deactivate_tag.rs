use lib_core::TagId;

use crate::protocols::TagRepository;
use crate::{Tag, TagError};

/// Deactivate a Tag, which drops out of the "N tags" count but keeps its name taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeactivateTag(pub TagId);

impl DeactivateTag {
    /// Returns the Tag as written, or refuses a Tag that isn't there.
    pub fn run(self, tags: &mut dyn TagRepository) -> Result<Tag, TagError> {
        let outcome = self.apply(tags);
        match &outcome {
            Ok(_) => tracing::info!(tag_id = %self.0, "Deactivated a Tag"),
            Err(refusal) => {
                tracing::info!(tag_id = %self.0, %refusal, "Refused to deactivate a Tag");
            }
        }
        outcome
    }

    fn apply(self, tags: &mut dyn TagRepository) -> Result<Tag, TagError> {
        let mut tag = tags.tag(self.0).ok_or(TagError::NotFound)?;
        tag.is_active = false;
        tags.upsert_tag(tag.clone());
        Ok(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryTags;

    #[test]
    fn deactivate_sets_active_to_false_and_keeps_the_rest() {
        let mut tags = InMemoryTags::stub();
        let before = tags.tag(TagId::seed(1)).expect("a stub Tag");

        let tag = DeactivateTag(before.id).run(&mut tags).expect("a stub Tag");

        assert_eq!(
            tag,
            Tag {
                is_active: false,
                ..before
            }
        );
        assert_eq!(tags.tag(tag.id), Some(tag));
    }

    #[test]
    fn a_missing_tag_is_refused() {
        let mut tags = InMemoryTags::stub();
        assert_eq!(
            DeactivateTag(TagId::seed(999)).run(&mut tags),
            Err(TagError::NotFound)
        );
    }
}
