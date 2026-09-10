//! # Units Delete Operations
//!
//! No "still in use" guard against Accounts here — Accounts don't exist yet in this build
//! order ([issue #69](https://github.com/IanTeda/Personal-Ledger/issues/69) adds the
//! `unit_id` foreign key). That ticket is where a delete-blocked-while-referenced check
//! belongs, once there's something to reference.

use lib_core as domain;

impl crate::Units {
    /// Delete a Unit by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Unit exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(name = "Delete Unit by id: ", level = "debug", skip(pool))]
    pub async fn delete_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let result = sqlx::query!(r#"DELETE FROM units WHERE id = ?"#, id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!("Unit {id} not found")));
        }

        Ok(())
    }

    /// Delete several Units by id in one transaction — either every id is deleted, or none
    /// are (atomic, matching NFR.1's reliability priority).
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] naming the first id that doesn't exist,
    /// rolling back the whole batch, or an error if the underlying query fails.
    #[tracing::instrument(name = "Delete many Units by id: ", level = "debug", skip(pool, ids))]
    pub async fn delete_many_by_id(
        ids: &[domain::RowID],
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let mut tx = pool.begin().await?;

        for id in ids {
            let result = sqlx::query!(r#"DELETE FROM units WHERE id = ?"#, id)
                .execute(&mut *tx)
                .await?;

            if result.rows_affected() == 0 {
                return Err(crate::Error::NotFound(format!("Unit {id} not found")));
            }
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_removes_the_row(pool: SqlitePool) {
        let unit = crate::Units::mock().insert(&pool).await.unwrap();

        crate::Units::delete_by_id(unit.id, &pool).await.unwrap();

        assert!(
            crate::Units::find_by_id(unit.id, &pool)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_errors_when_missing(pool: SqlitePool) {
        let result = crate::Units::delete_by_id(lib_core::RowID::new(), &pool).await;
        assert!(matches!(result, Err(crate::Error::NotFound(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_many_by_id_removes_every_row(pool: SqlitePool) {
        let a = crate::Units::mock().insert(&pool).await.unwrap();
        let b = crate::Units::mock().insert(&pool).await.unwrap();

        crate::Units::delete_many_by_id(&[a.id, b.id], &pool)
            .await
            .unwrap();

        assert!(
            crate::Units::find_by_id(a.id, &pool)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            crate::Units::find_by_id(b.id, &pool)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_many_by_id_rolls_back_on_a_missing_id(pool: SqlitePool) {
        let a = crate::Units::mock().insert(&pool).await.unwrap();
        let missing = lib_core::RowID::new();

        let result = crate::Units::delete_many_by_id(&[a.id, missing], &pool).await;

        assert!(result.is_err());
        assert!(
            crate::Units::find_by_id(a.id, &pool)
                .await
                .unwrap()
                .is_some(),
            "the batch should roll back, leaving `a` in place"
        );
    }
}
