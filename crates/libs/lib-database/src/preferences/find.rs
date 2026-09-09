//! # Preferences Query Operations
//!
//! Reading the singleton Preferences row -- mirrors [`crate::SyncUser::find_only`].

use lib_core as domain;

impl crate::Preferences {
    /// Find the singleton Preferences row, if it has been created yet.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find the Preferences row: ", level = "debug", skip(pool))]
    pub async fn find_only(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Option<Self>> {
        let preferences = sqlx::query_as!(
            crate::Preferences,
            r#"
                SELECT
                    id               AS "id!: domain::RowID",
                    default_unit_id  AS "default_unit_id: domain::RowID",
                    colour_theme     AS "colour_theme!: domain::HexColor",
                    date_format      AS "date_format!: domain::DateFormat",
                    number_format    AS "number_format!: domain::NumberFormat",
                    created_on       AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on       AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM preferences
                LIMIT 1
            "#,
        )
        .fetch_optional(pool)
        .await?;

        Ok(preferences)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_only_returns_none_when_empty(pool: SqlitePool) {
        let found = crate::Preferences::find_only(&pool).await.unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_only_returns_the_created_row(pool: SqlitePool) {
        let created = crate::Preferences::get_or_create_default(&pool).await.unwrap();

        let found = crate::Preferences::find_only(&pool).await.unwrap();

        assert_eq!(found.map(|p| p.id), Some(created.id));
    }
}
