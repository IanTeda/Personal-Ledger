//! # Transactions Update Operations
//!
//! `update` covers FR.19's "other fields" (date, amount, category, account, payee,
//! description) — locked while the Transaction's *current* status is Reconciled. `Status`
//! and `Flagged` go through their own dedicated methods instead, since they're always
//! settable independently of everything else (see `CONTEXT.md`'s Transaction Status and
//! Flagged entries) and of the Reconciled-lock in particular.

use lib_core as domain;

impl crate::Transactions {
    /// Replace this Transaction's date, amount, Category, Account, Payee, and description.
    /// Does not touch `status`/`is_flagged` — see [`Self::set_status`]/[`Self::set_flagged`].
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::TransactionReconciled`] if the Transaction's current
    /// status is Reconciled, [`crate::DatabaseError::TransactionCrossUnitMove`] if this
    /// would move it to an Account denominated in a different Unit, or
    /// [`crate::DatabaseError::NotFound`] if no Transaction exists with this `id`.
    #[tracing::instrument(name = "Update Transaction: ", level = "debug", skip(self, pool), fields(id = %self.id))]
    pub async fn update(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Self> {
        let current = Self::find_by_id(self.id, pool)
            .await?
            .ok_or_else(|| crate::Error::NotFound(format!("Transaction {} not found", self.id)))?;

        if current.status == domain::TransactionStatus::Reconciled {
            return Err(crate::Error::TransactionReconciled(self.id.to_string()));
        }

        if self.account_id != current.account_id {
            let old_account = crate::Accounts::find_by_id(current.account_id, pool)
                .await?
                .ok_or_else(|| {
                    crate::Error::NotFound(format!("Account {} not found", current.account_id))
                })?;
            let new_account = crate::Accounts::find_by_id(self.account_id, pool)
                .await?
                .ok_or_else(|| {
                    crate::Error::NotFound(format!("Account {} not found", self.account_id))
                })?;
            if old_account.unit_id != new_account.unit_id {
                return Err(crate::Error::TransactionCrossUnitMove(
                    self.id.to_string(),
                    self.account_id.to_string(),
                ));
            }
        }

        let result = sqlx::query!(
            r#"
                UPDATE transactions
                SET date = ?, amount = ?, category_id = ?, account_id = ?, payee_id = ?, description = ?
                WHERE id = ?
            "#,
            self.date,
            self.amount,
            self.category_id,
            self.account_id,
            self.payee_id,
            self.description,
            self.id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!(
                "Transaction {} not found",
                self.id
            )));
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::Error::NotFound(format!("Transaction {} not found after update", self.id))
        })
    }

    /// Set a Transaction's reconciliation-workflow status — always allowed, independent of
    /// the current status (this is how a Reconciled Transaction gets un-locked).
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Transaction exists with this `id`.
    #[tracing::instrument(name = "Set Transaction status: ", level = "debug", skip(pool))]
    pub async fn set_status(
        id: domain::RowID,
        status: domain::TransactionStatus,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Self> {
        let result = sqlx::query!(
            r#"UPDATE transactions SET status = ? WHERE id = ?"#,
            status,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!(
                "Transaction {id} not found"
            )));
        }

        Self::find_by_id(id, pool).await?.ok_or_else(|| {
            crate::Error::NotFound(format!("Transaction {id} not found after update"))
        })
    }

    /// Set a Transaction's Flagged marker — always allowed, independent of `status` and of
    /// the Reconciled-lock.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Transaction exists with this `id`.
    #[tracing::instrument(name = "Set Transaction flagged marker: ", level = "debug", skip(pool))]
    pub async fn set_flagged(
        id: domain::RowID,
        is_flagged: bool,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Self> {
        let result = sqlx::query!(
            r#"UPDATE transactions SET is_flagged = ? WHERE id = ?"#,
            is_flagged,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!(
                "Transaction {id} not found"
            )));
        }

        Self::find_by_id(id, pool).await?.ok_or_else(|| {
            crate::Error::NotFound(format!("Transaction {id} not found after update"))
        })
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
    async fn update_replaces_editable_fields(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        // `Transactions::mock()` picks a random status, which may be Reconciled -- pin it to
        // Open so this test exercises a plain, unlocked update rather than depending on luck.
        let mut transaction = crate::Transactions::mock(category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        let mut transaction = transaction.insert(&pool).await.unwrap();
        transaction.description = Some("Updated description".to_string());

        let updated = transaction.update(&pool).await.unwrap();

        assert_eq!(updated.description.as_deref(), Some("Updated description"));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_rejects_editing_a_reconciled_transaction(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        let mut transaction = crate::Transactions::mock(category_id, account_id)
            .insert(&pool)
            .await
            .unwrap();
        crate::Transactions::set_status(
            transaction.id,
            lib_core::TransactionStatus::Reconciled,
            &pool,
        )
        .await
        .unwrap();

        transaction.description = Some("Trying to sneak in a change".to_string());
        let result = transaction.update(&pool).await;

        assert!(matches!(
            result,
            Err(crate::Error::TransactionReconciled(_))
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn set_status_always_works_even_while_reconciled(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        let transaction = crate::Transactions::mock(category_id, account_id)
            .insert(&pool)
            .await
            .unwrap();
        crate::Transactions::set_status(
            transaction.id,
            lib_core::TransactionStatus::Reconciled,
            &pool,
        )
        .await
        .unwrap();

        // Un-reconciling is how a locked Transaction gets unlocked again.
        let unlocked = crate::Transactions::set_status(
            transaction.id,
            lib_core::TransactionStatus::Cleared,
            &pool,
        )
        .await
        .unwrap();

        assert_eq!(unlocked.status, lib_core::TransactionStatus::Cleared);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn set_flagged_always_works_even_while_reconciled(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        let transaction = crate::Transactions::mock(category_id, account_id)
            .insert(&pool)
            .await
            .unwrap();
        crate::Transactions::set_status(
            transaction.id,
            lib_core::TransactionStatus::Reconciled,
            &pool,
        )
        .await
        .unwrap();

        let flagged = crate::Transactions::set_flagged(transaction.id, true, &pool)
            .await
            .unwrap();

        assert!(flagged.is_flagged);
        assert_eq!(flagged.status, lib_core::TransactionStatus::Reconciled);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_rejects_a_move_to_a_different_unit_account(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        let mut transaction = crate::Transactions::mock(category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        let mut transaction = transaction.insert(&pool).await.unwrap();

        let other_unit = crate::Units::mock().insert(&pool).await.unwrap();
        let other_account = crate::Accounts::mock(other_unit.id)
            .insert(&pool)
            .await
            .unwrap();
        transaction.account_id = other_account.id;

        let result = transaction.update(&pool).await;

        assert!(matches!(
            result,
            Err(crate::Error::TransactionCrossUnitMove(_, _))
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_allows_a_move_to_a_same_unit_account(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        let unit_id = crate::Accounts::find_by_id(account_id, &pool)
            .await
            .unwrap()
            .unwrap()
            .unit_id;
        let mut transaction = crate::Transactions::mock(category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        let mut transaction = transaction.insert(&pool).await.unwrap();

        let same_unit_account = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();
        transaction.account_id = same_unit_account.id;

        let updated = transaction.update(&pool).await.unwrap();

        assert_eq!(updated.account_id, same_unit_account.id);
    }
}
