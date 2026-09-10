//! # SyncUser Insert Operations
//!
//! Provides database insertion for sync user records -- bootstrapping the Sync
//! Server's single auth account on first run (ADR-0010).

use lib_core as domain;

impl crate::SyncUser {
    /// Insert this sync user into the durable user store.
    ///
    /// # Errors
    /// Returns an error if the underlying INSERT or read-back SELECT fails (including a
    /// unique-constraint violation on `username`).
    #[tracing::instrument(
        name = "Insert new SyncUser into database: ",
        level = "debug",
        skip(self, pool),
        fields(id = % self.id, username = % self.username),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Self> {
        tracing::trace!(
            "Starting SyncUser insert for {} (id: {})",
            self.username,
            self.id
        );

        let insert_result = sqlx::query!(
            r#"
                INSERT INTO sync_users (id, username, password_hash, refresh_token_hash, created_on, updated_on)
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
                        "INSERT operation affected {} rows instead of 1 for sync user: {}",
                        result.rows_affected(),
                        self.username
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert sync user {}: {}", self.username, e);
                return Err(e.into());
            }
        }

        let sync_user = sqlx::query_as!(
            crate::SyncUser,
            r#"
                SELECT
                    id                   AS "id!: domain::RowID",
                    username,
                    password_hash,
                    refresh_token_hash,
                    created_on           AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on           AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM sync_users
                WHERE id = ?
            "#,
            self.id
        )
        .fetch_one(pool)
        .await?;

        tracing::trace!("SyncUser inserted and read back: {}", sync_user.username);

        Ok(sync_user)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn insert_persists_and_reads_back_a_sync_user(pool: SqlitePool) {
        let sync_user = crate::SyncUser::mock();

        let inserted = sync_user.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, sync_user.id);
        assert_eq!(inserted.username, sync_user.username);
        assert_eq!(inserted.password_hash, sync_user.password_hash);
        assert_eq!(inserted.refresh_token_hash, None);
    }

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn insert_rejects_a_duplicate_username(pool: SqlitePool) {
        let sync_user = crate::SyncUser::mock();
        sync_user.insert(&pool).await.unwrap();

        let duplicate = crate::sync_users::SyncUserBuilder::new()
            .with_id(lib_core::RowID::new())
            .with_username(sync_user.username.clone())
            .with_password_hash("$argon2id$v=19$m=19456,t=2,p=1$other$other".to_string())
            .build()
            .unwrap();

        let result = duplicate.insert(&pool).await;
        assert!(result.is_err());
    }
}
