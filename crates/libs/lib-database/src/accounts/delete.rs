//! # Accounts Delete Operations
//!
//! FR.14 also asks for "an option to transfer all the transactions under an account to
//! another account" on delete — deferred to
//! [issue #70](https://github.com/IanTeda/Personal-Ledger/issues/70) (Transactions), since
//! there's nothing to transfer until the `transactions` table exists. This is a bare delete
//! for now.

use lib_core as domain;

impl crate::Accounts {
    /// Delete an Account by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Account exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(name = "Delete Account by id: ", level = "debug", skip(pool))]
    pub async fn delete_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<()> {
        let result = sqlx::query!(r#"DELETE FROM accounts WHERE id = ?"#, id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Account {id} not found"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_unit(pool: &SqlitePool) -> lib_core::RowID {
        crate::Units::mock().insert(pool).await.unwrap().id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_removes_the_row(pool: SqlitePool) {
        let unit_id = seed_unit(&pool).await;
        let account = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();

        crate::Accounts::delete_by_id(account.id, &pool)
            .await
            .unwrap();

        assert!(
            crate::Accounts::find_by_id(account.id, &pool)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_errors_when_missing(pool: SqlitePool) {
        let result = crate::Accounts::delete_by_id(lib_core::RowID::new(), &pool).await;
        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }
}
