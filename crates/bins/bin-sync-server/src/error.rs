//! Crate error type for the Sync Server binary.
//!
//! One `Error` for the whole bin (#1312, point 7): the start-up failures `main.rs` carries up
//! with `?`, the fields a Client's `Push` or `Pull` sends that don't parse, and the `/token`
//! refusals. Each variant's `#[error]` text is the body a Client reads, so it is the response
//! wording, not log prose. [`Error::kind`] sorts each variant once, and the two boundaries,
//! [`From<Error> for tonic::Status`] and [`IntoResponse for Error`], pick their status from that
//! sort, answering an internal failure with "internal error" after logging it, so a library's
//! text never reaches a Client.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// What a Client reads for a failure that is the Sync Server's, not the request's.
const INTERNAL_ERROR: &str = "internal error";

/// How a boundary answers an [`Error`]: the one sort both protocols share, so a new variant is
/// decided once, in [`Error::kind`], rather than in each boundary's match.
enum Kind {
    /// The request was malformed or refused: `InvalidArgument` over gRPC, 400 over HTTP.
    Invalid,
    /// The caller isn't, or is no longer, a sync user: `Unauthenticated` over gRPC, 401 over HTTP.
    Unauthenticated,
    /// The Sync Server's own failure, logged and answered with [`INTERNAL_ERROR`].
    Internal,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Configuration loading/validation failures.
    #[error("Configuration error: {0}")]
    Config(#[from] lib_config::Error),

    /// Telemetry initialisation failures.
    #[error("Telemetry error: {0}")]
    Telemetry(#[from] lib_tracing::Error),

    /// Errors from the Sync Server's durable Change Set / `sync_users` store (connection,
    /// migration, query).
    #[error("Database error: {0}")]
    Database(#[from] lib_database::Error),

    /// Bind-address parsing failures.
    #[error("Invalid bind address: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    /// Network/listener I/O failures.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Argon2 password hashing failures (bootstrap account provisioning).
    #[error("Password hashing error: {0}")]
    PasswordHash(#[from] argon2::password_hash::Error),

    /// Signing or verifying an access token failed.
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    /// A Client sent an id field (`field` names it as the Client knows it) that isn't a `RowID`.
    #[error("Invalid {field} '{value}': {source}")]
    InvalidRowId {
        field: &'static str,
        value: String,
        source: lib_core::RowIDError,
    },

    /// A Client sent an HLC field that isn't a Hybrid Logical Clock.
    #[error("Invalid {field} '{value}': {source}")]
    InvalidHlc {
        field: &'static str,
        value: String,
        source: lib_core::HybridLogicalClockError,
    },

    /// A `/token` request left out a field its grant type needs.
    #[error("missing {0}")]
    MissingField(&'static str),

    /// The authorization code was never issued, already redeemed or has expired.
    #[error("invalid or expired code")]
    InvalidCode,

    /// The `redirect_uri` redeeming a code isn't the one it was issued to.
    #[error("redirect_uri does not match the authorization request")]
    RedirectMismatch,

    /// The PKCE `code_verifier` doesn't hash to the code's `code_challenge`.
    #[error("code_verifier does not match code_challenge")]
    PkceMismatch,

    /// A refresh was asked for before any sync user exists.
    #[error("no sync user provisioned")]
    NoSyncUser,

    /// The refresh token isn't the sync user's current one.
    #[error("invalid or already-rotated refresh token")]
    RefreshTokenRejected,
}

/// This crate's fallible results.
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    /// The one match that lists every variant, so the compiler asks for a sort for each new one.
    fn kind(&self) -> Kind {
        match self {
            Error::InvalidRowId { .. }
            | Error::InvalidHlc { .. }
            | Error::MissingField(_)
            | Error::InvalidCode
            | Error::RedirectMismatch
            | Error::PkceMismatch => Kind::Invalid,
            Error::NoSyncUser | Error::RefreshTokenRejected => Kind::Unauthenticated,
            Error::Config(_)
            | Error::Telemetry(_)
            | Error::Database(_)
            | Error::AddrParse(_)
            | Error::Io(_)
            | Error::PasswordHash(_)
            | Error::Jwt(_) => Kind::Internal,
        }
    }
}

impl From<Error> for tonic::Status {
    fn from(err: Error) -> Self {
        match err.kind() {
            Kind::Invalid => tonic::Status::invalid_argument(err.to_string()),
            Kind::Unauthenticated => tonic::Status::unauthenticated(err.to_string()),
            Kind::Internal => {
                tracing::error!("Sync request failed: {err}");
                tonic::Status::internal(INTERNAL_ERROR)
            }
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self.kind() {
            Kind::Invalid => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),
            Kind::Unauthenticated => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
            Kind::Internal => {
                tracing::error!("Auth request failed: {self}");
                (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database_error() -> Error {
        Error::Database(lib_database::Error::CategoryBuilder("DB down".to_string()))
    }

    async fn response_of(err: Error) -> (StatusCode, String) {
        let response = err.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[test]
    fn an_unparsable_id_maps_to_invalid_argument_naming_the_field() {
        let source = "not-an-id".parse::<lib_core::RowID>().unwrap_err();
        let status: tonic::Status = Error::InvalidRowId {
            field: "since_id",
            value: "not-an-id".to_string(),
            source: source.clone(),
        }
        .into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(
            status.message(),
            format!("Invalid since_id 'not-an-id': {source}")
        );
    }

    #[test]
    fn an_unparsable_hlc_maps_to_invalid_argument_naming_the_field() {
        let source = "garbage"
            .parse::<lib_core::HybridLogicalClock>()
            .unwrap_err();
        let status: tonic::Status = Error::InvalidHlc {
            field: "Change Set hlc",
            value: "garbage".to_string(),
            source: source.clone(),
        }
        .into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(
            status.message(),
            format!("Invalid Change Set hlc 'garbage': {source}")
        );
    }

    #[test]
    fn database_maps_to_internal() {
        let status: tonic::Status = database_error().into();
        assert_eq!(status.code(), tonic::Code::Internal);
        assert_eq!(status.message(), "internal error");
    }

    #[tokio::test]
    async fn token_refusals_answer_with_todays_status_and_body() {
        let cases = [
            (
                Error::MissingField("code"),
                StatusCode::BAD_REQUEST,
                "missing code",
            ),
            (
                Error::MissingField("refresh_token"),
                StatusCode::BAD_REQUEST,
                "missing refresh_token",
            ),
            (
                Error::InvalidCode,
                StatusCode::BAD_REQUEST,
                "invalid or expired code",
            ),
            (
                Error::RedirectMismatch,
                StatusCode::BAD_REQUEST,
                "redirect_uri does not match the authorization request",
            ),
            (
                Error::PkceMismatch,
                StatusCode::BAD_REQUEST,
                "code_verifier does not match code_challenge",
            ),
            (
                Error::NoSyncUser,
                StatusCode::UNAUTHORIZED,
                "no sync user provisioned",
            ),
            (
                Error::RefreshTokenRejected,
                StatusCode::UNAUTHORIZED,
                "invalid or already-rotated refresh token",
            ),
        ];
        for (err, status, body) in cases {
            assert_eq!(response_of(err).await, (status, body.to_string()));
        }
    }

    #[tokio::test]
    async fn an_internal_failure_answers_internal_error_without_its_text() {
        let jwt = Error::Jwt(jsonwebtoken::errors::ErrorKind::InvalidToken.into());
        for err in [database_error(), jwt] {
            assert_eq!(
                response_of(err).await,
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string()
                )
            );
        }
    }
}
