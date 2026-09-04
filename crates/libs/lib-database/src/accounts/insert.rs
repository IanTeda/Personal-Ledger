//! # Account Insert Operations
//!
//! Provides database insertion for account records -- bootstrapping the Sync Server's
//! single auth account on first run (ADR-0010).

use lib_core as domain;

impl crate::Account {
    /// Insert this account into the durable user store.
    ///
    /// # Errors
    /// Returns an error if the underlying INSERT or read-back SELECT fails (including a
    /// unique-constraint violation on `username`).
    #[tracing::instrument(
        name = "Insert new Account into database: ",
        level = "debug",
        skip(self, pool),
        fields(id = % self.id, username = % self.username),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        tracing::trace!(
            "Starting Account insert for {} (id: {})",
            self.username,
            self.id
        );

        let insert_result = sqlx::query!(
            r#"
                INSERT INTO accounts (id, username, password_hash, refresh_token_hash, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.username,
            self.password_hash,
            self.refresh_token_hash,
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
                        self.username
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert account {}: {}", self.username, e);
                return Err(e.into());
            }
        }

        let account = sqlx::query_as!(
            crate::Account,
            r#"
                SELECT
                    id                   AS "id!: domain::RowID",
                    username,
                    password_hash,
                    refresh_token_hash,
                    created_on           AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on           AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM accounts
                WHERE id = ?
            "#,
            self.id
        )
        .fetch_one(pool)
        .await?;

        tracing::trace!("Account inserted and read back: {}", account.username);

        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn insert_persists_and_reads_back_an_account(pool: SqlitePool) {
        let account = crate::Account::mock();

        let inserted = account.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, account.id);
        assert_eq!(inserted.username, account.username);
        assert_eq!(inserted.password_hash, account.password_hash);
        assert_eq!(inserted.refresh_token_hash, None);
    }

    #[sqlx::test]
    async fn insert_rejects_a_duplicate_username(pool: SqlitePool) {
        let account = crate::Account::mock();
        account.insert(&pool).await.unwrap();

        let duplicate = crate::accounts::AccountBuilder::new()
            .with_id(lib_core::RowID::new())
            .with_username(account.username.clone())
            .with_password_hash("$argon2id$v=19$m=19456,t=2,p=1$other$other".to_string())
            .build()
            .unwrap();

        let result = duplicate.insert(&pool).await;
        assert!(result.is_err());
    }
}
