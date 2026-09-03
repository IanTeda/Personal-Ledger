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

    let account = match lib_database::Account::find_by_username(&form.username, &state.pool).await {
        Ok(Some(account)) => account,
        Ok(None) => {
            return (StatusCode::UNAUTHORIZED, "invalid username or password").into_response();
        }
        Err(e) => {
            tracing::error!("Failed to look up account during login: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    let password = SecretString::from(form.password.clone());
    if !password::verify_password(&password, &account.password_hash) {
        return (StatusCode::UNAUTHORIZED, "invalid username or password").into_response();
    }

    let code = state.codes.issue(
        params.code_challenge.clone(),
        params.redirect_uri.clone(),
        account.id,
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
) -> impl IntoResponse {
    let account_id = match form.grant_type.as_str() {
        "authorization_code" => match redeem_authorization_code(&state, &form) {
            Ok(account_id) => account_id,
            Err(response) => return response.into_response(),
        },
        "refresh_token" => match redeem_refresh_token(&state, &form).await {
            Ok(account_id) => account_id,
            Err(response) => return response.into_response(),
        },
        _ => return (StatusCode::BAD_REQUEST, "unsupported grant_type").into_response(),
    };

    issue_token_pair(&state, account_id).await
}

/// A small `Copy`-able error response, so the `Result::Err` variant these helpers
/// return stays cheap instead of embedding a full `axum::response::Response`
/// (clippy's `result_large_err`).
type TokenError = (StatusCode, &'static str);

fn redeem_authorization_code(
    state: &AuthState,
    form: &TokenForm,
) -> Result<lib_domain::RowID, TokenError> {
    let code = form
        .code
        .as_deref()
        .ok_or((StatusCode::BAD_REQUEST, "missing code"))?;
    let code_verifier = form
        .code_verifier
        .as_deref()
        .ok_or((StatusCode::BAD_REQUEST, "missing code_verifier"))?;
    let redirect_uri = form
        .redirect_uri
        .as_deref()
        .ok_or((StatusCode::BAD_REQUEST, "missing redirect_uri"))?;

    let entry = state
        .codes
        .redeem(code)
        .ok_or((StatusCode::BAD_REQUEST, "invalid or expired code"))?;

    if entry.redirect_uri != redirect_uri {
        return Err((
            StatusCode::BAD_REQUEST,
            "redirect_uri does not match the authorization request",
        ));
    }

    if !pkce::verify(code_verifier, &entry.code_challenge) {
        return Err((
            StatusCode::BAD_REQUEST,
            "code_verifier does not match code_challenge",
        ));
    }

    Ok(entry.account_id)
}

async fn redeem_refresh_token(
    state: &AuthState,
    form: &TokenForm,
) -> Result<lib_domain::RowID, TokenError> {
    let refresh_token = form
        .refresh_token
        .as_deref()
        .ok_or((StatusCode::BAD_REQUEST, "missing refresh_token"))?;

    let account = lib_database::Account::find_only(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to look up account during refresh: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal error")
        })?
        .ok_or((StatusCode::UNAUTHORIZED, "no account provisioned"))?;

    let presented_hash = hash_token(refresh_token);
    if account.refresh_token_hash.as_deref() != Some(presented_hash.as_str()) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid or already-rotated refresh token",
        ));
    }

    Ok(account.id)
}

async fn issue_token_pair(
    state: &AuthState,
    account_id: lib_domain::RowID,
) -> axum::response::Response {
    let access_token = match jwt::issue_access_token(account_id, &state.signing_key) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to issue access token: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    let refresh_token = random_token();
    let refresh_token_hash = hash_token(&refresh_token);
    if let Err(e) = lib_database::Account::update_refresh_token_hash(
        account_id,
        Some(&refresh_token_hash),
        &state.pool,
    )
    .await
    {
        tracing::error!("Failed to persist rotated refresh token: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
    }

    Json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in: jwt::ACCESS_TOKEN_TTL_SECONDS,
    })
    .into_response()
}
