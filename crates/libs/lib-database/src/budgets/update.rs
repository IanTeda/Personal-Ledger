//! # Budgets Update Operations
//!
//! FR.26 scopes update to limit amount, period, and active status — `category_id` and
//! `unit_id` are immutable after creation, mirroring `Accounts::unit_id`.

impl crate::Budgets {
    /// Replace this Budget's limit amount, period, and active flag, then re-reads the row to
    /// confirm. `category_id`, `unit_id`, `id`, and `created_on` never change here.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Budget exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(
        name = "Update Budget: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id),
    )]
    pub async fn update(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        let result = sqlx::query!(
            r#"
                UPDATE budgets
                SET limit_amount = ?, period = ?, is_active = ?
                WHERE id = ?
            "#,
            self.limit_amount,
            self.period,
            self.is_active,
            self.id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Budget {} not found",
                self.id
            )));
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!("Budget {} not found after update", self.id))
        })
    }

    /// Activate or deactivate a Budget by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Budget exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(name = "Set Budget active flag: ", level = "debug", skip(pool))]
    pub async fn set_active(
        id: lib_core::RowID,
        is_active: bool,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        let result = sqlx::query!(
            r#"UPDATE budgets SET is_active = ? WHERE id = ?"#,
            is_active,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Budget {id} not found"
            )));
        }

        Self::find_by_id(id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!("Budget {id} not found after update"))
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_expense_category(pool: &SqlitePool) -> lib_core::RowID {
        let mut category = crate::Categories::mock();
        category.category_type = lib_core::CategoryTypes::Expense;
        category.insert(pool).await.unwrap().id
    }

    async fn seed_unit(pool: &SqlitePool) -> lib_core::RowID {
        crate::Units::mock().insert(pool).await.unwrap().id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_replaces_editable_fields(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit_id = seed_unit(&pool).await;
        let mut budget = crate::Budgets::mock(category_id, unit_id)
            .insert(&pool)
            .await
            .unwrap();
        budget.limit_amount = lib_core::Money::mock();
        budget.period = lib_core::BudgetPeriod::Yearly;

        let updated = budget.update(&pool).await.unwrap();

        assert_eq!(updated.period, lib_core::BudgetPeriod::Yearly);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn update_errors_when_budget_missing(pool: SqlitePool) {
        let budget = crate::Budgets::mock(lib_core::RowID::new(), lib_core::RowID::new());
        let result = budget.update(&pool).await;
        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn set_active_toggles_the_flag(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit_id = seed_unit(&pool).await;
        let budget = crate::Budgets::mock(category_id, unit_id)
            .insert(&pool)
            .await
            .unwrap();
        assert!(budget.is_active);

        let deactivated = crate::Budgets::set_active(budget.id, false, &pool)
            .await
            .unwrap();
        assert!(!deactivated.is_active);
    }
}
