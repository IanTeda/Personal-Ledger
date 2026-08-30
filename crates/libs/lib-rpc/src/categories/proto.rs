// -- ./src/categories.rs --

//! Categories module - gRPC services and types for financial category operations.
//!
//! This module provides re-exports of generated protobuf types and gRPC clients/servers
//! for the categories service. It handles all CRUD operations for financial categories
//! used in organising transactions and accounts in the personal ledger system.
//!
//! ## Services
//!
//! - **CategoriesService**: Handles CRUD operations for financial categories including
//!   batch operations and activation/deactivation.
//!
//! ## Types
//!
//! Core message types include:
//! - `Category`: The main category struct with all fields
//! - `CategoryTypes`: Enum defining category types (Asset, Expense, etc.)
//! - `Request/Response` types for all operations (Create, Get, Update, Delete, List, etc.)
//! - `CategoriesServiceClient`: gRPC client for connecting to categories service
//! - `CategoriesService`: Server trait for implementing categories service
//! - `CategoriesServiceServer`: Server implementation for categories service

// -------------------------- [ CATEGORIES ] ---------------------------------

/// gRPC client for the CategoriesService.
/// Provides methods for creating, reading, updating, deleting, and listing financial categories.
/// Supports batch operations and activation/deactivation.
pub use crate::generated::categories::categories_service_client::CategoriesServiceClient;

/// gRPC server trait and implementation for the CategoriesService.
/// Implement the `CategoriesService` trait to handle incoming gRPC requests for category management.
pub use crate::generated::categories::categories_service_server::{
    CategoriesService, CategoriesServiceServer,
};

/// Categories-related message types.
/// Includes structs for categories, requests, and responses used in the CategoriesService.
/// These are protobuf-generated types for serialization and deserialization.
pub use crate::generated::categories::{
    Category,
    CategoryTypes,
    CategoryCreateRequest,
    CategoryCreateResponse,
    CategoryGetRequest,
    CategoryGetResponse,
    CategoryGetByCodeRequest,
    CategoryGetByCodeResponse,
    CategoryGetBySlugRequest,
    CategoryGetBySlugResponse,
    CategoriesListRequest,
    CategoriesListResponse,
    CategoryUpdateRequest,
    CategoryUpdateResponse,
    CategoriesCreateBatchRequest,
    CategoriesCreateBatchResponse,
    CategoryDeleteRequest,
    CategoryDeleteResponse,
    CategoriesDeleteBatchRequest,
    CategoriesDeleteBatchResponse,
    CategoryActivateRequest,
    CategoryActivateResponse,
    CategoryDeactivateRequest,
    CategoryDeactivateResponse,
};
