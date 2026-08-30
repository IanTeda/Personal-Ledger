
//! # RPC Error Types
//!
//! This module defines error types used throughout the RPC layer of the Personal Ledger.
//! It provides a unified error handling approach that bridges the gap between
//! domain errors, database errors, and gRPC transport errors.
//!
//! ## Error Hierarchy
//!
//! The [`RpcError`] enum categorizes errors into several types:
//!
//! - **Connection**: Network/transport layer errors
//! - **Grpc**: gRPC protocol-level errors
//! - **Client**: Internal client state errors
//! - **Database**: Database operation failures
//! - **InvalidArgument**: Client-provided invalid arguments
//! - **Validation**: Business logic validation failures
//!
//! ## Error Conversion
//!
//! Errors are automatically converted to appropriate gRPC status codes:
//! - `400 Bad Request`: Validation and argument errors
//! - `500 Internal Server Error`: Server-side failures
//! - `502 Bad Gateway`: Connection issues
//!
//! ## Usage
//!
//! ```rust,no_run
//! use lib_rpc::error::RpcError;
//!
//! // Convert validation errors
//! let validation_error = RpcError::Validation("Invalid category code".to_string());
//!
//! // Convert to gRPC status
//! let status: tonic::Status = validation_error.into();
//! assert_eq!(status.code(), tonic::Code::InvalidArgument);
//! ```

pub type RpcResult<T> = std::result::Result<T, RpcError>;

/// Errors that can occur when using the RPC layer.
///
/// This enum represents all possible error conditions that can arise during
/// gRPC operations, providing a unified error handling interface that maps
/// to appropriate HTTP status codes and gRPC status codes.
///
/// ## Error Categories
///
/// Errors are categorised by their source and appropriate HTTP status code:
///
/// | Error Type | HTTP Status | Description |
/// |------------|-------------|-------------|
/// | `Validation` | 400 | Business logic validation failures |
/// | `InvalidArgument` | 400 | Malformed or invalid client arguments |
/// | `Connection` | 502 | Network/transport connectivity issues |
/// | `Grpc` | 500 | gRPC protocol-level errors |
/// | `Client` | 500 | Internal client state errors |
/// | `Database` | 500 | Database operation failures |
///
/// ## Examples
///
/// ```rust
/// use lib_rpc::error::RpcError;
///
/// // Create different types of errors
/// let validation = RpcError::Validation("Category name cannot be empty".to_string());
/// let db_error = RpcError::Database(lib_database::DatabaseError::Connection("DB down".to_string()));
///
/// // All errors can be converted to gRPC status
/// let status: tonic::Status = validation.into();
/// assert_eq!(status.code(), tonic::Code::InvalidArgument);
/// ```
#[derive(thiserror::Error, Debug)]
pub enum RpcError {
    /// Failed to establish or maintain connection to the gRPC service.
    ///
    /// This error occurs when there are network connectivity issues,
    /// DNS resolution failures, or TLS handshake problems.
    ///
    /// **HTTP Status**: 502 Bad Gateway
    #[error("connection error: {0}")]
    Connection(#[from] tonic::transport::Error),

    /// gRPC operation failed with a protocol-level status error.
    ///
    /// This wraps tonic::Status errors that come from the gRPC framework
    /// itself, such as timeouts, cancellations, or protocol violations.
    ///
    /// **HTTP Status**: 500 Internal Server Error
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    /// Internal client error indicating invalid state or configuration.
    ///
    /// This represents bugs or misconfigurations in the client code,
    /// such as attempting operations on disconnected clients.
    ///
    /// **HTTP Status**: 500 Internal Server Error
    #[error("client error: {0}")]
    Client(String),

    /// Database operation failed.
    ///
    /// This wraps database-specific errors from the data layer,
    /// such as connection failures, constraint violations, or query errors.
    ///
    /// **HTTP Status**: 500 Internal Server Error
    #[error("Database error: {0}")]
    Database(#[from] lib_database::DatabaseError),

    /// Client provided an invalid argument to an RPC method.
    ///
    /// This represents malformed or semantically invalid arguments
    /// that cannot be processed, such as invalid IDs or malformed data.
    ///
    /// **HTTP Status**: 400 Bad Request
    #[error("Invalid rpc argument: {0}")]
    InvalidArgument(String),

    /// Business logic validation failed.
    ///
    /// This occurs when client-provided data passes basic validation
    /// but fails business rule checks, such as duplicate codes or
    /// invalid relationships.
    ///
    /// **HTTP Status**: 400 Bad Request
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Convert an [`RpcError`] into a gRPC [`tonic::Status`].
///
/// This implementation maps each error variant to the appropriate gRPC status code
/// and message format. The mapping follows gRPC and HTTP conventions:
///
/// - Validation errors become `INVALID_ARGUMENT` (400)
/// - Connection errors become `UNAVAILABLE` (502)
/// - Server errors become `INTERNAL` (500)
///
/// ## Examples
///
/// ```rust
/// use lib_rpc::error::RpcError;
/// use tonic::Code;
///
/// let validation_error = RpcError::Validation("Invalid input".to_string());
/// let status: tonic::Status = validation_error.into();
/// assert_eq!(status.code(), Code::InvalidArgument);
///
/// let connection_error = RpcError::Connection(tonic::transport::Error::from(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused")));
/// let status: tonic::Status = connection_error.into();
/// assert_eq!(status.code(), Code::Unavailable);
/// ```
impl From<RpcError> for tonic::Status {
    fn from(err: RpcError) -> Self {
        match err {
            RpcError::Connection(e) => tonic::Status::unavailable(format!("Connection error: {}", e)),
            RpcError::Grpc(status) => tonic::Status::new(status.code(), status.message().to_string()),
            RpcError::Client(msg) => tonic::Status::internal(msg),
            RpcError::Database(db_err) => tonic::Status::internal(format!("Database error: {}", db_err)),
            RpcError::InvalidArgument(msg) => tonic::Status::invalid_argument(msg),
            RpcError::Validation(msg) => tonic::Status::invalid_argument(msg),
        }
    }
}

/// Get the HTTP status code corresponding to this error.
///
/// This method provides the HTTP status code that should be returned
/// when this error occurs in an HTTP context (e.g., REST API or web interface).
///
/// ## Return Values
///
/// - `400`: Client errors (validation, invalid arguments)
/// - `500`: Server errors (database, client state, gRPC protocol)
/// - `502`: Gateway errors (connection issues)
///
/// ## Examples
///
/// ```rust
/// use lib_rpc::error::RpcError;
///
/// let validation_error = RpcError::Validation("Invalid data".to_string());
/// assert_eq!(validation_error.http_status_code(), 400);
///
/// let db_error = RpcError::Database(lib_database::DatabaseError::Connection("DB down".to_string()));
/// assert_eq!(db_error.http_status_code(), 500);
///
/// let conn_error = RpcError::Connection(tonic::transport::Error::from(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused")));
/// assert_eq!(conn_error.http_status_code(), 502);
/// ```
impl RpcError {
    pub fn http_status_code(&self) -> u16 {
        match self {
            RpcError::Validation(_) => 400,
            RpcError::InvalidArgument(_) => 400,
            RpcError::Connection(_) => 502,
            RpcError::Grpc(_) => 500,
            RpcError::Client(_) => 500,
            RpcError::Database(_) => 500,
        }
    }
}