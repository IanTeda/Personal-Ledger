//! Tag remove and merge, the module's two Interactors: each rewrites the Splits that carry a Tag,
//! another domain's rows, before deleting the Tag, so they live here rather than in `lib-tags`.
//! The rules moved from `lib-ledger`'s `tag_commands.rs` unchanged (#1003), and every refusal is
//! checked before anything is written, so a refusal writes nothing.

use lib_core::TagId;
use lib_transactions::Transaction;

use crate::error::{Error, Result};
use crate::protocols::{TagMerged, TagRemoval, TagRemoved, TagRewrite};

/// Untags every Split carrying the Tag and deletes it, used or not. Amounts, Categories and Payees
/// are untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveTag {
    pub id: TagId,
}

impl RemoveTag {
    /// # Errors
    /// `NotFound` for a Tag that is gone.
    pub fn run(self, removal: &mut dyn TagRemoval) -> Result<TagRemoved> {
        let outcome = self.apply(removal);
        match &outcome {
            Ok(removed) => {
                tracing::info!(tag_id = %self.id, untagged = removed.untagged, "Removed a Tag");
            }
            Err(refusal) => {
                tracing::info!(tag_id = %self.id, %refusal, "Refused to remove a Tag");
            }
        }
        outcome
    }

    fn apply(self, removal: &mut dyn TagRemoval) -> Result<TagRemoved> {
        let tag = removal.tag(self.id).ok_or(Error::NotFound)?;
        let mut transactions = removal.transactions_carrying(self.id);
        for split in transactions.iter_mut().flat_map(|t| &mut t.splits) {
            split.tag_ids.retain(|tag| *tag != self.id);
        }
        let removed = TagRemoved {
            id: self.id,
            name: tag.name,
            untagged: transactions.len(),
        };
        removal.rewrite(TagRewrite {
            transactions,
            deleted: self.id,
        });
        Ok(removed)
    }
}

/// Folds `source` into `target` as one operation: each Split carrying the source carries the target
/// instead (once, if it already had it), then the source is deleted. The target keeps its name,
/// colour and Active flag; either may be inactive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeTags {
    pub source: TagId,
    pub target: TagId,
}

impl MergeTags {
    /// # Errors
    /// `MergeIntoItself` when the two are one Tag, and `NotFound` when either is gone.
    pub fn run(self, removal: &mut dyn TagRemoval) -> Result<TagMerged> {
        let outcome = self.apply(removal);
        match &outcome {
            Ok(merged) => tracing::info!(
                source_tag_id = %self.source,
                target_tag_id = %self.target,
                retagged = merged.retagged,
                "Merged a Tag into another"
            ),
            Err(refusal) => tracing::info!(
                source_tag_id = %self.source,
                target_tag_id = %self.target,
                %refusal,
                "Refused to merge a Tag"
            ),
        }
        outcome
    }

    fn apply(self, removal: &mut dyn TagRemoval) -> Result<TagMerged> {
        if self.source == self.target {
            return Err(Error::MergeIntoItself);
        }
        let source = removal.tag(self.source).ok_or(Error::NotFound)?;
        let target = removal.tag(self.target).ok_or(Error::NotFound)?;
        let mut transactions = removal.transactions_carrying(self.source);
        retag(&mut transactions, self.source, self.target);
        let merged = TagMerged {
            source: self.source,
            target: self.target,
            source_name: source.name,
            target_name: target.name,
            retagged: transactions.len(),
        };
        removal.rewrite(TagRewrite {
            transactions,
            deleted: self.source,
        });
        Ok(merged)
    }
}

/// Each Split's `source` becomes `target`, or goes where the Split already carries `target`.
fn retag(transactions: &mut [Transaction], source: TagId, target: TagId) {
    for split in transactions.iter_mut().flat_map(|t| &mut t.splits) {
        if let Some(index) = split.tag_ids.iter().position(|tag| *tag == source) {
            if split.tag_ids.contains(&target) {
                split.tag_ids.remove(index);
            } else if let Some(tag) = split.tag_ids.get_mut(index) {
                *tag = target;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use lib_tags::TagRepository as _;
    use lib_transactions::TransactionRepository as _;

    use super::*;
    use crate::fake::InMemoryTagRemoval;
    use crate::fake::world::{
        BALI, GIFT, ONE_OFF, SHARED, TAGGED_TWICE, WORK_TRIP, WORK_TRIP_DUPLICATE, world,
    };

    fn carrying(removal: &InMemoryTagRemoval, id: TagId) -> usize {
        removal.transactions_carrying(id).len()
    }

    fn tags_of(
        removal: &InMemoryTagRemoval,
        transaction: lib_core::TransactionId,
    ) -> Vec<Vec<TagId>> {
        removal
            .transactions
            .transaction(transaction)
            .map(|t| t.splits.into_iter().map(|split| split.tag_ids).collect())
            .unwrap_or_default()
    }

    #[test]
    fn remove_untags_every_split_and_deletes_the_tag() {
        let mut removal = world();
        let before = removal.clone();
        let untagged = carrying(&removal, SHARED);
        assert!(untagged > 0);

        let removed = RemoveTag { id: SHARED }
            .run(&mut removal)
            .expect("shared is on the Ledger");

        assert_eq!(
            removed,
            TagRemoved {
                id: SHARED,
                name: "shared".into(),
                untagged,
            }
        );
        assert_eq!(removal.tags.tag(SHARED), None);
        assert_eq!(carrying(&removal, SHARED), 0);
        assert_eq!(tags_of(&removal, TAGGED_TWICE)[0], [GIFT], "gift stays");
        let (after, before) = (
            removal.transactions.transactions(),
            before.transactions.transactions(),
        );
        assert_eq!(after.len(), before.len());
        for (after, before) in after.iter().zip(&before) {
            assert_eq!(after.total(), before.total());
            for (a, b) in after.splits.iter().zip(&before.splits) {
                assert_eq!((a.category_id, a.payee_id), (b.category_id, b.payee_id));
            }
        }
    }

    #[test]
    fn remove_deletes_an_unused_tag_too() {
        let mut removal = world();
        assert_eq!(carrying(&removal, ONE_OFF), 0);

        let removed = RemoveTag { id: ONE_OFF }
            .run(&mut removal)
            .expect("one-off is on the Ledger");

        assert_eq!(removed.untagged, 0);
        assert_eq!(removal.tags.tag(ONE_OFF), None);
    }

    #[test]
    fn removing_a_missing_tag_is_refused_and_writes_nothing() {
        let mut removal = world();
        let before = removal.clone();

        assert_eq!(
            RemoveTag {
                id: TagId::seed(999)
            }
            .run(&mut removal),
            Err(Error::NotFound)
        );
        assert_eq!(removal, before);
    }

    #[test]
    fn merge_retags_without_doubling_a_split_that_already_carries_the_target() {
        let mut removal = world();
        let retagged = carrying(&removal, WORK_TRIP_DUPLICATE);
        let expected = removal
            .transactions
            .transactions()
            .iter()
            .filter(|t| {
                t.splits.iter().any(|split| {
                    split.tag_ids.contains(&WORK_TRIP_DUPLICATE)
                        || split.tag_ids.contains(&WORK_TRIP)
                })
            })
            .count();

        let merged = MergeTags {
            source: WORK_TRIP_DUPLICATE,
            target: WORK_TRIP,
        }
        .run(&mut removal)
        .expect("both are on the Ledger");

        assert_eq!(
            merged,
            TagMerged {
                source: WORK_TRIP_DUPLICATE,
                target: WORK_TRIP,
                source_name: "Work Trip".into(),
                target_name: "work-trip".into(),
                retagged,
            }
        );
        assert_eq!(removal.tags.tag(WORK_TRIP_DUPLICATE), None);
        assert_eq!(carrying(&removal, WORK_TRIP_DUPLICATE), 0);
        assert_eq!(carrying(&removal, WORK_TRIP), expected);
        assert!(
            removal
                .transactions
                .transactions()
                .iter()
                .flat_map(|t| t.splits.iter())
                .all(|split| split
                    .tag_ids
                    .iter()
                    .filter(|tag| **tag == WORK_TRIP)
                    .count()
                    <= 1),
            "no Split carries the target twice"
        );
        assert_eq!(tags_of(&removal, TAGGED_TWICE)[1], [WORK_TRIP]);
        // The target keeps its own name and colour.
        let kept = removal.tags.tag(WORK_TRIP).expect("the target stays");
        assert_eq!(kept.name, "work-trip");
        assert_eq!(kept.color, Some(lib_tags::swatches()[4].clone()));
    }

    #[test]
    fn merge_allows_an_inactive_source_and_keeps_the_targets_active_flag() {
        let mut removal = world();
        let expected = carrying(&removal, GIFT) + carrying(&removal, BALI);

        MergeTags {
            source: BALI,
            target: GIFT,
        }
        .run(&mut removal)
        .expect("both are on the Ledger");

        assert_eq!(carrying(&removal, GIFT), expected);
        assert!(removal.tags.tag(GIFT).expect("the target stays").is_active);
    }

    #[test]
    fn a_refused_merge_writes_nothing() {
        let mut removal = world();
        let before = removal.clone();

        assert_eq!(
            MergeTags {
                source: GIFT,
                target: GIFT,
            }
            .run(&mut removal),
            Err(Error::MergeIntoItself)
        );
        assert_eq!(
            MergeTags {
                source: TagId::seed(999),
                target: GIFT,
            }
            .run(&mut removal),
            Err(Error::NotFound)
        );
        assert_eq!(
            MergeTags {
                source: GIFT,
                target: TagId::seed(999),
            }
            .run(&mut removal),
            Err(Error::NotFound)
        );
        assert_eq!(removal, before);
    }
}
