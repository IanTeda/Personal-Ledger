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
                INSERT INTO units (id, code, name, unit_kind, price_source_id, price_symbol, quote_unit_id, is_active, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.code,
            self.name,
            self.unit_kind,
            self.price_source_id,
            self.price_symbol,
            self.quote_unit_id,
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

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_round_trips_the_price_link(pool: SqlitePool) {
        let base = crate::Units::mock().insert(&pool).await.unwrap();
        assert_eq!(base.quote_unit_id, base.id, "the base Unit quotes itself");

        let btc = crate::units::UnitsBuilder::new()
            .with_code("BTC")
            .with_name("Bitcoin")
            .with_unit_kind(lib_core::UnitKind::Crypto)
            .with_price_source_id("coingecko")
            .with_price_symbol("bitcoin")
            .with_quote_unit_id(base.id)
            .build()
            .unwrap()
            .insert(&pool)
            .await
            .unwrap();

        assert_eq!(btc.price_source_id, "coingecko");
        assert_eq!(btc.price_symbol, "bitcoin");
        assert_eq!(btc.quote_unit_id, base.id);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_rejects_a_quote_unit_that_does_not_exist(pool: SqlitePool) {
        let unit = crate::units::UnitsBuilder::new()
            .with_code("BTC")
            .with_name("Bitcoin")
            .with_quote_unit_id(lib_core::RowID::new())
            .build()
            .unwrap();

        assert!(unit.insert(&pool).await.is_err());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn unit_kind_check_rejects_an_unknown_kind(pool: SqlitePool) {
        let unit = crate::Units::mock();

        let result = sqlx::query(
            "INSERT INTO units (id, code, name, unit_kind, price_source_id, price_symbol, quote_unit_id) \
             VALUES (?, 'ZZZ', 'Zed', 'gizmo', 'manual', '', ?)",
        )
        .bind(unit.id)
        .bind(unit.id)
        .execute(&pool)
        .await;

        assert!(result.is_err());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn ledger_table_holds_one_row_pointing_at_a_unit(pool: SqlitePool) {
        let unit = crate::Units::mock().insert(&pool).await.unwrap();

        sqlx::query("INSERT INTO ledger (id, base_unit_id) VALUES (1, ?)")
            .bind(unit.id)
            .execute(&pool)
            .await
            .unwrap();

        let second = sqlx::query("INSERT INTO ledger (id, base_unit_id) VALUES (2, ?)")
            .bind(unit.id)
            .execute(&pool)
            .await;
        assert!(second.is_err(), "only id = 1 is allowed");

        let missing = sqlx::query("UPDATE ledger SET base_unit_id = ? WHERE id = 1")
            .bind(lib_core::RowID::new())
            .execute(&pool)
            .await;
        assert!(missing.is_err(), "base_unit_id must reference a Unit");
    }
}
