//! Rows the legacy readers' tests total ([`crate::Payees::totals`],
//! [`crate::Budgets::current_progress`]) and the Balance Checks and Budgets name, written straight
//! to the tables since the Transactions, Accounts and Categories adapters take their domain crates'
//! ids.

use sqlx::SqlitePool;

use crate::writer::Writer;

/// Stores the stub's Units, Institutions, Accounts and Categories, which an adapter test's
/// Transactions, Bill Plans and Payees name by foreign key (#853, #858).
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

/// Stores a Bank Account in Unit `unit_id` with no Institution, returning its id.
pub(crate) async fn insert_account(pool: &SqlitePool, unit_id: lib_core::RowID) -> lib_core::RowID {
    let id = lib_core::RowID::new();
    sqlx::query(
        "INSERT INTO accounts (id, name, account_type, unit_id, balance, opened_at)
         VALUES (?, 'Test account', 'bank', ?, '0', '2026-01-01')",
    )
    .bind(id)
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
