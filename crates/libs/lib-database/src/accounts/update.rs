//! # Accounts Update Operations
//!
//! FR.13 scopes update to name, type, and active status — `unit_id` and `starting_balance`
//! are immutable after creation (a Unit is "fixed at creation", FR.10), so `update` only
//! touches the fields FR.13 actually allows.

use lib_core as domain;

impl crate::Accounts {
    /// Replace this Account's name, type, and active flag, then re-reads the row to confirm.
    /// `unit_id`, `starting_balance`, `id`, and `created_on` never change here.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Account exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(
        name = "Update Account: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id, name = %self.name),
    )]
    pub async fn update(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        let result = sqlx::query!(
            r#"
                UPDATE accounts
                SET name = ?, account_type = ?, is_active = ?
                WHERE id = ?
            "#,
            self.name,
            self.account_type,
            self.is_active,
            self.id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Account {} not found",
                self.id
            )));
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!("Account {} not found after update", self.id))
        })
    }

    /// Activate or deactivate an Account by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Account exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(name = "Set Account active flag: ", level = "debug", skip(pool))]
    pub async fn set_active(
        id: domain::RowID,
        is_active: bool,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        let result = sqlx::query!(
            r#"UPDATE accounts SET is_active = ? WHERE id = ?"#,
            is_active,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Account {id} not found"
            )));
        }

        Self::find_by_id(id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!("Account {id} not found after update"))
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_unit(pool: &SqlitePool) -> lib_core::RowID {
        crate::Units::mock().insert(pool).await.unwrap().id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_replaces_editable_fields(pool: SqlitePool) {
        let unit_id = seed_unit(&pool).await;
        let mut account = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();
        account.name = "Updated Name".to_string();
        account.account_type = lib_core::AccountType::Bank;

        let updated = account.update(&pool).await.unwrap();

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.account_type, lib_core::AccountType::Bank);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_errors_when_account_missing(pool: SqlitePool) {
        let account = crate::Accounts::mock(lib_core::RowID::new());
        let result = account.update(&pool).await;
        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn set_active_toggles_the_flag(pool: SqlitePool) {
        let unit_id = seed_unit(&pool).await;
        let account = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();
        assert!(account.is_active);

        let deactivated = crate::Accounts::set_active(account.id, false, &pool)
            .await
            .unwrap();
        assert!(!deactivated.is_active);
    }
}
