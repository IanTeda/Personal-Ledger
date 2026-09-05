//! # Balance Checks Query Operations
//!
//! FR.30 also asks for filtering by account and/or date range; deferred until a TUI filter
//! UI actually needs it, matching the precedent already set for Accounts (FR.12's
//! type/active-status filtering) — `find_all`/`find_all_with_pagination` cover what the list
//! screen uses today.

use lib_core as domain;

impl crate::BalanceChecks {
    /// Find a Balance Check by its id.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Balance Check by id: ", level = "debug", skip(pool))]
    pub async fn find_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let balance_check = sqlx::query_as!(
            crate::BalanceChecks,
            r#"
                SELECT
                    id                AS "id!: domain::RowID",
                    account_id        AS "account_id!: domain::RowID",
                    date              AS "date!: chrono::NaiveDate",
                    asserted_balance  AS "asserted_balance!: domain::Money",
                    created_on        AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on        AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM balance_checks
                WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(balance_check)
    }

    /// List every Balance Check, most recent date first.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all Balance Checks: ", level = "debug", skip(pool))]
    pub async fn find_all(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Vec<Self>> {
        let balance_checks = sqlx::query_as!(
            crate::BalanceChecks,
            r#"
                SELECT
                    id                AS "id!: domain::RowID",
                    account_id        AS "account_id!: domain::RowID",
                    date              AS "date!: chrono::NaiveDate",
                    asserted_balance  AS "asserted_balance!: domain::Money",
                    created_on        AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on        AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM balance_checks
                ORDER BY date DESC, id DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(balance_checks)
    }

    /// List Balance Checks with pagination, alongside the total (unpaginated) row count.
    ///
    /// # Errors
    /// Returns an error if either query fails.
    #[tracing::instrument(
        name = "Find Balance Checks with pagination: ",
        level = "debug",
        skip(pool)
    )]
    pub async fn find_all_with_pagination(
        offset: i64,
        limit: i64,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i64)> {
        let balance_checks = sqlx::query_as!(
            crate::BalanceChecks,
            r#"
                SELECT
                    id                AS "id!: domain::RowID",
                    account_id        AS "account_id!: domain::RowID",
                    date              AS "date!: chrono::NaiveDate",
                    asserted_balance  AS "asserted_balance!: domain::Money",
                    created_on        AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on        AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM balance_checks
                ORDER BY date DESC, id DESC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total_count =
            sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM balance_checks"#)
                .fetch_one(pool)
                .await?;

        Ok((balance_checks, total_count))
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
    async fn find_by_id_finds_an_existing_balance_check(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let balance_check = crate::BalanceChecks::mock(account_id)
            .insert(&pool)
            .await
            .unwrap();

        let found = crate::BalanceChecks::find_by_id(balance_check.id, &pool)
            .await
            .unwrap();

        assert_eq!(found.map(|b| b.id), Some(balance_check.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_returns_none_when_missing(pool: SqlitePool) {
        let found = crate::BalanceChecks::find_by_id(lib_core::RowID::new(), &pool)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_with_pagination_returns_a_page_and_the_total_count(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        for _ in 0..5 {
            crate::BalanceChecks::mock(account_id)
                .insert(&pool)
                .await
                .unwrap();
        }

        let (page, total) = crate::BalanceChecks::find_all_with_pagination(0, 2, &pool)
            .await
            .unwrap();

        assert_eq!(page.len(), 2);
        assert_eq!(total, 5);
    }
}
