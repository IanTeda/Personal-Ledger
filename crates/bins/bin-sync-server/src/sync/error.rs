//! # Sync Service errors
//!
//! What a `Push` or `Pull` handler can fail with, and the gRPC status each becomes.

pub type SyncResult<T> = Result<T, SyncError>;

#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    /// The Change Set log couldn't be read or written.
    #[error("Database error: {0}")]
    Database(#[from] lib_database::Error),

    /// A Client sent a field that doesn't parse, such as a malformed id or HLC.
    #[error("Invalid rpc argument: {0}")]
    InvalidArgument(String),
}

impl From<SyncError> for tonic::Status {
    fn from(err: SyncError) -> Self {
        match err {
            SyncError::Database(_) => tonic::Status::internal(err.to_string()),
            SyncError::InvalidArgument(msg) => tonic::Status::invalid_argument(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_argument_maps_to_invalid_argument() {
        let status: tonic::Status = SyncError::InvalidArgument("malformed id".to_string()).into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(status.message(), "malformed id");
    }

    #[test]
    fn database_maps_to_internal() {
        let error =
            SyncError::Database(lib_database::Error::CategoryBuilder("DB down".to_string()));

        let status: tonic::Status = error.into();
        assert_eq!(status.code(), tonic::Code::Internal);
        assert_eq!(
            status.message(),
            "Database error: Error building category: DB down"
        );
    }
}
