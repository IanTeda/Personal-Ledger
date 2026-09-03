//! # Sync Pull
//!
//! Handles a Client's `SyncService/Pull` call: return every Change Set after the
//! Client's cursor, oldest first -- including every Change Set queued while the Client
//! was offline, in one ordered batch (ADR-0009, FC-SYNC-003).

use lib_database as database;
use lib_domain as domain;

use crate::error::{RpcError, RpcResult};
use crate::sync::proto;

/// Pull Change Sets from the Sync Server's durable Change Set log since a cursor.
///
/// # Errors
/// Returns `tonic::Status::invalid_argument` if `since_id` can't be parsed, or
/// `tonic::Status::internal` for database failures.
#[tracing::instrument(name = "sync_pull", level = "debug", skip(service, request))]
pub async fn pull(
    service: &super::SyncService,
    request: tonic::Request<proto::PullRequest>,
) -> RpcResult<tonic::Response<proto::PullResponse>> {
    let pull_request = request.into_inner();

    let since_id = pull_request
        .since_id
        .map(|s| {
            s.parse::<domain::RowID>()
                .map_err(|e| RpcError::InvalidArgument(format!("Invalid since_id '{}': {}", s, e)))
        })
        .transpose()?;

    let limit = if pull_request.limit > 0 {
        pull_request.limit as i64
    } else {
        100
    };

    let change_sets =
        database::ChangeSet::find_since(since_id, limit, service.database_ref()).await?;

    tracing::debug!(since_id = ?since_id, count = change_sets.len(), "Pulled Change Sets");

    Ok(tonic::Response::new(proto::PullResponse {
        change_sets: change_sets
            .into_iter()
            .map(proto::ChangeSet::from)
            .collect(),
    }))
}
