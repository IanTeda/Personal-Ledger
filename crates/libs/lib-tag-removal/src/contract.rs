//! The contract every stored removal port keeps (ADR-0046), run against both the in-memory port
//! here and the `lib-database` adapter, so the fake the Clients' tests use can be trusted: a remove
//! or merge lands the rewritten Splits and the Tag's delete together, in the one commit after the
//! call and not before, a write that fails lands none of it, and a refusal writes nothing. The
//! reads the Dialogs open on keep one check too.
//!
//! Each check is a function generic over a [`RemovalFactory`]: a consumer runs it from its own
//! test, since an adapter's tests need their own set-up (a fresh database per test).

use lib_core::{TagId, TransactionId};
use lib_tags::Tag;
use lib_transactions::Transaction;

use crate::error::Error;
use crate::fake::InMemoryTagRemoval;
use crate::fake::world::{
    BALI, DUPLICATE_ONLY, GIFT, SHARED, SHARED_ONLY, TAGGED_TWICE, WORK_TRIP, WORK_TRIP_DUPLICATE,
    world,
};
use crate::interactor::{MergeTags, RemoveTag};
use crate::protocols::{TagRemoval, TagRemovalReads};

/// The rows a store holds, in an order two stores compare in: Tags and Transactions by id.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub tags: Vec<Tag>,
    pub transactions: Vec<Transaction>,
}

impl Stored {
    pub fn new(mut tags: Vec<Tag>, mut transactions: Vec<Transaction>) -> Self {
        tags.sort_by_key(|tag| tag.id);
        transactions.sort_by_key(|transaction| transaction.id);
        Self { tags, transactions }
    }

    fn of(rows: &InMemoryTagRemoval) -> Self {
        Self::new(
            rows.tags.tags.clone(),
            rows.transactions.transactions.clone(),
        )
    }

    fn tag(&self, id: TagId) -> Option<&Tag> {
        self.tags.iter().find(|tag| tag.id == id)
    }

    /// Each Split's Tags on Transaction `id`, in order.
    fn split_tags(&self, id: TransactionId) -> Vec<Vec<TagId>> {
        self.transactions
            .iter()
            .find(|transaction| transaction.id == id)
            .map(|transaction| {
                transaction
                    .splits
                    .iter()
                    .map(|split| split.tag_ids.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether any Split carries Tag `id`.
    fn carries(&self, id: TagId) -> bool {
        self.transactions
            .iter()
            .flat_map(|transaction| &transaction.splits)
            .any(|split| split.tag_ids.contains(&id))
    }
}

/// Builds the ports a contract check runs on.
pub trait RemovalFactory {
    type Ports: TagRemoval;

    /// Ports holding `rows`' Tags and Transactions, as if stored and loaded.
    fn create(&mut self, rows: InMemoryTagRemoval) -> Self::Ports;

    /// What is stored now, read back as a restart would, without ending the writes made on the
    /// ports: what a port that committed for itself would already show.
    fn uncommitted(&mut self) -> Stored;

    /// Ends the writes made on `ports` with one commit, then reads back what is stored, as a
    /// restart would.
    fn stored(&mut self, ports: Self::Ports) -> Stored;

    /// Makes every Tag delete from now on fail, as a store that refuses it would.
    fn break_tag_deletes(&mut self, ports: &mut Self::Ports);

    /// Ends the writes made on `ports` and settles them, then reads back what is stored, with the
    /// number of failures reported.
    fn recover(&mut self, ports: Self::Ports) -> (Stored, usize);
}

/// The in-memory port is its own storage. It keeps a copy as of the last commit, which a failed
/// write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryTagRemoval>,
    broken: bool,
    failures: usize,
}

impl InMemoryFactory {
    fn commit(&mut self, ports: &InMemoryTagRemoval) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn read(&self) -> Stored {
        Stored::of(self.committed.as_ref().expect("create ran first"))
    }
}

impl RemovalFactory for InMemoryFactory {
    type Ports = InMemoryTagRemoval;

    fn create(&mut self, rows: InMemoryTagRemoval) -> InMemoryTagRemoval {
        self.committed = Some(rows.clone());
        rows
    }

    fn uncommitted(&mut self) -> Stored {
        self.read()
    }

    fn stored(&mut self, ports: InMemoryTagRemoval) -> Stored {
        self.commit(&ports);
        self.read()
    }

    fn break_tag_deletes(&mut self, _ports: &mut InMemoryTagRemoval) {
        self.broken = true;
    }

    fn recover(&mut self, ports: InMemoryTagRemoval) -> (Stored, usize) {
        self.commit(&ports);
        (self.read(), std::mem::take(&mut self.failures))
    }
}

/// A remove stores every Split untagged and the Tag gone, the Splits' other Tags kept; the
/// Transactions themselves all stay.
pub fn a_remove_untags_and_deletes<F: RemovalFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    RemoveTag { id: SHARED }
        .run(&mut ports)
        .expect("shared is on the Ledger");
    let stored = factory.stored(ports);

    assert_eq!(stored.tag(SHARED), None);
    assert!(!stored.carries(SHARED));
    assert_eq!(stored.split_tags(SHARED_ONLY), [Vec::<TagId>::new()]);
    assert_eq!(
        stored.split_tags(TAGGED_TWICE),
        [vec![GIFT], vec![WORK_TRIP_DUPLICATE, WORK_TRIP]]
    );
    assert_eq!(stored.transactions.len(), before.transactions.len());
    assert_eq!(stored.tags.len(), before.tags.len() - 1);
}

/// A merge stores the source's Splits carrying the target, once where a Split carried both, and
/// the source gone; the target keeps its name, colour and Active flag.
pub fn a_merge_retags_once_and_deletes_the_source<F: RemovalFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    MergeTags {
        source: WORK_TRIP_DUPLICATE,
        target: WORK_TRIP,
    }
    .run(&mut ports)
    .expect("both are on the Ledger");
    let stored = factory.stored(ports);

    assert_eq!(stored.tag(WORK_TRIP_DUPLICATE), None);
    assert!(!stored.carries(WORK_TRIP_DUPLICATE));
    assert_eq!(
        stored.split_tags(TAGGED_TWICE),
        [vec![SHARED, GIFT], vec![WORK_TRIP]],
        "a Split already carrying the target carries it once"
    );
    assert_eq!(stored.split_tags(DUPLICATE_ONLY), [vec![WORK_TRIP]]);
    assert_eq!(stored.tag(WORK_TRIP), before.tag(WORK_TRIP));
    assert_eq!(stored.tag(BALI), before.tag(BALI));
}

/// Nothing a remove writes is stored before the one commit that ends the call: the port commits
/// nothing for itself, so the bin's commit lands the Splits and the delete as one.
pub fn the_writes_wait_for_the_one_commit<F: RemovalFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    RemoveTag { id: SHARED }
        .run(&mut ports)
        .expect("shared is on the Ledger");

    assert_eq!(factory.uncommitted(), before, "nothing is stored yet");
    let stored = factory.stored(ports);
    assert_eq!(stored.tag(SHARED), None);
    assert!(!stored.carries(SHARED));
}

/// A remove whose Tag delete fails stores none of it: every Split keeps the Tag, which stands,
/// and the failure is reported.
pub fn a_failing_remove_lands_nothing<F: RemovalFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    factory.break_tag_deletes(&mut ports);
    RemoveTag { id: SHARED }
        .run(&mut ports)
        .expect("the port takes the write");
    let (stored, failures) = factory.recover(ports);

    assert_eq!(stored, before);
    assert!(failures > 0, "the failure is reported");
}

/// A merge into itself, and a remove or merge naming a Tag not on the Ledger, are refused by the
/// port's own reads, before anything is written.
pub fn a_refusal_writes_nothing<F: RemovalFactory>(factory: &mut F) {
    let mut ports = factory.create(world());
    let unknown = TagId::seed(999);

    assert_eq!(
        MergeTags {
            source: GIFT,
            target: GIFT,
        }
        .run(&mut ports),
        Err(Error::MergeIntoItself)
    );
    assert_eq!(
        RemoveTag { id: unknown }.run(&mut ports),
        Err(Error::NotFound)
    );
    assert_eq!(
        MergeTags {
            source: unknown,
            target: GIFT,
        }
        .run(&mut ports),
        Err(Error::NotFound)
    );
    assert_eq!(
        MergeTags {
            source: GIFT,
            target: unknown,
        }
        .run(&mut ports),
        Err(Error::NotFound)
    );

    assert_eq!(factory.stored(ports), Stored::of(&world()));
}

/// Reads over [`world`]'s rows rank every Tag by the distinct Transactions carrying it, most-used
/// first and ties A–Z ignoring case, a Tag nothing carries counted none.
pub fn the_reads_rank_every_tag_by_usage(reads: &dyn TagRemovalReads) {
    let ranked: Vec<(String, usize)> = reads
        .tags_by_usage()
        .into_iter()
        .map(|count| (count.tag.name, count.transactions))
        .collect();
    let expected: Vec<(String, usize)> = [
        ("shared", 2),
        ("Work Trip", 2),
        ("work-trip", 2),
        ("Bali 2025", 1),
        ("gift", 1),
        ("Japan Trip 2026", 0),
        ("one-off", 0),
        ("reimbursable", 0),
        ("tax-deductible", 0),
    ]
    .into_iter()
    .map(|(name, count)| (name.to_string(), count))
    .collect();

    assert_eq!(ranked, expected);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_lands_a_remove() {
        a_remove_untags_and_deletes(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_lands_a_merge() {
        a_merge_retags_once_and_deletes_the_source(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_waits_for_the_commit() {
        the_writes_wait_for_the_one_commit(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_lands_no_part_of_a_failing_remove() {
        a_failing_remove_lands_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_writes_nothing_for_a_refusal() {
        a_refusal_writes_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_reads_rank_every_tag_by_usage() {
        the_reads_rank_every_tag_by_usage(&world());
    }
}
