// -- ./src/sync/proto.rs --

//! Sync module - gRPC services and types for Change Set push/pull operations.
//!
//! This module provides re-exports of generated protobuf types and gRPC clients/servers
//! for the sync service. The Sync Server does not expose full Ledger CRUD; Clients
//! propagate their local edits to each other only by pushing and pulling Change Sets
//! through it (ADR-0009).
//!
//! ## Services
//!
//! - **SyncService**: Push and pull Change Sets through the Sync Server's durable log.
//!
//! ## Types
//!
//! - `ChangeSet`: One field-level edit, mirroring `lib_database::ChangeSet`
//! - `PushRequest`/`PushResponse`, `PullRequest`/`PullResponse`
//! - `SyncServiceClient`: gRPC client for connecting to the sync service
//! - `SyncService`: Server trait for implementing the sync service
//! - `SyncServiceServer`: Server implementation for the sync service

/// gRPC client for the SyncService.
/// Provides methods for pushing and pulling Change Sets through the Sync Server.
pub use crate::generated::sync::sync_service_client::SyncServiceClient;

/// gRPC server trait and implementation for the SyncService.
/// Implement the `SyncService` trait to handle incoming push/pull requests.
pub use crate::generated::sync::sync_service_server::{SyncService, SyncServiceServer};

/// Sync-related message types.
/// Includes structs for Change Sets, requests, and responses used in the SyncService.
/// These are protobuf-generated types for serialization and deserialization.
pub use crate::generated::sync::{ChangeSet, PullRequest, PullResponse, PushRequest, PushResponse};
