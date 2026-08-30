//! Category deletion operations for the Personal Ledger database.
//!
//! This module provides functions for deleting category records from the SQLite database.
//! It supports single deletions, bulk deletions with transactional guarantees, and
//! targeted deletions by various criteria.
//!
//! All deletion operations are permanent and cannot be undone. They ensure data integrity
//! by checking for existence before deletion and using transactions for atomic bulk operations.
//!
//! The module follows these key principles:
//! - **Safety**: Existence checks prevent silent failures
//! - **Atomicity**: Bulk operations use transactions to ensure consistency
//! - **Observability**: Detailed tracing from TRACE to ERROR levels
//! - **Security**: No sensitive data logged; operations are idempotent where possible

use lib_domain as domain;


impl crate::Categories {

    /// Deletes the current category instance from the database.
    ///
    /// This method permanently removes the category record associated with this instance.
    /// It checks for the category's existence before deletion and returns an error if not found.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<()>` indicating success or failure.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - The category with the given ID does not exist in the database.
    /// - A database connection or query execution error occurs.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lib_database::Categories;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut category = Categories::mock();
    /// category.id = lib_domain::RowID::new();
    /// // Assume category is inserted first...
    /// category.delete(pool).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security
    /// This function does not perform any input validation beyond database constraints.
    /// Ensure category IDs are validated before calling this function.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, WARN on not found, ERROR on database failures.
    #[tracing::instrument(
        name = "Category delete",
        level = "debug",
        skip(pool),
        fields(
            category_id = %self.id,
            category_code = %self.code
        ),
        err
    )]
    pub async fn delete(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<()> {
        tracing::trace!(
            category_id = %self.id,
            category_code = %self.code,
            "Starting category deletion operation"
        );

        let delete_query = sqlx::query!(
            r#"
                DELETE FROM categories
                WHERE id = ?
            "#,
            self.id
        );

        let rows_affected = delete_query.execute(pool).await?.rows_affected();

        if rows_affected == 0 {
            tracing::warn!(
                category_id = %self.id,
                "Category deletion failed - category not found"
            );
            return Err(crate::DatabaseError::NotFound(format!(
                "Category with id {} not found",
                self.id
            )));
        }

        tracing::info!(
            category_id = %self.id,
            category_code = %self.code,
            "Deleted category from database"
        );

        Ok(())
    }

    /// Deletes a category by its unique ID.
    ///
    /// This function permanently removes a category record by its ID. It checks for existence
    /// before deletion and returns an error if the category is not found.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the category to delete.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<()>` indicating success or failure.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - The category with the given ID does not exist in the database.
    /// - A database connection or query execution error occurs.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lib_database::Categories;
    /// # use lib_domain::RowID;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let category_id = RowID::from(123);
    /// Categories::delete_by_id(category_id, pool).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security
    /// This function does not perform any input validation beyond database constraints.
    /// Ensure IDs are validated before calling this function.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, WARN on not found, ERROR on database failures.
    #[tracing::instrument(
        name = "Category delete by ID",
        level = "debug",
        skip(pool),
        fields(category_id = %id),
        err
    )]
    pub async fn delete_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<()> {
        tracing::trace!(
            category_id = %id,
            "Starting category deletion by ID operation"
        );

        let delete_query = sqlx::query!(
            r#"
                DELETE FROM categories
                WHERE id = ?
            "#,
            id
        );

        let rows_affected = delete_query.execute(pool).await?.rows_affected();

        if rows_affected == 0 {
            tracing::warn!(
                category_id = %id,
                "Category deletion by ID failed - category not found"
            );
            return Err(crate::DatabaseError::NotFound(format!(
                "Category with id {} not found",
                id
            )));
        }

        tracing::info!(
            category_id = %id,
            "Deleted category by ID from database"
        );

        Ok(())
    }

    /// Deletes multiple categories by their IDs in a single transaction.
    ///
    /// This function permanently removes multiple category records atomically within a database transaction.
    /// If any deletion fails (e.g., category not found), the entire operation is rolled back.
    ///
    /// # Arguments
    /// * `ids` - A slice of unique identifiers for the categories to delete.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<()>` indicating success or failure.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - Any category with the given IDs does not exist in the database.
    /// - A database connection, transaction, or query execution error occurs.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lib_database::Categories;
    /// # use lib_domain::RowID;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let ids = vec![RowID::from(123), RowID::from(456)];
    /// Categories::delete_many_by_id(&ids, pool).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Performance
    /// This operation uses a database transaction. For large numbers of IDs, consider the transaction size
    /// and database performance implications. The transaction holds locks until completion.
    ///
    /// # Security
    /// This function does not perform any input validation beyond database constraints.
    /// Ensure IDs are validated before calling this function.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, INFO on success, WARN on individual failures, ERROR on transaction rollback.
    #[tracing::instrument(
        name = "Bulk category delete",
        level = "debug",
        skip(pool, ids),
        fields(category_count = %ids.len()),
        err
    )]
    pub async fn delete_many_by_id(
        ids: &[domain::RowID],
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<()> {
        let category_count = ids.len();

        if category_count == 0 {
            tracing::debug!("Bulk delete called with empty ID list, returning early");
            return Ok(());
        }

        tracing::trace!(
            category_count = %category_count,
            "Starting bulk category deletion operation"
        );

        // Use a transaction for atomicity
        let mut tx = pool.begin().await?;
        tracing::debug!("Database transaction started for bulk delete");

        for &id in ids {
            tracing::debug!(
                category_id = %id,
                "Processing category deletion in bulk operation"
            );

            let delete_query = sqlx::query!(
                r#"
                    DELETE FROM categories
                    WHERE id = ?
                "#,
                id
            );

            let rows_affected = delete_query.execute(&mut *tx).await?.rows_affected();

            if rows_affected == 0 {
                tracing::warn!(
                    category_id = %id,
                    "Category not found during bulk delete, rolling back transaction"
                );
                return Err(crate::DatabaseError::NotFound(format!(
                    "Category with id {} not found",
                    id
                )));
            }
        }

        // Commit the transaction
        tx.commit().await?;
        tracing::debug!("Database transaction committed for bulk delete");

        tracing::info!(
            category_count = %category_count,
            "Successfully deleted multiple categories from database"
        );

        Ok(())
    }

    /// Deletes all inactive categories from the database.
    ///
    /// This function permanently removes all categories where `is_active` is false.
    /// It returns the number of categories deleted.
    ///
    /// # Arguments
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<u64>` containing the number of categories deleted.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lib_database::Categories;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let deleted_count = Categories::delete_inactive(pool).await?;
    /// println!("Deleted {} inactive categories", deleted_count);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security
    /// This function performs a bulk deletion without individual checks. Use with caution.
    ///
    /// # Performance
    /// This operation scans the entire categories table to find inactive records.
    /// Consider the table size and query performance implications for large datasets.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, ERROR on database failures.
    #[tracing::instrument(
        name = "Delete inactive categories",
        level = "debug",
        skip(pool),
        fields(operation = "delete_inactive"),
        err
    )]
    pub async fn delete_inactive(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<u64> {
        tracing::trace!("Starting delete inactive categories operation");

        let delete_query = sqlx::query!(
            r#"
                DELETE FROM categories
                WHERE is_active = false
            "#
        );

        let rows_affected = delete_query.execute(pool).await?.rows_affected();

        tracing::info!(
            deleted_count = %rows_affected,
            "Deleted inactive categories from database"
        );

        Ok(rows_affected)
    }

    /// Deletes a category by its code.
    ///
    /// This function permanently removes a category record by its unique code. It checks for existence
    /// before deletion and returns an error if the category is not found.
    ///
    /// # Arguments
    /// * `code` - The unique code of the category to delete (e.g., "FOO.BAR.BAZ").
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<()>` indicating success or failure.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - The category with the given code does not exist in the database.
    /// - A database connection or query execution error occurs.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lib_database::Categories;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// Categories::delete_by_code("FOO.BAR.BAZ", pool).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security
    /// This function does not perform any input validation beyond database constraints.
    /// Ensure codes are validated before calling this function.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, WARN on not found, ERROR on database failures.
    #[tracing::instrument(
        name = "Category delete by code",
        level = "debug",
        skip(pool),
        fields(category_code = %code),
        err
    )]
    pub async fn delete_by_code(
        code: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<()> {
        tracing::trace!(
            category_code = %code,
            "Starting category deletion by code operation"
        );

        let delete_query = sqlx::query!(
            r#"
                DELETE FROM categories
                WHERE code = ?
            "#,
            code
        );

        let rows_affected = delete_query.execute(pool).await?.rows_affected();

        if rows_affected == 0 {
            tracing::warn!(
                category_code = %code,
                "Category deletion by code failed - category not found"
            );
            return Err(crate::DatabaseError::NotFound(format!(
                "Category with code '{}' not found",
                code
            )));
        }

        tracing::info!(
            category_code = %code,
            "Deleted category by code from database"
        );

        Ok(())
    }

    /// Deletes a category by its URL slug.
    ///
    /// This function permanently removes a category record by its URL slug. It checks for existence
    /// before deletion and returns an error if the category is not found.
    ///
    /// # Arguments
    /// * `slug` - The URL slug of the category to delete.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<()>` indicating success or failure.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - The category with the given slug does not exist in the database.
    /// - A database connection or query execution error occurs.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lib_database::Categories;
    /// # use lib_domain::UrlSlug;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let slug = UrlSlug::from("groceries");
    /// Categories::delete_by_url_slug(&slug, pool).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security
    /// This function does not perform any input validation beyond database constraints.
    /// Ensure slugs are validated before calling this function.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, WARN on not found, ERROR on database failures.
    #[tracing::instrument(
        name = "Category delete by URL slug",
        level = "debug",
        skip(pool),
        fields(category_slug = %slug.as_str()),
        err
    )]
    pub async fn delete_by_url_slug(
        slug: &domain::UrlSlug,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<()> {
        tracing::trace!(
            category_slug = %slug.as_str(),
            "Starting category deletion by URL slug operation"
        );

        let slug_str = slug.as_str();

        let delete_query = sqlx::query!(
            r#"
                DELETE FROM categories
                WHERE url_slug = ?
            "#,
            slug_str
        );

        let rows_affected = delete_query.execute(pool).await?.rows_affected();

        if rows_affected == 0 {
            tracing::warn!(
                category_slug = %slug.as_str(),
                "Category deletion by URL slug failed - category not found"
            );
            return Err(crate::DatabaseError::NotFound(format!(
                "Category with URL slug '{}' not found",
                slug.as_str()
            )));
        }

        tracing::info!(
            category_slug = %slug.as_str(),
            "Deleted category by URL slug from database"
        );

        Ok(())
    }

    /// Deletes all categories from the database.
    ///
    /// This function permanently removes all category records. Use with extreme caution
    /// as this operation cannot be undone and will affect all data.
    ///
    /// # Arguments
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<u64>` containing the number of categories deleted.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use lib_database::Categories;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// // WARNING: This deletes all categories!
    /// let deleted_count = Categories::delete_all(pool).await?;
    /// println!("Deleted all {} categories", deleted_count);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security
    /// This function performs a destructive bulk deletion. Only use in testing or admin scenarios.
    /// Consider adding authentication checks in production code.
    ///
    /// # Performance
    /// This operation deletes all records from the categories table.
    /// Consider the table size and database performance implications for large datasets.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, WARN on success (due to destructive nature), ERROR on database failures.
    #[tracing::instrument(
        name = "Delete all categories",
        level = "debug",
        skip(pool),
        fields(operation = "delete_all"),
        err
    )]
    pub async fn delete_all(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<u64> {
        tracing::trace!("Starting delete all categories operation");

        let delete_query = sqlx::query!(
            r#"
                DELETE FROM categories
            "#
        );

        let rows_affected = delete_query.execute(pool).await?.rows_affected();

        tracing::warn!(
            deleted_count = %rows_affected,
            "Deleted all categories from database - this is a destructive operation"
        );

        Ok(rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use fake::Fake;

    /// Helper function to insert a test category
    async fn insert_test_category(pool: &SqlitePool, category: &crate::Categories) -> domain::RowID {
        let id_str = category.id.to_string();
        let url_slug_str = category.url_slug.as_ref().map(|s| s.to_string());
        let category_type_str = category.category_type.as_str();
        let color_str = category.color.as_ref().map(|c| c.to_string());
        let created_on_str = category.created_on.to_rfc3339();
        let updated_on_str = category.updated_on.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO categories (
                id, code, name, description, url_slug, category_type,
                color, icon, is_active, created_on, updated_on
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id_str,
            category.code,
            category.name,
            category.description,
            url_slug_str,
            category_type_str,
            color_str,
            category.icon,
            category.is_active,
            created_on_str,
            updated_on_str
        )
        .execute(pool)
        .await
        .unwrap();

        category.id
    }

    mod single_deletions {
        use super::*;

        #[sqlx::test]
        async fn test_delete_existing_category(pool: SqlitePool) {
            let category = crate::Categories::mock();
            insert_test_category(&pool, &category).await;

            let result = category.delete(&pool).await;
            assert!(result.is_ok());

            // Verify deletion by trying to delete again (should fail)
            let result2 = category.delete(&pool).await;
            assert!(matches!(result2, Err(crate::DatabaseError::NotFound(_))));
        }

        #[sqlx::test]
        async fn test_delete_nonexistent_category(pool: SqlitePool) {
            let category = crate::Categories::mock();

            let result = category.delete(&pool).await;
            assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
        }

        #[sqlx::test]
        async fn test_delete_by_id(pool: SqlitePool) {
            let category = crate::Categories::mock();
            let id = insert_test_category(&pool, &category).await;

            let result = crate::Categories::delete_by_id(id, &pool).await;
            assert!(result.is_ok());

            // Verify deletion by trying to delete again
            let result2 = crate::Categories::delete_by_id(id, &pool).await;
            assert!(matches!(result2, Err(crate::DatabaseError::NotFound(_))));
        }

        #[sqlx::test]
        async fn test_delete_by_code(pool: SqlitePool) {
            let category = crate::Categories::mock();
            insert_test_category(&pool, &category).await;

            let result = crate::Categories::delete_by_code(&category.code, &pool).await;
            assert!(result.is_ok());

            // Verify deletion by trying to delete again
            let result2 = crate::Categories::delete_by_code(&category.code, &pool).await;
            assert!(matches!(result2, Err(crate::DatabaseError::NotFound(_))));
        }

        #[sqlx::test]
        async fn test_delete_by_url_slug(pool: SqlitePool) {
            let mut category = crate::Categories::mock();
            category.url_slug = Some(domain::UrlSlug::from("test-slug"));
            insert_test_category(&pool, &category).await;

            let result = crate::Categories::delete_by_url_slug(category.url_slug.as_ref().unwrap(), &pool).await;
            assert!(result.is_ok());

            // Verify deletion by trying to delete again
            let result2 = crate::Categories::delete_by_url_slug(category.url_slug.as_ref().unwrap(), &pool).await;
            assert!(matches!(result2, Err(crate::DatabaseError::NotFound(_))));
        }

        /// Property-based test: Delete operations handle varied mock data
        #[sqlx::test]
        async fn test_delete_handles_various_category_configurations(pool: SqlitePool) {
            for i in 0..20 {
                let mut category = crate::Categories::mock();
                // Ensure unique identifiers for this test
                category.code = format!("DEL{:03}.TST.DEL", i);
                category.name = format!("Test Category {}", i);
                category.url_slug = Some(lib_domain::UrlSlug::from(format!("test-category-{}", i)));
                insert_test_category(&pool, &category).await;

                // Test delete by ID
                let result = crate::Categories::delete_by_id(category.id, &pool).await;
                assert!(result.is_ok(), "Delete by ID should succeed for mock data");

                // Verify deletion
                let result2 = crate::Categories::delete_by_id(category.id, &pool).await;
                assert!(matches!(result2, Err(crate::DatabaseError::NotFound(_))));
            }
        }

        /// Edge case: Delete with empty or invalid codes/slugs
        #[sqlx::test]
        async fn test_delete_by_code_edge_cases(pool: SqlitePool) {
            // Non-existent code
            let result = crate::Categories::delete_by_code("NON.EXISTENT.CODE", &pool).await;
            assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));

            // Empty code (if allowed by validation, but should fail)
            let result = crate::Categories::delete_by_code("", &pool).await;
            assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
        }

        #[sqlx::test]
        async fn test_delete_by_url_slug_edge_cases(pool: SqlitePool) {
            // Non-existent slug
            let slug = lib_domain::UrlSlug::from("non-existent-slug");
            let result = crate::Categories::delete_by_url_slug(&slug, &pool).await;
            assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
        }
    }

    mod bulk_operations {
        use super::*;

        #[sqlx::test]
        async fn test_delete_many_by_id_success(pool: SqlitePool) {
            let mut categories = Vec::new();
            let mut ids = Vec::new();

            for _ in 0..3 {
                let category = crate::Categories::mock();
                let id = insert_test_category(&pool, &category).await;
                categories.push(category);
                ids.push(id);
            }

            let result = crate::Categories::delete_many_by_id(&ids, &pool).await;
            assert!(result.is_ok());

            // Verify all deleted by trying to delete again
            for &id in &ids {
                let result2 = crate::Categories::delete_by_id(id, &pool).await;
                assert!(matches!(result2, Err(crate::DatabaseError::NotFound(_))));
            }
        }

        #[sqlx::test]
        async fn test_delete_many_by_id_fails_on_nonexistent(pool: SqlitePool) {
            let category = crate::Categories::mock();
            let valid_id = insert_test_category(&pool, &category).await;
            let invalid_id = domain::RowID::mock();

            let ids = vec![valid_id, invalid_id];
            let result = crate::Categories::delete_many_by_id(&ids, &pool).await;

            assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
        }

        /// Edge case: Empty ID list
        #[sqlx::test]
        async fn test_delete_many_by_id_empty_list(pool: SqlitePool) {
            let ids: Vec<domain::RowID> = vec![];
            let result = crate::Categories::delete_many_by_id(&ids, &pool).await;
            assert!(result.is_ok(), "Empty list should succeed without error");
        }

        /// Property-based test: Bulk delete handles varied mock data atomically
        #[sqlx::test]
        async fn test_delete_many_by_id_randomised(pool: SqlitePool) {
            for i in 0..10 {
                let count = (1..5).fake::<usize>(); // Random count 1-4
                let mut ids = Vec::new();

                for j in 0..count {
                    let mut category = crate::Categories::mock();
                    // Ensure unique identifiers for this test
                    category.code = format!("DEL{:03}.RND{:03}.TST", i, j);
                    category.name = format!("Test Category {} {}", i, j);
                    category.url_slug = Some(lib_domain::UrlSlug::from(format!("test-category-{}-{}", i, j)));
                    let id = insert_test_category(&pool, &category).await;
                    ids.push(id);
                }

                let result = crate::Categories::delete_many_by_id(&ids, &pool).await;
                assert!(result.is_ok(), "Bulk delete should succeed for random mock data");

                // Verify all deleted
                for &id in &ids {
                    let result2 = crate::Categories::delete_by_id(id, &pool).await;
                    assert!(matches!(result2, Err(crate::DatabaseError::NotFound(_))));
                }
            }
        }
    }

    mod bulk_deletions {
        use super::*;

        #[sqlx::test]
        async fn test_delete_inactive(pool: SqlitePool) {
            // Insert active and inactive categories
            let mut active_category = crate::Categories::mock();
            active_category.is_active = true;
            insert_test_category(&pool, &active_category).await;

            let mut inactive_category = crate::Categories::mock();
            inactive_category.is_active = false;
            insert_test_category(&pool, &inactive_category).await;

            let deleted_count = crate::Categories::delete_inactive(&pool).await.unwrap();
            assert_eq!(deleted_count, 1);

            // Verify inactive deleted by trying to delete inactive again (should be 0)
            let deleted_count2 = crate::Categories::delete_inactive(&pool).await.unwrap();
            assert_eq!(deleted_count2, 0);
        }

        #[sqlx::test]
        async fn test_delete_all(pool: SqlitePool) {
            // Insert multiple categories
            for _ in 0..3 {
                let category = crate::Categories::mock();
                insert_test_category(&pool, &category).await;
            }

            let deleted_count = crate::Categories::delete_all(&pool).await.unwrap();
            assert_eq!(deleted_count, 3);

            // Verify all deleted by trying to delete all again
            let deleted_count2 = crate::Categories::delete_all(&pool).await.unwrap();
            assert_eq!(deleted_count2, 0);
        }

        /// Property-based test: Delete inactive/all handle varied data
        #[sqlx::test]
        async fn test_delete_inactive_randomised(pool: SqlitePool) {
            for i in 0..10 {
                // Insert mix of active/inactive
                let mut active_count = 0;
                let mut inactive_count = 0;

                for j in 0..5 {
                    let mut category = crate::Categories::mock();
                    category.is_active = (0..2).fake::<u8>() == 1; // Random true/false
                    // Ensure unique identifiers
                    category.code = format!("INA{:03}.RND{:03}.TST", i, j);
                    category.name = format!("Inactive Test {} {}", i, j);
                    category.url_slug = Some(lib_domain::UrlSlug::from(format!("inactive-test-{}-{}", i, j)));
                    if category.is_active {
                        active_count += 1;
                    } else {
                        inactive_count += 1;
                    }
                    insert_test_category(&pool, &category).await;
                }

                let deleted = crate::Categories::delete_inactive(&pool).await.unwrap();
                assert_eq!(deleted, inactive_count as u64);

                // Verify active remain
                let remaining = crate::Categories::delete_all(&pool).await.unwrap();
                assert_eq!(remaining, active_count as u64);
            }
        }
    }

    mod validation_and_edge_cases {
        use super::*;

        /// Test that mock data passes basic validation rules
        #[sqlx::test]
        async fn test_mock_data_validation(pool: SqlitePool) {
            for _ in 0..20 {
                let category = crate::Categories::mock();

                // Basic validation: code format, non-empty name, etc.
                assert!(category.code.contains('.'));
                let parts: Vec<&str> = category.code.split('.').collect();
                assert_eq!(parts.len(), 3);
                assert!(!category.name.is_empty());

                // Test deletion works with valid mock data
                insert_test_category(&pool, &category).await;
                let result = category.delete(&pool).await;
                assert!(result.is_ok());
            }
        }

        /// Test optional fields randomisation
        #[test]
        fn test_mock_randomises_optional_fields() {
            let mut has_description = false;
            let mut has_none_description = false;
            let mut has_icon = false;
            let mut has_none_icon = false;

            for _ in 0..50 {
                let category = crate::Categories::mock();
                if category.description.is_some() {
                    has_description = true;
                } else {
                    has_none_description = true;
                }
                if category.icon.is_some() {
                    has_icon = true;
                } else {
                    has_none_icon = true;
                }
            }

            assert!(has_description && has_none_description, "Description should be randomised");
            assert!(has_icon && has_none_icon, "Icon should be randomised");
        }
    }
}
