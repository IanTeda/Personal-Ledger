// -- ./src/utilities.rs --

//! Utilities module - gRPC services and types for utility operations.
//!
//! This module provides re-exports of generated protobuf types and gRPC clients/servers
//! for the utilities service. It includes health check functionality and other utility
//! operations for the personal ledger system.
//!
//! ## Services
//!
//! - **UtilitiesService**: Provides utility operations like health checks via ping.
//!
//! ## Types
//!
//! - `PingRequest`: Empty request for ping operations
//! - `PingResponse`: Response containing a pong message
//! - `UtilitiesServiceClient`: gRPC client for connecting to utilities service
//! - `UtilitiesService`: Server trait for implementing utilities service
//! - `UtilitiesServiceServer`: Server implementation for utilities service



/// gRPC client for the UtilitiesService.
/// Provides methods for utility operations, such as health checks via ping.
pub use crate::generated::utilities::utilities_service_client::UtilitiesServiceClient;

/// gRPC server trait and implementation for the UtilitiesService.
/// Implement the `UtilitiesService` trait to handle utility requests like ping.
pub use crate::generated::utilities::utilities_service_server::{
    UtilitiesService, UtilitiesServiceServer,
};

/// Utilities-related message types.
/// Includes structs for ping requests and responses used in the UtilitiesService.
/// These are protobuf-generated types for serialization and deserialization.
pub use crate::generated::utilities::{
    PingRequest,
    PingResponse,
};
