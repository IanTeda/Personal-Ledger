//! lib-rpc - gRPC services and types for the personal ledger.
//!
//! This crate provides re-exports of generated protobuf types and gRPC clients/servers
//! for the sync and utilities services. It serves as the main interface for interacting
//! with the personal ledger's gRPC APIs.
//!
//! It is the wire protocol only and holds no storage: `bin-sync-server` implements
//! [`SyncService`] over its Change Set log, since only bins depend on `lib-database`
//! (`docs/development/viper-architecture.md`, Dependencies).
//!
//! ## Services
//!
//! - **SyncService**: Pushes and pulls Change Sets between Clients and the Sync Server (FR.39a).
//! - **UtilitiesService**: Provides utility operations like health checks.
//!
//! ## Usage
//!
//! Use the re-exported clients and servers to build gRPC clients or implement servers.
//! Message types are available for constructing requests and handling responses.

mod generated;
mod sync;
mod utilities;

// Re-export sync module to maintain flat API
pub use sync::*;

// Re-export utilities module to maintain flat API
pub use utilities::*;
