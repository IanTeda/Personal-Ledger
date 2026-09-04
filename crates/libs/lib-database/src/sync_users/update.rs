//! # SyncUser Update Operations
//!
//! Targeted update for the one field that actually changes after a sync user is
//! bootstrapped: its current refresh-token hash, rewritten on every rotation
//! (ADR-0010) or cleared (`None`) on logout/invalidation.

use lib_core as domain;

impl crate::SyncUser {
    /// Replace this sync user's stored refresh-token hash.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no sync user exists with `id`, or
    /// an error if the underlying query fails.
    #[tracing::instrument(
        name = "Update SyncUser refresh_token_hash: ",
        level = "debug",
        skip(pool, refresh_token_hash),
        fields(sync_user_id = %id, has_new_token = refresh_token_hash.is_some()),
    )]
    pub async fn update_refresh_token_hash(
        id: domain::RowID,
        refresh_token_hash: Option<&str>,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        let result = sqlx::query!(
            r#"UPDATE sync_users SET refresh_token_hash = ? WHERE id = ?"#,
            refresh_token_hash,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "SyncUser {id} not found"
            )));
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
            id
        )
        .fetch_one(pool)
        .await?;

        Ok(sync_user)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn update_refresh_token_hash_sets_a_new_value(pool: SqlitePool) {
        let sync_user = crate::SyncUser::mock().insert(&pool).await.unwrap();

        let updated =
            crate::SyncUser::update_refresh_token_hash(sync_user.id, Some("new-hash"), &pool)
                .await
                .unwrap();

        assert_eq!(updated.refresh_token_hash.as_deref(), Some("new-hash"));
    }

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn update_refresh_token_hash_can_clear_it(pool: SqlitePool) {
        let sync_user = crate::SyncUser::mock().insert(&pool).await.unwrap();
        crate::SyncUser::update_refresh_token_hash(sync_user.id, Some("some-hash"), &pool)
            .await
            .unwrap();

        let cleared = crate::SyncUser::update_refresh_token_hash(sync_user.id, None, &pool)
            .await
            .unwrap();

        assert_eq!(cleared.refresh_token_hash, None);
    }

    #[sqlx::test(migrations = "migrations/sync-server")]
    async fn update_refresh_token_hash_errors_when_sync_user_missing(pool: SqlitePool) {
        let result =
            crate::SyncUser::update_refresh_token_hash(lib_core::RowID::new(), Some("x"), &pool)
                .await;

        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }
}
