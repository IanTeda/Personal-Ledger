//! The Tags writes (ADR-0038): each a command value with its own refusal, checked before anything
//! is written so a refusal leaves the Ledger untouched, and recording an upsert or delete per row
//! it wrote (ADR-0039).
//!
//! Single-Tag rules (names, the active flag) stay in `lib_tags`. What lives here crosses domains:
//! removing or merging a Tag rewrites the Splits that carry it, so those Transactions are recorded
//! as written too.

use lib_core::TagId;
use lib_tags::{
    Tag, TagDraft, TagError, edit_tag, get, insert_tag, merge_tags, remove_tag, set_active,
    transaction_count,
};

use crate::{Ledger, LedgerChange, LedgerCommand};

impl Ledger {
    /// Adds a new, active Tag and returns its id.
    pub fn add_tag(&mut self, draft: &TagDraft) -> Result<TagId, TagError> {
        let id = insert_tag(&mut self.tags, draft)?;
        self.record_tag(id);
        Ok(id)
    }

    /// Saves Tag `id`'s name and colour, then its Active flag, as one write: if either is refused
    /// neither lands.
    pub fn edit_tag(
        &mut self,
        id: TagId,
        draft: &TagDraft,
        is_active: bool,
    ) -> Result<(), TagError> {
        let mut tags = self.tags.clone();
        edit_tag(&mut tags, id, draft)?;
        set_active(&mut tags, id, is_active)?;
        self.tags = tags;
        self.record_tag(id);
        Ok(())
    }

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
    fn replace_transactions(&mut self, transactions: Vec<lib_transactions::Transaction>) {
        for (after, before) in transactions.iter().zip(&self.transactions) {
            if after != before {
                self.pending
                    .push(LedgerChange::UpsertTransaction(after.clone()));
            }
        }
        self.transactions = transactions;
    }

    fn record_tag(&mut self, id: TagId) {
        if let Some(tag) = get(&self.tags, id) {
            self.pending.push(LedgerChange::UpsertTag(Tag::clone(tag)));
        }
    }
}

/// [`Ledger::add_tag`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddTag(pub TagDraft);

impl LedgerCommand for AddTag {
    type Outcome = TagId;
    type Refusal = TagError;
    fn run(self, ledger: &mut Ledger) -> Result<TagId, TagError> {
        ledger.add_tag(&self.0)
    }
}

/// [`Ledger::edit_tag`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTag {
    pub id: TagId,
    pub draft: TagDraft,
    pub is_active: bool,
}

impl LedgerCommand for EditTag {
    type Outcome = ();
    type Refusal = TagError;
    fn run(self, ledger: &mut Ledger) -> Result<(), TagError> {
        ledger.edit_tag(self.id, &self.draft, self.is_active)
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
    fn add_records_the_new_tag() {
        let mut ledger = Ledger::stub();
        let id = AddTag(draft("holiday")).run(&mut ledger).unwrap();

        assert_eq!(get(ledger.tags(), id).unwrap().name, "holiday");
        let changes = ledger.drain_changes();
        assert!(matches!(&changes[..], [LedgerChange::UpsertTag(tag)] if tag.id == id));
    }

    #[test]
    fn a_refused_add_or_edit_leaves_the_ledger_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.tags().to_vec();
        let shared = id_of(&ledger, "shared");

        assert_eq!(
            AddTag(draft("Tax Deductible")).run(&mut ledger),
            Err(TagError::DuplicateName("tax-deductible".to_owned()))
        );
        assert_eq!(
            EditTag {
                id: shared,
                draft: draft("gift!"),
                is_active: false,
            }
            .run(&mut ledger),
            Err(TagError::DuplicateName("gift".to_owned()))
        );
        assert_eq!(
            EditTag {
                id: TagId::seed(999),
                draft: draft("x"),
                is_active: true,
            }
            .run(&mut ledger),
            Err(TagError::NotFound)
        );
        assert_eq!(ledger.tags(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn edit_saves_the_name_and_the_active_flag_together() {
        let mut ledger = Ledger::stub();
        let gift = id_of(&ledger, "gift");

        EditTag {
            id: gift,
            draft: draft("Gifts"),
            is_active: false,
        }
        .run(&mut ledger)
        .unwrap();

        let tag = get(ledger.tags(), gift).unwrap();
        assert_eq!((tag.name.as_str(), tag.is_active), ("Gifts", false));
        assert_eq!(ledger.drain_changes().len(), 1);
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
}
