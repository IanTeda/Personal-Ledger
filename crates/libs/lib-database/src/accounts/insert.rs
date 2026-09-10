//! # Accounts Insert Operations

use lib_core as domain;

impl crate::Accounts {
    /// Insert this Account into the database.
    ///
    /// # Errors
    /// Returns an error if the underlying INSERT or read-back SELECT fails — including a
    /// foreign-key violation if `unit_id` doesn't reference a real Unit (SQLite's
    /// `foreign_keys` pragma, enabled for every connection).
    #[tracing::instrument(
        name = "Insert new Account into database: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id, name = %self.name),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Self> {
        let insert_result = sqlx::query!(
            r#"
                INSERT INTO accounts (id, name, account_type, unit_id, starting_balance, is_active, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.name,
            self.account_type,
            self.unit_id,
            self.starting_balance,
            self.is_active,
            self.created_on,
            self.updated_on
        )
        .execute(pool)
        .await;

        match insert_result {
            Ok(result) => {
                if result.rows_affected() != 1 {
                    tracing::warn!(
                        "INSERT operation affected {} rows instead of 1 for account: {}",
                        result.rows_affected(),
                        self.name
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert account {}: {}", self.name, e);
                return Err(e.into());
            }
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::Error::NotFound(format!("Account {} not found after insert", self.id))
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_unit(pool: &SqlitePool) -> lib_core::RowID {
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        unit.id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_persists_and_reads_back_an_account(pool: SqlitePool) {
        let unit_id = seed_unit(&pool).await;
        let account = crate::Accounts::mock(unit_id);

        let inserted = account.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, account.id);
        assert_eq!(inserted.name, account.name);
        assert_eq!(inserted.unit_id, unit_id);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_rejects_an_unknown_unit_id(pool: SqlitePool) {
        let account = crate::Accounts::mock(lib_core::RowID::new());

        let result = account.insert(&pool).await;

        assert!(
            result.is_err(),
            "foreign_keys pragma should reject the unknown unit_id"
        );
    }
}
