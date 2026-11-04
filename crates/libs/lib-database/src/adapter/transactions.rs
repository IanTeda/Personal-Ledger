//! The Transactions adapter: `lib-transactions`' repository port over the `transactions`, `splits`
//! and `split_tags` tables, `lib-bills`' `BillTransactions` read port from the same rows, and the
//! Splits half of `lib-categories`' `CategoryUsage`, counted in the file (ADR-0046, #926).
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
use lib_bills::{BillSplit, SplitRef};
use lib_core::{
    AccountId, BillScheduleEntryId, CategoryId, Money, PayeeId, TagId, TransactionId,
    TransactionStatus,
};
use lib_transactions::{Split, Transaction, TransactionRepository};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One Split carrying a Tag, with what the Transactions know of it: the rows a Tag's usage is
/// counted from (#994). Which Unit the Account holds is the Accounts' to say.
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedSplit {
    pub transaction: TransactionId,
    pub date: NaiveDate,
    pub account: AccountId,
    pub amount: Money,
}

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
            Self::seed_in(&mut conn, &transactions).await?;
            conn.commit().await?;
            Ok(())
        })
    }

    /// [`Self::seed`]'s writes on `conn`, so a seed of several domains commits them in one
    /// transaction (#886).
    ///
    /// # Errors
    /// Returns the error of a write.
    pub(crate) async fn seed_in(
        conn: &mut SqliteConnection,
        transactions: &[Transaction],
    ) -> Result<()> {
        for transaction in transactions {
            upsert(conn, transaction).await?;
        }
        Ok(())
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

impl TransactionsAdapter {
    /// The Splits filed under Category `id` in the file, blocking: what the last commit left, not
    /// a write still pending. It only words the Categories Dialogs, so a count that can't be read
    /// is logged and answered from the rows in memory, rather than as a 0 that would say none.
    pub fn category_splits(&self, id: CategoryId) -> usize {
        self.pending
            .writer()
            .run(move |pool| async move {
                let count = sqlx::query_scalar!(
                    r#"SELECT COUNT(*) AS "splits!: i64" FROM splits WHERE category_id = ?"#,
                    id
                )
                .fetch_one(&pool)
                .await?;
                usize::try_from(count).map_err(|_| {
                    Error::Validation(format!("Category {id}'s Splits count is {count}"))
                })
            })
            .unwrap_or_else(|error| {
                tracing::error!(%error, category = %id, "Failed to count the Splits filed under a Category");
                category_splits(&self.rows, id)
            })
    }
}

impl TransactionsAdapter {
    /// How many Transactions are booked to each of `accounts` that has any, counted in the file,
    /// blocking: what the last commit left, not a write still pending. Counts only `accounts`'
    /// Transactions, through `idx_transactions_account_id`, rather than grouping the whole table;
    /// the list is a Unit's Accounts or one, so the placeholders stay few. It only feeds a figure
    /// the Account Dialogs and the Units page show, so a count that can't be read is logged and
    /// answered as none.
    pub fn account_transaction_counts(&self, accounts: &[AccountId]) -> HashMap<AccountId, usize> {
        let accounts = accounts.to_vec();
        self.pending
            .writer()
            .run(move |pool| async move {
                let mut conn = pool.acquire().await?;
                count_account_transactions(&mut conn, &accounts).await
            })
            .inspect_err(|error| {
                tracing::error!(%error, "Failed to count the Transactions booked to Accounts");
            })
            .unwrap_or_default()
    }

    /// The Splits naming Payee `id` in the file, blocking: what the last commit left, not a write
    /// still pending. `None` when the count can't be read, logged, so a Payee that may be in use is
    /// never offered for deleting (#840).
    pub fn payee_splits(&self, id: PayeeId) -> Option<usize> {
        self.pending
            .writer()
            .run(move |pool| async move {
                let count = sqlx::query_scalar!(
                    r#"SELECT COUNT(*) AS "splits!: i64" FROM splits WHERE payee_id = ?"#,
                    id
                )
                .fetch_one(&pool)
                .await?;
                usize::try_from(count)
                    .map_err(|_| Error::Validation(format!("Payee {id}'s Splits count is {count}")))
            })
            .inspect_err(|error| {
                tracing::error!(%error, payee = %id, "Failed to count the Splits naming a Payee");
            })
            .ok()
    }

    /// The writes this adapter joins, and through them the file it reads.
    pub(crate) fn file(&self) -> &PendingWrites {
        &self.pending
    }

    /// The Splits carrying Tag `id` in the file, blocking: what the last commit left, not a write
    /// still pending. It only feeds figures the Tags page shows, so a read that can't be made is
    /// logged and answered as none (#862).
    pub fn tagged_splits(&self, id: TagId) -> Vec<TaggedSplit> {
        self.pending
            .writer()
            .run(move |pool| async move {
                let mut conn = pool.acquire().await?;
                read_tagged_splits(&mut conn, id).await
            })
            .inspect_err(|error| {
                tracing::error!(%error, tag = %id, "Failed to read the Splits carrying a Tag");
            })
            .unwrap_or_default()
    }

    /// Distinct Transactions per Tag, for every Tag a Split carries, in one grouped read of the
    /// file. A read that can't be made is logged and answered as none.
    pub fn tag_transaction_counts(&self) -> HashMap<TagId, usize> {
        self.pending
            .writer()
            .run(|pool| async move {
                let mut conn = pool.acquire().await?;
                count_tag_transactions(&mut conn).await
            })
            .inspect_err(|error| {
                tracing::error!(%error, "Failed to count the Transactions carrying each Tag");
            })
            .unwrap_or_default()
    }
}

/// The Splits among `transactions` carrying Tag `id`, one each however often the Split lists it:
/// how a store reads what it holds.
pub(crate) fn tagged_splits(transactions: &[Transaction], id: TagId) -> Vec<TaggedSplit> {
    transactions
        .iter()
        .flat_map(|transaction| {
            transaction
                .splits
                .iter()
                .filter(|split| split.tag_ids.contains(&id))
                .map(|split| TaggedSplit {
                    transaction: transaction.id,
                    date: transaction.date,
                    account: transaction.account_id,
                    amount: split.amount.clone(),
                })
        })
        .collect()
}

/// Distinct Transactions per Tag among `transactions`: how a store counts what it holds.
pub(crate) fn tag_transaction_counts(transactions: &[Transaction]) -> HashMap<TagId, usize> {
    let mut counts = HashMap::new();
    for transaction in transactions {
        let tags: std::collections::HashSet<TagId> = transaction
            .splits
            .iter()
            .flat_map(|split| split.tag_ids.iter().copied())
            .collect();
        for tag in tags {
            *counts.entry(tag).or_default() += 1;
        }
    }
    counts
}

/// The Transactions among `transactions` booked to each of `accounts` that has any: how a store
/// counts what it holds.
pub(crate) fn account_transaction_counts(
    transactions: &[Transaction],
    accounts: &[AccountId],
) -> HashMap<AccountId, usize> {
    let mut counts = HashMap::new();
    for transaction in transactions {
        if accounts.contains(&transaction.account_id) {
            *counts.entry(transaction.account_id).or_default() += 1;
        }
    }
    counts
}

/// The Splits among `transactions` naming Payee `id`: how a store counts what it holds.
pub(crate) fn payee_splits(transactions: &[Transaction], id: PayeeId) -> usize {
    transactions
        .iter()
        .flat_map(|transaction| &transaction.splits)
        .filter(|split| split.payee_id == Some(id))
        .count()
}

/// The Splits among `transactions` filed under Category `id`: how a store counts what it holds.
pub(crate) fn category_splits(transactions: &[Transaction], id: CategoryId) -> usize {
    transactions
        .iter()
        .flat_map(|transaction| &transaction.splits)
        .filter(|split| split.category_id == id)
        .count()
}

/// `transactions`' Splits dated `from` to `to`, as Bills reads them: the one place a store answers
/// `BillTransactions::splits_between` from.
pub(crate) fn splits_between(
    transactions: &[Transaction],
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<BillSplit> {
    transactions
        .iter()
        .filter(|row| (from..=to).contains(&row.date))
        .flat_map(bill_splits)
        .collect()
}

/// `transactions`' Splits dated `from` to `to`, as `lib-budgets`' `BudgetSplits` reads them, each
/// with its Transaction's date and Account, in the Transactions' order.
pub(crate) fn budget_splits(
    transactions: &[Transaction],
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<lib_budgets::BudgetSplit> {
    transactions
        .iter()
        .filter(|row| (from..=to).contains(&row.date))
        .flat_map(|row| {
            row.splits.iter().map(|split| lib_budgets::BudgetSplit {
                date: row.date,
                account_id: row.account_id,
                category_id: split.category_id,
                amount: split.amount.clone(),
                payee_id: split.payee_id,
            })
        })
        .collect()
}

/// `transactions` as `lib-documents`' `DocumentTransactions` reads them: each whole, with its
/// total and the Payee each Split names.
pub(crate) fn document_transactions(
    transactions: &[Transaction],
) -> Vec<lib_documents::DocumentTransaction> {
    transactions
        .iter()
        .map(|transaction| lib_documents::DocumentTransaction {
            id: transaction.id,
            date: transaction.date,
            amount: transaction.total(),
            account_id: transaction.account_id,
            payee_ids: transaction
                .splits
                .iter()
                .map(|split| split.payee_id)
                .collect(),
            memo: transaction.description.clone(),
        })
        .collect()
}

/// The Split `split` names among `transactions`, as Bills reads it.
pub(crate) fn paid_split(transactions: &[Transaction], split: SplitRef) -> Option<BillSplit> {
    let row = transactions
        .iter()
        .find(|row| row.id == split.transaction_id)?;
    bill_splits(row).nth(split.split_index)
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

    /// A deleted Transaction drops the Document Links to it.
    fn cascades_to(&self) -> Option<Domain> {
        matches!(self, Self::Delete(_)).then_some(Domain::Documents)
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

/// The counts [`TransactionsAdapter::account_transaction_counts`] reads.
async fn count_account_transactions(
    conn: &mut SqliteConnection,
    accounts: &[AccountId],
) -> Result<HashMap<AccountId, usize>> {
    if accounts.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = vec!["?"; accounts.len()].join(", ");
    let sql = format!(
        "SELECT account_id, COUNT(*) FROM transactions WHERE account_id IN ({placeholders}) \
         GROUP BY account_id"
    );
    let mut query = sqlx::query_as::<_, (AccountId, i64)>(&sql);
    for account in accounts {
        query = query.bind(*account);
    }
    query
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|(account_id, count)| {
            usize::try_from(count)
                .map(|count| (account_id, count))
                .map_err(|_| {
                    Error::Validation(format!(
                        "Account {account_id}'s Transactions count is {count}"
                    ))
                })
        })
        .collect()
}

/// The Splits [`TransactionsAdapter::tagged_splits`] reads, in the Transactions' order.
async fn read_tagged_splits(conn: &mut SqliteConnection, id: TagId) -> Result<Vec<TaggedSplit>> {
    let rows = sqlx::query!(
        r#"SELECT
               t.id         AS "transaction_id!: TransactionId",
               t.date       AS "date!: NaiveDate",
               t.account_id AS "account_id!: AccountId",
               s.amount     AS "amount!: Money"
           FROM splits s
           JOIN transactions t ON t.id = s.transaction_id
           WHERE EXISTS (
               SELECT 1 FROM split_tags st
               WHERE st.transaction_id = s.transaction_id
                 AND st.position = s.position
                 AND st.tag_id = ?
           )
           ORDER BY t.rowid, s.position"#,
        id,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| TaggedSplit {
            transaction: row.transaction_id,
            date: row.date,
            account: row.account_id,
            amount: row.amount,
        })
        .collect())
}

/// Distinct Transactions per Tag, for every Tag a Split carries, in one grouped read. A Tag
/// nothing carries has no row.
async fn count_tag_transactions(conn: &mut SqliteConnection) -> Result<HashMap<TagId, usize>> {
    let rows = sqlx::query!(
        r#"SELECT
               tag_id                          AS "tag_id!: TagId",
               COUNT(DISTINCT transaction_id)  AS "transactions!: i64"
           FROM split_tags
           GROUP BY tag_id"#
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.tag_id,
                usize::try_from(row.transactions).unwrap_or_default(),
            )
        })
        .collect())
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

    /// The adapter on one test database, with its pending batch. The stub's Accounts and Payees are
    /// stored first, which a Transaction and a Split name. A reload commits, loads afresh, and
    /// fails the check if the writer reported a failure, so a write SQLite refuses can't pass
    /// unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            crate::test_support::store_stub_references(&writer);
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
        let seeded = stub_transactions();
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

    fn stub_transactions() -> Vec<Transaction> {
        lib_ledger::Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date"))
            .transactions()
            .to_vec()
    }

    /// Bills reads the stored Splits flattened as the in-memory store flattens its own, after a
    /// reload as before it.
    #[sqlx::test(migrations = "migrations/client")]
    async fn bills_read_the_splits_back_after_a_reload(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let seeded = stub_transactions();
        let memory = InMemoryTransactions {
            transactions: seeded.clone(),
        };
        let adapter = factory.create(memory.clone());
        let adapter = factory.reload(adapter);

        let (from, to) = (NaiveDate::MIN, NaiveDate::MAX);
        let splits = splits_between(&adapter.rows, from, to);
        assert_eq!(splits, splits_between(&memory.transactions, from, to));
        assert_eq!(
            splits.len(),
            seeded.iter().map(|t| t.splits.len()).sum::<usize>()
        );

        let split = seeded
            .iter()
            .find(|transaction| transaction.is_split())
            .expect("the stub has a split Transaction");
        let window = splits_between(&adapter.rows, split.date, split.date);
        assert!(window.iter().all(|read| read.date == split.date));
        assert!(
            window
                .iter()
                .any(|read| read.split.transaction_id == split.id)
        );

        let second = SplitRef {
            transaction_id: split.id,
            split_index: 1,
        };
        let paid = paid_split(&adapter.rows, second).expect("its second Split is read");
        assert_eq!(Some(paid.clone()), paid_split(&memory.transactions, second));
        assert_eq!(
            (paid.date, paid.account_id, &paid.description),
            (split.date, split.account_id, &split.description)
        );
        assert_eq!(paid.amount, split.splits[1].amount);
        assert_eq!(paid.category_id, split.splits[1].category_id);
        assert_eq!(paid.payee_id, split.splits[1].payee_id);
        assert_eq!(
            paid_split(
                &adapter.rows,
                SplitRef {
                    transaction_id: TransactionId::new(),
                    split_index: 0,
                }
            ),
            None
        );
    }

    /// The count is the file's: a Split written is counted once committed, as the in-memory
    /// store counts its own rows.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_category_splits_are_counted_in_the_file(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let seeded = stub_transactions();
        let mut memory = InMemoryTransactions {
            transactions: seeded.clone(),
        };
        let mut adapter = factory.create(memory.clone());
        let category = seeded[0].splits[0].category_id;
        let before = adapter.category_splits(category);
        assert!(before > 0, "the stub files a Split under it");
        assert_eq!(
            before,
            crate::TransactionsStore::category_splits(&memory, category)
        );

        let mut booked = seeded[0].clone();
        booked.id = TransactionId::new();
        booked.splits.truncate(1);
        TransactionRepository::upsert_transaction(&mut adapter, booked.clone());
        TransactionRepository::upsert_transaction(&mut memory, booked);
        assert_eq!(
            adapter.category_splits(category),
            before,
            "a write still pending isn't in the file"
        );
        factory.commit(&mut adapter);

        assert_eq!(adapter.category_splits(category), before + 1);
        assert_eq!(
            crate::TransactionsStore::category_splits(&memory, category),
            before + 1
        );
        assert_eq!(adapter.category_splits(CategoryId::new()), 0);
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
