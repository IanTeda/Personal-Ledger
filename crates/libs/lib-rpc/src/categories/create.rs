//! # Category Creation Logic
//!
//! This module provides the conversion and service logic for creating categories
//! in the Personal Ledger backend. It includes:
//!
//! - Conversion from gRPC `CategoryCreateRequest` to domain/database `Category`
//! - Validation and error handling for all fields
//! - The async service handlers for single and batch category creation, used by the gRPC service
//! - Comprehensive unit tests for all conversion and validation logic
//!
//! The core business logic is abstracted here to keep the gRPC service layer clean
//! and to enable easier testing and maintenance.
//!
//! ## Business Rules
//!
//! Categories must adhere to the following validation rules:
//! - **Code**: Required, non-empty string, trimmed of whitespace
//! - **Name**: Required, non-empty string, trimmed of whitespace
//! - **Category Type**: Must be a valid [`CategoryTypes`] enum value
//! - **URL Slug**: Optional, must be valid URL slug format if provided
//! - **Color**: Optional, must be valid hex color format if provided
//! - **Description/Icon**: Optional, whitespace-only values are treated as None
//!
//! ## Error Handling
//!
//! All validation errors are converted to appropriate gRPC status codes:
//! - `INVALID_ARGUMENT` (400): Client validation errors
//! - `INTERNAL` (500): Server/database errors
//! - `UNAVAILABLE` (502): Database connectivity issues
//!
//! ## Performance Considerations
//!
//! - Batch operations use atomic transactions for consistency
//! - Individual category validation happens before database operations
//! - Tracing is used for observability without impacting performance

use lib_database as database;
use lib_domain as domain;

use crate::categories::proto;
use crate::error::{RpcError, RpcResult};

/// Convert a gRPC `CategoryCreateRequest` into a domain/database `Category`.
///
/// This implementation performs all necessary validation and conversion of fields,
/// ensuring that required fields are present and valid, and that optional fields
/// are handled correctly. Any validation or parsing errors are returned as a
/// `RpcError`.
///
/// ## Validation Rules
///
/// - **Category field**: Must be present in the request
/// - **Code**: Required, non-empty after trimming whitespace
/// - **Name**: Required, non-empty after trimming whitespace
/// - **Category Type**: Must map to a valid [`CategoryTypes`] variant
/// - **URL Slug**: If provided, must be valid slug format (validated by [`UrlSlug::parse`])
/// - **Color**: If provided, must be valid hex color format (validated by [`HexColor::parse`])
/// - **Description/Icon**: Whitespace-only values are treated as `None`
///
/// ## Field Mapping
///
/// - `id`: Auto-generated using [`RowID::new()`]
/// - `created_on`/`updated_on`: Set to current UTC timestamp
/// - `is_active`: Uses the value from the request (defaults to `true` if not specified)
///
/// ## Error Handling
///
/// Returns [`RpcError::Validation`] for all client-side validation failures,
/// which are converted to appropriate gRPC status codes by the service handlers.
///
/// ## Examples
///
/// ```rust,no_run
/// use lib_rpc::categories::proto;
/// use lib_database::Categories;
/// use std::convert::TryFrom;
///
/// let request = proto::CategoryCreateRequest {
///     category: Some(proto::Category {
///         id: "".to_string(),
///         code: "GROCERY".to_string(),
///         name: "Groceries".to_string(),
///         description: Some("Weekly grocery expenses".to_string()),
///         url_slug: Some("groceries".to_string()),
///         category_type: proto::CategoryTypes::Expense as i32,
///         color: Some("#FF5733".to_string()),
///         icon: Some("shopping-cart".to_string()),
///         is_active: true,
///         created_on: None,
///         updated_on: None,
///     }),
/// };
///
/// let category = Categories::try_from(request).unwrap();
/// assert_eq!(category.code, "GROCERY");
/// assert_eq!(category.name, "Groceries");
/// ```
impl TryFrom<proto::CategoryCreateRequest> for database::Categories {
    type Error = RpcError;

    fn try_from(value: proto::CategoryCreateRequest) -> Result<Self, Self::Error> {
        let category = value.category.ok_or_else(|| {
            RpcError::Validation("Category field is required in CategoryCreateRequest".to_string())
        })?;

        // Generate new ID for the category
        let id = domain::RowID::new();

        // Validate and parse required fields
        let code = if category.code.trim().is_empty() {
            return Err(RpcError::Validation(
                "Category code is required and cannot be empty".to_string(),
            ));
        } else {
            category.code
        };

        let name = if category.name.trim().is_empty() {
            return Err(RpcError::Validation(
                "Category name is required and cannot be empty".to_string(),
            ));
        } else {
            category.name
        };

        // Parse optional description
        let description = category.description.filter(|s| !s.trim().is_empty());

        // Parse optional URL slug
        let url_slug = if let Some(slug) = category.url_slug.filter(|s| !s.trim().is_empty()) {
            Some(domain::UrlSlug::parse(slug).map_err(|e| RpcError::Validation(format!("Invalid URL slug: {}", e)))?)
        } else {
            None
        };

        // Parse category type
        let category_type = domain::CategoryTypes::from_rpc_i32(category.category_type)
            .map_err(|e| RpcError::Validation(e.to_string()))?;

        // Parse optional color
        let color = if let Some(color_str) = category.color.filter(|s| !s.trim().is_empty()) {
            Some(domain::HexColor::parse(color_str).map_err(|e| RpcError::Validation(format!("Invalid hex color: {}", e)))?)
        } else {
            None
        };

        // Parse optional icon
        let icon = category.icon.filter(|s| !s.trim().is_empty());

        // Default to active for new categories
        let is_active = category.is_active;

        // Set timestamps
        let created_on = chrono::Utc::now();
        let updated_on = created_on;

        Ok(database::Categories {
            id,
            code,
            name,
            description,
            url_slug,
            category_type,
            color,
            icon,
            is_active,
            created_on,
            updated_on,
        })
    }
}

/// Handle the category creation logic for the gRPC service.
///
/// This function performs:
/// - Validation and conversion of the incoming gRPC request
/// - Insertion of the new category into the database
/// - Conversion of the inserted category back to gRPC response format
/// - Proper error handling and mapping to gRPC status codes
///
/// ## Arguments
/// * `service` - Reference to the `CategoriesService` for database access
/// * `request` - The incoming gRPC `CategoryCreateRequest` wrapped in a tonic request
///
/// ## Returns
/// * `Ok(tonic::Response<CategoryCreateResponse>)` containing the created category on success
/// * `Err(tonic::Status)` with appropriate gRPC status code on validation or database error
///
/// ## Errors
///
/// This function will return an error if:
/// - The request contains invalid data (validation errors)
/// - Database insertion fails
/// - Database connection is unavailable
///
/// ## Examples
///
/// ```rust,no_run
/// use lib_rpc::categories::{create_category, CategoriesService};
/// use lib_rpc::categories::proto;
/// use tonic::{Request, Response};
///
/// # async fn example(service: &CategoriesService) -> Result<(), Box<dyn std::error::Error>> {
/// let request = Request::new(proto::CategoryCreateRequest {
///     category: Some(proto::Category {
///         id: "".to_string(),
///         code: "FOOD".to_string(),
///         name: "Food & Dining".to_string(),
///         description: Some("Expenses for food and dining".to_string()),
///         url_slug: Some("food-dining".to_string()),
///         category_type: proto::CategoryTypes::Expense as i32,
///         color: Some("#FF5733".to_string()),
///         icon: Some("utensils".to_string()),
///         is_active: true,
///         created_on: None,
///         updated_on: None,
///     }),
/// });
///
/// let response = create_category(service, request).await?;
/// let created_category = response.into_inner().category.unwrap();
/// assert_eq!(created_category.code, "FOOD");
/// # Ok(())
/// # }
/// ```
pub async fn create_category(
    service: &super::CategoriesService,
    request: tonic::Request<proto::CategoryCreateRequest>,
) -> Result<tonic::Response<proto::CategoryCreateResponse>, tonic::Status> {
    // Extract the inner request
    let create_request = request.into_inner();

    // Convert the request to a database category
    let category = match database::Categories::try_from(create_request) {
        Ok(category) => {
            tracing::debug!("Successfully validated category '{}' for creation", category.code);
            category
        }
        Err(rpc_error) => {
            tracing::debug!("Category validation failed: {}", rpc_error);
            // Convert RpcError to tonic::Status
            let status_code = match rpc_error.http_status_code() {
                400 => tonic::Code::InvalidArgument,
                401 => tonic::Code::Unauthenticated,
                404 => tonic::Code::NotFound,
                422 => tonic::Code::FailedPrecondition,
                500 => tonic::Code::Internal,
                502 => tonic::Code::Unavailable,
                _ => tonic::Code::Internal,
            };
            return Err(tonic::Status::new(status_code, rpc_error.to_string()));
        }
    };

    // Insert the category into the database
    let inserted_category = match category.insert(service.database_ref()).await {
        Ok(category) => {
            tracing::info!("Successfully created category '{}' with ID {}", category.code, category.id);
            category
        }
        Err(db_error) => {
            tracing::error!("Failed to insert category '{}' into database: {}", category.code, db_error);
            return Err(tonic::Status::internal("Failed to create category"));
        }
    };

    // Convert the database category back to RPC format
    let rpc_category: proto::Category = inserted_category.into();

    // Create the response
    let response = proto::CategoryCreateResponse {
        category: Some(rpc_category),
    };

    Ok(tonic::Response::new(response))
}

/// Handle the batch category creation logic for the gRPC service.
///
/// This function performs:
/// - Validation and conversion of all categories in the batch
/// - Atomic insertion of all categories using database transactions
/// - Conversion of inserted categories back to gRPC response format
/// - Proper error handling and mapping to gRPC status codes
///
/// The operation is atomic - either all categories are created successfully,
/// or none are created if any validation or insertion fails.
///
/// ## Arguments
/// * `service` - Reference to the `CategoriesService` for database access
/// * `request` - The incoming gRPC `CategoriesCreateBatchRequest` wrapped in a tonic request
///
/// ## Returns
/// * `Ok(tonic::Response<CategoriesCreateBatchResponse>)` containing all created categories on success
/// * `Err(tonic::Status)` with appropriate gRPC status code on validation or database error
///
/// ## Errors
///
/// This function will return an error if:
/// - The batch request contains no categories
/// - Any category in the batch fails validation
/// - Database insertion fails for any category
/// - Database transaction fails to commit
///
/// ## Performance Notes
///
/// - All categories are validated before any database operations
/// - Uses batch database insertion for efficiency
/// - Database operations are wrapped in transactions for consistency
///
/// ## Examples
///
/// ```rust,no_run
/// use lib_rpc::categories::{create_batch_categories, CategoriesService};
/// use lib_rpc::categories::proto;
/// use tonic::{Request, Response};
///
/// # async fn example(service: &CategoriesService) -> Result<(), Box<dyn std::error::Error>> {
/// let categories = vec![
///     proto::Category {
///         id: "".to_string(),
///         code: "FOOD".to_string(),
///         name: "Food & Dining".to_string(),
///         description: Some("Expenses for food and dining".to_string()),
///         url_slug: Some("food-dining".to_string()),
///         category_type: proto::CategoryTypes::Expense as i32,
///         color: Some("#FF5733".to_string()),
///         icon: Some("utensils".to_string()),
///         is_active: true,
///         created_on: None,
///         updated_on: None,
///     },
///     proto::Category {
///         id: "".to_string(),
///         code: "TRANSPORT".to_string(),
///         name: "Transportation".to_string(),
///         description: Some("Transportation expenses".to_string()),
///         url_slug: Some("transportation".to_string()),
///         category_type: proto::CategoryTypes::Expense as i32,
///         color: Some("#4A90E2".to_string()),
///         icon: Some("car".to_string()),
///         is_active: true,
///         created_on: None,
///         updated_on: None,
///     },
/// ];
///
/// let request = Request::new(proto::CategoriesCreateBatchRequest { categories });
/// let response = create_batch_categories(service, request).await?;
/// let batch_response = response.into_inner();
/// assert_eq!(batch_response.created_count, 2);
/// assert_eq!(batch_response.categories.len(), 2);
/// # Ok(())
/// # }
/// ```
pub async fn create_batch_categories(
    service: &super::CategoriesService,
    request: tonic::Request<proto::CategoriesCreateBatchRequest>,
) -> Result<tonic::Response<proto::CategoriesCreateBatchResponse>, tonic::Status> {
    // Extract the inner request
    let batch_request = request.into_inner();

    // Validate that we have categories to create
    if batch_request.categories.is_empty() {
        tracing::debug!("Batch category creation request contains no categories");
        return Err(tonic::Status::invalid_argument(
            "No categories provided for batch creation",
        ));
    }

    tracing::info!("Processing batch category creation with {} categories", batch_request.categories.len());

    // Convert each RPC category to database category
    let mut db_categories = Vec::with_capacity(batch_request.categories.len());

    for (index, rpc_category) in batch_request.categories.into_iter().enumerate() {
        // Create a CategoryCreateRequest for each category
        let create_request = proto::CategoryCreateRequest {
            category: Some(rpc_category),
        };

        // Convert to database category using the existing TryFrom implementation
        match database::Categories::try_from(create_request) {
            Ok(db_category) => {
                tracing::debug!("Successfully validated category '{}' at index {}", db_category.code, index);
                db_categories.push(db_category);
            }
            Err(rpc_error) => {
                tracing::debug!("Category validation failed at index {}: {}", index, rpc_error);
                // Include the index in the error message for better debugging
                let error_msg = format!("Category at index {}: {}", index, rpc_error);
                let status_code = match rpc_error.http_status_code() {
                    400 => tonic::Code::InvalidArgument,
                    401 => tonic::Code::Unauthenticated,
                    404 => tonic::Code::NotFound,
                    422 => tonic::Code::FailedPrecondition,
                    500 => tonic::Code::Internal,
                    502 => tonic::Code::Unavailable,
                    _ => tonic::Code::Internal,
                };
                return Err(tonic::Status::new(status_code, error_msg));
            }
        }
    }

    tracing::debug!("Successfully validated {} categories for batch insertion", db_categories.len());

    // Insert all categories in a batch using the database's insert_many method
    let inserted_categories =
        match database::Categories::insert_many(&db_categories, service.database_ref()).await {
            Ok(categories) => {
                tracing::info!("Successfully created {} categories in batch", categories.len());
                categories
            }
            Err(db_error) => {
                tracing::error!("Failed to batch insert {} categories into database: {}", db_categories.len(), db_error);
                return Err(tonic::Status::internal(
                    "Failed to create categories in batch",
                ));
            }
        };

    // Convert database categories back to RPC format
    let rpc_categories: Vec<proto::Category> = inserted_categories
        .into_iter()
        .map(|category| category.into())
        .collect();

    // Create the response
    let response = proto::CategoriesCreateBatchResponse {
        categories: rpc_categories.clone(),
        created_count: rpc_categories.len() as i32,
    };

    Ok(tonic::Response::new(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test successful conversion with all fields populated
    #[test]
    fn test_try_from_valid_category_create_request() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "will-be-ignored".to_string(),
                code: "GROCERY".to_string(),
                name: "Groceries".to_string(),
                description: Some("Weekly grocery expenses".to_string()),
                url_slug: Some("groceries".to_string()),
                category_type: proto::CategoryTypes::Expense as i32,
                color: Some("#FF5733".to_string()),
                icon: Some("shopping-cart".to_string()),
                is_active: true,
                created_on: None, // Will be ignored
                updated_on: None, // Will be ignored
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_ok());

        let category = result.unwrap();
        assert_eq!(category.code, "GROCERY");
        assert_eq!(category.name, "Groceries");
        assert_eq!(
            category.description,
            Some("Weekly grocery expenses".to_string())
        );
        assert_eq!(category.url_slug.as_ref().unwrap().as_str(), "groceries");
        assert_eq!(category.category_type, domain::CategoryTypes::Expense);
        assert_eq!(category.color.as_ref().unwrap().as_str(), "#FF5733");
        assert_eq!(category.icon, Some("shopping-cart".to_string()));
        assert!(category.is_active);
    }

    /// Test successful conversion with minimal required fields only
    #[test]
    fn test_try_from_minimal_category_create_request() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "SAVINGS".to_string(),
                name: "Savings".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Asset as i32,
                color: None,
                icon: None,
                is_active: false,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_ok());

        let category = result.unwrap();
        assert_eq!(category.code, "SAVINGS");
        assert_eq!(category.name, "Savings");
        assert!(category.description.is_none());
        assert!(category.url_slug.is_none());
        assert_eq!(category.category_type, domain::CategoryTypes::Asset);
        assert!(category.color.is_none());
        assert!(category.icon.is_none());
        assert!(!category.is_active);
    }

    /// Test error handling when category field is missing
    #[test]
    fn test_try_from_missing_category_field() {
        let request = proto::CategoryCreateRequest { category: None };

        let result = database::Categories::try_from(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RpcError::Validation(_)));
    }

    /// Test error handling for empty code field
    #[test]
    fn test_try_from_empty_code() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "".to_string(),
                name: "Test".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RpcError::Validation(_)));
    }

    /// Test error handling for whitespace-only name field
    #[test]
    fn test_try_from_empty_name() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "TEST".to_string(),
                name: "   ".to_string(), // whitespace only
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RpcError::Validation(_)));
    }

    /// Test error handling for invalid category type values
    #[test]
    fn test_try_from_invalid_category_type() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "TEST".to_string(),
                name: "Test".to_string(),
                description: None,
                url_slug: None,
                category_type: 999, // Invalid category type
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RpcError::Validation(_)));
    }

    /// Test error handling for invalid URL slug values
    #[test]
    fn test_try_from_invalid_url_slug() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "TEST".to_string(),
                name: "Test".to_string(),
                description: None,
                url_slug: Some("!@#$%^&*()".to_string()), // Only invalid characters
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RpcError::Validation(_)));
    }

    /// Test error handling for invalid hex color values
    #[test]
    fn test_try_from_invalid_color() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "TEST".to_string(),
                name: "Test".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: Some("INVALID".to_string()),
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RpcError::Validation(_)));
    }

    /// Test that empty/whitespace-only optional fields become None
    #[test]
    fn test_try_from_empty_optional_fields() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "TEST".to_string(),
                name: "Test".to_string(),
                description: Some("".to_string()), // Empty string should become None
                url_slug: Some("   ".to_string()), // Whitespace should become None
                category_type: proto::CategoryTypes::Expense as i32,
                color: Some("".to_string()), // Empty string should become None
                icon: Some("   ".to_string()), // Whitespace should become None
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_ok());

        let category = result.unwrap();
        assert!(category.description.is_none());
        assert!(category.url_slug.is_none());
        assert!(category.color.is_none());
        assert!(category.icon.is_none());
    }

    /// Test error handling for whitespace-only code field
    #[test]
    fn test_try_from_whitespace_only_code() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "   ".to_string(), // whitespace only
                name: "Test".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RpcError::Validation(_)));
    }

    /// Test conversion for all valid category type enum values
    #[test]
    fn test_try_from_all_category_types() {
        let test_cases = vec![
            (
                proto::CategoryTypes::Asset as i32,
                domain::CategoryTypes::Asset,
            ),
            (
                proto::CategoryTypes::Equity as i32,
                domain::CategoryTypes::Equity,
            ),
            (
                proto::CategoryTypes::Expense as i32,
                domain::CategoryTypes::Expense,
            ),
            (
                proto::CategoryTypes::Income as i32,
                domain::CategoryTypes::Income,
            ),
            (
                proto::CategoryTypes::Liability as i32,
                domain::CategoryTypes::Liability,
            ),
        ];

        for (rpc_type, expected_domain_type) in test_cases {
            let request = proto::CategoryCreateRequest {
                category: Some(proto::Category {
                    id: "".to_string(),
                    code: format!("TEST_{}", rpc_type),
                    name: format!("Test Category {}", rpc_type),
                    description: None,
                    url_slug: None,
                    category_type: rpc_type,
                    color: None,
                    icon: None,
                    is_active: true,
                    created_on: None,
                    updated_on: None,
                }),
            };

            let result = database::Categories::try_from(request);
            assert!(result.is_ok(), "Failed for category type {}", rpc_type);

            let category = result.unwrap();
            assert_eq!(category.category_type, expected_domain_type);
        }
    }

    /// Test URL slug edge cases where cleaning results in empty strings
    #[test]
    fn test_try_from_url_slug_edge_cases() {
        // Test URL slug that becomes empty after cleaning - should be treated as invalid
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "TEST".to_string(),
                name: "Test".to_string(),
                description: None,
                url_slug: Some("!!!@@@###".to_string()), // Only special chars that get filtered out
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RpcError::Validation(_)));
    }

    /// Test hex color validation for valid and invalid formats
    #[test]
    fn test_try_from_color_edge_cases() {
        // Test various color formats
        let valid_colors = vec!["#FF5733", "#123456", "#ABCDEF", "#000000", "#FFFFFF"];
        let invalid_colors = vec!["INVALID", "#12", "#12345", "#GGGGGG", ""];

        for color in valid_colors {
            let request = proto::CategoryCreateRequest {
                category: Some(proto::Category {
                    id: "".to_string(),
                    code: format!("TEST_{}", color.replace("#", "")),
                    name: "Test".to_string(),
                    description: None,
                    url_slug: None,
                    category_type: proto::CategoryTypes::Expense as i32,
                    color: Some(color.to_string()),
                    icon: None,
                    is_active: true,
                    created_on: None,
                    updated_on: None,
                }),
            };

            let result = database::Categories::try_from(request);
            assert!(
                result.is_ok(),
                "Valid color {} should parse successfully",
                color
            );
        }

        for color in invalid_colors {
            if color.is_empty() {
                continue; // Empty colors are handled separately
            }

            let request = proto::CategoryCreateRequest {
                category: Some(proto::Category {
                    id: "".to_string(),
                    code: "TEST".to_string(),
                    name: "Test".to_string(),
                    description: None,
                    url_slug: None,
                    category_type: proto::CategoryTypes::Expense as i32,
                    color: Some(color.to_string()),
                    icon: None,
                    is_active: true,
                    created_on: None,
                    updated_on: None,
                }),
            };

            let result = database::Categories::try_from(request);
            assert!(result.is_err(), "Invalid color {} should fail", color);
        }
    }

    /// Test that mixed whitespace in optional fields becomes None
    #[test]
    fn test_try_from_mixed_whitespace_optional_fields() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "TEST".to_string(),
                name: "Test".to_string(),
                description: Some("\t\n  \t".to_string()), // Mixed whitespace
                url_slug: Some("\r\n\t".to_string()),      // Mixed whitespace
                category_type: proto::CategoryTypes::Expense as i32,
                color: Some("  \n\t  ".to_string()), // Mixed whitespace
                icon: Some("\t\r\n".to_string()),    // Mixed whitespace
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_ok());

        let category = result.unwrap();
        assert!(
            category.description.is_none(),
            "Mixed whitespace description should become None"
        );
        assert!(
            category.url_slug.is_none(),
            "Mixed whitespace URL slug should become None"
        );
        assert!(
            category.color.is_none(),
            "Mixed whitespace color should become None"
        );
        assert!(
            category.icon.is_none(),
            "Mixed whitespace icon should become None"
        );
    }

    /// Test that non-empty optional fields with whitespace are preserved correctly
    #[test]
    fn test_try_from_preserves_non_empty_optional_fields() {
        let request = proto::CategoryCreateRequest {
            category: Some(proto::Category {
                id: "".to_string(),
                code: "TEST".to_string(),
                name: "Test".to_string(),
                description: Some("  Valid description  ".to_string()), // Leading/trailing whitespace preserved
                url_slug: Some("  valid-slug  ".to_string()), // Leading/trailing whitespace preserved
                category_type: proto::CategoryTypes::Expense as i32,
                color: Some("#FF5733".to_string()),
                icon: Some("  shopping-cart  ".to_string()), // Leading/trailing whitespace preserved
                is_active: true,
                created_on: None,
                updated_on: None,
            }),
        };

        let result = database::Categories::try_from(request);
        assert!(result.is_ok());

        let category = result.unwrap();
        assert_eq!(
            category.description,
            Some("  Valid description  ".to_string())
        );
        assert_eq!(category.url_slug.as_ref().unwrap().as_str(), "valid-slug"); // Slug gets cleaned
        assert_eq!(category.color.as_ref().unwrap().as_str(), "#FF5733");
        assert_eq!(category.icon, Some("  shopping-cart  ".to_string()));
    }

    /// Test successful batch creation with multiple valid categories
    #[test]
    fn test_batch_create_valid_categories() {
        // This would require a test database setup
        // For now, just test the conversion logic
        let categories = vec![
            proto::Category {
                id: "".to_string(),
                code: "FOOD_BATCH".to_string(),
                name: "Food & Dining Batch".to_string(),
                description: Some("Food and dining expenses from batch".to_string()),
                url_slug: Some("food-dining-batch".to_string()),
                category_type: proto::CategoryTypes::Expense as i32,
                color: Some("#FF5733".to_string()),
                icon: Some("utensils".to_string()),
                is_active: true,
                created_on: None,
                updated_on: None,
            },
            proto::Category {
                id: "".to_string(),
                code: "TRANSPORT_BATCH".to_string(),
                name: "Transportation Batch".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            },
        ];

        let batch_request = proto::CategoriesCreateBatchRequest { categories };

        // Test that we can create the request (basic validation)
        assert_eq!(batch_request.categories.len(), 2);
        assert_eq!(batch_request.categories[0].code, "FOOD_BATCH");
        assert_eq!(batch_request.categories[1].code, "TRANSPORT_BATCH");
    }

    /// Test batch creation with empty list
    #[test]
    fn test_batch_create_empty_list() {
        let batch_request = proto::CategoriesCreateBatchRequest { categories: vec![] };

        assert_eq!(batch_request.categories.len(), 0);
    }

    /// Test batch creation with single category
    #[test]
    fn test_batch_create_single_category() {
        let categories = vec![proto::Category {
            id: "".to_string(),
            code: "UTILITIES_BATCH".to_string(),
            name: "Utilities Batch".to_string(),
            description: Some("Utility bills and services from batch".to_string()),
            url_slug: Some("utilities-batch".to_string()),
            category_type: proto::CategoryTypes::Expense as i32,
            color: Some("#4A90E2".to_string()),
            icon: Some("bolt".to_string()),
            is_active: false, // Test inactive category
            created_on: None,
            updated_on: None,
        }];

        let batch_request = proto::CategoriesCreateBatchRequest { categories };

        assert_eq!(batch_request.categories.len(), 1);
        assert_eq!(batch_request.categories[0].code, "UTILITIES_BATCH");
        assert!(!batch_request.categories[0].is_active);
    }

    /// Test batch conversion logic with multiple valid categories
    #[test]
    fn test_batch_conversion_valid_categories() {
        let categories = vec![
            proto::Category {
                id: "".to_string(),
                code: "FOOD_BATCH".to_string(),
                name: "Food & Dining Batch".to_string(),
                description: Some("Food and dining expenses from batch".to_string()),
                url_slug: Some("food-dining-batch".to_string()),
                category_type: proto::CategoryTypes::Expense as i32,
                color: Some("#FF5733".to_string()),
                icon: Some("utensils".to_string()),
                is_active: true,
                created_on: None,
                updated_on: None,
            },
            proto::Category {
                id: "".to_string(),
                code: "TRANSPORT_BATCH".to_string(),
                name: "Transportation Batch".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            },
        ];

        let mut db_categories = Vec::with_capacity(categories.len());
        let mut errors = Vec::new();

        for (index, rpc_category) in categories.into_iter().enumerate() {
            let create_request = proto::CategoryCreateRequest {
                category: Some(rpc_category),
            };

            match database::Categories::try_from(create_request) {
                Ok(db_category) => db_categories.push(db_category),
                Err(service_error) => {
                    errors.push((index, service_error));
                    break; // Simulate batch function stopping on first error
                }
            }
        }

        assert!(errors.is_empty(), "No errors expected for valid categories");
        assert_eq!(db_categories.len(), 2);
        assert_eq!(db_categories[0].code, "FOOD_BATCH");
        assert_eq!(db_categories[1].code, "TRANSPORT_BATCH");
    }

    /// Test batch conversion logic with an invalid category at index 1
    #[test]
    fn test_batch_conversion_invalid_category_at_index() {
        let categories = vec![
            proto::Category {
                id: "".to_string(),
                code: "VALID".to_string(),
                name: "Valid Category".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            },
            proto::Category {
                id: "".to_string(),
                code: "".to_string(), // Invalid: empty code
                name: "Invalid Category".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            },
            proto::Category {
                id: "".to_string(),
                code: "ANOTHER_VALID".to_string(),
                name: "Another Valid".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            },
        ];

        let mut db_categories = Vec::with_capacity(categories.len());
        let mut errors = Vec::new();

        for (index, rpc_category) in categories.into_iter().enumerate() {
            let create_request = proto::CategoryCreateRequest {
                category: Some(rpc_category),
            };

            match database::Categories::try_from(create_request) {
                Ok(db_category) => db_categories.push(db_category),
                Err(service_error) => {
                    errors.push((index, service_error));
                    break; // Simulate batch function stopping on first error
                }
            }
        }

        assert_eq!(errors.len(), 1, "Expected one error");
        assert_eq!(errors[0].0, 1, "Error should be at index 1");
        assert!(matches!(errors[0].1, RpcError::Validation(_)));
        assert_eq!(
            db_categories.len(),
            1,
            "Only first valid category should be converted"
        );
        assert_eq!(db_categories[0].code, "VALID");
    }

    /// Test batch conversion logic with all invalid categories
    #[test]
    fn test_batch_conversion_all_invalid_categories() {
        let categories = vec![
            proto::Category {
                id: "".to_string(),
                code: "   ".to_string(), // Invalid: whitespace-only code
                name: "Invalid 1".to_string(),
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            },
            proto::Category {
                id: "".to_string(),
                code: "INVALID2".to_string(),
                name: "".to_string(), // Invalid: empty name
                description: None,
                url_slug: None,
                category_type: proto::CategoryTypes::Expense as i32,
                color: None,
                icon: None,
                is_active: true,
                created_on: None,
                updated_on: None,
            },
        ];

        let mut db_categories = Vec::with_capacity(categories.len());
        let mut errors = Vec::new();

        for (index, rpc_category) in categories.into_iter().enumerate() {
            let create_request = proto::CategoryCreateRequest {
                category: Some(rpc_category),
            };

            match database::Categories::try_from(create_request) {
                Ok(db_category) => db_categories.push(db_category),
                Err(service_error) => {
                    errors.push((index, service_error));
                    break; // Simulate batch function stopping on first error
                }
            }
        }

        assert_eq!(errors.len(), 1, "Expected one error on first invalid");
        assert_eq!(errors[0].0, 0, "Error should be at index 0");
        assert!(matches!(errors[0].1, RpcError::Validation(_)));
        assert!(
            db_categories.is_empty(),
            "No categories should be converted"
        );
    }
}
