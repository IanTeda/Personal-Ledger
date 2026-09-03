use crate::error::RpcError;
use crate::sync::proto;
use lib_database as database;

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

impl From<database::ChangeSet> for proto::ChangeSet {
    fn from(change_set: database::ChangeSet) -> Self {
        Self {
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
}

impl TryFrom<proto::ChangeSet> for database::ChangeSet {
    type Error = RpcError;

    fn try_from(change_set: proto::ChangeSet) -> Result<Self, Self::Error> {
        let id = change_set.id.parse().map_err(|e| {
            RpcError::InvalidArgument(format!("Invalid Change Set id '{}': {}", change_set.id, e))
        })?;
        let row_id = change_set.row_id.parse().map_err(|e| {
            RpcError::InvalidArgument(format!(
                "Invalid Change Set row_id '{}': {}",
                change_set.row_id, e
            ))
        })?;
        let hlc = change_set.hlc.parse().map_err(|e| {
            RpcError::InvalidArgument(format!(
                "Invalid Change Set hlc '{}': {}",
                change_set.hlc, e
            ))
        })?;
        let client_id = change_set.client_id.parse().map_err(|e| {
            RpcError::InvalidArgument(format!(
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
}

#[tonic::async_trait]
impl proto::SyncService for SyncService {
    async fn push(
        &self,
        request: tonic::Request<proto::PushRequest>,
    ) -> Result<tonic::Response<proto::PushResponse>, tonic::Status> {
        Ok(crate::sync::push::push(self, request).await?)
    }

    async fn pull(
        &self,
        request: tonic::Request<proto::PullRequest>,
    ) -> Result<tonic::Response<proto::PullResponse>, tonic::Status> {
        Ok(crate::sync::pull::pull(self, request).await?)
    }
}
