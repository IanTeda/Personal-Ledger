//! # Sync Service (ADR-0009, FR.39a)
//!
//! The Sync Server's `SyncService/Push` and `SyncService/Pull` handlers over its durable
//! Change Set log. `lib-rpc` holds the wire protocol; the handlers live here because only
//! bins depend on `lib-database` (`docs/development/viper-architecture.md`, Dependencies).

mod error;
mod pull;
mod push;

use lib_database as database;
use lib_rpc as proto;

pub use error::{SyncError, SyncResult};

pub struct SyncService {
    database_pool: std::sync::Arc<sqlx::SqlitePool>,
}

impl SyncService {
    pub fn new(database_pool: std::sync::Arc<sqlx::SqlitePool>) -> Self {
        Self { database_pool }
    }

    pub fn database_ref(&self) -> &sqlx::SqlitePool {
        &self.database_pool
    }
}

/// A stored Change Set in its wire form, for a `Pull` response and the integration tests'
/// stand-in Clients' `Push`. A free function, not a `From`, since both types are foreign
/// to this crate.
pub fn to_proto(change_set: database::ChangeSet) -> proto::ChangeSet {
    proto::ChangeSet {
        id: change_set.id.to_string(),
        table_name: change_set.table_name,
        row_id: change_set.row_id.to_string(),
        field_name: change_set.field_name,
        value: change_set.value,
        hlc: change_set.hlc.to_string(),
        client_id: change_set.client_id.to_string(),
        version: change_set.version,
    }
}

/// A pushed Change Set parsed for the log, failing on any field that doesn't parse.
pub fn from_proto(change_set: proto::ChangeSet) -> SyncResult<database::ChangeSet> {
    let id = change_set.id.parse().map_err(|e| {
        SyncError::InvalidArgument(format!("Invalid Change Set id '{}': {}", change_set.id, e))
    })?;
    let row_id = change_set.row_id.parse().map_err(|e| {
        SyncError::InvalidArgument(format!(
            "Invalid Change Set row_id '{}': {}",
            change_set.row_id, e
        ))
    })?;
    let hlc = change_set.hlc.parse().map_err(|e| {
        SyncError::InvalidArgument(format!(
            "Invalid Change Set hlc '{}': {}",
            change_set.hlc, e
        ))
    })?;
    let client_id = change_set.client_id.parse().map_err(|e| {
        SyncError::InvalidArgument(format!(
            "Invalid Change Set client_id '{}': {}",
            change_set.client_id, e
        ))
    })?;

    Ok(database::change_sets::ChangeSetBuilder::new()
        .with_id(id)
        .with_table_name(change_set.table_name)
        .with_row_id(row_id)
        .with_field_name(change_set.field_name)
        .with_value_opt(change_set.value)
        .with_hlc(hlc)
        .with_client_id(client_id)
        .with_version_opt(Some(change_set.version))
        .build()?)
}

#[tonic::async_trait]
impl proto::SyncService for SyncService {
    async fn push(
        &self,
        request: tonic::Request<proto::PushRequest>,
    ) -> Result<tonic::Response<proto::PushResponse>, tonic::Status> {
        Ok(push::push(self, request).await?)
    }

    async fn pull(
        &self,
        request: tonic::Request<proto::PullRequest>,
    ) -> Result<tonic::Response<proto::PullResponse>, tonic::Status> {
        Ok(pull::pull(self, request).await?)
    }
}
