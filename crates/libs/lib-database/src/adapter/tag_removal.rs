//! The Tag Removal adapter: `lib-tag-removal`'s atomic port over the Tags and Transactions stores
//! (ADR-0046), after the Unit Deletion adapter. It holds no rows of its own. A remove's or merge's
//! rewritten Transactions go to the Transactions store first, then the Tag's delete to the Tags
//! store, and every write joins the [`PendingWrites`] every adapter shares, so the bin's one commit
//! after the call sends them as one transaction: a refused Tag delete takes the rewritten Splits
//! down with it. `split_tags.tag_id` has no cascade, so no Split may carry the Tag when it goes.
//!
//! [`PendingWrites`]: crate::PendingWrites

use lib_core::TagId;
use lib_tag_removal::{TagRemoval, TagRewrite};
use lib_tags::Tag;
use lib_transactions::Transaction;

use super::store::{SharedTagsStore, SharedTransactionsStore};

/// The removal port over the stores the Tags and Transactions modules also hold.
pub struct TagRemovalAdapter {
    tags: SharedTagsStore,
    transactions: SharedTransactionsStore,
}

impl TagRemovalAdapter {
    pub fn new(tags: SharedTagsStore, transactions: SharedTransactionsStore) -> Self {
        Self { tags, transactions }
    }
}

impl TagRemoval for TagRemovalAdapter {
    fn tag(&self, id: TagId) -> Option<Tag> {
        self.tags.borrow().tags().tag(id)
    }

    fn transactions_carrying(&self, id: TagId) -> Vec<Transaction> {
        lib_tag_removal::protocols::carrying(self.transactions.borrow().all_transactions(), id)
    }

    /// The Transactions first, then the Tag: nothing may carry the Tag when it goes.
    fn rewrite(&mut self, rewrite: TagRewrite) {
        let TagRewrite {
            transactions,
            deleted,
        } = rewrite;
        {
            let mut store = self.transactions.borrow_mut();
            for transaction in transactions {
                store.upsert_transaction(transaction);
            }
        }
        self.tags.borrow_mut().delete_tag(deleted);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use lib_tag_removal::contract::{self, RemovalFactory, Stored};
    use lib_tag_removal::fake::InMemoryTagRemoval;
    use lib_tag_removal::fake::world::world;
    use lib_transactions::TransactionRepository as _;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{
        AccountsAdapter, PendingWrites, StoredTagRemovalReads, TagsAdapter, TransactionsAdapter,
        Writer,
    };

    /// The two adapters on one test database and one pending batch, as start-up opens them. The
    /// sample's Units, Institutions, Categories and Accounts are stored first, which the world's
    /// Transactions name.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            crate::UnitsAdapter::seed_from(
                &writer,
                lib_units::fake::InMemoryPorts::sample(chrono::Utc::now()),
            )
            .expect("the sample Units are written");
            crate::InstitutionsAdapter::seed(&writer, crate::seed::sample_institutions())
                .expect("the sample Institutions are written");
            crate::CategoriesAdapter::seed(&writer, crate::seed::sample_categories())
                .expect("the sample Categories are written");
            AccountsAdapter::seed(&writer, crate::seed::sample_accounts())
                .expect("the sample Accounts are written");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        /// The Tags and Transactions stored now, read with fresh adapters as a restart would.
        fn read(&self) -> Stored {
            self.writer.flush().expect("the writer is running");
            let fresh = PendingWrites::new(self.writer.clone());
            let tags = TagsAdapter::load(fresh.clone()).expect("the Tags load");
            let transactions = TransactionsAdapter::load(fresh).expect("the Transactions load");
            Stored::new(tags.tags().tags(), transactions.transactions())
        }

        /// Commits, settles the writes, and reads back what is stored, with the failures reported.
        fn settle(&mut self) -> (Stored, usize) {
            self.pending.commit();
            self.writer.flush().expect("the writer is running");
            let failures = std::iter::from_fn(|| self.failures.try_recv().ok()).count();
            (self.read(), failures)
        }

        /// The world's rows stored, and the two stores loaded on the one pending batch.
        fn stores(&self, rows: InMemoryTagRemoval) -> (SharedTagsStore, SharedTransactionsStore) {
            TagsAdapter::seed(&self.writer, rows.tags.tags).expect("the Tags are written");
            TransactionsAdapter::seed(&self.writer, rows.transactions.transactions)
                .expect("the Transactions are written");
            let tags = TagsAdapter::load(self.pending.clone()).expect("the Tags load");
            let transactions =
                TransactionsAdapter::load(self.pending.clone()).expect("the Transactions load");
            (
                Rc::new(RefCell::new(tags)),
                Rc::new(RefCell::new(transactions)),
            )
        }
    }

    impl RemovalFactory for AdapterFactory {
        type Ports = TagRemovalAdapter;

        fn create(&mut self, rows: InMemoryTagRemoval) -> TagRemovalAdapter {
            let (tags, transactions) = self.stores(rows);
            TagRemovalAdapter::new(tags, transactions)
        }

        fn uncommitted(&mut self) -> Stored {
            self.read()
        }

        fn stored(&mut self, _ports: TagRemovalAdapter) -> Stored {
            let (stored, failures) = self.settle();
            assert_eq!(failures, 0, "a write failed");
            stored
        }

        fn break_tag_deletes(&mut self, _ports: &mut TagRemovalAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_tag_delete BEFORE DELETE ON tags \
                         BEGIN SELECT RAISE(ABORT, 'deletes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, _ports: TagRemovalAdapter) -> (Stored, usize) {
            self.settle()
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_remove_untags_and_deletes(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_remove_untags_and_deletes(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_merge_retags_once_and_deletes_the_source(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_merge_retags_once_and_deletes_the_source(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_writes_wait_for_the_one_commit(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::the_writes_wait_for_the_one_commit(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failing_remove_lands_nothing(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_failing_remove_lands_nothing(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_refusal_writes_nothing(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_refusal_writes_nothing(&mut AdapterFactory::new(options));
    }

    /// Over the adapters, the Dialogs' reads count the Splits in the file.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_stored_reads_rank_every_tag_by_usage(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let factory = AdapterFactory::new(options);
        let (tags, transactions) = factory.stores(world());

        let reads = StoredTagRemovalReads { tags, transactions };
        contract::the_reads_rank_every_tag_by_usage(&reads);
        contract::the_reads_count_one_tag(&reads);
    }
}
