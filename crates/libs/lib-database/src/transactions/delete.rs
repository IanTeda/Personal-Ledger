//! # Transactions Delete Operations
//!
//! FR.20's "reverse its effect on the linked account's Balance" needs no extra code: Balance
//! is compute-on-read (`starting_balance` + `SUM(transactions.amount)`, see "Decide the TUI
//! app's data model and persistence layer"), so a plain DELETE already removes this
//! Transaction's contribution.

use lib_core as domain;

impl crate::Transactions {
    /// Delete a Transaction by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Transaction exists with this `id`,
    /// or an error if the underlying query fails.
    #[tracing::instrument(name = "Delete Transaction by id: ", level = "debug", skip(pool))]
    pub async fn delete_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<()> {
        let result = sqlx::query!(r#"DELETE FROM transactions WHERE id = ?"#, id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!("Transaction {id} not found")));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_category_and_account(pool: &SqlitePool) -> (lib_core::RowID, lib_core::RowID) {
        let category = crate::Categories::mock().insert(pool).await.unwrap();
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        let account = crate::Accounts::mock(unit.id).insert(pool).await.unwrap();
        (category.id, account.id)
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_removes_the_row(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        let transaction = crate::Transactions::mock(category_id, account_id)
            .insert(&pool)
            .await
            .unwrap();

        crate::Transactions::delete_by_id(transaction.id, &pool).await.unwrap();

        assert!(
            crate::Transactions::find_by_id(transaction.id, &pool)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_errors_when_missing(pool: SqlitePool) {
        let result = crate::Transactions::delete_by_id(lib_core::RowID::new(), &pool).await;
        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }
}
