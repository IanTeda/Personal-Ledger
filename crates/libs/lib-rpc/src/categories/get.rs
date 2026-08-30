//! # Category Retrieval Logic
//!
//! This module provides the service logic for retrieving categories
//! from the Personal Ledger backend. It includes:
//!
//! - Getting a category by its unique ID
//! - Getting a category by its unique code
//! - Getting a category by its URL slug
//! - Proper error handling for not found cases and database errors

use crate::{database, rpc};

/// Handle the category retrieval by ID logic for the gRPC service.
///
/// This function performs:
/// - Parsing the ID from the request
/// - Querying the database for the category
/// - Converting the database category to gRPC response format
/// - Proper error handling for not found cases and database errors
///
/// # Arguments
/// * `service` - Reference to the `CategoriesService` (for DB access)
/// * `request` - The incoming gRPC request
///
/// # Returns
/// * `Ok(tonic::Response<CategoryGetResponse>)` on success
/// * `Err(tonic::Status)` on not found or database error
pub async fn get_category(
    service: &super::CategoriesService,
    request: tonic::Request<rpc::CategoryGetRequest>,
) -> Result<tonic::Response<rpc::CategoryGetResponse>, tonic::Status> {
    // Extract the inner request
    let get_request = request.into_inner();

    // Parse the ID from string to RowID
    let category_id = match get_request.id.parse::<crate::domain::RowID>() {
        Ok(id) => id,
        Err(_) => {
            return Err(tonic::Status::invalid_argument("Invalid category ID format"));
        }
    };

    // Query the database for the category
    let category = match database::Categories::find_by_id(category_id, service.database_ref()).await {
        Ok(Some(category)) => category,
        Ok(None) => {
            return Err(tonic::Status::not_found(format!("Category with ID '{}' not found", get_request.id)));
        }
        Err(db_error) => {
            tracing::error!("Failed to find category by ID {}: {}", get_request.id, db_error);
            return Err(tonic::Status::internal("Failed to retrieve category"));
        }
    };

    // Convert the database category to RPC format
    let rpc_category: rpc::Category = category.into();

    // Create the response
    let response = rpc::CategoryGetResponse {
        category: Some(rpc_category),
    };

    Ok(tonic::Response::new(response))
}

/// Handle the category retrieval by code logic for the gRPC service.
///
/// This function performs:
/// - Extracting the code from the request
/// - Querying the database for the category by code
/// - Converting the database category to gRPC response format
/// - Proper error handling for not found cases and database errors
///
/// # Arguments
/// * `service` - Reference to the `CategoriesService` (for DB access)
/// * `request` - The incoming gRPC request
///
/// # Returns
/// * `Ok(tonic::Response<CategoryGetByCodeResponse>)` on success
/// * `Err(tonic::Status)` on not found or database error
pub async fn get_category_by_code(
    service: &super::CategoriesService,
    request: tonic::Request<rpc::CategoryGetByCodeRequest>,
) -> Result<tonic::Response<rpc::CategoryGetByCodeResponse>, tonic::Status> {
    // Extract the inner request
    let get_request = request.into_inner();

    // Validate that code is not empty
    if get_request.code.trim().is_empty() {
        return Err(tonic::Status::invalid_argument("Category code cannot be empty"));
    }

    // Query the database for the category
    let category = match database::Categories::find_by_code(&get_request.code, service.database_ref()).await {
        Ok(Some(category)) => category,
        Ok(None) => {
            return Err(tonic::Status::not_found(format!("Category with code '{}' not found", get_request.code)));
        }
        Err(db_error) => {
            tracing::error!("Failed to find category by code {}: {}", get_request.code, db_error);
            return Err(tonic::Status::internal("Failed to retrieve category"));
        }
    };

    // Convert the database category to RPC format
    let rpc_category: rpc::Category = category.into();

    // Create the response
    let response = rpc::CategoryGetByCodeResponse {
        category: Some(rpc_category),
    };

    Ok(tonic::Response::new(response))
}

/// Handle the category retrieval by URL slug logic for the gRPC service.
///
/// This function performs:
/// - Extracting and parsing the URL slug from the request
/// - Querying the database for the category by slug
/// - Converting the database category to gRPC response format
/// - Proper error handling for not found cases and database errors
///
/// # Arguments
/// * `service` - Reference to the `CategoriesService` (for DB access)
/// * `request` - The incoming gRPC request
///
/// # Returns
/// * `Ok(tonic::Response<CategoryGetBySlugResponse>)` on success
/// * `Err(tonic::Status)` on not found or database error
pub async fn get_category_by_slug(
    service: &super::CategoriesService,
    request: tonic::Request<rpc::CategoryGetBySlugRequest>,
) -> Result<tonic::Response<rpc::CategoryGetBySlugResponse>, tonic::Status> {
    // Extract the inner request
    let get_request = request.into_inner();

    // Validate that slug is not empty
    if get_request.url_slug.trim().is_empty() {
        return Err(tonic::Status::invalid_argument("URL slug cannot be empty"));
    }

    // Parse the URL slug
    let url_slug = match crate::domain::UrlSlug::parse(&get_request.url_slug) {
        Ok(slug) => slug,
        Err(_) => {
            return Err(tonic::Status::invalid_argument("Invalid URL slug format"));
        }
    };

    // Query the database for the category
    let category = match database::Categories::find_by_url_slug(&url_slug, service.database_ref()).await {
        Ok(Some(category)) => category,
        Ok(None) => {
            return Err(tonic::Status::not_found(format!("Category with slug '{}' not found", get_request.url_slug)));
        }
        Err(db_error) => {
            tracing::error!("Failed to find category by slug {}: {}", get_request.url_slug, db_error);
            return Err(tonic::Status::internal("Failed to retrieve category"));
        }
    };

    // Convert the database category to RPC format
    let rpc_category: rpc::Category = category.into();

    // Create the response
    let response = rpc::CategoryGetBySlugResponse {
        category: Some(rpc_category),
    };

    Ok(tonic::Response::new(response))
}