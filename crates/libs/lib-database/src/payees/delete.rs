//! # Payees Delete Operations
//!
//! A bare delete: SQLite's `foreign_keys` pragma (enabled for every connection, see
//! `DatabaseConnection::new`) rejects deleting a Payee still referenced by a Transaction's
//! `payee_id` or by its own Payee Alias history, with no extra guard code needed here.

use lib_core as domain;

impl crate::Payees {
    /// Delete a Payee by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Payee exists with this `id`, or an
    /// error if the underlying query fails — including a foreign-key violation if any
    /// Transaction or Payee Alias still references it.
    #[tracing::instrument(name = "Delete Payee by id: ", level = "debug", skip(pool))]
    pub async fn delete_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<()> {
        let result = sqlx::query!(r#"DELETE FROM payees WHERE id = ?"#, id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Payee {id} not found"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_removes_the_row(pool: SqlitePool) {
        let payee = crate::Payees::mock().insert(&pool).await.unwrap();

        crate::Payees::delete_by_id(payee.id, &pool).await.unwrap();

        assert!(
            crate::Payees::find_by_id(payee.id, &pool)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_errors_when_missing(pool: SqlitePool) {
        let result = crate::Payees::delete_by_id(lib_core::RowID::new(), &pool).await;
        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_rejects_a_payee_still_referenced_by_an_alias(pool: SqlitePool) {
        let mut payee = crate::Payees::mock();
        payee.name = "Kmart".to_string();
        let payee = payee.insert(&pool).await.unwrap();
        crate::Payees::rename(payee.id, "Kmart AU", &pool)
            .await
            .unwrap();

        let result = crate::Payees::delete_by_id(payee.id, &pool).await;

        assert!(
            result.is_err(),
            "the foreign_keys pragma should reject deleting a Payee its own alias references"
        );
    }
}
