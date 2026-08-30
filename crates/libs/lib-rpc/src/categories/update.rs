//! # Category Listing Logic
//!
//! This module provides the service logic for listing categories
//! from the Personal Ledger backend. It includes:
//!
//! - Flexible category listing with filtering, sorting, and pagination
//! - Support for filtering by category type and active status
//! - Proper error handling and response formatting

use crate::{database, domain, rpc};

/// Handle the category listing logic for the gRPC service.
///
/// This function performs:
/// - Parsing and validation of filter parameters from the request
/// - Querying the database with flexible filtering, sorting, and pagination
/// - Converting database categories to gRPC response format
/// - Proper error handling for database errors
///
/// # Arguments
/// * `service` - Reference to the `CategoriesService` (for DB access)
/// * `request` - The incoming gRPC request
///
/// # Returns
/// * `Ok(tonic::Response<CategoriesListResponse>)` on success
/// * `Err(tonic::Status)` on database error
pub async fn list_categories(
    service: &super::CategoriesService,
    request: tonic::Request<rpc::CategoriesListRequest>,
) -> Result<tonic::Response<rpc::CategoriesListResponse>, tonic::Status> {
    // Extract the inner request
    let list_request = request.into_inner();

    // Parse optional filters
    let category_type_filter = match list_request.category_type {
        Some(ct) if ct != 0 => {
            // Try to convert the proto enum to domain enum
            match domain::CategoryTypes::from_rpc_i32(ct) {
                Ok(domain_type) => Some(domain_type),
                Err(_) => return Err(tonic::Status::invalid_argument("Invalid category type")),
            }
        }
        _ => None,
    };

    let is_active_filter = list_request.is_active;

    // Parse sorting parameters
    let sort_by = list_request.sort_by.filter(|s| !s.trim().is_empty());

    let sort_desc = list_request.sort_desc;

    // Validate pagination parameters
    if list_request.offset < 0 {
        return Err(tonic::Status::invalid_argument("Offset cannot be negative"));
    }

    if list_request.limit <= 0 {
        return Err(tonic::Status::invalid_argument("Limit must be positive"));
    }

    if list_request.limit > 1000 {
        return Err(tonic::Status::invalid_argument("Limit cannot exceed 1000"));
    }

    // Query the database with filters
    let (categories, total_count) = match database::Categories::find_with_filters(
        category_type_filter,
        is_active_filter,
        sort_by.as_deref(),
        sort_desc,
        list_request.offset,
        list_request.limit,
        service.database_ref(),
    ).await {
        Ok(result) => result,
        Err(db_error) => {
            tracing::error!("Failed to list categories: {}", db_error);
            return Err(tonic::Status::internal("Failed to retrieve categories"));
        }
    };

    // Convert database categories to RPC format
    let rpc_categories: Vec<rpc::Category> = categories
        .into_iter()
        .map(|category| category.into())
        .collect();

    // Create the response
    let response = rpc::CategoriesListResponse {
        categories: rpc_categories,
        total_count,
        offset: list_request.offset,
        limit: list_request.limit,
    };

    Ok(tonic::Response::new(response))
}