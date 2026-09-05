//! # Account Balance
//!
//! Computes an Account's current Balance on read (FR.34, "Decide the TUI app's data model
//! and persistence layer"): `starting_balance` plus the sum of every Transaction posted
//! against it. Summed in Rust via `BigDecimal`, not SQL `SUM()`, since `Money` is stored as
//! arbitrary-precision TEXT that SQLite's own `SUM()` can't total correctly — the same
//! approach `Budgets::current_progress` already uses.

impl crate::Accounts {
    /// This Account's current Balance: `starting_balance` plus every Transaction posted
    /// against it, in this Account's own Unit (Transactions never cross Units).
    ///
    /// # Errors
    /// Returns an error if the underlying query fails.
    #[tracing::instrument(
        name = "Compute Account balance: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id),
    )]
    pub async fn balance(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<lib_core::Money> {
        let amounts = sqlx::query_scalar!(
            r#"SELECT amount AS "amount!: lib_core::Money" FROM transactions WHERE account_id = ?"#,
            self.id
        )
        .fetch_all(pool)
        .await?;

        let total = amounts
            .iter()
            .fold(self.starting_balance.0.clone(), |acc, amount| {
                acc + &amount.0
            });

        Ok(lib_core::Money::from(total))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_account(pool: &SqlitePool, starting_balance: &str) -> crate::Accounts {
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        let mut account = crate::Accounts::mock(unit.id);
        account.starting_balance = starting_balance.parse().unwrap();
        account.insert(pool).await.unwrap()
    }

    async fn seed_category(pool: &SqlitePool) -> lib_core::RowID {
        crate::Categories::mock().insert(pool).await.unwrap().id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn balance_is_just_starting_balance_with_no_transactions(pool: SqlitePool) {
        let account = seed_account(&pool, "100").await;

        let balance = account.balance(&pool).await.unwrap();

        assert_eq!(balance, "100".parse().unwrap());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn balance_adds_every_transaction_to_the_starting_balance(pool: SqlitePool) {
        let account = seed_account(&pool, "100").await;
        let category_id = seed_category(&pool).await;

        for amount in ["-20", "-30", "5"] {
            let mut transaction = crate::Transactions::mock(category_id, account.id);
            transaction.status = lib_core::TransactionStatus::Open;
            transaction.amount = amount.parse().unwrap();
            transaction.insert(&pool).await.unwrap();
        }

        let balance = account.balance(&pool).await.unwrap();

        assert_eq!(balance, "55".parse().unwrap());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn balance_ignores_transactions_on_other_accounts(pool: SqlitePool) {
        let account = seed_account(&pool, "100").await;
        let other_account = seed_account(&pool, "0").await;
        let category_id = seed_category(&pool).await;

        let mut transaction = crate::Transactions::mock(category_id, other_account.id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.amount = "-999".parse().unwrap();
        transaction.insert(&pool).await.unwrap();

        let balance = account.balance(&pool).await.unwrap();

        assert_eq!(balance, "100".parse().unwrap());
    }
}
