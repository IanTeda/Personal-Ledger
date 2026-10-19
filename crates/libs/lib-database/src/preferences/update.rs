//! # Preferences Update Operations
//!
//! Getting-or-creating the singleton Preferences row, and updating its editable fields.

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
                INSERT INTO preferences (id, colour_theme, colour_appearance, date_style, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?)
            "#,
            preferences.id,
            preferences.colour_theme,
            preferences.colour_appearance,
            preferences.date_style,
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

    /// Replace this Preferences row's editable fields (`colour_theme`,
    /// `colour_appearance`, `date_style`) with its current values, then re-reads the row to
    /// confirm. `id` and `created_on` never change; `updated_on` is refreshed by the
    /// `trg_preferences_set_updated_on` trigger.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if the Preferences row doesn't exist yet
    /// (call [`Self::get_or_create_default`] first), or an error if the underlying query
    /// fails.
    #[tracing::instrument(
        name = "Update Preferences: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id),
    )]
    pub async fn update(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Self> {
        let result = sqlx::query!(
            r#"
                UPDATE preferences
                SET colour_theme = ?, colour_appearance = ?, date_style = ?
                WHERE id = ?
            "#,
            self.colour_theme,
            self.colour_appearance,
            self.date_style,
            self.id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!(
                "Preferences {} not found",
                self.id
            )));
        }

        Self::find_only(pool).await?.ok_or_else(|| {
            crate::Error::NotFound(format!("Preferences {} not found after update", self.id))
        })
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

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_replaces_editable_fields(pool: SqlitePool) {
        let mut preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();
        preferences.colour_theme = Some("nord".to_string());
        preferences.colour_appearance = Some("light".to_string());
        preferences.date_style = Some(lib_core::DateStyle::Iso);

        let updated = preferences.update(&pool).await.unwrap();

        assert_eq!(updated.colour_theme.as_deref(), Some("nord"));
        assert_eq!(updated.colour_appearance.as_deref(), Some("light"));
        assert_eq!(updated.date_style, Some(lib_core::DateStyle::Iso));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn date_style_defaults_to_none_and_can_be_cleared(pool: SqlitePool) {
        let mut preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();
        assert_eq!(preferences.date_style, None);

        preferences.date_style = Some(lib_core::DateStyle::Long);
        let set = preferences.update(&pool).await.unwrap();
        assert_eq!(set.date_style, Some(lib_core::DateStyle::Long));

        let mut cleared = set;
        cleared.date_style = None;
        let cleared = cleared.update(&pool).await.unwrap();
        assert_eq!(cleared.date_style, None);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn colour_theme_and_appearance_default_to_none_and_can_be_cleared(pool: SqlitePool) {
        let mut preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();
        assert_eq!(preferences.colour_theme, None);
        assert_eq!(preferences.colour_appearance, None);

        preferences.colour_theme = Some("high_contrast".to_string());
        preferences.colour_appearance = Some("dark".to_string());
        let set = preferences.update(&pool).await.unwrap();
        assert_eq!(set.colour_theme.as_deref(), Some("high_contrast"));
        assert_eq!(set.colour_appearance.as_deref(), Some("dark"));

        let mut cleared = set;
        cleared.colour_theme = None;
        cleared.colour_appearance = None;
        let cleared = cleared.update(&pool).await.unwrap();
        assert_eq!(cleared.colour_theme, None);
        assert_eq!(cleared.colour_appearance, None);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn unknown_colour_theme_id_round_trips(pool: SqlitePool) {
        let mut preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();
        preferences.colour_theme = Some("from_a_newer_release".to_string());

        let updated = preferences.update(&pool).await.unwrap();

        assert_eq!(
            updated.colour_theme.as_deref(),
            Some("from_a_newer_release")
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_errors_when_preferences_missing(pool: SqlitePool) {
        let preferences = crate::Preferences::mock();
        let result = preferences.update(&pool).await;
        assert!(matches!(result, Err(crate::Error::NotFound(_))));
    }
}
