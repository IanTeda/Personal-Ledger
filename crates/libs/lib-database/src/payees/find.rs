//! # Payees Query Operations

use lib_core as domain;

impl crate::Payees {
    /// Find a Payee by its id.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Payee by id: ", level = "debug", skip(pool))]
    pub async fn find_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let payee = sqlx::query_as!(
            crate::Payees,
            r#"
                SELECT
                    id           AS "id!: domain::RowID",
                    name,
                    is_active,
                    created_on   AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on   AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM payees
                WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(payee)
    }

    /// Find a Payee by its exact name, case-insensitively (the `payees.name` column is
    /// declared `COLLATE NOCASE`, so `=` already compares case-insensitively).
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Payee by name: ", level = "debug", skip(pool))]
    pub async fn find_by_name(
        name: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let payee = sqlx::query_as!(
            crate::Payees,
            r#"
                SELECT
                    id           AS "id!: domain::RowID",
                    name,
                    is_active,
                    created_on   AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on   AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM payees
                WHERE name = ?
            "#,
            name
        )
        .fetch_optional(pool)
        .await?;

        Ok(payee)
    }

    /// List every Payee, ordered by name.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all Payees: ", level = "debug", skip(pool))]
    pub async fn find_all(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Vec<Self>> {
        let payees = sqlx::query_as!(
            crate::Payees,
            r#"
                SELECT
                    id           AS "id!: domain::RowID",
                    name,
                    is_active,
                    created_on   AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on   AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM payees
                ORDER BY name ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(payees)
    }

    /// List every active Payee, ordered by name — the suggestion picker's source list.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all active Payees: ", level = "debug", skip(pool))]
    pub async fn find_all_active(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let payees = sqlx::query_as!(
            crate::Payees,
            r#"
                SELECT
                    id           AS "id!: domain::RowID",
                    name,
                    is_active,
                    created_on   AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on   AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM payees
                WHERE is_active = TRUE
                ORDER BY name ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(payees)
    }

    /// List Payees with pagination, alongside the total (unpaginated) row count.
    ///
    /// # Errors
    /// Returns an error if either query fails.
    #[tracing::instrument(name = "Find Payees with pagination: ", level = "debug", skip(pool))]
    pub async fn find_all_with_pagination(
        offset: i64,
        limit: i64,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i64)> {
        let payees = sqlx::query_as!(
            crate::Payees,
            r#"
                SELECT
                    id           AS "id!: domain::RowID",
                    name,
                    is_active,
                    created_on   AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on   AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM payees
                ORDER BY name ASC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total_count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM payees"#)
            .fetch_one(pool)
            .await?;

        Ok((payees, total_count))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_finds_an_existing_payee(pool: SqlitePool) {
        let payee = crate::Payees::mock().insert(&pool).await.unwrap();

        let found = crate::Payees::find_by_id(payee.id, &pool).await.unwrap();

        assert_eq!(found.map(|p| p.id), Some(payee.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_returns_none_when_missing(pool: SqlitePool) {
        let found = crate::Payees::find_by_id(lib_core::RowID::new(), &pool)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_name_matches_case_insensitively(pool: SqlitePool) {
        let mut payee = crate::Payees::mock();
        payee.name = "Woolworths".to_string();
        payee.insert(&pool).await.unwrap();

        let found = crate::Payees::find_by_name("WOOLWORTHS", &pool)
            .await
            .unwrap();

        assert_eq!(found.map(|p| p.name), Some("Woolworths".to_string()));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_active_excludes_inactive_payees(pool: SqlitePool) {
        let active = crate::Payees::mock().insert(&pool).await.unwrap();
        let inactive = crate::Payees::mock().insert(&pool).await.unwrap();
        crate::Payees::set_active(inactive.id, false, &pool)
            .await
            .unwrap();

        let found = crate::Payees::find_all_active(&pool).await.unwrap();

        assert!(found.iter().any(|p| p.id == active.id));
        assert!(!found.iter().any(|p| p.id == inactive.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_with_pagination_returns_a_page_and_the_total_count(pool: SqlitePool) {
        for _ in 0..5 {
            crate::Payees::mock().insert(&pool).await.unwrap();
        }

        let (page, total) = crate::Payees::find_all_with_pagination(0, 2, &pool)
            .await
            .unwrap();

        assert_eq!(page.len(), 2);
        assert_eq!(total, 5);
    }
}
