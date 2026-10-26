use lib_core::TagId;

use crate::protocols::TagRepository;
use crate::{Tag, TagError};

/// Reactivate a Tag, which counts in the "N tags" figure again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactivateTag(pub TagId);

impl ReactivateTag {
    /// Returns the Tag as written, or refuses a Tag that isn't there.
    pub fn run(self, tags: &mut dyn TagRepository) -> Result<Tag, TagError> {
        let outcome = self.apply(tags);
        match &outcome {
            Ok(_) => tracing::info!(tag_id = %self.0, "Reactivated a Tag"),
            Err(refusal) => {
                tracing::info!(tag_id = %self.0, %refusal, "Refused to reactivate a Tag");
            }
        }
        outcome
    }

    fn apply(self, tags: &mut dyn TagRepository) -> Result<Tag, TagError> {
        let mut tag = tags.tag(self.0).ok_or(TagError::NotFound)?;
        tag.is_active = true;
        tags.upsert_tag(tag.clone());
        Ok(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryTags;

    #[test]
    fn reactivate_sets_active_to_true_and_keeps_the_rest() {
        let mut tags = InMemoryTags::stub();
        let before = tags.tag(TagId::seed(8)).expect("a stub Tag");

        let tag = ReactivateTag(before.id).run(&mut tags).expect("a stub Tag");

        assert_eq!(
            tag,
            Tag {
                is_active: true,
                ..before
            }
        );
        assert_eq!(tags.tag(tag.id), Some(tag));
    }

    #[test]
    fn a_missing_tag_is_refused() {
        let mut tags = InMemoryTags::stub();
        assert_eq!(
            ReactivateTag(TagId::seed(999)).run(&mut tags),
            Err(TagError::NotFound)
        );
    }
}
