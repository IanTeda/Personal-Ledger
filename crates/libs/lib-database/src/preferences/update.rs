//! # Preferences Update Operations
//!
//! Getting-or-creating the singleton Preferences row, and updating its editable fields.

use lib_core as domain;

impl crate::Preferences {
    /// Get the singleton Preferences row, creating it on first use.
    ///
    /// Mirrors `bin-sync-server`'s `bootstrap_account`: singleton by convention (a
    /// [`Self::find_only`] check before inserting), not a database constraint -- see
    /// `sync_users` for the precedent this follows.
    ///
    /// The first call also seeds a default "USD" Unit (`code`/`name`/`unit_kind`/
    /// `decimal_places`) for `default_unit_id` to point at, if no Unit with that code
    /// already exists. This happens here, in Rust, rather than as a plain-SQL `INSERT` in
    /// the migration: a Unit's `id` must be a genuine UUIDv7 (see
    /// `lib_core::RowID`/the `payee_aliases` migration's comment on the same constraint),
    /// which plain SQL cannot generate.
    ///
    /// # Errors
    /// Returns an error if seeding the default Unit or inserting the Preferences row fails.
    #[tracing::instrument(
        name = "Get or create the Preferences row: ",
        level = "debug",
        skip(pool)
    )]
    pub async fn get_or_create_default(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Self> {
        if let Some(existing) = Self::find_only(pool).await? {
            return Ok(existing);
        }

        let default_unit = match crate::Units::find_by_code("USD", pool).await? {
            Some(unit) => unit,
            None => {
                let usd = crate::units::UnitsBuilder::new()
                    .with_code("USD")
                    .with_name("US Dollar")
                    .with_unit_kind(domain::UnitKind::Fiat)
                    .with_decimal_places(2)
                    .build()
                    .map_err(|e| {
                        crate::Error::Generic(format!("Failed to build default USD Unit: {e}"))
                    })?;
                usd.insert(pool).await?
            }
        };

        let preferences = crate::preferences::PreferencesBuilder::new()
            .with_default_unit_id(Some(default_unit.id))
            .build();

        let insert_result = sqlx::query!(
            r#"
                INSERT INTO preferences (id, default_unit_id, colour_theme, date_format, number_format, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            preferences.id,
            preferences.default_unit_id,
            preferences.colour_theme,
            preferences.date_format,
            preferences.number_format,
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

    /// Replace this Preferences row's editable fields (`default_unit_id`, `colour_theme`,
    /// `date_format`, `number_format`) with its current values, then re-reads the row to
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
                SET default_unit_id = ?, colour_theme = ?, date_format = ?, number_format = ?
                WHERE id = ?
            "#,
            self.default_unit_id,
            self.colour_theme,
            self.date_format,
            self.number_format,
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
    async fn get_or_create_default_seeds_a_usd_unit_and_preferences_row(pool: SqlitePool) {
        let preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();

        let usd = crate::Units::find_by_code("USD", &pool)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(preferences.default_unit_id, Some(usd.id));
        assert_eq!(usd.name, "US Dollar");
        assert_eq!(usd.decimal_places, 2);
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

        let all_units = crate::Units::find_all(&pool).await.unwrap();
        assert_eq!(
            all_units.len(),
            1,
            "a second call must not seed a second USD Unit"
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn get_or_create_default_reuses_an_existing_usd_unit(pool: SqlitePool) {
        let existing_usd = crate::units::UnitsBuilder::new()
            .with_code("USD")
            .with_name("Existing US Dollar")
            .build()
            .unwrap()
            .insert(&pool)
            .await
            .unwrap();

        let preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();

        assert_eq!(preferences.default_unit_id, Some(existing_usd.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_replaces_editable_fields(pool: SqlitePool) {
        let mut preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();
        preferences.colour_theme = lib_core::HexColor::from_rgb(0, 255, 0);
        preferences.date_format = lib_core::DateFormat::Iso;

        let updated = preferences.update(&pool).await.unwrap();

        assert_eq!(
            updated.colour_theme,
            lib_core::HexColor::from_rgb(0, 255, 0)
        );
        assert_eq!(updated.date_format, lib_core::DateFormat::Iso);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_errors_when_preferences_missing(pool: SqlitePool) {
        let preferences = crate::Preferences::mock();
        let result = preferences.update(&pool).await;
        assert!(matches!(result, Err(crate::Error::NotFound(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn hard_deleting_the_default_unit_clears_default_unit_id_via_fk(pool: SqlitePool) {
        let preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();
        let default_unit_id = preferences.default_unit_id.unwrap();

        crate::Units::delete_by_id(default_unit_id, &pool)
            .await
            .unwrap();

        let reloaded = crate::Preferences::find_only(&pool).await.unwrap().unwrap();
        assert_eq!(
            reloaded.default_unit_id, None,
            "ON DELETE SET NULL should clear default_unit_id, not block the Unit delete"
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_clears_default_unit_id_when_set_to_none(pool: SqlitePool) {
        let mut preferences = crate::Preferences::get_or_create_default(&pool)
            .await
            .unwrap();
        preferences.default_unit_id = None;

        let updated = preferences.update(&pool).await.unwrap();

        assert_eq!(updated.default_unit_id, None);
    }
}
