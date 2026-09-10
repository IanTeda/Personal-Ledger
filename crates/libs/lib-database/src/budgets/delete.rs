//! # Budgets Delete Operations

use lib_core as domain;

impl crate::Budgets {
    /// Delete a Budget by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Budget exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(name = "Delete Budget by id: ", level = "debug", skip(pool))]
    pub async fn delete_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let result = sqlx::query!(r#"DELETE FROM budgets WHERE id = ?"#, id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::Error::NotFound(format!("Budget {id} not found")));
        }

        Ok(())
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
    async fn delete_by_id_removes_the_row(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit_id = seed_unit(&pool).await;
        let budget = crate::Budgets::mock(category_id, unit_id)
            .insert(&pool)
            .await
            .unwrap();

        crate::Budgets::delete_by_id(budget.id, &pool)
            .await
            .unwrap();

        assert!(
            crate::Budgets::find_by_id(budget.id, &pool)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn delete_by_id_errors_when_missing(pool: SqlitePool) {
        let result = crate::Budgets::delete_by_id(lib_core::RowID::new(), &pool).await;
        assert!(matches!(result, Err(crate::Error::NotFound(_))));
    }
}
