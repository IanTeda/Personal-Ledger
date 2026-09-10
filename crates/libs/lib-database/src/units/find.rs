//! # Units Query Operations

use lib_core as domain;

impl crate::Units {
    /// Find a Unit by its id.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Unit by id: ", level = "debug", skip(pool))]
    pub async fn find_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Option<Self>> {
        let unit = sqlx::query_as!(
            crate::Units,
            r#"
                SELECT
                    id             AS "id!: domain::RowID",
                    code,
                    name,
                    unit_kind      AS "unit_kind!: domain::UnitKind",
                    decimal_places,
                    is_active,
                    created_on     AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on     AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM units
                WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(unit)
    }

    /// Find a Unit by its unique code (e.g. "AUD").
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Unit by code: ", level = "debug", skip(pool))]
    pub async fn find_by_code(
        code: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Option<Self>> {
        let unit = sqlx::query_as!(
            crate::Units,
            r#"
                SELECT
                    id             AS "id!: domain::RowID",
                    code,
                    name,
                    unit_kind      AS "unit_kind!: domain::UnitKind",
                    decimal_places,
                    is_active,
                    created_on     AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on     AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM units
                WHERE code = ?
            "#,
            code
        )
        .fetch_optional(pool)
        .await?;

        Ok(unit)
    }

    /// List every Unit, ordered by code.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all Units: ", level = "debug", skip(pool))]
    pub async fn find_all(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Vec<Self>> {
        let units = sqlx::query_as!(
            crate::Units,
            r#"
                SELECT
                    id             AS "id!: domain::RowID",
                    code,
                    name,
                    unit_kind      AS "unit_kind!: domain::UnitKind",
                    decimal_places,
                    is_active,
                    created_on     AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on     AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM units
                ORDER BY code ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(units)
    }

    /// List every active Unit, ordered by code — the set a picker (e.g. Settings' default
    /// Unit for new Accounts) should offer.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find active Units: ", level = "debug", skip(pool))]
    pub async fn find_active(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Vec<Self>> {
        let units = sqlx::query_as!(
            crate::Units,
            r#"
                SELECT
                    id             AS "id!: domain::RowID",
                    code,
                    name,
                    unit_kind      AS "unit_kind!: domain::UnitKind",
                    decimal_places,
                    is_active,
                    created_on     AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on     AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM units
                WHERE is_active = TRUE
                ORDER BY code ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(units)
    }

    /// List Units with pagination, alongside the total (unpaginated) row count.
    ///
    /// # Errors
    /// Returns an error if either query fails.
    #[tracing::instrument(name = "Find Units with pagination: ", level = "debug", skip(pool))]
    pub async fn find_all_with_pagination(
        offset: i64,
        limit: i64,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<(Vec<Self>, i64)> {
        let units = sqlx::query_as!(
            crate::Units,
            r#"
                SELECT
                    id             AS "id!: domain::RowID",
                    code,
                    name,
                    unit_kind      AS "unit_kind!: domain::UnitKind",
                    decimal_places,
                    is_active,
                    created_on     AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on     AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM units
                ORDER BY code ASC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total_count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM units"#)
            .fetch_one(pool)
            .await?;

        Ok((units, total_count))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_finds_an_existing_unit(pool: SqlitePool) {
        let unit = crate::Units::mock().insert(&pool).await.unwrap();

        let found = crate::Units::find_by_id(unit.id, &pool).await.unwrap();

        assert_eq!(found.map(|u| u.id), Some(unit.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_code_finds_an_existing_unit(pool: SqlitePool) {
        let unit = crate::Units::mock().insert(&pool).await.unwrap();

        let found = crate::Units::find_by_code(&unit.code, &pool).await.unwrap();

        assert_eq!(found.map(|u| u.id), Some(unit.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_returns_none_when_missing(pool: SqlitePool) {
        let found = crate::Units::find_by_id(lib_core::RowID::new(), &pool)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_active_excludes_inactive_units(pool: SqlitePool) {
        let active = crate::Units::mock().insert(&pool).await.unwrap();
        let mut inactive = crate::Units::mock();
        inactive.is_active = false;
        inactive.insert(&pool).await.unwrap();

        let found = crate::Units::find_active(&pool).await.unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, active.id);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_with_pagination_returns_a_page_and_the_total_count(pool: SqlitePool) {
        for _ in 0..5 {
            crate::Units::mock().insert(&pool).await.unwrap();
        }

        let (page, total) = crate::Units::find_all_with_pagination(0, 2, &pool)
            .await
            .unwrap();

        assert_eq!(page.len(), 2);
        assert_eq!(total, 5);
    }
}
