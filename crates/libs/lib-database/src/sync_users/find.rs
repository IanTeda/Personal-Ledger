//! # SyncUser Query Operations
//!
//! Read operations against the `sync_users` table -- looking up the sync user presented
//! at login (`find_by_username`), and fetching the single bootstrap sync user by
//! whichever consumer just needs "the" sync user rather than a specific username
//! (`find_only`).

use lib_core as domain;

impl crate::SyncUser {
    /// Find a sync user by its unique username.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find SyncUser by username: ", level = "debug", skip(pool))]
    pub async fn find_by_username(
        username: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Option<Self>> {
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
                WHERE username = ?
            "#,
            username
        )
        .fetch_optional(pool)
        .await?;

        Ok(sync_user)
    }

    /// Find the single bootstrap sync user, if one has been provisioned yet.
    ///
    /// Single-account this cycle (ADR-0010) -- callers that don't need a specific
    /// username (e.g. redeeming a refresh token) use this instead of guessing one.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find the single SyncUser: ", level = "debug", skip(pool))]
    pub async fn find_only(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Option<Self>> {
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
                LIMIT 1
            "#,
        )
        .fetch_optional(pool)
        .await?;

        Ok(sync_user)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn find_by_username_finds_an_existing_sync_user(pool: SqlitePool) {
        let sync_user = crate::SyncUser::mock().insert(&pool).await.unwrap();

        let found = crate::SyncUser::find_by_username(&sync_user.username, &pool)
            .await
            .unwrap();

        assert_eq!(found.map(|a| a.id), Some(sync_user.id));
    }

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn find_by_username_returns_none_when_missing(pool: SqlitePool) {
        let found = crate::SyncUser::find_by_username("does-not-exist", &pool)
            .await
            .unwrap();

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn find_only_returns_none_when_empty(pool: SqlitePool) {
        let found = crate::SyncUser::find_only(&pool).await.unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn find_only_returns_the_bootstrap_sync_user(pool: SqlitePool) {
        let sync_user = crate::SyncUser::mock().insert(&pool).await.unwrap();

        let found = crate::SyncUser::find_only(&pool).await.unwrap();

        assert_eq!(found.map(|a| a.id), Some(sync_user.id));
    }
}
