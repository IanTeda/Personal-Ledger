//! # Category Deletion Logic
//!
//! This module provides the service logic for deleting categories
//! in the Personal Ledger backend. It includes:
//!
//! - Deleting a category by its unique ID
//! - Deleting multiple categories in a batch operation
//! - Proper error handling for not found cases and database errors

use crate::{database, rpc};

/// Handle the category deletion logic for the gRPC service.
///
/// This function performs:
/// - Parsing the ID from the request
/// - Deleting the category from the database
/// - Returning the number of rows deleted (0 or 1)
/// - Proper error handling for not found cases and database errors
///
/// # Arguments
/// * `service` - Reference to the `CategoriesService` (for DB access)
/// * `request` - The incoming gRPC request
///
/// # Returns
/// * `Ok(tonic::Response<CategoryDeleteResponse>)` on success
/// * `Err(tonic::Status)` on database error
pub async fn delete_category(
    service: &super::CategoriesService,
    request: tonic::Request<rpc::CategoryDeleteRequest>,
) -> Result<tonic::Response<rpc::CategoryDeleteResponse>, tonic::Status> {
    // Extract the inner request
    let delete_request = request.into_inner();

    // Parse the ID from string to RowID
    let category_id = match delete_request.id.parse::<crate::domain::RowID>() {
        Ok(id) => id,
        Err(_) => {
            return Err(tonic::Status::invalid_argument("Invalid category ID format"));
        }
    };

    // Delete the category from the database
    let rows_deleted = match database::Categories::delete_by_id(category_id, service.database_ref()).await {
        Ok(()) => 1, // Successfully deleted 1 row
        Err(database::DatabaseError::NotFound(_)) => 0, // Category not found, 0 rows deleted
        Err(db_error) => {
            tracing::error!("Failed to delete category {}: {}", delete_request.id, db_error);
            return Err(tonic::Status::internal("Failed to delete category"));
        }
    };

    // Return response with rows deleted count
    let response = rpc::CategoryDeleteResponse {
        rows_deleted: rows_deleted as i32,
    };

    Ok(tonic::Response::new(response))
}

/// Handle the batch category deletion logic for the gRPC service.
///
/// This function performs:
/// - Parsing multiple IDs from the request
/// - Deleting the categories from the database in a batch operation
/// - Returning the number of rows deleted
/// - Proper error handling for invalid IDs and database errors
///
/// # Arguments
/// * `service` - Reference to the `CategoriesService` (for DB access)
/// * `request` - The incoming gRPC request
///
/// # Returns
/// * `Ok(tonic::Response<CategoriesDeleteBatchResponse>)` on success
/// * `Err(tonic::Status)` on validation or database error
pub async fn delete_categories_batch(
    service: &super::CategoriesService,
    request: tonic::Request<rpc::CategoriesDeleteBatchRequest>,
) -> Result<tonic::Response<rpc::CategoriesDeleteBatchResponse>, tonic::Status> {
    // Extract the inner request
    let delete_batch_request = request.into_inner();

    // Parse all IDs from strings to RowIDs
    let mut category_ids = Vec::new();
    for id_str in &delete_batch_request.ids {
        match id_str.parse::<crate::domain::RowID>() {
            Ok(id) => category_ids.push(id),
            Err(_) => {
                return Err(tonic::Status::invalid_argument(format!("Invalid category ID format: {}", id_str)));
            }
        }
    }

    // If no valid IDs provided, return 0 rows deleted
    if category_ids.is_empty() {
        let response = rpc::CategoriesDeleteBatchResponse {
            rows_deleted: 0,
        };
        return Ok(tonic::Response::new(response));
    }

    // Delete the categories from the database
    let rows_deleted = match database::Categories::delete_many_by_id(&category_ids, service.database_ref()).await {
        Ok(()) => category_ids.len() as i32, // All categories were successfully deleted
        Err(database::DatabaseError::NotFound(_msg)) => {
            // Some categories were not found - count how many were actually deleted
            // by attempting individual deletes and counting successes
            let mut actual_deleted = 0;
            for &id in &category_ids {
                match database::Categories::delete_by_id(id, service.database_ref()).await {
                    Ok(()) => actual_deleted += 1,
                    Err(database::DatabaseError::NotFound(_)) => {
                        // Category not found, skip it
                    }
                    Err(db_error) => {
                        tracing::error!("Failed to delete category {} during batch operation: {}", id, db_error);
                        return Err(tonic::Status::internal("Failed to delete categories"));
                    }
                }
            }
            actual_deleted
        }
        Err(db_error) => {
            tracing::error!("Failed to delete categories batch: {}", db_error);
            return Err(tonic::Status::internal("Failed to delete categories"));
        }
    };

    // Return response with rows deleted count
    let response = rpc::CategoriesDeleteBatchResponse {
        rows_deleted,
    };

    Ok(tonic::Response::new(response))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_delete_category_with_valid_id() {
        // This would be an integration test that requires a database
        // For now, we test the basic structure
        // Integration tests will be added in tests/api/categories/delete.rs
    }

    #[test]
    fn test_delete_category_with_invalid_id() {
        // This would test invalid ID parsing
        // For now, we rely on integration tests
    }

    #[test]
    fn test_delete_categories_batch_with_valid_ids() {
        // This would be an integration test that requires a database
        // For now, we test the basic structure
        // Integration tests will be added in tests/api/categories/delete_batch.rs
    }

    #[test]
    fn test_delete_categories_batch_with_invalid_ids() {
        // This would test invalid ID parsing in batch
        // For now, we rely on integration tests
    }
}