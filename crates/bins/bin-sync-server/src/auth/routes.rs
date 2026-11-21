//! # `/authorize` and `/token` HTTP routes
//!
//! The OAuth2 Authorization Code + PKCE surface ADR-0010 locked in: a login form the
//! system browser renders (`/authorize`), and the token endpoint a native-app Client
//! calls directly (`/token`) to redeem an authorization code or rotate a refresh token.
//! Merged into the same listener as the gRPC services in `main.rs` via
//! `tonic::service::Routes::into_axum_router()` (ADR-0010's "one listener, not two").

use std::sync::Arc;

use axum::extract::{Form, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use secrecy::SecretString;
use sha2::{Digest, Sha256};

use super::codes::CodeStore;
use super::{jwt, password, pkce};
use crate::{Error, Result};

/// Shared state for the `/authorize` and `/token` handlers.
#[derive(Clone)]
pub struct AuthState {
    pub pool: Arc<sqlx::SqlitePool>,
    pub codes: Arc<CodeStore>,
    pub signing_key: SecretString,
}

/// Build the `/authorize` + `/token` router.
pub fn routes(state: AuthState) -> Router {
    Router::new()
        .route("/authorize", get(get_authorize).post(post_authorize))
        .route("/token", post(post_token))
        .with_state(state)
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// Percent-encode a Client-supplied value for safe inclusion in a redirect URL's
/// query component (RFC 3986 §2.3's unreserved set passes through unescaped).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Query parameters a Client's authorization request carries -- both on the initial
/// `GET` and resubmitted (via the login form's `action` URL) on the `POST`.
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AuthorizeParams {
    pub response_type: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub redirect_uri: String,
    pub state: Option<String>,
}

/// `GET /authorize` -- render the login form. Rejects anything that isn't a
/// PKCE/`S256` Authorization Code request up front, per RFC 7636 §4.3.
#[tracing::instrument(name = "GET /authorize", level = "debug", skip(params))]
async fn get_authorize(Query(params): Query<AuthorizeParams>) -> impl IntoResponse {
    if params.response_type != "code" || params.code_challenge_method != "S256" {
        return (
            StatusCode::BAD_REQUEST,
            "unsupported response_type or code_challenge_method",
        )
            .into_response();
    }

    Html(login_form_html(&params)).into_response()
}

fn login_form_html(params: &AuthorizeParams) -> String {
    let query = serde_urlencoded::to_string(params).unwrap_or_default();
    format!(
        r#"<!doctype html>
<html>
<head><title>Personal Ledger Sync Server</title></head>
<body>
<h1>Sign in</h1>
<form method="post" action="/authorize?{query}">
  <label>Username <input type="text" name="username" autofocus></label><br>
  <label>Password <input type="password" name="password"></label><br>
  <button type="submit">Sign in</button>
</form>
</body>
</html>"#
    )
}

/// Login form submission.
#[derive(Debug, serde::Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/// `POST /authorize` -- validate credentials, mint an authorization code, redirect
/// back to the Client's loopback `redirect_uri` (RFC 8252 §7.3).
#[tracing::instrument(name = "POST /authorize", level = "debug", skip(state, form), fields(username = %form.username))]
async fn post_authorize(
    State(state): State<AuthState>,
    Query(params): Query<AuthorizeParams>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if params.response_type != "code" || params.code_challenge_method != "S256" {
        return (
            StatusCode::BAD_REQUEST,
            "unsupported response_type or code_challenge_method",
        )
            .into_response();
    }

    let sync_user =
        match lib_database::SyncUser::find_by_username(&form.username, &state.pool).await {
            Ok(Some(sync_user)) => sync_user,
            Ok(None) => {
                return (StatusCode::UNAUTHORIZED, "invalid username or password").into_response();
            }
            Err(e) => return Error::from(e).into_response(),
        };

    let password = SecretString::from(form.password.clone());
    if !password::verify_password(&password, &sync_user.password_hash) {
        return (StatusCode::UNAUTHORIZED, "invalid username or password").into_response();
    }

    let code = state.codes.issue(
        params.code_challenge.clone(),
        params.redirect_uri.clone(),
        sync_user.id,
    );

    let mut location = format!("{}?code={code}", params.redirect_uri);
    if let Some(state_param) = &params.state {
        location.push_str(&format!("&state={}", percent_encode(state_param)));
    }

    Redirect::to(&location).into_response()
}

/// `POST /token` request body -- both grant types flattened onto one form (the fields
/// each grant type doesn't use are simply absent), since `serde_urlencoded` doesn't
/// support internally-tagged enums over flat form data.
#[derive(Debug, serde::Deserialize)]
pub struct TokenForm {
    pub grant_type: String,
    pub code: Option<String>,
    pub code_verifier: Option<String>,
    pub redirect_uri: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}

/// `POST /token` -- redeem an authorization code (RFC 7636 §4.6's PKCE check) or
/// rotate a refresh token (ADR-0010: each redemption both mints a new access token
/// and invalidates/replaces the refresh token, so a reused old one is detectable).
#[tracing::instrument(name = "POST /token", level = "debug", skip(state, form), fields(grant_type = %form.grant_type))]
async fn post_token(
    State(state): State<AuthState>,
    Form(form): Form<TokenForm>,
) -> axum::response::Response {
    let sync_user_id = match form.grant_type.as_str() {
        "authorization_code" => redeem_authorization_code(&state, &form),
        "refresh_token" => redeem_refresh_token(&state, &form).await,
        _ => return (StatusCode::BAD_REQUEST, "unsupported grant_type").into_response(),
    };

    match sync_user_id {
        Ok(sync_user_id) => match issue_token_pair(&state, sync_user_id).await {
            Ok(token_pair) => Json(token_pair).into_response(),
            Err(e) => e.into_response(),
        },
        Err(e) => e.into_response(),
    }
}

fn redeem_authorization_code(state: &AuthState, form: &TokenForm) -> Result<lib_core::RowID> {
    let code = form.code.as_deref().ok_or(Error::MissingField("code"))?;
    let code_verifier = form
        .code_verifier
        .as_deref()
        .ok_or(Error::MissingField("code_verifier"))?;
    let redirect_uri = form
        .redirect_uri
        .as_deref()
        .ok_or(Error::MissingField("redirect_uri"))?;

    let entry = state.codes.redeem(code).ok_or(Error::InvalidCode)?;

    if entry.redirect_uri != redirect_uri {
        return Err(Error::RedirectMismatch);
    }

    if !pkce::verify(code_verifier, &entry.code_challenge) {
        return Err(Error::PkceMismatch);
    }

    Ok(entry.sync_user_id)
}

async fn redeem_refresh_token(state: &AuthState, form: &TokenForm) -> Result<lib_core::RowID> {
    let refresh_token = form
        .refresh_token
        .as_deref()
        .ok_or(Error::MissingField("refresh_token"))?;

    let sync_user = lib_database::SyncUser::find_only(&state.pool)
        .await?
        .ok_or(Error::NoSyncUser)?;

    let presented_hash = hash_token(refresh_token);
    if sync_user.refresh_token_hash.as_deref() != Some(presented_hash.as_str()) {
        return Err(Error::RefreshTokenRejected);
    }

    Ok(sync_user.id)
}

/// Mint an access token and rotate the refresh token, so the old one stops redeeming.
async fn issue_token_pair(
    state: &AuthState,
    sync_user_id: lib_core::RowID,
) -> Result<TokenResponse> {
    let access_token = jwt::issue_access_token(sync_user_id, &state.signing_key)?;

    let refresh_token = random_token();
    let refresh_token_hash = hash_token(&refresh_token);
    lib_database::SyncUser::update_refresh_token_hash(
        sync_user_id,
        Some(&refresh_token_hash),
        &state.pool,
    )
    .await?;

    Ok(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in: jwt::ACCESS_TOKEN_TTL_SECONDS,
    })
}
