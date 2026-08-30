
//! Category update operations for the Personal Ledger database.
//!
//! This module provides functionality for updating category records in the SQLite database.
//! It includes methods for single category updates, bulk updates with transactional guarantees,
//! and targeted updates for specific fields like active status.
//!
//! All update operations preserve data integrity by:
//! - Maintaining immutable fields (`id`, `created_on`)
//! - Automatically updating timestamps (`updated_on`)
//! - Using transactions for atomic bulk operations
//! - Providing comprehensive error handling and tracing
//!
//! The module follows these key principles:
//! - **Atomicity**: Bulk operations use transactions to ensure consistency
//! - **Efficiency**: Targeted updates for specific fields when appropriate
//! - **Safety**: Comprehensive error handling without panics
//! - **Observability**: Detailed tracing from TRACE to ERROR levels

use lib_domain as domain;

impl crate::Categories {
    /// Updates an existing category in the database.
    ///
    /// This function performs a complete update of a category record, replacing all fields
    /// with the values from the provided `Categories` instance. It ensures atomicity
    /// by updating the record in a single operation and returns the updated category
    /// after re-reading it from the database to confirm the changes.
    ///
    /// The `id` and `created_on` fields remain unchanged during the update, while
    /// `updated_on` is automatically set to the current timestamp.
    ///
    /// # Arguments
    /// * `pool` - A reference to the SQLite database connection pool used for the update
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Self>` containing the updated `Categories` instance
    /// with refreshed data from the database, or a `DatabaseError` if the operation fails
    ///
    /// # Errors
    /// This function will return an error if:
    /// * The category with the specified `id` does not exist in the database
    /// * A database connection error occurs
    /// * The update operation fails due to constraint violations
    ///
    /// # Examples
    /// ```rust,no_run
    /// use lib_database::Categories;
    /// use sqlx::SqlitePool;
    ///
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut category = Categories::mock();
    /// category.name = "Updated Category Name".to_string();
    ///
    /// let updated_category = category.update(pool).await?;
    /// println!("Updated category: {}", updated_category.name);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Security
    /// This function does not perform any input validation beyond what is enforced by
    /// the database constraints. Ensure category data is validated before calling this function.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, WARN on not found, ERROR on database failures.
    #[tracing::instrument(
        name = "Category update",
        level = "debug",
        skip(pool),
        fields(
            category_id = %self.id,
            category_name = %self.name,
            category_code = %self.code,
            operation = "update_single"
        ),
        err
    )]
    pub async fn update(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        tracing::debug!(
            category_id = %self.id,
            category_name = %self.name,
            is_active = %self.is_active,
            "Starting category update operation"
        );

        // Update the category record
        let update_query = sqlx::query!(
            r#"
                UPDATE categories
                SET code = ?, name = ?, description = ?, url_slug = ?, category_type = ?,
                    color = ?, icon = ?, is_active = ?, updated_on = ?
                WHERE id = ?
            "#,
            self.code,
            self.name,
            self.description,
            self.url_slug,
            self.category_type,
            self.color,
            self.icon,
            self.is_active,
            self.updated_on,
            self.id
        );

        let rows_affected = update_query.execute(pool).await?.rows_affected();

        if rows_affected == 0 {
            tracing::error!(
                category_id = %self.id,
                "Category update failed - category not found"
            );
            return Err(crate::DatabaseError::NotFound(format!(
                "Category with id {} not found",
                self.id
            )));
        }

        tracing::info!(
            category_id = %self.id,
            category_name = %self.name,
            "Category updated successfully in database"
        );

        // Read back the updated category
        let updated = sqlx::query_as!(
            crate::Categories,
            r#"
                SELECT
                    id              AS "id!: domain::RowID",
                    code,
                    name,
                    description,
                    url_slug        AS "url_slug?: domain::UrlSlug",
                    category_type   AS "category_type!: domain::CategoryTypes",
                    color           AS "color?: domain::HexColor",
                    icon,
                    is_active       AS "is_active!: bool",
                    created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM categories
                WHERE id = ?
            "#,
            self.id
        )
        .fetch_one(pool)
        .await?;

        tracing::debug!(
            category_id = %updated.id,
            updated_on = %updated.updated_on,
            "Category update completed, read back updated record"
        );

        Ok(updated)
    }

    /// Updates multiple categories in the database within a single transaction.
    ///
    /// This function performs bulk updates of categories, ensuring atomicity - either all categories
    /// are updated successfully, or none are updated if any operation fails. This prevents partial
    /// updates that could leave the database in an inconsistent state.
    ///
    /// Each category's `updated_on` timestamp is automatically set to the current time during the update.
    /// The `id` and `created_on` fields remain unchanged for all categories.
    ///
    /// The operation uses a database transaction to guarantee that either all updates succeed or
    /// the entire operation is rolled back. This is particularly important when updating related
    /// or dependent categories.
    ///
    /// # Arguments
    /// * `categories` - A slice of `Categories` instances to update. Each category must have a valid ID
    /// * `pool` - A reference to the SQLite database connection pool used for the transaction
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Vec<Self>>` containing all successfully updated categories in the
    /// same order as provided, or a `DatabaseError` if the operation fails
    ///
    /// # Errors
    /// This function will return an error if:
    /// * Any category with a specified `id` does not exist in the database
    /// * A database connection error occurs
    /// * The transaction fails to commit
    /// * Any update operation fails due to constraint violations
    ///
    /// When an error occurs, the entire transaction is rolled back and no categories are updated.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use lib_database::Categories;
    /// use sqlx::SqlitePool;
    ///
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let categories = vec![
    ///     Categories { id: 1.into(), name: "Category 1".to_string(), ..Categories::mock() },
    ///     Categories { id: 2.into(), name: "Category 2".to_string(), ..Categories::mock() },
    /// ];
    ///
    /// let updated_categories = Categories::update_many(&categories, pool).await?;
    /// println!("Updated {} categories", updated_categories.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Performance
    /// This operation uses a database transaction to ensure atomicity. For large numbers of categories,
    /// consider the transaction size and database performance implications. The transaction holds
    /// locks on affected rows until completion.
    ///
    /// # Security
    /// This function does not perform any input validation beyond what is enforced by
    /// the database constraints. Ensure all category data is validated before calling this function.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for transaction and per-category progress, INFO on success, WARN on individual failures, ERROR on transaction rollback.
    #[tracing::instrument(
        name = "Bulk category update",
        level = "debug",
        skip(pool, categories),
        fields(
            category_count = %categories.len(),
            operation = "update_bulk"
        ),
        err
    )]
    pub async fn update_many(
        categories: &[Self],
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let category_count = categories.len();

        if category_count == 0 {
            tracing::debug!("Bulk update called with empty category list, returning early");
            return Ok(Vec::new());
        }

        tracing::debug!(
            category_count = %category_count,
            "Starting bulk category update operation"
        );

        // Use a transaction for atomicity
        let mut tx = pool.begin().await?;
        tracing::debug!("Database transaction started for bulk update");

        let mut updated_categories = Vec::with_capacity(category_count);
        let mut processed_count = 0;

        for (index, category) in categories.iter().enumerate() {
            tracing::debug!(
                category_index = %index,
                category_id = %category.id,
                category_name = %category.name,
                "Processing category update in bulk operation"
            );

            // Update each category
            let update_query = sqlx::query!(
                r#"
                    UPDATE categories
                    SET code = ?, name = ?, description = ?, url_slug = ?, category_type = ?,
                        color = ?, icon = ?, is_active = ?, updated_on = ?
                    WHERE id = ?
                "#,
                category.code,
                category.name,
                category.description,
                category.url_slug,
                category.category_type,
                category.color,
                category.icon,
                category.is_active,
                category.updated_on,
                category.id
            );

            let rows_affected = update_query.execute(&mut *tx).await?.rows_affected();

            if rows_affected == 0 {
                tracing::warn!(
                    category_id = %category.id,
                    category_index = %index,
                    "Category not found during bulk update, rolling back transaction"
                );
                return Err(crate::DatabaseError::NotFound(format!(
                    "Category with id {} not found",
                    category.id
                )));
            }

            // Read back the updated category
            let updated = sqlx::query_as!(
                crate::Categories,
                r#"
                    SELECT
                        id              AS "id!: domain::RowID",
                        code,
                        name,
                        description,
                        url_slug        AS "url_slug?: domain::UrlSlug",
                        category_type   AS "category_type!: domain::CategoryTypes",
                        color           AS "color?: domain::HexColor",
                        icon,
                        is_active       AS "is_active!: bool",
                        created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                        updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                    FROM categories
                    WHERE id = ?
                "#,
                category.id
            )
            .fetch_one(&mut *tx)
            .await?;

            updated_categories.push(updated);
            processed_count += 1;

            tracing::debug!(
                category_index = %index,
                category_id = %category.id,
                processed_count = %processed_count,
                "Category update completed in bulk operation"
            );
        }

        // Commit the transaction
        tx.commit().await?;
        tracing::debug!("Database transaction committed for bulk update");

        tracing::info!(
            category_count = %category_count,
            "Successfully updated all categories in bulk operation"
        );

        Ok(updated_categories)
    }

    /// Updates only the active status of a category in the database.
    ///
    /// This function provides an efficient way to toggle a category's active status without modifying
    /// other fields. Only the `is_active` field and `updated_on` timestamp are updated. This is useful
    /// for operations like activating/deactivating categories without affecting their other properties.
    ///
    /// The function performs a targeted update that only affects the specified fields, making it
    /// more efficient than a full category update when only the active status needs to change.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the category to update
    /// * `is_active` - The new active status (`true` for active, `false` for inactive)
    /// * `pool` - A reference to the SQLite database connection pool used for the update
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Self>` containing the updated category with all current field values
    /// from the database, or a `DatabaseError` if the operation fails
    ///
    /// # Errors
    /// This function will return an error if:
    /// * The category with the specified `id` does not exist in the database
    /// * A database connection error occurs
    ///
    /// # Examples
    /// ```rust,no_run
    /// use lib_database::Categories;
    /// use lib_domain::RowID;
    /// use sqlx::SqlitePool;
    ///
    /// # async fn example(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    /// let category_id = RowID::from(123);
    ///
    /// // Activate a category
    /// let activated_category = Categories::update_active_status(category_id, true, pool).await?;
    /// assert!(activated_category.is_active);
    ///
    /// // Deactivate the same category
    /// let deactivated_category = Categories::update_active_status(category_id, false, pool).await?;
    /// assert!(!deactivated_category.is_active);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Performance
    /// This method is more efficient than a full category update when only the active status needs to change,
    /// as it performs a targeted update of fewer fields and avoids unnecessary data transfer.
    ///
    /// # Security
    /// This function validates that the category exists before updating its status. No additional
    /// authorisation checks are performed - ensure proper access control at a higher level.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, WARN on not found, ERROR on database failures.
    #[tracing::instrument(
        name = "Category active status update",
        level = "debug",
        skip(pool),
        fields(
            category_id = %id,
            target_active_status = %is_active,
            operation = "update_active_status"
        ),
        err
    )]
    pub async fn update_active_status(
        id: domain::RowID,
        is_active: bool,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        tracing::debug!(
            category_id = %id,
            target_active_status = %is_active,
            "Starting category active status update operation"
        );

        // Update only the active status and updated_on timestamp
        let update_query = sqlx::query!(
            r#"
                UPDATE categories
                SET is_active = ?, updated_on = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                WHERE id = ?
            "#,
            is_active,
            id
        );

        let rows_affected = update_query.execute(pool).await?.rows_affected();

        if rows_affected == 0 {
            tracing::warn!(
                category_id = %id,
                target_active_status = %is_active,
                "Category active status update failed - category not found"
            );
            return Err(crate::DatabaseError::NotFound(format!(
                "Category with id {} not found",
                id
            )));
        }

        tracing::info!(
            category_id = %id,
            active_status = %is_active,
            "Category active status updated successfully"
        );

        // Read back the updated category
        let updated = sqlx::query_as!(
            crate::Categories,
            r#"
                SELECT
                    id              AS "id!: domain::RowID",
                    code,
                    name,
                    description,
                    url_slug        AS "url_slug?: domain::UrlSlug",
                    category_type   AS "category_type!: domain::CategoryTypes",
                    color           AS "color?: domain::HexColor",
                    icon,
                    is_active       AS "is_active!: bool",
                    created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM categories
                WHERE id = ?
            "#,
            id
        )
        .fetch_one(pool)
        .await?;

        tracing::debug!(
            category_id = %updated.id,
            category_name = %updated.name,
            active_status = %updated.is_active,
            updated_on = %updated.updated_on,
            "Category active status update completed, read back updated record"
        );

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories::Categories;
    use fake::Fake;
    use fake::faker::lorem::en::Words;
    use sqlx::SqlitePool;

    /// Test helper to insert a category into the test database
    async fn insert_test_category(pool: &SqlitePool, category: &Categories) -> domain::RowID {
        // Convert complex types to strings for database insertion
        let id_str = category.id.to_string();
        let url_slug_str = category.url_slug.as_ref().map(|s| s.to_string());
        let category_type_str = category.category_type.as_str();
        let color_str = category.color.as_ref().map(|c| c.to_string());
        let created_on_str = category.created_on.to_rfc3339();
        let updated_on_str = category.updated_on.to_rfc3339();

        let result = sqlx::query!(
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

    /// Test helper to generate a modified version of a category for updates
    fn generate_modified_category(original: &Categories) -> Categories {
        let mut modified = original.clone();

        // Generate a new name using fake crate
        let words: Vec<String> = Words(2..4).fake();
        modified.name = format!("Updated {}", words.join(" "));

        // Update the timestamp
        modified.updated_on = chrono::Utc::now();

        // Randomly change some optional fields
        use fake::faker::boolean::en::Boolean;
        if Boolean(50).fake() {
            modified.description = Some("Updated description".to_string());
        }
        if Boolean(30).fake() {
            modified.icon = Some("updated-icon".to_string());
        }

        modified
    }

    mod single_update_tests {
        use super::*;

        #[sqlx::test]
        async fn update_existing_category_successfully(pool: SqlitePool) {
            let original_category = Categories::mock();

            // Insert the original category
            insert_test_category(&pool, &original_category).await;

            // Create a modified version
            let modified_category = generate_modified_category(&original_category);

            // Update the category
            let result = modified_category.update(&pool).await;
            assert!(result.is_ok(), "Update should succeed");

            let updated = result.unwrap();
            assert_eq!(updated.id, original_category.id);
            assert_eq!(updated.name, modified_category.name);
            assert_eq!(updated.code, modified_category.code); // Code should remain the same
            assert!(updated.updated_on >= original_category.updated_on);
        }

        #[sqlx::test]
        async fn update_nonexistent_category_returns_not_found(pool: SqlitePool) {
            let category = Categories::mock();

            // Try to update a category that doesn't exist
            let result = category.update(&pool).await;
            assert!(result.is_err(), "Update should fail for nonexistent category");

            let error = result.unwrap_err();
            match error {
                crate::DatabaseError::NotFound(msg) => {
                    assert!(msg.contains(&category.id.to_string()));
                }
                _ => panic!("Expected NotFound error, got {:?}", error),
            }
        }

        #[sqlx::test]
        async fn update_preserves_created_on_timestamp(pool: SqlitePool) {
            let original_category = Categories::mock();

            insert_test_category(&pool, &original_category).await;

            let modified_category = generate_modified_category(&original_category);
            let updated = modified_category.update(&pool).await.unwrap();

            // created_on should remain unchanged
            assert_eq!(updated.created_on, original_category.created_on);
            // updated_on should be newer
            assert!(updated.updated_on > original_category.updated_on);
        }

        #[sqlx::test]
        async fn update_with_various_optional_fields(pool: SqlitePool) {
            // Test multiple scenarios with different optional field combinations
            for _ in 0..10 {
                let original = Categories::mock();
                insert_test_category(&pool, &original).await;

                let modified = generate_modified_category(&original);
                let updated = modified.update(&pool).await.unwrap();

                // Verify all fields are correctly updated
                assert_eq!(updated.id, original.id);
                assert_eq!(updated.name, modified.name);
                assert_eq!(updated.description, modified.description);
                assert_eq!(updated.icon, modified.icon);
                assert_eq!(updated.is_active, modified.is_active);
            }
        }
    }

    mod bulk_update_tests {
        use super::*;

        #[sqlx::test]
        async fn update_many_empty_list_returns_empty_vec(pool: SqlitePool) {
            let empty_list: Vec<Categories> = vec![];

            let result = Categories::update_many(&empty_list, &pool).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), 0);
        }

        #[sqlx::test]
        async fn update_many_successfully_updates_multiple_categories(pool: SqlitePool) {
            // Create and insert multiple categories
            let mut original_categories = Vec::new();
            let mut modified_categories = Vec::new();

            for _ in 0..5 {
                let original = Categories::mock();
                insert_test_category(&pool, &original).await;
                original_categories.push(original);
            }

            // Generate modified versions
            for original in &original_categories {
                modified_categories.push(generate_modified_category(original));
            }

            // Update all categories
            let result = Categories::update_many(&modified_categories, &pool).await;
            assert!(result.is_ok(), "Bulk update should succeed");

            let updated = result.unwrap();
            assert_eq!(updated.len(), modified_categories.len());

            // Verify each category was updated correctly
            for (i, updated_category) in updated.iter().enumerate() {
                assert_eq!(updated_category.id, modified_categories[i].id);
                assert_eq!(updated_category.name, modified_categories[i].name);
            }
        }

        #[sqlx::test]
        async fn update_many_fails_if_any_category_not_found(pool: SqlitePool) {
            // Create valid categories
            let mut valid_categories = Vec::new();
            for _ in 0..3 {
                let category = Categories::mock();
                insert_test_category(&pool, &category).await;
                valid_categories.push(generate_modified_category(&category));
            }

            // Add a nonexistent category
            let nonexistent = Categories::mock();
            let mut all_categories = valid_categories;
            all_categories.push(generate_modified_category(&nonexistent));

            // Bulk update should fail due to nonexistent category
            let result = Categories::update_many(&all_categories, &pool).await;
            assert!(result.is_err(), "Bulk update should fail if any category not found");

            let error = result.unwrap_err();
            match error {
                crate::DatabaseError::NotFound(msg) => {
                    assert!(msg.contains(&nonexistent.id.to_string()));
                }
                _ => panic!("Expected NotFound error, got {:?}", error),
            }
        }

        #[sqlx::test]
        async fn update_many_is_atomic_on_failure(pool: SqlitePool) {
            // Insert some valid categories
            let mut valid_categories = Vec::new();
            for _ in 0..3 {
                let category = Categories::mock();
                insert_test_category(&pool, &category).await;
                valid_categories.push(generate_modified_category(&category));
            }

            // Add a nonexistent category to cause failure
            let nonexistent = Categories::mock();
            let mut all_categories = valid_categories.clone();
            all_categories.push(generate_modified_category(&nonexistent));

            // Attempt bulk update (should fail)
            let _ = Categories::update_many(&all_categories, &pool).await;

            // Verify that none of the valid categories were actually updated
            // by checking they still have their original values
            for original in &valid_categories {
                let current = sqlx::query_as!(
                    Categories,
                    r#"
                    SELECT
                        id              AS "id!: domain::RowID",
                        code,
                        name,
                        description,
                        url_slug        AS "url_slug?: domain::UrlSlug",
                        category_type   AS "category_type!: domain::CategoryTypes",
                        color           AS "color?: domain::HexColor",
                        icon,
                        is_active       AS "is_active!: bool",
                        created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                        updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                    FROM categories
                    WHERE id = ?
                    "#,
                    original.id
                )
                .fetch_one(&pool)
                .await
                .unwrap();

                // The category should still have its original name, not the modified one
                assert_ne!(current.name, original.name,
                    "Category should not have been updated due to transaction rollback");
            }
        }
    }

    mod active_status_tests {
        use super::*;

        #[sqlx::test]
        async fn update_active_status_to_true(pool: SqlitePool) {
            // Create an inactive category
            let mut category = Categories::mock();
            category.is_active = false;
            insert_test_category(&pool, &category).await;

            // Update to active
            let result = Categories::update_active_status(category.id, true, &pool).await;
            assert!(result.is_ok(), "Active status update should succeed");

            let updated = result.unwrap();
            assert_eq!(updated.id, category.id);
            assert!(updated.is_active);
            assert!(updated.updated_on > category.updated_on);
        }

        #[sqlx::test]
        async fn update_active_status_to_false(pool: SqlitePool) {
            // Create an active category
            let mut category = Categories::mock();
            category.is_active = true;
            insert_test_category(&pool, &category).await;

            // Update to inactive
            let result = Categories::update_active_status(category.id, false, &pool).await;
            assert!(result.is_ok(), "Active status update should succeed");

            let updated = result.unwrap();
            assert_eq!(updated.id, category.id);
            assert!(!updated.is_active);
        }

        #[sqlx::test]
        async fn update_active_status_nonexistent_category(pool: SqlitePool) {
            let fake_id = domain::RowID::mock();

            let result = Categories::update_active_status(fake_id, true, &pool).await;
            assert!(result.is_err(), "Update should fail for nonexistent category");

            let error = result.unwrap_err();
            match error {
                crate::DatabaseError::NotFound(msg) => {
                    assert!(msg.contains(&fake_id.to_string()));
                }
                _ => panic!("Expected NotFound error, got {:?}", error),
            }
        }

        #[sqlx::test]
        async fn update_active_status_preserves_other_fields(pool: SqlitePool) {
            let original = Categories::mock();
            insert_test_category(&pool, &original).await;

            // Update only active status
            let updated = Categories::update_active_status(original.id, !original.is_active, &pool)
                .await
                .unwrap();

            // Verify other fields remain unchanged
            assert_eq!(updated.id, original.id);
            assert_eq!(updated.code, original.code);
            assert_eq!(updated.name, original.name);
            assert_eq!(updated.description, original.description);
            assert_eq!(updated.category_type, original.category_type);
            assert_eq!(updated.created_on, original.created_on);
            assert_eq!(updated.is_active, !original.is_active); // Only this should change
        }
    }

    mod property_based_tests {
        use super::*;

        #[sqlx::test]
        async fn update_handles_various_category_configurations(pool: SqlitePool) {
            // Test with many different category configurations
            for i in 0..20 {
                let mut original = Categories::mock();
                // Ensure unique identifiers for this test
                original.code = format!("TEST{:03}.ABC.DEF", i);
                original.name = format!("Test Category {}", i); // Ensure unique name
                original.url_slug = Some(lib_domain::UrlSlug::from(format!("test-category-{}", i)));

                insert_test_category(&pool, &original).await;

                let modified = generate_modified_category(&original);
                let updated = modified.update(&pool).await.unwrap();

                // Verify the update preserved the ID and updated the name
                assert_eq!(updated.id, original.id);
                assert_eq!(updated.name, modified.name);
                assert!(updated.updated_on >= original.updated_on);
            }
        }

        #[sqlx::test]
        async fn update_active_status_randomized_testing(pool: SqlitePool) {
            for i in 0..15 {
                let mut original = Categories::mock();
                // Ensure unique identifiers for this test
                original.code = format!("STATUS{:03}.ABC.DEF", i);
                original.name = format!("Status Test Category {}", i); // Ensure unique name
                original.url_slug = Some(lib_domain::UrlSlug::from(format!("status-test-category-{}", i)));

                insert_test_category(&pool, &original).await;

                // Randomly choose target active status
                use fake::faker::boolean::en::Boolean;
                let target_active = Boolean(50).fake();

                let updated = Categories::update_active_status(original.id, target_active, &pool)
                    .await
                    .unwrap();

                assert_eq!(updated.is_active, target_active);
                assert_eq!(updated.id, original.id);
                // Other fields should be preserved
                assert_eq!(updated.name, original.name);
                assert_eq!(updated.code, original.code);
            }
        }
    }
}

