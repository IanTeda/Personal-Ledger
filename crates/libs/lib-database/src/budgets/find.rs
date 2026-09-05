//! # Budgets Query Operations
//!
//! FR.25 also asks for filtering by Category and/or active status; deferred until a TUI
//! filter UI actually needs it, matching the precedent already set for Accounts (FR.12) and
//! Balance Checks (FR.30) — `find_all`/`find_all_active`/`find_all_with_pagination` cover
//! what the list screen uses today.

use lib_core as domain;

impl crate::Budgets {
    /// Find a Budget by its id.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Budget by id: ", level = "debug", skip(pool))]
    pub async fn find_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let budget = sqlx::query_as!(
            crate::Budgets,
            r#"
                SELECT
                    id            AS "id!: domain::RowID",
                    category_id   AS "category_id!: domain::RowID",
                    unit_id       AS "unit_id!: domain::RowID",
                    limit_amount  AS "limit_amount!: domain::Money",
                    period        AS "period!: domain::BudgetPeriod",
                    is_active,
                    created_on    AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on    AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM budgets
                WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(budget)
    }

    /// List every Budget.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all Budgets: ", level = "debug", skip(pool))]
    pub async fn find_all(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Vec<Self>> {
        let budgets = sqlx::query_as!(
            crate::Budgets,
            r#"
                SELECT
                    id            AS "id!: domain::RowID",
                    category_id   AS "category_id!: domain::RowID",
                    unit_id       AS "unit_id!: domain::RowID",
                    limit_amount  AS "limit_amount!: domain::Money",
                    period        AS "period!: domain::BudgetPeriod",
                    is_active,
                    created_on    AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on    AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM budgets
                ORDER BY created_on ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(budgets)
    }

    /// List every active Budget — the source list the progress-bar list screen shows.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all active Budgets: ", level = "debug", skip(pool))]
    pub async fn find_all_active(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let budgets = sqlx::query_as!(
            crate::Budgets,
            r#"
                SELECT
                    id            AS "id!: domain::RowID",
                    category_id   AS "category_id!: domain::RowID",
                    unit_id       AS "unit_id!: domain::RowID",
                    limit_amount  AS "limit_amount!: domain::Money",
                    period        AS "period!: domain::BudgetPeriod",
                    is_active,
                    created_on    AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on    AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM budgets
                WHERE is_active = TRUE
                ORDER BY created_on ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(budgets)
    }

    /// List Budgets with pagination, alongside the total (unpaginated) row count.
    ///
    /// # Errors
    /// Returns an error if either query fails.
    #[tracing::instrument(name = "Find Budgets with pagination: ", level = "debug", skip(pool))]
    pub async fn find_all_with_pagination(
        offset: i64,
        limit: i64,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i64)> {
        let budgets = sqlx::query_as!(
            crate::Budgets,
            r#"
                SELECT
                    id            AS "id!: domain::RowID",
                    category_id   AS "category_id!: domain::RowID",
                    unit_id       AS "unit_id!: domain::RowID",
                    limit_amount  AS "limit_amount!: domain::Money",
                    period        AS "period!: domain::BudgetPeriod",
                    is_active,
                    created_on    AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on    AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM budgets
                ORDER BY created_on ASC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total_count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM budgets"#)
            .fetch_one(pool)
            .await?;

        Ok((budgets, total_count))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_expense_category(pool: &SqlitePool) -> lib_core::RowID {
        let mut category = crate::Categories::mock();
        category.category_type = lib_core::CategoryTypes::Expense;
        category.insert(pool).await.unwrap().id
    }

    async fn seed_unit(pool: &SqlitePool) -> lib_core::RowID {
        crate::Units::mock().insert(pool).await.unwrap().id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_finds_an_existing_budget(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit_id = seed_unit(&pool).await;
        let budget = crate::Budgets::mock(category_id, unit_id)
            .insert(&pool)
            .await
            .unwrap();

        let found = crate::Budgets::find_by_id(budget.id, &pool).await.unwrap();

        assert_eq!(found.map(|b| b.id), Some(budget.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_returns_none_when_missing(pool: SqlitePool) {
        let found = crate::Budgets::find_by_id(lib_core::RowID::new(), &pool)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_active_excludes_inactive_budgets(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit_id = seed_unit(&pool).await;
        let active = crate::Budgets::mock(category_id, unit_id)
            .insert(&pool)
            .await
            .unwrap();
        let inactive = crate::Budgets::mock(category_id, unit_id)
            .insert(&pool)
            .await
            .unwrap();
        crate::Budgets::set_active(inactive.id, false, &pool)
            .await
            .unwrap();

        let found = crate::Budgets::find_all_active(&pool).await.unwrap();

        assert!(found.iter().any(|b| b.id == active.id));
        assert!(!found.iter().any(|b| b.id == inactive.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_with_pagination_returns_a_page_and_the_total_count(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit_id = seed_unit(&pool).await;
        for _ in 0..5 {
            crate::Budgets::mock(category_id, unit_id)
                .insert(&pool)
                .await
                .unwrap();
        }

        let (page, total) = crate::Budgets::find_all_with_pagination(0, 2, &pool)
            .await
            .unwrap();

        assert_eq!(page.len(), 2);
        assert_eq!(total, 5);
    }
}
