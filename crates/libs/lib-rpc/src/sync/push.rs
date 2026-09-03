//! # Sync Push
//!
//! Handles a Client's `SyncService/Push` call: insert every pushed Change Set into the
//! Sync Server's durable Change Set log (ADR-0009).

use lib_database as database;

use crate::error::RpcResult;
use crate::sync::proto;

/// Push one or more Change Sets into the Sync Server's durable Change Set log.
///
/// # Errors
/// Returns `tonic::Status::invalid_argument` if any Change Set's `id`/`row_id`/`hlc`/
/// `client_id` can't be parsed, or `tonic::Status::internal` for database failures.
#[tracing::instrument(name = "sync_push", level = "debug", skip(service, request))]
pub async fn push(
    service: &super::SyncService,
    request: tonic::Request<proto::PushRequest>,
) -> RpcResult<tonic::Response<proto::PushResponse>> {
    let push_request = request.into_inner();

    tracing::debug!(
        count = push_request.change_sets.len(),
        "Received Change Sets to push"
    );

    let mut accepted_count = 0;
    for change_set in push_request.change_sets {
        let change_set: database::ChangeSet = change_set.try_into()?;
        change_set.insert(service.database_ref()).await?;
        accepted_count += 1;
    }

    Ok(tonic::Response::new(proto::PushResponse { accepted_count }))
}
