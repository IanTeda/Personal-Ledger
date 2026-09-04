//! # Account Query Operations
//!
//! Read operations against the `accounts` table -- looking up the account presented at
//! login (`find_by_username`), and fetching the single bootstrap account by whichever
//! consumer just needs "the" account rather than a specific username (`find_only`).

use lib_core as domain;

impl crate::Account {
    /// Find an account by its unique username.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Account by username: ", level = "debug", skip(pool))]
    pub async fn find_by_username(
        username: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
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
                WHERE username = ?
            "#,
            username
        )
        .fetch_optional(pool)
        .await?;

        Ok(account)
    }

    /// Find the single bootstrap account, if one has been provisioned yet.
    ///
    /// Single-account this cycle (ADR-0010) -- callers that don't need a specific
    /// username (e.g. redeeming a refresh token) use this instead of guessing one.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find the single Account: ", level = "debug", skip(pool))]
    pub async fn find_only(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Option<Self>> {
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
                LIMIT 1
            "#,
        )
        .fetch_optional(pool)
        .await?;

        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn find_by_username_finds_an_existing_account(pool: SqlitePool) {
        let account = crate::Account::mock().insert(&pool).await.unwrap();

        let found = crate::Account::find_by_username(&account.username, &pool)
            .await
            .unwrap();

        assert_eq!(found.map(|a| a.id), Some(account.id));
    }

    #[sqlx::test]
    async fn find_by_username_returns_none_when_missing(pool: SqlitePool) {
        let found = crate::Account::find_by_username("does-not-exist", &pool)
            .await
            .unwrap();

        assert!(found.is_none());
    }

    #[sqlx::test]
    async fn find_only_returns_none_when_empty(pool: SqlitePool) {
        let found = crate::Account::find_only(&pool).await.unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test]
    async fn find_only_returns_the_bootstrap_account(pool: SqlitePool) {
        let account = crate::Account::mock().insert(&pool).await.unwrap();

        let found = crate::Account::find_only(&pool).await.unwrap();

        assert_eq!(found.map(|a| a.id), Some(account.id));
    }
}
