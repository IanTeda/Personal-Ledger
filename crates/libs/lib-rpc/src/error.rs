//! The crate's error type. Nothing hand-written here is fallible yet: the service handlers answer the wire with a `tonic::Status`, as the generated traits require, so
//! `Error` wraps that `Status` for a caller that holds one and keeps `Generic` for the infrastructure failures still to come. The generated files are build output
//! and never name this type.

/// Why a `lib-rpc` call failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An infrastructure failure with no variant of its own yet.
    #[error("{0}")]
    Generic(String),

    /// A gRPC call answered with a non-OK status.
    #[error(transparent)]
    Status(#[from] tonic::Status),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
