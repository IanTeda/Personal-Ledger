//! # Account Update Operations
//!
//! Targeted update for the one field that actually changes after an account is
//! bootstrapped: its current refresh-token hash, rewritten on every rotation
//! (ADR-0010) or cleared (`None`) on logout/invalidation.

use lib_core as domain;

impl crate::Account {
    /// Replace this account's stored refresh-token hash.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no account exists with `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(
        name = "Update Account refresh_token_hash: ",
        level = "debug",
        skip(pool, refresh_token_hash),
        fields(account_id = %id, has_new_token = refresh_token_hash.is_some()),
    )]
    pub async fn update_refresh_token_hash(
        id: domain::RowID,
        refresh_token_hash: Option<&str>,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        let result = sqlx::query!(
            r#"UPDATE accounts SET refresh_token_hash = ? WHERE id = ?"#,
            refresh_token_hash,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Account {id} not found"
            )));
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
            id
        )
        .fetch_one(pool)
        .await?;

        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn update_refresh_token_hash_sets_a_new_value(pool: SqlitePool) {
        let account = crate::Account::mock().insert(&pool).await.unwrap();

        let updated =
            crate::Account::update_refresh_token_hash(account.id, Some("new-hash"), &pool)
                .await
                .unwrap();

        assert_eq!(updated.refresh_token_hash.as_deref(), Some("new-hash"));
    }

    #[sqlx::test]
    async fn update_refresh_token_hash_can_clear_it(pool: SqlitePool) {
        let account = crate::Account::mock().insert(&pool).await.unwrap();
        crate::Account::update_refresh_token_hash(account.id, Some("some-hash"), &pool)
            .await
            .unwrap();

        let cleared = crate::Account::update_refresh_token_hash(account.id, None, &pool)
            .await
            .unwrap();

        assert_eq!(cleared.refresh_token_hash, None);
    }

    #[sqlx::test]
    async fn update_refresh_token_hash_errors_when_account_missing(pool: SqlitePool) {
        let result =
            crate::Account::update_refresh_token_hash(lib_core::RowID::new(), Some("x"), &pool)
                .await;

        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }
}
