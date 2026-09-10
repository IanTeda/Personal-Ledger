//! # Units Update Operations

use lib_core as domain;

impl crate::Units {
    /// Replace this Unit's editable fields (`code`, `name`, `unit_kind`, `decimal_places`,
    /// `is_active`) with its current values, then re-reads the row to confirm. `id` and
    /// `created_on` never change; `updated_on` is refreshed by the `trg_units_set_updated_on`
    /// trigger.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Unit exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(
        name = "Update Unit: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id, code = %self.code),
    )]
    pub async fn update(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Self> {
        let result = sqlx::query!(
            r#"
                UPDATE units
                SET code = ?, name = ?, unit_kind = ?, decimal_places = ?, is_active = ?
                WHERE id = ?
            "#,
            self.code,
            self.name,
            self.unit_kind,
            self.decimal_places,
            self.is_active,
            self.id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!(
                "Unit {} not found",
                self.id
            )));
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::Error::NotFound(format!("Unit {} not found after update", self.id))
        })
    }

    /// Activate or deactivate a Unit by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Unit exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(name = "Set Unit active flag: ", level = "debug", skip(pool))]
    pub async fn set_active(
        id: domain::RowID,
        is_active: bool,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Self> {
        let result = sqlx::query!(
            r#"UPDATE units SET is_active = ? WHERE id = ?"#,
            is_active,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!("Unit {id} not found")));
        }

        Self::find_by_id(id, pool)
            .await?
            .ok_or_else(|| crate::Error::NotFound(format!("Unit {id} not found after update")))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_replaces_editable_fields(pool: SqlitePool) {
        let mut unit = crate::Units::mock().insert(&pool).await.unwrap();
        unit.name = "Updated Name".to_string();
        unit.decimal_places = 8;

        let updated = unit.update(&pool).await.unwrap();

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.decimal_places, 8);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_errors_when_unit_missing(pool: SqlitePool) {
        let unit = crate::Units::mock();
        let result = unit.update(&pool).await;
        assert!(matches!(result, Err(crate::Error::NotFound(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn set_active_toggles_the_flag(pool: SqlitePool) {
        let unit = crate::Units::mock().insert(&pool).await.unwrap();
        assert!(unit.is_active);

        let deactivated = crate::Units::set_active(unit.id, false, &pool)
            .await
            .unwrap();
        assert!(!deactivated.is_active);
    }
}
