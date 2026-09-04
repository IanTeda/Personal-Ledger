//! # Category Activation Module
//!
//! This module provides the gRPC handler for activating categories in the Personal Ledger system.
//!
//! ## Overview
//!
//! The activation process involves validating the incoming request, parsing the category ID,
//! updating the database to set `is_active = true`, and returning the updated category.
//!
//! ## Behaviour
//!
//! - Validates the category ID format.
//! - Updates the category's active status via the database layer.
//! - Returns structured errors for invalid IDs, not-found categories, or database failures.
//!
//! ## Tracing
//!
//! Uses structured tracing for observability, logging key events like ID parsing and errors.
//!
//! ## Examples
//!
//! The handler is called by the generated gRPC server glue code and is not typically invoked directly.
//!
//! ## Notes
//!
//! - Uses Australian English in comments and documentation.
//! - Relies on `lib_database` for persistence and `lib_core` for types.

//-- Workspace library crates
use lib_database as database;
use lib_core as domain;

//-- RPC Library modules
use crate::categories::proto;
use crate::error::{RpcError, RpcResult};

/// Activate a category by ID.
///
/// This function serves as the gRPC handler for category activation. It extracts the request,
/// validates and parses the category ID, updates the database, and returns the activated category.
///
/// # Arguments
/// * `service` - Reference to the categories service containing the database connection.
/// * `request` - The incoming gRPC request containing the [`proto::CategoryActivateRequest`](crates/libs/lib-rpc/src/categories/proto.rs).
///
/// # Returns
/// Returns a `tonic::Response` containing the [`proto::CategoryActivateResponse`](crates/libs/lib-rpc/src/categories/proto.rs) with the updated category on success.
///
/// # Errors
/// * Returns `tonic::Status::invalid_argument` if the category ID cannot be parsed as a [`domain::RowID`](crates/libs/lib-core/src/lib.rs).
/// * Returns `tonic::Status::not_found` if no category exists with the provided ID.
/// * Returns `tonic::Status::internal` for unexpected database errors.
///
/// # Examples
/// ```no_run
/// // Called by generated gRPC server code.
/// let response = activate_category(&service, request).await?;
/// ```
///
/// # Panics
/// This function does not panic under normal operation; errors are handled via `Result`.
///
/// # Tracing
/// Logs debug information for the category ID and errors for failures.
#[tracing::instrument(
    name = "activate_category",
    level = "debug",
    skip(service)
)]
pub async fn activate_category(
    service: &super::CategoriesService,
    request: tonic::Request<proto::CategoryActivateRequest>,
) -> RpcResult<tonic::Response<proto::CategoryActivateResponse>> {
    // Extract the inner request
    let activate_request = request.into_inner();

    // Parse the ID from string to RowID
    let category_id = match activate_request.id.parse::<domain::RowID>() {
        Ok(id) => id,
        Err(_) => {
            return Err(RpcError::InvalidArgument(format!("Could not parse the Category ID '{}' into a UUIDv7", activate_request.id)));
        }
    };
    
    tracing::debug!(category_id = %category_id, "Parsed category id");

    // Update the category's active status to true
    let updated_category = database::Categories::update_active_status(category_id, true, service.database_ref()).await?;

    // Convert to RPC category and return response
    let rpc_category: proto::Category = updated_category.into();
    let response = proto::CategoryActivateResponse {
        category: Some(rpc_category),
    };

    Ok(tonic::Response::new(response))
}

#[cfg(test)]
/// Test module for category activation functionality.
///
/// This module contains unit tests for the activation logic, including edge cases and error handling.
/// Tests use the `fake` crate for randomised data and `sqlx::test` for database-backed scenarios.
///
/// ## Test Coverage
///
/// - Successful activation of inactive categories.
/// - Handling of already active categories.
/// - Error cases for nonexistent categories.
/// - Randomised field testing for robustness.
/// - Timestamp updates on activation.
///
/// ## Notes
///
/// - Uses deterministic seeds where possible for reproducible tests.
/// - Creates and tears down test data in-memory via SQLite.
mod tests {
    use super::*;
    use fake::Fake;
    use fake::faker::boolean::en::Boolean;
    use lib_core::{RowID, UrlSlug, HexColor, CategoryTypes};
    use lib_database::Categories;
    use sqlx::SqlitePool;
    use chrono::Utc;
    use crate::CategoriesService;

    /// Helper function to create a mock category for testing.
    ///
    /// Generates realistic test data using the fake crate, ensuring
    /// optional fields are randomised probabilistically. This helps
    /// test the robustness of activation logic across different data shapes.
    ///
    /// # Returns
    /// A [`Categories`](crates/libs/lib-database/src/categories/model.rs) instance with randomised fields.
    ///
    /// # Examples
    /// ```
    /// let category = create_mock_category();
    /// assert!(!category.is_active);
    /// ```
    ///
    /// # Notes
    /// - Uses probabilistic randomisation for optional fields (e.g., 60% chance for description).
    /// - Always sets `is_active` to `false` for activation test scenarios.
    fn create_mock_category() -> Categories {
        let name: String = fake::faker::lorem::en::Words(1..3).fake::<Vec<String>>().join(" ");
        let description = if Boolean(60).fake() {
            Some(fake::faker::lorem::en::Sentences(3..8).fake::<Vec<String>>().join(" "))
        } else {
            None
        };
        let url_slug = Some(UrlSlug::parse(name.replace(" ", "-")).unwrap());
        let color = if Boolean(70).fake() {
            Some(HexColor::parse("#FF5733").unwrap()) // Example fixed color for simplicity
        } else {
            None
        };
        let icon = if Boolean(50).fake() {
            Some(fake::faker::lorem::en::Word().fake::<String>())
        } else {
            None
        };

        Categories {
            id: RowID::new(),
            code: format!("{:03}.{:03}.{:03}",
                rand::random::<u8>() % 100,
                rand::random::<u8>() % 100,
                rand::random::<u8>() % 100,
            ),
            name,
            description,
            url_slug,
            category_type: CategoryTypes::Expense, // Default for simplicity
            color,
            icon,
            is_active: false, // Start inactive for activation tests
            created_on: Utc::now(),
            updated_on: Utc::now(),
        }
    }

    /// Helper function to insert a test category into the database.
    ///
    /// Converts complex types to strings for insertion, matching
    /// patterns from other test modules. Ensures the categories table exists.
    ///
    /// # Arguments
    /// * `pool` - Reference to the test database pool.
    /// * `category` - The category to insert.
    ///
    /// # Returns
    /// The [`RowID`](crates/libs/lib-core/src/lib.rs) of the inserted category.
    ///
    /// # Panics
    /// Panics if database insertion fails.
    ///
    /// # Examples
    /// ```no_run
    /// let category = create_mock_category();
    /// let id = insert_test_category(&pool, &category).await;
    /// ```
    ///
    /// # Notes
    /// - Creates the table schema if it doesn't exist to support isolated tests.
    /// - Handles type conversions for domain types like `UrlSlug` and `HexColor`.
    async fn insert_test_category(pool: &SqlitePool, category: &Categories) -> RowID {
        // Create the categories table if it doesn't exist
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS categories (
                id TEXT PRIMARY KEY,
                code TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                url_slug TEXT,
                category_type TEXT NOT NULL,
                color TEXT,
                icon TEXT,
                is_active BOOLEAN NOT NULL DEFAULT 0,
                created_on TEXT NOT NULL,
                updated_on TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .unwrap();

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

    mod activation_logic {
        use super::*;

        /// Tests successful activation of an inactive category.
        ///
        /// Verifies that the service correctly activates a category
        /// and returns the updated instance. Ensures the database update
        /// and response construction work as expected.
        ///
        /// # Test Steps
        /// 1. Create and insert an inactive category.
        /// 2. Call `activate_category` via the service.
        /// 3. Assert the response contains the activated category.
        ///
        /// # Panics
        /// Panics if the test setup or assertions fail.
        #[sqlx::test]
        async fn test_activate_inactive_category(pool: SqlitePool) {
            let mut category = create_mock_category();
            category.is_active = false;
            let id = insert_test_category(&pool, &category).await;

            let service = CategoriesService::new(std::sync::Arc::new(pool));
            let request = tonic::Request::new(proto::CategoryActivateRequest {
                id: id.to_string(),
            });

            let result = activate_category(&service, request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            let activated = response.get_ref().category.as_ref().unwrap();
            assert_eq!(activated.id, id.to_string());
            assert!(activated.is_active);
        }

        /// Tests activation of an already active category.
        ///
        /// Ensures no errors occur and the category remains active.
        /// Verifies idempotent behaviour for activation.
        ///
        /// # Test Steps
        /// 1. Create and insert an active category.
        /// 2. Call `activate_category`.
        /// 3. Assert the category stays active without errors.
        ///
        /// # Panics
        /// Panics if the test setup or assertions fail.
        #[sqlx::test]
        async fn test_activate_already_active_category(pool: SqlitePool) {
            let mut category = create_mock_category();
            category.is_active = true;
            let id = insert_test_category(&pool, &category).await;

            let service = CategoriesService::new(std::sync::Arc::new(pool));
            let request = tonic::Request::new(proto::CategoryActivateRequest {
                id: id.to_string(),
            });

            let result = activate_category(&service, request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            let activated = response.get_ref().category.as_ref().unwrap();
            assert!(activated.is_active);
        }

        /// Tests activation with a nonexistent category ID.
        ///
        /// Verifies proper error handling for invalid IDs.
        /// Ensures the service returns `NotFound` status appropriately.
        ///
        /// # Test Steps
        /// 1. Use a mock ID that doesn't exist in the database.
        /// 2. Call `activate_category`.
        /// 3. Assert the error is `NotFound`.
        ///
        /// # Panics
        /// Panics if the test setup or assertions fail.
        #[sqlx::test]
        async fn test_activate_nonexistent_category(pool: SqlitePool) {
            // Create the table without inserting any category
            sqlx::query!(
                r#"
                CREATE TABLE IF NOT EXISTS categories (
                    id TEXT PRIMARY KEY,
                    code TEXT NOT NULL,
                    name TEXT NOT NULL,
                    description TEXT,
                    url_slug TEXT,
                    category_type TEXT NOT NULL,
                    color TEXT,
                    icon TEXT,
                    is_active BOOLEAN NOT NULL DEFAULT 0,
                    created_on TEXT NOT NULL,
                    updated_on TEXT NOT NULL
                )
                "#,
            )
            .execute(&pool)
            .await
            .unwrap();

            let fake_id = RowID::mock();
            let service = CategoriesService::new(std::sync::Arc::new(pool));
            let request = tonic::Request::new(proto::CategoryActivateRequest {
                id: fake_id.to_string(),
            });

            let result = activate_category(&service, request).await;
            assert!(result.is_err());
            let status = result.unwrap_err();
            assert!(matches!(status, RpcError::InvalidArgument(_)));
        }
    }

    mod edge_cases {
        use super::*;

        /// Tests activation with randomised category configurations.
        ///
        /// Runs multiple iterations to ensure robustness across
        /// different optional field combinations. Helps catch issues
        /// with field serialisation or database handling.
        ///
        /// # Test Steps
        /// 1. Generate 10 randomised categories.
        /// 2. Activate each and verify success.
        /// 3. Check that core fields are preserved.
        ///
        /// # Panics
        /// Panics if any iteration fails.
        ///
        /// # Notes
        /// - Uses probabilistic randomisation to simulate real-world variability.
        #[sqlx::test]
        async fn test_activate_with_randomised_fields(pool: SqlitePool) {
            for _ in 0..10 {
                let mut category = create_mock_category();
                category.is_active = false;
                let id = insert_test_category(&pool, &category).await;

                let service = CategoriesService::new(std::sync::Arc::new(pool.clone()));
                let request = tonic::Request::new(proto::CategoryActivateRequest {
                    id: id.to_string(),
                });

                let result = activate_category(&service, request).await;
                assert!(result.is_ok());
                let response = result.unwrap();
                let activated = response.get_ref().category.as_ref().unwrap();
                assert!(activated.is_active);
                // Ensure other fields are preserved
                assert_eq!(activated.name, category.name);
                assert_eq!(activated.code, category.code);
            }
        }

        /// Tests that timestamps are updated correctly on activation.
        ///
        /// Verifies `updated_on` is set to a later time, ensuring
        /// the database update logic correctly modifies timestamps.
        ///
        /// # Test Steps
        /// 1. Insert a category with a known `updated_on`.
        /// 2. Activate it.
        /// 3. Assert `updated_on` is now later.
        ///
        /// # Panics
        /// Panics if the test setup or assertions fail.
        ///
        /// # Notes
        /// - Compares timestamps to ensure monotonic updates.
        #[sqlx::test]
        async fn test_activation_updates_timestamp(pool: SqlitePool) {
            let mut category = create_mock_category();
            category.is_active = false;
            let original_updated = category.updated_on;
            let id = insert_test_category(&pool, &category).await;

            let service = CategoriesService::new(std::sync::Arc::new(pool));
            let request = tonic::Request::new(proto::CategoryActivateRequest {
                id: id.to_string(),
            });

            let result = activate_category(&service, request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            let activated = response.get_ref().category.as_ref().unwrap();
            let updated_on = activated.updated_on.as_ref().unwrap();
            let updated_time = chrono::DateTime::from_timestamp(updated_on.seconds, updated_on.nanos as u32).unwrap();
            assert!(updated_time > original_updated);
        }
    }
}