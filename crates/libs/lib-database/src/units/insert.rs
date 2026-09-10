//! # Units Insert Operations

use lib_core as domain;

impl crate::Units {
    /// Insert this Unit into the database.
    ///
    /// # Errors
    /// Returns an error if the underlying INSERT or read-back SELECT fails (including a
    /// unique-constraint violation on `code`).
    #[tracing::instrument(
        name = "Insert new Unit into database: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id, code = %self.code),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Self> {
        let insert_result = sqlx::query!(
            r#"
                INSERT INTO units (id, code, name, unit_kind, decimal_places, is_active, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.code,
            self.name,
            self.unit_kind,
            self.decimal_places,
            self.is_active,
            self.created_on,
            self.updated_on
        )
        .execute(pool)
        .await;

        match insert_result {
            Ok(result) => {
                if result.rows_affected() != 1 {
                    tracing::warn!(
                        "INSERT operation affected {} rows instead of 1 for unit: {}",
                        result.rows_affected(),
                        self.code
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert unit {}: {}", self.code, e);
                return Err(e.into());
            }
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::Error::NotFound(format!("Unit {} not found after insert", self.id))
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_persists_and_reads_back_a_unit(pool: SqlitePool) {
        let unit = crate::Units::mock();

        let inserted = unit.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, unit.id);
        assert_eq!(inserted.code, unit.code);
        assert_eq!(inserted.name, unit.name);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_rejects_a_duplicate_code(pool: SqlitePool) {
        let unit = crate::Units::mock();
        unit.insert(&pool).await.unwrap();

        let duplicate = crate::units::UnitsBuilder::new()
            .with_id(lib_core::RowID::new())
            .with_code(unit.code.clone())
            .with_name("Duplicate".to_string())
            .build()
            .unwrap();

        let result = duplicate.insert(&pool).await;
        assert!(result.is_err());
    }
}
