//! lib-rpc - gRPC services and types for the personal ledger.
//!
//! This crate provides re-exports of generated protobuf types and gRPC clients/servers
//! for categories and utilities services. It serves as the main interface for interacting
//! with the personal ledger's gRPC APIs.
//!
//! ## Services
//!
//! - **CategoriesService**: Handles CRUD operations for financial categories.
//! - **UtilitiesService**: Provides utility operations like health checks.
//!
//! ## Usage
//!
//! Use the re-exported clients and servers to build gRPC clients or implement servers.
//! Message types are available for constructing requests and handling responses.

#![allow(unused)] // For development only

mod categories;
pub(crate) mod error;
mod generated;
mod sync;
mod utilities;

// Re-export categories module to maintain flat API
pub use categories::*;

// Re-export sync module to maintain flat API
pub use sync::*;

// Re-export utilities module to maintain flat API
pub use utilities::*;
