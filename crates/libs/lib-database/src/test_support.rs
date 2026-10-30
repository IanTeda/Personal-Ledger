//! Rows the legacy readers' tests total ([`crate::Payees::totals`],
//! [`crate::Budgets::current_progress`]) and the Balance Checks and Budgets name, written straight
//! to the tables since the Transactions, Accounts and Categories adapters take their domain crates'
//! ids. Also the factory `lib-transactions`' read checks run each adapter on.

use std::sync::mpsc;

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

use crate::commit::{Commit, PendingWrites};
use crate::writer::{WriteFailure, Writer};

/// An adapter on one test database, as `lib-transactions`' read checks run it: `store` writes the
/// rows they hand over as their own domain's, with whatever those rows name, and `load` loads the
/// adapter that answers them. A reload commits, loads afresh, and fails the check if the writer
/// reported a failure, so a write SQLite refuses can't pass unseen.
pub(crate) struct TransactionReadsFactory<Row, Reads> {
    writer: Writer,
    pending: PendingWrites,
    failures: mpsc::Receiver<WriteFailure>,
    store: fn(&Writer, Vec<Row>),
    load: fn(PendingWrites) -> Reads,
}

impl<Row, Reads> TransactionReadsFactory<Row, Reads> {
    pub(crate) fn new(
        options: SqliteConnectOptions,
        store: fn(&Writer, Vec<Row>),
        load: fn(PendingWrites) -> Reads,
    ) -> Self {
        let (writer, failures) = Writer::spawn(options).expect("the test database opens");
        Self {
            pending: PendingWrites::new(writer.clone()),
            writer,
            failures,
            store,
            load,
        }
    }
}

impl<Row, Reads> lib_transactions::contract::ReadsFactory<Row>
    for TransactionReadsFactory<Row, Reads>
{
    type Reads = Reads;

    fn create(&mut self, rows: Vec<Row>) -> Reads {
        (self.store)(&self.writer, rows);
        (self.load)(self.pending.clone())
    }

    fn reload(&mut self, _reads: Reads) -> Reads {
        self.pending.commit();
        let fresh = (self.load)(self.pending.clone());
        self.writer.flush().expect("the writer is running");
        if let Some(failure) = self.failures.try_iter().next() {
            panic!("a write failed: {failure:?}");
        }
        fresh
    }
}

/// Stores the stub's Units, Institutions, Accounts, Categories and Tags, which an adapter test's
/// Transactions, Bill Plans and Payees name by foreign key (#853, #858, #862).
pub(crate) fn store_stub_references(writer: &Writer) {
    crate::UnitsAdapter::seed_from(
        writer,
        lib_units::fake::InMemoryPorts::stub(chrono::Utc::now()),
    )
    .expect("the stub Units are written");
    crate::InstitutionsAdapter::seed(writer, crate::seed::stub_institutions())
        .expect("the stub Institutions are written");
    crate::AccountsAdapter::seed(writer, crate::seed::stub_accounts())
        .expect("the stub Accounts are written");
    crate::CategoriesAdapter::seed(writer, crate::seed::stub_categories())
        .expect("the stub Categories are written");
    crate::TagsAdapter::seed(writer, crate::seed::stub_tags()).expect("the stub Tags are written");
}

/// Stores a top-level Category of `category_type`, returning its id.
pub(crate) async fn insert_category(
    pool: &SqlitePool,
    category_type: lib_core::CategoryTypes,
) -> lib_core::RowID {
    let id = lib_core::RowID::new();
    sqlx::query("INSERT INTO categories (id, name, category_type) VALUES (?, 'Test category', ?)")
        .bind(id)
        .bind(category_type)
        .execute(pool)
        .await
        .expect("the Category is written");
    id
}

/// Stores a Bank Account in Unit `unit_id` with the Personal Institution, which it stores first if
/// it isn't, returning its id.
pub(crate) async fn insert_account(pool: &SqlitePool, unit_id: lib_core::RowID) -> lib_core::RowID {
    let institution = lib_core::InstitutionId::SEED_PERSONAL;
    sqlx::query(
        "INSERT OR IGNORE INTO institutions (id, name, products) VALUES (?, 'Personal', 'cash')",
    )
    .bind(institution)
    .execute(pool)
    .await
    .expect("the Institution is written");
    let id = lib_core::RowID::new();
    sqlx::query(
        "INSERT INTO accounts (id, name, institution_id, account_type, unit_id, balance, opened_at)
         VALUES (?, 'Test account', ?, 'bank', ?, '0', '2026-01-01')",
    )
    .bind(id)
    .bind(institution)
    .bind(unit_id)
    .execute(pool)
    .await
    .expect("the Account is written");
    id
}

/// One stored Transaction with a single Split.
pub(crate) struct OneSplit {
    pub(crate) account_id: lib_core::RowID,
    pub(crate) category_id: lib_core::RowID,
    pub(crate) payee_id: Option<lib_core::RowID>,
    pub(crate) amount: &'static str,
    pub(crate) date: chrono::NaiveDate,
}

impl OneSplit {
    pub(crate) fn new(
        account_id: lib_core::RowID,
        category_id: lib_core::RowID,
        amount: &'static str,
        date: chrono::NaiveDate,
    ) -> Self {
        Self {
            account_id,
            category_id,
            payee_id: None,
            amount,
            date,
        }
    }

    pub(crate) async fn insert(self, pool: &SqlitePool) {
        let id = lib_core::RowID::new();
        sqlx::query("INSERT INTO transactions (id, date, account_id) VALUES (?, ?, ?)")
            .bind(id)
            .bind(self.date)
            .bind(self.account_id)
            .execute(pool)
            .await
            .expect("the Transaction is written");
        sqlx::query(
            "INSERT INTO splits (transaction_id, position, amount, category_id, payee_id)
             VALUES (?, 0, ?, ?, ?)",
        )
        .bind(id)
        .bind(self.amount)
        .bind(self.category_id)
        .bind(self.payee_id)
        .execute(pool)
        .await
        .expect("the Split is written");
    }
}
