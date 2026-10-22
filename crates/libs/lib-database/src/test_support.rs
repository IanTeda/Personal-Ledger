//! Rows the legacy readers' tests total ([`crate::Accounts::balance`],
//! [`crate::Categories::totals`], [`crate::Payees::totals`], [`crate::Budgets::current_progress`]),
//! written straight to the tables since the Transactions adapter takes `lib-transactions`' ids.

use sqlx::SqlitePool;

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
