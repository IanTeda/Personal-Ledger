//! # gRPC auth interceptor
//!
//! Protects `SyncService` (FC-SYNC-007): every Push/Pull call must carry a valid
//! `authorization: Bearer <jwt>` header, checked with [`crate::auth::jwt::verify_access_token`].
//! `UtilitiesService`/Ping stays open -- this interceptor is wired only onto
//! `SyncServiceServer` in `main.rs`, matching the ticket's "sync endpoints" scope.

use secrecy::SecretString;

/// A [`tonic::service::Interceptor`] that requires a valid bearer access token.
#[derive(Clone)]
pub struct AuthInterceptor {
    signing_key: SecretString,
}

impl AuthInterceptor {
    pub fn new(signing_key: SecretString) -> Self {
        Self { signing_key }
    }
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let token = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(|| {
                tonic::Status::unauthenticated("missing or malformed authorization header")
            })?;

        super::jwt::verify_access_token(token, &self.signing_key)
            .map_err(|_| tonic::Status::unauthenticated("invalid or expired access token"))?;

        Ok(request)
    }
}
