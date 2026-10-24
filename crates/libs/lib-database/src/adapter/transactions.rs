//! The Transactions adapter: `lib-transactions`' repository port over the `transactions`, `splits`
//! and `split_tags` tables (ADR-0046).
//!
//! It loads every row into memory, blocking, and serves reads from there. A port write changes
//! memory at once and joins the [`PendingWrites`] every adapter shares, which the bin commits
//! after each call, so a Transaction and the rows another domain writes with it land together.
//!
//! A Transaction is saved whole: its row, then its Splits and their Tags written afresh, so an
//! edit that drops a Split leaves none behind. A Split's Match to a Bill Schedule entry is the
//! Bills adapter's to write, so a rewrite keeps it on the Split at the same position. Rows load in the order first written, which keeps
//! the stub's newest-first order and puts an import after it, as the Ledger read copy has them.

use std::collections::HashMap;

use chrono::NaiveDate;
use lib_bills::{BillSplit, BillTransactions, SplitRef};
use lib_core::{
    AccountId, BillScheduleEntryId, CategoryId, Money, PayeeId, TagId, TransactionId,
    TransactionStatus,
};
use lib_transactions::{Split, Transaction, TransactionRepository};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One port write, saved in the order it was made.
#[derive(Debug, Clone)]
enum TransactionsChange {
    Upsert(Transaction),
    Delete(TransactionId),
}

/// The Transactions domain, held in memory and saved through the shared writer.
pub struct TransactionsAdapter {
    rows: Vec<Transaction>,
    pending: PendingWrites,
}

impl TransactionsAdapter {
    /// Loads every Transaction with its Splits and Tags through `pending`'s writer, blocking until
    /// they are read. The adapter's writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read, or [`Error::Validation`] for a Split with no Transaction.
    #[tracing::instrument(name = "Load the Transactions", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self { rows, pending })
    }

    /// Writes `transactions` in one transaction, blocking until it lands. Each is an upsert, so
    /// seeding again leaves the same rows.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Transactions", level = "debug", skip_all, err)]
    pub fn seed(writer: &Writer, transactions: Vec<Transaction>) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            for transaction in &transactions {
                upsert(&mut conn, transaction).await?;
            }
            conn.commit().await?;
            Ok(())
        })
    }

    /// Drops the Transactions changes not yet committed and loads the rows again, blocking. For
    /// after a [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Transactions`], so
    /// memory matches the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Transactions", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.rows = fresh.rows;
        self.pending.discard(Domain::Transactions);
        Ok(())
    }
}

impl TransactionRepository for TransactionsAdapter {
    fn transactions(&self) -> Vec<Transaction> {
        self.rows.clone()
    }

    fn upsert_transaction(&mut self, transaction: Transaction) {
        match self.rows.iter_mut().find(|row| row.id == transaction.id) {
            Some(row) => *row = transaction.clone(),
            None => self.rows.push(transaction.clone()),
        }
        self.pending.push(TransactionsChange::Upsert(transaction));
    }

    fn delete_transaction(&mut self, id: TransactionId) {
        self.rows.retain(|row| row.id != id);
        self.pending.push(TransactionsChange::Delete(id));
    }
}

/// Bills' read port, answered from the rows in memory, each Split flattened as Bills reads it.
impl BillTransactions for TransactionsAdapter {
    fn splits_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<BillSplit> {
        self.rows
            .iter()
            .filter(|row| (from..=to).contains(&row.date))
            .flat_map(bill_splits)
            .collect()
    }

    fn paid_split(&self, split: SplitRef) -> Option<BillSplit> {
        let row = self
            .rows
            .iter()
            .find(|row| row.id == split.transaction_id)?;
        bill_splits(row).nth(split.split_index)
    }
}

/// `transaction`'s Splits as Bills reads them, each with its Transaction's date, Account and memo.
fn bill_splits(transaction: &Transaction) -> impl Iterator<Item = BillSplit> + '_ {
    transaction
        .splits
        .iter()
        .enumerate()
        .map(|(split_index, split)| BillSplit {
            split: SplitRef {
                transaction_id: transaction.id,
                split_index,
            },
            date: transaction.date,
            account_id: transaction.account_id,
            description: transaction.description.clone(),
            amount: split.amount.clone(),
            category_id: split.category_id,
            payee_id: split.payee_id,
        })
}

impl Change for TransactionsChange {
    fn domain(&self) -> Domain {
        Domain::Transactions
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::Upsert(transaction) => upsert(conn, &transaction).await,
                Self::Delete(id) => {
                    // Its Splits and their Tags go with it, by the schema's cascade.
                    sqlx::query!("DELETE FROM transactions WHERE id = ?", id)
                        .execute(conn)
                        .await?;
                    Ok(())
                }
            }
        })
    }
}

async fn upsert(conn: &mut SqliteConnection, transaction: &Transaction) -> Result<()> {
    sqlx::query!(
        "INSERT INTO transactions (id, date, account_id, description, status, is_flagged)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             date = excluded.date,
             account_id = excluded.account_id,
             description = excluded.description,
             status = excluded.status,
             is_flagged = excluded.is_flagged",
        transaction.id,
        transaction.date,
        transaction.account_id,
        transaction.description,
        transaction.status,
        transaction.is_flagged,
    )
    .execute(&mut *conn)
    .await?;
    // The Splits are written afresh, and their Tags go with the old ones by the cascade. Their
    // Matches are kept by position.
    let matches: HashMap<i64, BillScheduleEntryId> = sqlx::query!(
        r#"SELECT position, bill_schedule_entry_id AS "entry_id!: BillScheduleEntryId"
           FROM splits
           WHERE transaction_id = ? AND bill_schedule_entry_id IS NOT NULL"#,
        transaction.id
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| (row.position, row.entry_id))
    .collect();
    sqlx::query!(
        "DELETE FROM splits WHERE transaction_id = ?",
        transaction.id
    )
    .execute(&mut *conn)
    .await?;
    for (position, split) in (0_i64..).zip(&transaction.splits) {
        let entry_id = matches.get(&position);
        sqlx::query!(
            "INSERT INTO splits
                 (transaction_id, position, amount, category_id, payee_id, bill_schedule_entry_id)
             VALUES (?, ?, ?, ?, ?, ?)",
            transaction.id,
            position,
            split.amount,
            split.category_id,
            split.payee_id,
            entry_id,
        )
        .execute(&mut *conn)
        .await?;
        for (tag_position, tag_id) in (0_i64..).zip(&split.tag_ids) {
            sqlx::query!(
                "INSERT INTO split_tags (transaction_id, position, tag_position, tag_id)
                 VALUES (?, ?, ?, ?)",
                transaction.id,
                position,
                tag_position,
                tag_id,
            )
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

async fn read(conn: &mut SqliteConnection) -> Result<Vec<Transaction>> {
    let mut tags: HashMap<(TransactionId, i64), Vec<TagId>> = HashMap::new();
    for row in sqlx::query!(
        r#"SELECT
               transaction_id AS "transaction_id!: TransactionId",
               position,
               tag_id         AS "tag_id!: TagId"
           FROM split_tags
           ORDER BY transaction_id, position, tag_position"#
    )
    .fetch_all(&mut *conn)
    .await?
    {
        tags.entry((row.transaction_id, row.position))
            .or_default()
            .push(row.tag_id);
    }

    let mut splits: HashMap<TransactionId, Vec<Split>> = HashMap::new();
    for row in sqlx::query!(
        r#"SELECT
               transaction_id AS "transaction_id!: TransactionId",
               position,
               amount         AS "amount!: Money",
               category_id    AS "category_id!: CategoryId",
               payee_id       AS "payee_id?: PayeeId"
           FROM splits
           ORDER BY transaction_id, position"#
    )
    .fetch_all(&mut *conn)
    .await?
    {
        splits.entry(row.transaction_id).or_default().push(Split {
            amount: row.amount,
            category_id: row.category_id,
            payee_id: row.payee_id,
            tag_ids: tags
                .remove(&(row.transaction_id, row.position))
                .unwrap_or_default(),
        });
    }

    let transactions: Vec<Transaction> = sqlx::query!(
        r#"SELECT
               id          AS "id!: TransactionId",
               date        AS "date!: NaiveDate",
               account_id  AS "account_id!: AccountId",
               description,
               status      AS "status!: TransactionStatus",
               is_flagged
           FROM transactions
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| Transaction {
        splits: splits.remove(&row.id).unwrap_or_default(),
        id: row.id,
        date: row.date,
        account_id: row.account_id,
        status: row.status,
        is_flagged: row.is_flagged,
        description: row.description,
    })
    .collect();

    // The foreign keys make these unreachable; a file written some other way is refused.
    if let Some(orphan) = splits.keys().next() {
        return Err(Error::Validation(format!(
            "Split for Transaction {orphan}, which isn't stored"
        )));
    }
    Ok(transactions)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use lib_transactions::contract::{self, PortsFactory, snapshot};
    use lib_transactions::fake::InMemoryTransactions;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::PayeesAdapter;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;

    /// The adapter on one test database, with its pending batch. The stub's Payees are stored
    /// first, which a Split names. A reload commits, loads afresh, and fails the check if the
    /// writer reported a failure, so a write SQLite refuses can't pass unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            PayeesAdapter::seed(&writer, crate::seed::stub_payees())
                .expect("the stub Payees are written");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }
    }

    impl PortsFactory for AdapterFactory {
        type Ports = TransactionsAdapter;

        fn create(&mut self, seed: InMemoryTransactions) -> TransactionsAdapter {
            TransactionsAdapter::seed(&self.writer, seed.transactions)
                .expect("the seed is written");
            TransactionsAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reload(&mut self, _ports: TransactionsAdapter) -> TransactionsAdapter {
            self.pending.commit();
            let fresh = TransactionsAdapter::load(self.pending.clone()).expect("the rows load");
            if let Ok(failure) = self.failures.try_recv() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut TransactionsAdapter) {
            self.pending.commit();
        }

        fn break_writes(&mut self, _ports: &mut TransactionsAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_transactions BEFORE INSERT ON transactions
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut ports: TransactionsAdapter) -> (TransactionsAdapter, usize) {
            self.writer.flush().expect("the writer is running");
            let mut failures = 0;
            while let Ok(failure) = self.failures.try_recv() {
                if failure.domains == [Domain::Transactions] {
                    failures += 1;
                }
            }
            ports.reload().expect("the rows load");
            (ports, failures)
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn splits_and_tags_round_trip(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::splits_and_tags_round_trip(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_deleted_transaction_stays_deleted(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_deleted_transaction_stays_deleted(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// The contract can't see the tables, so this checks a delete leaves no Split or Tag behind.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_delete_takes_its_splits_and_tags_with_it(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut adapter = factory.create(InMemoryTransactions::default());
        let seeded =
            lib_ledger::Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date"))
                .transactions()
                .to_vec();
        let tagged = seeded
            .iter()
            .find(|transaction| transaction.splits.iter().any(|s| !s.tag_ids.is_empty()))
            .expect("the stub tags a Split")
            .clone();
        adapter.upsert_transaction(tagged.clone());
        factory.commit(&mut adapter);
        adapter.delete_transaction(tagged.id);
        let adapter = factory.reload(adapter);

        assert!(snapshot(&adapter).is_empty());
        let left: (i64, i64) = factory
            .writer
            .run(|pool| async move {
                Ok(sqlx::query_as(
                    "SELECT (SELECT COUNT(*) FROM splits), (SELECT COUNT(*) FROM split_tags)",
                )
                .fetch_one(&pool)
                .await?)
            })
            .expect("the counts read");
        assert_eq!(left, (0, 0));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_database_loads_no_transactions(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        let adapter = TransactionsAdapter::load(PendingWrites::new(writer)).expect("it loads");
        assert!(adapter.transactions().is_empty());
    }
}
