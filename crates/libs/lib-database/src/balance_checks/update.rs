//! # Balance Checks Update Operations
//!
//! FR.31 scopes update to date and the asserted balance amount — `account_id` is immutable
//! after creation, mirroring `Accounts::unit_id`.

impl crate::BalanceChecks {
    /// Replace this Balance Check's date and asserted balance, then re-reads the row to
    /// confirm. `account_id`, `id`, and `created_on` never change here.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Balance Check exists with this `id`,
    /// or an error if the underlying query fails.
    #[tracing::instrument(
        name = "Update Balance Check: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id),
    )]
    pub async fn update(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Self> {
        let result = sqlx::query!(
            r#"
                UPDATE balance_checks
                SET date = ?, asserted_balance = ?
                WHERE id = ?
            "#,
            self.date,
            self.asserted_balance,
            self.id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!(
                "Balance Check {} not found",
                self.id
            )));
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::Error::NotFound(format!("Balance Check {} not found after update", self.id))
        })
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
    async fn update_replaces_editable_fields(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let mut balance_check = crate::BalanceChecks::mock(account_id)
            .insert(&pool)
            .await
            .unwrap();
        balance_check.asserted_balance = lib_core::Money::mock();
        balance_check.date = chrono::Utc::now().date_naive();

        let updated = balance_check.update(&pool).await.unwrap();

        assert_eq!(updated.asserted_balance, balance_check.asserted_balance);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_errors_when_balance_check_missing(pool: SqlitePool) {
        let balance_check = crate::BalanceChecks::mock(lib_core::RowID::new());
        let result = balance_check.update(&pool).await;
        assert!(matches!(result, Err(crate::Error::NotFound(_))));
    }
}
