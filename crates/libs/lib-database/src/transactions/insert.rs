//! # Transactions Insert Operations

use lib_core as domain;

impl crate::Transactions {
    /// Insert this Transaction into the database.
    ///
    /// # Errors
    /// Returns an error if the underlying INSERT or read-back SELECT fails — including a
    /// foreign-key violation if `category_id`/`account_id` don't reference real rows
    /// (SQLite's `foreign_keys` pragma, enabled for every connection).
    #[tracing::instrument(
        name = "Insert new Transaction into database: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        let insert_result = sqlx::query!(
            r#"
                INSERT INTO transactions (id, date, amount, category_id, account_id, payee_id, description, status, is_flagged, updated_on)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.date,
            self.amount,
            self.category_id,
            self.account_id,
            self.payee_id,
            self.description,
            self.status,
            self.is_flagged,
            self.updated_on
        )
        .execute(pool)
        .await;

        match insert_result {
            Ok(result) => {
                if result.rows_affected() != 1 {
                    tracing::warn!(
                        "INSERT operation affected {} rows instead of 1 for transaction: {}",
                        result.rows_affected(),
                        self.id
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert transaction {}: {}", self.id, e);
                return Err(e.into());
            }
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!(
                "Transaction {} not found after insert",
                self.id
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_category_and_account(pool: &SqlitePool) -> (lib_core::RowID, lib_core::RowID) {
        let category = crate::Categories::mock().insert(pool).await.unwrap();
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        let account = crate::Accounts::mock(unit.id).insert(pool).await.unwrap();
        (category.id, account.id)
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_persists_and_reads_back_a_transaction(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        let transaction = crate::Transactions::mock(category_id, account_id);

        let inserted = transaction.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, transaction.id);
        assert_eq!(inserted.category_id, category_id);
        assert_eq!(inserted.account_id, account_id);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_rejects_an_unknown_account_id(pool: SqlitePool) {
        let (category_id, _account_id) = seed_category_and_account(&pool).await;
        let transaction = crate::Transactions::mock(category_id, lib_core::RowID::new());

        let result = transaction.insert(&pool).await;

        assert!(
            result.is_err(),
            "foreign_keys pragma should reject the unknown account_id"
        );
    }
}
