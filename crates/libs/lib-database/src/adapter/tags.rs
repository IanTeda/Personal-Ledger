//! The Tags adapter: `lib-tags`' repository port over the `tags` table, and
//! `lib-transactions`' `TransactionTags` read port from the same rows (ADR-0046, #862, #925).
//!
//! It loads every Tag into memory, blocking, and serves the rows from there in the order they were
//! added (rowid order, which an upsert keeps). A port write changes memory at once and joins the
//! [`PendingWrites`] every adapter shares, which the bin commits after each input.
//!
//! The port has no delete: Tag remove and merge rewrite Splits, so they are `lib-tag-removal`'s
//! (#1002). [`TagsAdapter::delete_tag`] is the write its `TagRemovalAdapter` sends their deletes
//! through, after the Transactions that drop the Tag: `split_tags.tag_id`'s foreign key refuses
//! deleting a Tag a Split still carries.
//!
//! What carries a Tag is other domains' rows, so `StoredTagReads` counts it from the Transactions,
//! Accounts and Units stores (#994), not here.

use lib_core::{HexColor, TagId};
use lib_tags::{Tag, TagRepository};
use lib_transactions::TransactionTag;
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One write, saved in the order it was made.
#[derive(Debug, Clone)]
enum TagsChange {
    Upsert(Tag),
    Delete(TagId),
}

/// The Tags domain, held in memory and saved through the shared writer.
pub struct TagsAdapter {
    tags: StoredTags,
}

/// The rows, with the batch their writes join.
struct StoredTags {
    rows: Vec<Tag>,
    pending: PendingWrites,
}

impl TagsAdapter {
    /// Loads every Tag through `pending`'s writer, blocking until they are read. The adapter's
    /// writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read.
    #[tracing::instrument(name = "Load the Tags", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self {
            tags: StoredTags { rows, pending },
        })
    }

    /// Writes `tags` in one transaction, blocking until it lands. Each is an upsert, so seeding
    /// again leaves the same rows. For before the Transactions whose Splits carry them.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Tags", level = "debug", skip_all, err)]
    pub fn seed(writer: &Writer, tags: Vec<Tag>) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            Self::seed_in(&mut conn, &tags).await?;
            conn.commit().await?;
            Ok(())
        })
    }

    /// [`Self::seed`]'s writes on `conn`, so a seed of several domains commits them in one
    /// transaction (#886).
    ///
    /// # Errors
    /// Returns the error of a write.
    pub(crate) async fn seed_in(conn: &mut SqliteConnection, tags: &[Tag]) -> Result<()> {
        for tag in tags {
            upsert(conn, tag).await?;
        }
        Ok(())
    }

    /// Drops the Tags changes not yet committed and loads the rows again, blocking. For after a
    /// [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Tags`], so memory matches the
    /// file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Tags", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.tags.pending.clone())?;
        self.tags.rows = fresh.tags.rows;
        self.tags.pending.discard(Domain::Tags);
        Ok(())
    }

    /// Removes Tag `id`: Tag delete's and merge's delete, which the Desktop's forwarding shim
    /// sends once the Splits carrying it are retagged (#813 moves it to `lib-reassign`).
    pub fn delete_tag(&mut self, id: TagId) {
        self.tags.rows.retain(|row| row.id != id);
        self.tags.pending.push(TagsChange::Delete(id));
    }

    pub fn tags(&self) -> &dyn TagRepository {
        &self.tags
    }

    pub fn tags_mut(&mut self) -> &mut dyn TagRepository {
        &mut self.tags
    }
}

impl TagRepository for StoredTags {
    fn tags(&self) -> Vec<Tag> {
        self.rows.clone()
    }

    fn upsert_tag(&mut self, tag: Tag) {
        match self.rows.iter_mut().find(|row| row.id == tag.id) {
            Some(row) => *row = tag.clone(),
            None => self.rows.push(tag.clone()),
        }
        self.pending.push(TagsChange::Upsert(tag));
    }
}

impl Change for TagsChange {
    fn domain(&self) -> Domain {
        Domain::Tags
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::Upsert(tag) => upsert(conn, &tag).await,
                Self::Delete(id) => delete(conn, id).await,
            }
        })
    }
}

async fn upsert(conn: &mut SqliteConnection, tag: &Tag) -> Result<()> {
    let color = tag.color.as_ref().map(HexColor::as_str);
    sqlx::query!(
        "INSERT INTO tags (id, name, color, is_active)
         VALUES (?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             color = excluded.color,
             is_active = excluded.is_active",
        tag.id,
        tag.name,
        color,
        tag.is_active,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn delete(conn: &mut SqliteConnection, id: TagId) -> Result<()> {
    sqlx::query!("DELETE FROM tags WHERE id = ?", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn read(conn: &mut SqliteConnection) -> Result<Vec<Tag>> {
    sqlx::query!(
        r#"SELECT id AS "id!: TagId", name, color, is_active
           FROM tags
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| {
        let color = row
            .color
            .map(|color| {
                HexColor::parse(&color).map_err(|error| {
                    Error::Validation(format!("Tag {} has the colour {color:?}: {error}", row.id))
                })
            })
            .transpose()?;
        Ok(Tag {
            id: row.id,
            name: row.name,
            color,
            is_active: row.is_active,
        })
    })
    .collect()
}

/// `tags` as Transactions reads them.
pub(crate) fn transaction_tags(tags: &[Tag]) -> Vec<TransactionTag> {
    tags.iter()
        .map(|tag| TransactionTag {
            id: tag.id,
            name: tag.name.clone(),
            color: tag.color.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::mpsc;

    use chrono::NaiveDate;

    use bigdecimal::BigDecimal;
    use lib_accounts::default_accounts;
    use lib_core::{AccountId, CategoryId, Money, Total, TransactionId, TransactionStatus, UnitId};
    use lib_tags::contract::{self, PortsFactory, StoredPorts};
    use lib_tags::default_tags;
    use lib_tags::{TagUsage, TagUse};
    use lib_transactions::{Split, Transaction};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::test_support::TransactionReadsFactory;
    use crate::writer::WriteFailure;
    use crate::{AccountsAdapter, StoredTagReads, TransactionsAdapter, UnitsAdapter};

    /// The adapter with the reads that count what carries its Tags, over the other stores loaded
    /// on the same file, as a Client holds them (#994).
    struct Ports {
        tags: TagsAdapter,
        reads: StoredTagReads,
    }

    impl StoredPorts for Ports {
        fn tags(&mut self) -> &mut dyn TagRepository {
            &mut self.tags.tags
        }

        fn usage(&self) -> &dyn TagUsage {
            &self.reads
        }
    }

    /// The adapter on one test database, with its pending batch. The stub's Units, Accounts and
    /// Categories are stored first, which a carrying Transaction names, and the stub Tags a
    /// Split's tags name. A reload commits, loads afresh, and fails the check if the writer
    /// reported a failure, so a write SQLite refuses can't pass unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            crate::test_support::store_stub_references(&writer);
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        /// `tags` with the reads over the stores loaded now on the file.
        fn ports(&self, tags: TagsAdapter) -> Ports {
            let load = |pending: &PendingWrites| {
                (
                    Rc::new(RefCell::new(
                        TransactionsAdapter::load(pending.clone()).expect("the rows load"),
                    )),
                    Rc::new(RefCell::new(
                        AccountsAdapter::load(pending.clone()).expect("the rows load"),
                    )),
                    Rc::new(RefCell::new(
                        UnitsAdapter::load(pending.clone()).expect("the rows load"),
                    )),
                )
            };
            let (transactions, accounts, units) = load(&self.pending);
            Ports {
                tags,
                reads: StoredTagReads {
                    transactions,
                    accounts,
                    units,
                },
            }
        }

        /// Settles the writes sent so far and returns the failures reported since the last call.
        fn failures(&self) -> Vec<WriteFailure> {
            self.writer.flush().expect("the writer is running");
            self.failures.try_iter().collect()
        }
    }

    /// `lib-transactions`' read check: each Tag is stored active.
    #[sqlx::test(migrations = "migrations/client")]
    async fn transactions_read_the_tags_rows(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        fn store(writer: &Writer, rows: Vec<TransactionTag>) {
            let tags = rows
                .into_iter()
                .map(|row| Tag {
                    id: row.id,
                    name: row.name,
                    color: row.color,
                    is_active: true,
                })
                .collect();
            TagsAdapter::seed(writer, tags).expect("the Tags are written");
        }
        lib_transactions::contract::tags_read_back(&mut TransactionReadsFactory::new(
            options,
            store,
            crate::test_support::transaction_reads,
        ));
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// An Account in the stub holding `unit`.
    fn account_in(unit: UnitId) -> AccountId {
        default_accounts()
            .into_iter()
            .find(|account| account.unit == unit)
            .expect("the stub has an Account in the Unit")
            .id
    }

    /// A Transaction on `account` and `date` with one Split of `amount` per entry, each carrying
    /// `tags`.
    fn transaction_carrying(
        account_id: AccountId,
        date: NaiveDate,
        amounts: &[&str],
        tags: &[TagId],
    ) -> Transaction {
        Transaction {
            id: TransactionId::new(),
            date,
            account_id,
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: amounts
                .iter()
                .map(|amount| Split {
                    amount: amount.parse().expect("a decimal"),
                    category_id: CategoryId::seed(7),
                    payee_id: None,
                    tag_ids: tags.to_vec(),
                })
                .collect(),
        }
    }

    impl PortsFactory for AdapterFactory {
        type Ports = Ports;

        fn create(&mut self, seed: Vec<Tag>) -> Ports {
            TagsAdapter::seed(&self.writer, seed).expect("the seed is written");
            self.ports(TagsAdapter::load(self.pending.clone()).expect("the seed loads"))
        }

        fn reload(&mut self, _ports: Ports) -> Ports {
            self.pending.commit();
            let fresh = TagsAdapter::load(self.pending.clone()).expect("the rows load");
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            self.ports(fresh)
        }

        fn commit(&mut self, _ports: &mut Ports) {
            self.pending.commit();
        }

        /// Stores Transactions whose Splits carry the Tag so the SQL answers `uses`: the sum on
        /// the first Split, nothing on the rest, the newest on the last Transaction. A `Mixed`
        /// total takes one Transaction in the base Unit and the rest in another.
        fn carry_tag(&mut self, _ports: &mut Ports, id: TagId, uses: TagUse) {
            let aud = account_in(UnitId::SEED_AUD);
            let Some(last_used) = uses.last_used else {
                return;
            };
            let accounts = match &uses.total {
                Total::Mixed => vec![aud, account_in(UnitId::SEED_VAS)],
                _ => vec![aud],
            };
            let sum = match &uses.total {
                Total::Single { amount, .. } => amount.to_string(),
                _ => "0".into(),
            };
            let transactions = (0..uses.transactions)
                .map(|n| {
                    let first = n == 0;
                    let amounts = [if first { sum.as_str() } else { "0" }];
                    let date = if n + 1 == uses.transactions {
                        last_used
                    } else {
                        last_used - chrono::Days::new(1 + n as u64)
                    };
                    transaction_carrying(accounts[n % accounts.len()], date, &amounts, &[id])
                })
                .collect();
            TransactionsAdapter::seed(&self.writer, transactions)
                .expect("the carrying Transactions are written");
        }

        fn break_writes(&mut self, _ports: &mut Ports) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_tags BEFORE INSERT ON tags
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut ports: Ports) -> (Ports, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Tags])
                .count();
            ports.tags.reload().expect("the rows load");
            (ports, failures)
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_added_tag_reloads_the_same(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::an_added_tag_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_edited_tag_reloads_the_same(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::an_edited_tag_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn deactivating_and_reactivating_reload_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::deactivating_and_reactivating_reload_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_name_is_unique_once_normalised(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_name_is_unique_once_normalised(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn tags_list_in_the_order_added(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::tags_list_in_the_order_added(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn usage_answers_the_figures_for_a_tag(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::usage_answers_the_figures_for_a_tag(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// The file refuses a Split carrying a Tag that isn't stored, and a Tag a Split still carries
    /// can't be deleted, which backstops the retag Tag delete and merge do first.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_a_split_carrying_no_stored_tag_and_deleting_one_a_split_carries(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let aud = account_in(UnitId::SEED_AUD);

        assert!(
            TransactionsAdapter::seed(
                &factory.writer,
                vec![transaction_carrying(aud, day(), &["-1"], &[TagId::new()])]
            )
            .is_err(),
            "no such Tag"
        );

        let mut ports = factory.create(default_tags());
        let carried = TagId::seed(2);
        TransactionsAdapter::seed(
            &factory.writer,
            vec![transaction_carrying(aud, day(), &["-1"], &[carried])],
        )
        .expect("a stored Tag may be carried");

        ports.tags.delete_tag(carried);
        factory.pending.commit();
        let failures = factory.failures();
        assert_eq!(failures.len(), 1, "the delete is refused");
        assert_eq!(failures[0].domains, [Domain::Tags]);
        ports.tags.reload().expect("the rows load");
        assert!(ports.tags().tag(carried).is_some(), "the Tag stays");

        let uncarried = TagId::seed(1);
        ports.tags.delete_tag(uncarried);
        factory.pending.commit();
        assert!(factory.failures().is_empty());
        let mut ports = factory.reload(ports);
        assert!(ports.tags().tag(uncarried).is_none(), "an unused Tag goes");
    }

    /// Two Splits of one Transaction carrying the Tag count the Transaction once and sum both.
    #[sqlx::test(migrations = "migrations/client")]
    async fn usage_counts_a_transaction_once_and_sums_its_splits(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let ports = factory.create(default_tags());
        let tags = &ports.reads;
        let shared = TagId::seed(2);
        TransactionsAdapter::seed(
            &factory.writer,
            vec![transaction_carrying(
                account_in(UnitId::SEED_AUD),
                day(),
                &["-10.50", "-4.50"],
                &[shared, shared],
            )],
        )
        .expect("the Transaction is written");

        let used = tags.tag_usage(shared);
        assert_eq!(used.transactions, 1);
        let unused = TagId::seed(1);
        let missing = TagId::seed(999);
        assert_eq!(
            tags.transactions_by_tag(&[shared, unused, missing]),
            HashMap::from([(shared, 1), (unused, 0), (missing, 0)]),
            "the batch counts the Transaction once and answers every id asked"
        );
        assert_eq!(used.last_used, Some(day()));
        assert_eq!(
            used.total,
            Total::Single {
                unit: "aud".into(),
                amount: Money(BigDecimal::from(-15)),
            }
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_database_loads_no_tags(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        let adapter = TagsAdapter::load(PendingWrites::new(writer)).expect("it loads");
        assert!(TagsAdapter::tags(&adapter).tags().is_empty());
    }
}
