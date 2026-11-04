//! The in-memory removal port, for tests and UI-first work, which answers the Dialogs' reads from
//! its own rows too, and [`world`], the small Ledger the tests and the contract remove from. It
//! holds its rows in plain fields so a test can set up or read state directly. The Dialogs' Router
//! is `lib-tags`' own recording one.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use lib_core::TagId;
use lib_tags::fake::InMemoryTags;
use lib_tags::{Tag, TagRepository};
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{Transaction, TransactionRepository};

use crate::protocols::{
    SharedTagRemoval, TagCount, TagRemoval, TagRemovalReads, TagRewrite, by_usage, carrying,
};

/// A remove's two stores in memory, the Tags and the Transactions, written together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryTagRemoval {
    pub tags: InMemoryTags,
    pub transactions: InMemoryTransactions,
}

impl InMemoryTagRemoval {
    /// This port shared, as the Dialogs hold it, with a handle a test reads it back through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedTagRemoval) {
        let rows = Rc::new(RefCell::new(self));
        let shared: SharedTagRemoval = rows.clone();
        (rows, shared)
    }
}

impl TagRemoval for InMemoryTagRemoval {
    fn tag(&self, id: TagId) -> Option<Tag> {
        self.tags.tag(id)
    }

    fn transactions_carrying(&self, id: TagId) -> Vec<Transaction> {
        carrying(self.transactions.transactions(), id)
    }

    fn rewrite(&mut self, rewrite: TagRewrite) {
        for transaction in rewrite.transactions {
            self.transactions.upsert_transaction(transaction);
        }
        self.tags.tags.retain(|tag| tag.id != rewrite.deleted);
    }
}

/// Counted from the rows held, so a test that books a Split sees it in the Dialogs' figures.
impl TagRemovalReads for InMemoryTagRemoval {
    fn tags_by_usage(&self) -> Vec<TagCount> {
        let mut counts: HashMap<TagId, usize> = HashMap::new();
        for transaction in &self.transactions.transactions {
            let tags: HashSet<TagId> = transaction
                .splits
                .iter()
                .flat_map(|split| split.tag_ids.iter().copied())
                .collect();
            for tag in tags {
                *counts.entry(tag).or_default() += 1;
            }
        }
        by_usage(&self.tags.tags, &counts)
    }
}

/// The small Ledger the tests and the contract remove from: `lib-tags`' stub Tags and five
/// Transactions on the stub's ANZ Everyday. Two carry shared, two Work Trip and two work-trip (one
/// Split carrying both, the case a merge must not double), one gift and one Bali 2025; nothing
/// carries the other four, and the fifth Transaction carries no Tag.
pub mod world {
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_core::{AccountId, CategoryId, Money, TagId, TransactionId, TransactionStatus};
    use lib_tags::fake::InMemoryTags;
    use lib_transactions::fake::InMemoryTransactions;
    use lib_transactions::{Split, Transaction};

    use super::InMemoryTagRemoval;

    pub const SHARED: TagId = TagId::seed(2);
    pub const WORK_TRIP: TagId = TagId::seed(5);
    pub const GIFT: TagId = TagId::seed(6);
    /// Nothing carries it.
    pub const ONE_OFF: TagId = TagId::seed(7);
    /// Inactive.
    pub const BALI: TagId = TagId::seed(8);
    /// work-trip's duplicate, which differs from it only in case and punctuation.
    pub const WORK_TRIP_DUPLICATE: TagId = TagId::seed(9);

    /// Carries shared alone.
    pub const SHARED_ONLY: TransactionId = TransactionId::seed(1);
    /// Its first Split carries shared and gift, its second Work Trip and work-trip.
    pub const TAGGED_TWICE: TransactionId = TransactionId::seed(2);
    /// Carries Work Trip alone.
    pub const DUPLICATE_ONLY: TransactionId = TransactionId::seed(3);
    /// Its first Split carries work-trip, its second Bali 2025.
    pub const WORK_TRIP_AND_BALI: TransactionId = TransactionId::seed(4);
    /// Carries no Tag.
    pub const UNTAGGED: TransactionId = TransactionId::seed(5);

    /// The stub's ANZ Everyday and Groceries, which the file's foreign keys need.
    const EVERYDAY: AccountId = AccountId::seed(2);
    const GROCERIES: CategoryId = CategoryId::seed(7);

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, day).unwrap_or_default()
    }

    fn split(cents: i64, tag_ids: &[TagId]) -> Split {
        Split {
            amount: Money(BigDecimal::new(cents.into(), 2)),
            category_id: GROCERIES,
            payee_id: None,
            tag_ids: tag_ids.to_vec(),
        }
    }

    fn transaction(id: TransactionId, day: u32, splits: Vec<Split>) -> Transaction {
        Transaction {
            id,
            date: date(day),
            account_id: EVERYDAY,
            status: TransactionStatus::Cleared,
            is_flagged: false,
            description: Some("Groceries".to_string()),
            splits,
        }
    }

    /// The Tags and Transactions described above.
    pub fn world() -> InMemoryTagRemoval {
        InMemoryTagRemoval {
            tags: InMemoryTags::stub(),
            transactions: InMemoryTransactions {
                transactions: vec![
                    transaction(SHARED_ONLY, 1, vec![split(-4_250, &[SHARED])]),
                    transaction(
                        TAGGED_TWICE,
                        2,
                        vec![
                            split(-1_999, &[SHARED, GIFT]),
                            split(-3_000, &[WORK_TRIP_DUPLICATE, WORK_TRIP]),
                        ],
                    ),
                    transaction(
                        DUPLICATE_ONLY,
                        3,
                        vec![split(-12_000, &[WORK_TRIP_DUPLICATE])],
                    ),
                    transaction(
                        WORK_TRIP_AND_BALI,
                        4,
                        vec![split(-800, &[WORK_TRIP]), split(-650, &[BALI])],
                    ),
                    transaction(UNTAGGED, 5, vec![split(-2_500, &[])]),
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::world::{BALI, GIFT, SHARED, WORK_TRIP, WORK_TRIP_DUPLICATE, world};
    use super::*;

    #[test]
    fn the_reads_rank_the_held_tags_by_the_transactions_carrying_them() {
        let ranked: Vec<(TagId, usize)> = world()
            .tags_by_usage()
            .into_iter()
            .map(|count| (count.tag.id, count.transactions))
            .collect();

        assert_eq!(ranked.len(), lib_tags::default_tags().len());
        assert_eq!(
            ranked[..4],
            [
                (SHARED, 2),
                (WORK_TRIP_DUPLICATE, 2),
                (WORK_TRIP, 2),
                (BALI, 1)
            ]
        );
        assert!(ranked.contains(&(GIFT, 1)));
    }
}
