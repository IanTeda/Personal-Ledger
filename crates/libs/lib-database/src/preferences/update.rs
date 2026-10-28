//! # Preferences Update Operations
//!
//! Getting-or-creating the singleton Preferences row. It has no editable field yet, so there is
//! no update until a Ledger-scoped Preference joins it.

impl crate::Preferences {
    /// Get the singleton Preferences row, creating it on first use.
    ///
    /// Mirrors `bin-sync-server`'s `bootstrap_account`: singleton by convention (a
    /// [`Self::find_only`] check before inserting), not a database constraint -- see
    /// `sync_users` for the precedent this follows.
    ///
    /// The insert happens here, in Rust, rather than as a plain-SQL `INSERT` in the
    /// migration: the row's `id` must be a genuine UUIDv7 (see `lib_core::RowID`), which
    /// plain SQL cannot generate.
    ///
    /// # Errors
    /// Returns an error if inserting the Preferences row fails.
    #[tracing::instrument(
        name = "Get or create the Preferences row: ",
        level = "debug",
        skip(pool)
    )]
    pub async fn get_or_create_default(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Self> {
        if let Some(existing) = Self::find_only(pool).await? {
            return Ok(existing);
        }

        let preferences = crate::preferences::PreferencesBuilder::new().build();

        let insert_result = sqlx::query!(
            r#"
                INSERT INTO preferences (id, created_on, updated_on)
                VALUES (?, ?, ?)
            "#,
            preferences.id,
            preferences.created_on,
            preferences.updated_on
        )
        .execute(pool)
        .await?;

        if insert_result.rows_affected() != 1 {
            tracing::warn!(
                "INSERT operation affected {} rows instead of 1 for preferences",
                insert_result.rows_affected()
            );
        }

        Self::find_only(pool)
            .await?
            .ok_or_else(|| crate::Error::NotFound("Preferences not found after insert".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn get_or_create_default_does_not_create_a_unit(pool: SqlitePool) {
        crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();

        let all_units = crate::Units::find_all(&pool).await.unwrap();
        assert!(all_units.is_empty());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn get_or_create_default_is_idempotent(pool: SqlitePool) {
        let first = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();
        let second = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();

        assert_eq!(first.id, second.id);
    }
}
