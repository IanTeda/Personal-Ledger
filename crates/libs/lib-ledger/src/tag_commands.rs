//! The Tags writes (ADR-0038): each a command value with its own refusal, checked before anything
//! is written so a refusal leaves the Ledger untouched, and recording an upsert or delete per row
//! it wrote (ADR-0039).
//!
//! Single-Tag rules (names, the active flag) are `lib_tags`, which the Tags page writes itself (#863). What lives here crosses domains:
//! removing or merging a Tag rewrites the Splits that carry it, so those Transactions are recorded
//! as written too.

use lib_core::TagId;
use lib_tags::{Tag, TagError, get};
use lib_transactions::Transaction;

use crate::tag_usage::transaction_count;
use crate::{Ledger, LedgerChange, LedgerCommand};

/// Untags every Split carrying Tag `id` and deletes it, used or not. Amounts, Categories and
/// Payees are untouched.
fn remove_tag(
    tags: &mut Vec<Tag>,
    transactions: &mut [Transaction],
    id: TagId,
) -> Result<(), TagError> {
    get(tags, id).ok_or(TagError::NotFound)?;
    for split in transactions.iter_mut().flat_map(|t| &mut t.splits) {
        split.tag_ids.retain(|tag| *tag != id);
    }
    tags.retain(|t| t.id != id);
    Ok(())
}

/// Folds `source` into `target` as one operation: each Split carrying the source carries the
/// target instead (once, if it already had it), then the source is deleted. The target keeps its
/// name, colour and `is_active`; either may be inactive.
fn merge_tags(
    tags: &mut Vec<Tag>,
    transactions: &mut [Transaction],
    source: TagId,
    target: TagId,
) -> Result<(), TagError> {
    if source == target {
        return Err(TagError::MergeIntoItself);
    }
    get(tags, source).ok_or(TagError::NotFound)?;
    get(tags, target).ok_or(TagError::NotFound)?;
    for split in transactions.iter_mut().flat_map(|t| &mut t.splits) {
        if let Some(index) = split.tag_ids.iter().position(|tag| *tag == source) {
            if split.tag_ids.contains(&target) {
                split.tag_ids.remove(index);
            } else {
                split.tag_ids[index] = target;
            }
        }
    }
    tags.retain(|t| t.id != source);
    Ok(())
}

impl Ledger {
    /// Hard-deletes Tag `id`, used or not, and untags every Split that carried it. Returns how
    /// many Transactions were untagged.
    pub fn remove_tag(&mut self, id: TagId) -> Result<usize, TagError> {
        let untagged = transaction_count(&self.transactions, id);
        let mut tags = self.tags.clone();
        let mut transactions = self.transactions.clone();
        remove_tag(&mut tags, &mut transactions, id)?;
        self.tags = tags;
        self.pending.push(LedgerChange::DeleteTag(id));
        self.replace_transactions(transactions);
        Ok(untagged)
    }

    /// Folds Tag `source` into `target` and deletes the source. Returns how many Transactions
    /// carried the source and so were retagged.
    pub fn merge_tags(&mut self, source: TagId, target: TagId) -> Result<usize, TagError> {
        let retagged = transaction_count(&self.transactions, source);
        let mut tags = self.tags.clone();
        let mut transactions = self.transactions.clone();
        merge_tags(&mut tags, &mut transactions, source, target)?;
        self.tags = tags;
        self.pending.push(LedgerChange::DeleteTag(source));
        self.replace_transactions(transactions);
        Ok(retagged)
    }

    /// Swaps in `transactions`, recording an upsert for each one that differs.
    fn replace_transactions(&mut self, transactions: Vec<Transaction>) {
        for (after, before) in transactions.iter().zip(&self.transactions) {
            if after != before {
                self.pending
                    .push(LedgerChange::UpsertTransaction(after.clone()));
            }
        }
        self.transactions = transactions;
    }
}

/// [`Ledger::remove_tag`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveTag(pub TagId);

impl LedgerCommand for RemoveTag {
    type Outcome = usize;
    type Refusal = TagError;
    fn run(self, ledger: &mut Ledger) -> Result<usize, TagError> {
        ledger.remove_tag(self.0)
    }
}

/// [`Ledger::merge_tags`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeTags {
    pub source: TagId,
    pub target: TagId,
}

impl LedgerCommand for MergeTags {
    type Outcome = usize;
    type Refusal = TagError;
    fn run(self, ledger: &mut Ledger) -> Result<usize, TagError> {
        ledger.merge_tags(self.source, self.target)
    }
}

#[cfg(test)]
mod tests {
    use lib_tags::{TagDraft, insert_tag};

    use super::*;

    fn draft(name: &str) -> TagDraft {
        TagDraft {
            name: name.to_owned(),
            color: None,
        }
    }

    fn id_of(ledger: &Ledger, name: &str) -> TagId {
        ledger
            .tags()
            .iter()
            .find(|tag| tag.name == name)
            .expect("the seed has the Tag")
            .id
    }

    #[test]
    fn remove_untags_the_splits_and_records_each_rewritten_transaction() {
        let mut ledger = Ledger::stub();
        let shared = id_of(&ledger, "shared");
        let carrying = transaction_count(ledger.transactions(), shared);
        assert!(carrying > 0);
        let transactions = ledger.transactions().len();

        let untagged = RemoveTag(shared).run(&mut ledger).unwrap();

        assert_eq!(untagged, carrying);
        assert!(get(ledger.tags(), shared).is_none());
        assert_eq!(transaction_count(ledger.transactions(), shared), 0);
        assert_eq!(ledger.transactions().len(), transactions);
        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 1 + carrying);
        assert_eq!(changes[0], LedgerChange::DeleteTag(shared));
        assert!(
            changes[1..]
                .iter()
                .all(|change| matches!(change, LedgerChange::UpsertTransaction(_)))
        );
    }

    #[test]
    fn removing_a_missing_tag_is_refused_and_changes_nothing() {
        let mut ledger = Ledger::stub();
        let before = ledger.transactions().to_vec();

        assert_eq!(
            RemoveTag(TagId::seed(999)).run(&mut ledger),
            Err(TagError::NotFound)
        );
        assert_eq!(ledger.transactions(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn merge_retags_the_source_and_deletes_it() {
        let mut ledger = Ledger::stub();
        let source = id_of(&ledger, "Work Trip");
        let target = id_of(&ledger, "work-trip");
        let carrying = transaction_count(ledger.transactions(), source);
        assert!(carrying > 0);

        let retagged = MergeTags { source, target }.run(&mut ledger).unwrap();

        assert_eq!(retagged, carrying);
        assert!(get(ledger.tags(), source).is_none());
        assert_eq!(transaction_count(ledger.transactions(), source), 0);
        let changes = ledger.drain_changes();
        assert_eq!(changes[0], LedgerChange::DeleteTag(source));
        assert_eq!(changes.len(), 1 + carrying);
    }

    #[test]
    fn a_refused_merge_leaves_the_ledger_untouched() {
        let mut ledger = Ledger::stub();
        let gift = id_of(&ledger, "gift");
        let tags = ledger.tags().to_vec();
        let transactions = ledger.transactions().to_vec();

        assert_eq!(
            MergeTags {
                source: gift,
                target: gift,
            }
            .run(&mut ledger),
            Err(TagError::MergeIntoItself)
        );
        assert_eq!(
            MergeTags {
                source: TagId::seed(999),
                target: gift,
            }
            .run(&mut ledger),
            Err(TagError::NotFound)
        );
        assert_eq!(ledger.tags(), tags);
        assert_eq!(ledger.transactions(), transactions);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn remove_untags_every_split_and_keeps_the_transactions() {
        let ledger = Ledger::stub();
        let mut tags = ledger.tags().to_vec();
        let mut transactions = ledger.transactions().to_vec();
        let before = transactions.clone();
        let shared = id_of(&ledger, "shared");

        remove_tag(&mut tags, &mut transactions, shared).unwrap();

        assert!(get(&tags, shared).is_none());
        assert_eq!(transaction_count(&transactions, shared), 0);
        assert_eq!(transactions.len(), before.len());
        for (after, before) in transactions.iter().zip(&before) {
            assert_eq!(after.total(), before.total());
            for (a, b) in after.splits.iter().zip(&before.splits) {
                assert_eq!((a.category_id, a.payee_id), (b.category_id, b.payee_id));
            }
        }
        assert_eq!(
            remove_tag(&mut tags, &mut transactions, shared),
            Err(TagError::NotFound)
        );
    }

    #[test]
    fn remove_deletes_an_unused_tag_too() {
        let ledger = Ledger::stub();
        let mut tags = ledger.tags().to_vec();
        let mut transactions = ledger.transactions().to_vec();
        let new = insert_tag(&mut tags, &draft("unused")).unwrap();

        remove_tag(&mut tags, &mut transactions, new).unwrap();

        assert!(get(&tags, new).is_none());
    }

    #[test]
    fn merge_retags_without_doubling_a_split_that_already_carries_the_target() {
        let ledger = Ledger::stub();
        let mut tags = ledger.tags().to_vec();
        let (source, target, other) = (
            id_of(&ledger, "Work Trip"),
            id_of(&ledger, "work-trip"),
            TagId::seed(1),
        );
        let mut transactions = ledger.transactions()[..3].to_vec();
        let split = |tag_ids: &[TagId]| {
            let mut split = transactions[0].splits[0].clone();
            split.tag_ids = tag_ids.to_vec();
            split
        };
        let splits = [
            vec![split(&[source])],
            vec![split(&[source, target]), split(&[other])],
            vec![split(&[target])],
        ];
        for (transaction, splits) in transactions.iter_mut().zip(splits) {
            transaction.splits = splits;
        }

        merge_tags(&mut tags, &mut transactions, source, target).unwrap();

        assert!(get(&tags, source).is_none());
        assert_eq!(transactions[0].splits[0].tag_ids, vec![target]);
        assert_eq!(transactions[1].splits[0].tag_ids, vec![target]);
        assert_eq!(transactions[1].splits[1].tag_ids, vec![other]);
        assert_eq!(transaction_count(&transactions, target), 3);
        // The target keeps its own name and colour.
        let kept = get(&tags, target).unwrap();
        assert_eq!(kept.name, "work-trip");
        assert_eq!(kept.color, Some(lib_tags::swatches()[4].clone()));
    }

    #[test]
    fn merge_allows_an_inactive_source_and_keeps_the_targets_active_flag() {
        let ledger = Ledger::stub();
        let mut tags = ledger.tags().to_vec();
        let mut transactions = ledger.transactions().to_vec();
        let gift = id_of(&ledger, "gift");
        let bali = id_of(&ledger, "Bali 2025");
        let expected =
            transaction_count(&transactions, gift) + transaction_count(&transactions, bali);

        merge_tags(&mut tags, &mut transactions, bali, gift).unwrap();

        assert_eq!(transaction_count(&transactions, gift), expected);
        assert!(get(&tags, gift).unwrap().is_active);
    }
}
