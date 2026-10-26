use crate::protocols::TagRepository;
use crate::{Tag, TagDraft, TagError, insert_tag};

/// Add the Tag a [`TagDraft`] describes, active, under a fresh id, after every Tag already there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddTag(pub TagDraft);

impl AddTag {
    /// Returns the Tag as written, or refuses a name that is missing or taken once normalised.
    pub fn run(self, tags: &mut dyn TagRepository) -> Result<Tag, TagError> {
        let mut rows = tags.tags();
        let outcome = insert_tag(&mut rows, &self.0).and_then(|id| {
            rows.into_iter()
                .find(|tag| tag.id == id)
                .ok_or(TagError::NotFound)
        });
        match &outcome {
            Ok(tag) => {
                tags.upsert_tag(tag.clone());
                tracing::info!(tag_id = %tag.id, "Added a Tag");
            }
            Err(refusal) => tracing::info!(%refusal, "Refused to add a Tag"),
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
    fn adding_a_tag_writes_it_active_after_the_others() {
        let mut tags = InMemoryTags::stub();
        let seeded = tags.tags().len();

        let added = AddTag(draft(" holiday "))
            .run(&mut tags)
            .expect("a new name");

        assert_eq!(tags.tags().len(), seeded + 1);
        assert_eq!(tags.tags().last(), Some(&added));
        assert_eq!((added.name.as_str(), added.is_active), ("holiday", true));
    }

    #[test]
    fn a_name_taken_once_normalised_is_refused_and_nothing_is_written() {
        let mut tags = InMemoryTags::stub();
        let before = tags.clone();

        assert_eq!(
            AddTag(draft("Tax Deductible")).run(&mut tags),
            Err(TagError::DuplicateName("tax-deductible".into()))
        );
        assert_eq!(tags, before);
    }
}
