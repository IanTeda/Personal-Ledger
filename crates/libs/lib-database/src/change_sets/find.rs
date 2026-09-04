//! # Change Set Query Operations
//!
//! Provides read operations against the durable `change_sets` log -- primarily the
//! Sync Server's pull cursor query: every Change Set after the last one a Client has
//! already applied.

use lib_core as domain;

impl crate::ChangeSet {
    /// Find every Change Set with an `id` greater than `since_id`, oldest first, up to
    /// `limit` rows. Passing `since_id: None` returns Change Sets from the start of the
    /// log -- what a Client pulls the first time it connects (including the "was
    /// offline, catching up" case: every Change Set queued while it was down comes back
    /// in one ordered batch).
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(
        name = "Find Change Sets since a given RowID: ",
        level = "debug",
        skip(pool),
        fields(since_id = ?since_id, limit = %limit),
    )]
    pub async fn find_since(
        since_id: Option<domain::RowID>,
        limit: i64,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let change_sets = match since_id {
            Some(since_id) => {
                sqlx::query_as!(
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
                        WHERE id > ?
                        ORDER BY id ASC
                        LIMIT ?
                    "#,
                    since_id,
                    limit
                )
                .fetch_all(pool)
                .await?
            }
            None => {
                sqlx::query_as!(
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
                        ORDER BY id ASC
                        LIMIT ?
                    "#,
                    limit
                )
                .fetch_all(pool)
                .await?
            }
        };

        tracing::trace!(
            "Found {} Change Set(s) since {:?}",
            change_sets.len(),
            since_id
        );

        Ok(change_sets)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn find_since_none_returns_everything_oldest_first(pool: SqlitePool) {
        let first = crate::ChangeSet::mock().insert(&pool).await.unwrap();
        let second = crate::ChangeSet::mock().insert(&pool).await.unwrap();

        let found = crate::ChangeSet::find_since(None, 100, &pool)
            .await
            .unwrap();

        let ids: Vec<_> = found.iter().map(|c| c.id).collect();
        assert!(ids.contains(&first.id));
        assert!(ids.contains(&second.id));
        // Chronologically sortable RowIDs -> insertion order is preserved.
        let first_pos = ids.iter().position(|id| *id == first.id).unwrap();
        let second_pos = ids.iter().position(|id| *id == second.id).unwrap();
        assert!(first_pos < second_pos);
    }

    #[sqlx::test]
    async fn find_since_a_cursor_excludes_earlier_change_sets(pool: SqlitePool) {
        let first = crate::ChangeSet::mock().insert(&pool).await.unwrap();
        let second = crate::ChangeSet::mock().insert(&pool).await.unwrap();

        let found = crate::ChangeSet::find_since(Some(first.id), 100, &pool)
            .await
            .unwrap();

        assert!(!found.iter().any(|c| c.id == first.id));
        assert!(found.iter().any(|c| c.id == second.id));
    }

    #[sqlx::test]
    async fn find_since_respects_the_limit(pool: SqlitePool) {
        for _ in 0..5 {
            crate::ChangeSet::mock().insert(&pool).await.unwrap();
        }

        let found = crate::ChangeSet::find_since(None, 2, &pool).await.unwrap();

        assert_eq!(found.len(), 2);
    }
}
