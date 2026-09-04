//! # Change Set Insert Operations
//!
//! Provides database insertion for Change Set records -- how a Client's pushed edits,
//! or the Sync Server's own record of them, land in the durable `change_sets` log.

use lib_core as domain;

impl crate::ChangeSet {
    /// Insert this Change Set into the durable log.
    ///
    /// # Errors
    /// Returns an error if the underlying INSERT or read-back SELECT fails.
    #[tracing::instrument(
        name = "Insert new Change Set into database: ",
        level = "debug",
        skip(self, pool),
        fields(
            id = % self.id,
            table_name = % self.table_name,
            row_id = % self.row_id,
            field_name = % self.field_name,
            client_id = % self.client_id,
        ),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        tracing::trace!(
            "Starting Change Set insert for {}.{}.{} (id: {})",
            self.table_name,
            self.row_id,
            self.field_name,
            self.id
        );

        // SQLite doesn't reliably support `RETURNING *` with sqlx's compile-time checked
        // macros -- insert first, then read the row back explicitly (same two-step
        // pattern as `categories/insert.rs`).
        let hlc_text = self.hlc.to_string();
        let insert_result = sqlx::query!(
            r#"
                INSERT INTO change_sets (id, table_name, row_id, field_name, value, hlc, client_id, version, created_on)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.table_name,
            self.row_id,
            self.field_name,
            self.value,
            hlc_text,
            self.client_id,
            self.version,
            self.created_on
        )
        .execute(pool)
        .await;

        match insert_result {
            Ok(result) => {
                if result.rows_affected() != 1 {
                    tracing::warn!(
                        "INSERT operation affected {} rows instead of 1 for Change Set: {}",
                        result.rows_affected(),
                        self.id
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert Change Set {}: {}", self.id, e);
                return Err(e.into());
            }
        }

        let change_set = sqlx::query_as!(
            crate::ChangeSet,
            r#"
                SELECT
                    id           AS "id!: domain::RowID",
                    table_name,
                    row_id       AS "row_id!: domain::RowID",
                    field_name,
                    value,
                    hlc          AS "hlc!: domain::HybridLogicalClock",
                    client_id    AS "client_id!: domain::RowID",
                    version,
                    created_on   AS "created_on!: chrono::DateTime<chrono::Utc>"
                FROM change_sets
                WHERE id = ?
            "#,
            self.id
        )
        .fetch_one(pool)
        .await?;

        tracing::trace!("Change Set inserted and read back: {}", change_set.id);

        Ok(change_set)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn insert_persists_and_reads_back_a_change_set(pool: SqlitePool) {
        let change_set = crate::ChangeSet::mock();

        let inserted = change_set.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, change_set.id);
        assert_eq!(inserted.table_name, change_set.table_name);
        assert_eq!(inserted.row_id, change_set.row_id);
        assert_eq!(inserted.field_name, change_set.field_name);
        assert_eq!(inserted.value, change_set.value);
        assert_eq!(inserted.hlc, change_set.hlc);
        assert_eq!(inserted.client_id, change_set.client_id);
    }

    #[sqlx::test]
    async fn insert_preserves_a_null_value(pool: SqlitePool) {
        let change_set = crate::change_sets::ChangeSetBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_table_name("categories")
            .with_row_id(lib_core::RowID::mock())
            .with_field_name("description")
            .with_value_opt(None)
            .with_hlc(lib_core::HybridLogicalClock::mock())
            .with_client_id(lib_core::RowID::mock())
            .build()
            .unwrap();

        let inserted = change_set.insert(&pool).await.unwrap();

        assert_eq!(inserted.value, None);
    }
}
