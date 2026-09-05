//! # Balance Checks Insert Operations

impl crate::BalanceChecks {
    /// Insert this Balance Check into the database.
    ///
    /// # Errors
    /// Returns an error if the underlying INSERT or read-back SELECT fails — including a
    /// foreign-key violation if `account_id` doesn't reference a real Account (SQLite's
    /// `foreign_keys` pragma, enabled for every connection).
    #[tracing::instrument(
        name = "Insert new Balance Check into database: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id, account_id = %self.account_id),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        let insert_result = sqlx::query!(
            r#"
                INSERT INTO balance_checks (id, account_id, date, asserted_balance, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.account_id,
            self.date,
            self.asserted_balance,
            self.created_on,
            self.updated_on
        )
        .execute(pool)
        .await;

        match insert_result {
            Ok(result) => {
                if result.rows_affected() != 1 {
                    tracing::warn!(
                        "INSERT operation affected {} rows instead of 1 for balance check: {}",
                        result.rows_affected(),
                        self.id
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert balance check {}: {}", self.id, e);
                return Err(e.into());
            }
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!(
                "Balance Check {} not found after insert",
                self.id
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_account(pool: &SqlitePool) -> lib_core::RowID {
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        crate::Accounts::mock(unit.id)
            .insert(pool)
            .await
            .unwrap()
            .id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_persists_and_reads_back_a_balance_check(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let balance_check = crate::BalanceChecks::mock(account_id);

        let inserted = balance_check.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, balance_check.id);
        assert_eq!(inserted.account_id, account_id);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_rejects_an_unknown_account_id(pool: SqlitePool) {
        let balance_check = crate::BalanceChecks::mock(lib_core::RowID::new());

        let result = balance_check.insert(&pool).await;

        assert!(
            result.is_err(),
            "foreign_keys pragma should reject the unknown account_id"
        );
    }
}
