//! # Accounts Query Operations

use lib_core as domain;

impl crate::Accounts {
    /// Find an Account by its id.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Account by id: ", level = "debug", skip(pool))]
    pub async fn find_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let account = sqlx::query_as!(
            crate::Accounts,
            r#"
                SELECT
                    id                AS "id!: domain::RowID",
                    name,
                    account_type      AS "account_type!: domain::AccountType",
                    unit_id           AS "unit_id!: domain::RowID",
                    starting_balance  AS "starting_balance!: domain::Money",
                    is_active,
                    created_on        AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on        AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM accounts
                WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(account)
    }

    /// List every Account, ordered by name.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all Accounts: ", level = "debug", skip(pool))]
    pub async fn find_all(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Vec<Self>> {
        let accounts = sqlx::query_as!(
            crate::Accounts,
            r#"
                SELECT
                    id                AS "id!: domain::RowID",
                    name,
                    account_type      AS "account_type!: domain::AccountType",
                    unit_id           AS "unit_id!: domain::RowID",
                    starting_balance  AS "starting_balance!: domain::Money",
                    is_active,
                    created_on        AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on        AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM accounts
                ORDER BY name ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    /// List every active Account, ordered by name — a picker source (e.g. Balance Checks'
    /// create form) for entities that shouldn't default to an inactive Account.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all active Accounts: ", level = "debug", skip(pool))]
    pub async fn find_all_active(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let accounts = sqlx::query_as!(
            crate::Accounts,
            r#"
                SELECT
                    id                AS "id!: domain::RowID",
                    name,
                    account_type      AS "account_type!: domain::AccountType",
                    unit_id           AS "unit_id!: domain::RowID",
                    starting_balance  AS "starting_balance!: domain::Money",
                    is_active,
                    created_on        AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on        AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM accounts
                WHERE is_active = TRUE
                ORDER BY name ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    /// List Accounts with pagination, alongside the total (unpaginated) row count.
    ///
    /// # Errors
    /// Returns an error if either query fails.
    #[tracing::instrument(name = "Find Accounts with pagination: ", level = "debug", skip(pool))]
    pub async fn find_all_with_pagination(
        offset: i64,
        limit: i64,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i64)> {
        let accounts = sqlx::query_as!(
            crate::Accounts,
            r#"
                SELECT
                    id                AS "id!: domain::RowID",
                    name,
                    account_type      AS "account_type!: domain::AccountType",
                    unit_id           AS "unit_id!: domain::RowID",
                    starting_balance  AS "starting_balance!: domain::Money",
                    is_active,
                    created_on        AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on        AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM accounts
                ORDER BY name ASC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total_count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM accounts"#)
            .fetch_one(pool)
            .await?;

        Ok((accounts, total_count))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_unit(pool: &SqlitePool) -> lib_core::RowID {
        crate::Units::mock().insert(pool).await.unwrap().id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_finds_an_existing_account(pool: SqlitePool) {
        let unit_id = seed_unit(&pool).await;
        let account = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();

        let found = crate::Accounts::find_by_id(account.id, &pool)
            .await
            .unwrap();

        assert_eq!(found.map(|a| a.id), Some(account.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_returns_none_when_missing(pool: SqlitePool) {
        let found = crate::Accounts::find_by_id(lib_core::RowID::new(), &pool)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_with_pagination_returns_a_page_and_the_total_count(pool: SqlitePool) {
        let unit_id = seed_unit(&pool).await;
        for _ in 0..5 {
            crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();
        }

        let (page, total) = crate::Accounts::find_all_with_pagination(0, 2, &pool)
            .await
            .unwrap();

        assert_eq!(page.len(), 2);
        assert_eq!(total, 5);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_active_excludes_inactive_accounts(pool: SqlitePool) {
        let unit_id = seed_unit(&pool).await;
        let active = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();
        let inactive = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();
        crate::Accounts::set_active(inactive.id, false, &pool)
            .await
            .unwrap();

        let found = crate::Accounts::find_all_active(&pool).await.unwrap();

        assert!(found.iter().any(|a| a.id == active.id));
        assert!(!found.iter().any(|a| a.id == inactive.id));
    }
}
