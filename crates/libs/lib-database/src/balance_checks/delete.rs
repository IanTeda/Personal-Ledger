//! # Balance Checks Delete Operations

use lib_core as domain;

impl crate::BalanceChecks {
    /// Delete a Balance Check by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Balance Check exists with this `id`,
    /// or an error if the underlying query fails.
    #[tracing::instrument(name = "Delete Balance Check by id: ", level = "debug", skip(pool))]
    pub async fn delete_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<()> {
        let result = sqlx::query!(r#"DELETE FROM balance_checks WHERE id = ?"#, id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Balance Check {id} not found"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_account(pool: &SqlitePool) -> lib_core::RowID {
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        crate::Accounts::mock(unit.id)
            .insert(pool)
            .await
            .unwrap()
            .id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_removes_the_row(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let balance_check = crate::BalanceChecks::mock(account_id)
            .insert(&pool)
            .await
            .unwrap();

        crate::BalanceChecks::delete_by_id(balance_check.id, &pool)
            .await
            .unwrap();

        assert!(
            crate::BalanceChecks::find_by_id(balance_check.id, &pool)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_errors_when_missing(pool: SqlitePool) {
        let result = crate::BalanceChecks::delete_by_id(lib_core::RowID::new(), &pool).await;
        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }
}
