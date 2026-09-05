//! # Budgets Insert Operations

impl crate::Budgets {
    /// Insert this Budget into the database.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::BudgetCategoryNotExpense`] if `category_id` doesn't
    /// reference an Expense-type Category, or an error if the underlying INSERT or read-back
    /// SELECT fails — including a foreign-key violation if `category_id`/`unit_id` don't
    /// reference real rows (SQLite's `foreign_keys` pragma, enabled for every connection).
    #[tracing::instrument(
        name = "Insert new Budget into database: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id, category_id = %self.category_id),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        let category = crate::Categories::find_by_id(self.category_id, pool)
            .await?
            .ok_or_else(|| {
                crate::DatabaseError::NotFound(format!("Category {} not found", self.category_id))
            })?;
        if category.category_type != lib_core::CategoryTypes::Expense {
            return Err(crate::DatabaseError::BudgetCategoryNotExpense(
                self.category_id.to_string(),
            ));
        }

        let insert_result = sqlx::query!(
            r#"
                INSERT INTO budgets (id, category_id, unit_id, limit_amount, period, is_active, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.category_id,
            self.unit_id,
            self.limit_amount,
            self.period,
            self.is_active,
            self.created_on,
            self.updated_on
        )
        .execute(pool)
        .await;

        match insert_result {
            Ok(result) => {
                if result.rows_affected() != 1 {
                    tracing::warn!(
                        "INSERT operation affected {} rows instead of 1 for budget: {}",
                        result.rows_affected(),
                        self.id
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert budget {}: {}", self.id, e);
                return Err(e.into());
            }
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!("Budget {} not found after insert", self.id))
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
    async fn insert_persists_and_reads_back_a_budget(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit_id = seed_unit(&pool).await;
        let budget = crate::Budgets::mock(category_id, unit_id);

        let inserted = budget.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, budget.id);
        assert_eq!(inserted.category_id, category_id);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_rejects_a_non_expense_category(pool: SqlitePool) {
        let mut category = crate::Categories::mock();
        category.category_type = lib_core::CategoryTypes::Income;
        let category = category.insert(&pool).await.unwrap();
        let unit_id = seed_unit(&pool).await;
        let budget = crate::Budgets::mock(category.id, unit_id);

        let result = budget.insert(&pool).await;

        assert!(matches!(
            result,
            Err(crate::DatabaseError::BudgetCategoryNotExpense(_))
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_rejects_an_unknown_unit_id(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let budget = crate::Budgets::mock(category_id, lib_core::RowID::new());

        let result = budget.insert(&pool).await;

        assert!(
            result.is_err(),
            "foreign_keys pragma should reject the unknown unit_id"
        );
    }
}
