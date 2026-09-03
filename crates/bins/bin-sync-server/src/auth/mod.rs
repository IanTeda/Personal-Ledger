//! # Sync Server auth (ADR-0010, FC-SYNC-007)
//!
//! OAuth2 Authorization Code + PKCE over a loopback redirect, with the Sync Server
//! acting as its own authorization *and* resource server: [`routes`] serves the
//! `/authorize` login form and `/token` endpoint, [`interceptor::AuthInterceptor`]
//! protects `SyncService`'s gRPC calls with the resulting bearer JWT.

mod codes;
pub mod interceptor;
pub mod jwt;
mod password;
mod pkce;
mod routes;

pub use codes::CodeStore;
pub use routes::{AuthState, routes};

/// Hash passwords for the bootstrap account (Argon2). Verification is used internally
/// by [`routes`]'s `/authorize` handler via `super::password` directly.
pub use password::hash_password;
