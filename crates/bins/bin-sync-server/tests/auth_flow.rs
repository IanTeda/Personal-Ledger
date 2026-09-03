//! # Auth Flow Demo (FC-SYNC-007)
//!
//! Proves ADR-0010's OAuth2 Authorization Code + PKCE flow actually protects the Sync
//! Server's `SyncService` endpoints end-to-end: an unauthenticated call is rejected, a
//! Client can complete the browser-hop-and-loopback-redirect dance to obtain a bearer
//! token, that token authorises `SyncService`, PKCE is actually enforced (not just
//! present), the token endpoint checks `redirect_uri` matches the authorization
//! request, and refresh tokens rotate (a used-and-replaced one is rejected).
//!
//! Scoped to this ticket's Non-goals: no real system browser is launched and no OS
//! keychain is touched -- this test plays the "browser after a human filled the login
//! form" role with direct HTTP calls via `reqwest`, and captures the loopback redirect
//! with a real (but throwaway) `axum` listener, exactly the RFC 8252 mechanic a real
//! native-app Client uses -- see `docs/adr/0010-oauth2-pkce-native-app-auth.md`.

use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lib_database::{Account, DatabaseConfig, DatabaseConnection};
use lib_rpc::{
    PullRequest, SyncService, SyncServiceClient, SyncServiceServer, UtilitiesService,
    UtilitiesServiceServer,
};
use secrecy::SecretString;
use sha2::{Digest, Sha256};

const TEST_USERNAME: &str = "auth-flow-test-user";
const TEST_PASSWORD: &str = "correct horse battery staple";

/// Start the merged gRPC + auth HTTP Sync Server (mirroring `main.rs`'s "one
/// listener, not two" shape) bound to an ephemeral port, with one bootstrap account
/// seeded.
async fn spawn_sync_server(db_path: &std::path::Path) -> std::net::SocketAddr {
    let connection = DatabaseConnection::new(DatabaseConfig {
        url: format!("sqlite://{}?mode=rwc", db_path.display()),
        ..DatabaseConfig::default()
    })
    .await
    .expect("Sync Server database connection should establish");
    let pool = Arc::new(connection.into_pool());
    sqlx::migrate!("../../libs/lib-database/migrations")
        .run(&*pool)
        .await
        .expect("Sync Server migrations should apply");

    let password_hash =
        bin_sync_server::auth::hash_password(&SecretString::from(TEST_PASSWORD.to_string()))
            .expect("test password should hash");
    let account = Account {
        id: lib_domain::RowID::new(),
        username: TEST_USERNAME.to_string(),
        password_hash,
        refresh_token_hash: None,
        created_on: chrono::Utc::now(),
        updated_on: chrono::Utc::now(),
    };
    account
        .insert(&pool)
        .await
        .expect("bootstrap account should insert");

    let signing_key_material = "test-only-signing-key-not-for-production".to_string();
    let interceptor_key = SecretString::from(signing_key_material.clone());
    let auth_state = bin_sync_server::auth::AuthState {
        pool: pool.clone(),
        codes: Arc::new(bin_sync_server::auth::CodeStore::new()),
        signing_key: SecretString::from(signing_key_material),
    };

    let sync_service = SyncServiceServer::with_interceptor(
        SyncService::new(pool),
        bin_sync_server::auth::interceptor::AuthInterceptor::new(interceptor_key),
    );
    let grpc_routes =
        tonic::service::Routes::new(UtilitiesServiceServer::new(UtilitiesService::default()))
            .add_service(sync_service);
    let router = grpc_routes
        .into_axum_router()
        .merge(bin_sync_server::auth::routes(auth_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("should bind an ephemeral port");
    let addr = listener
        .local_addr()
        .expect("bound listener has a local address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("Sync Server should serve without error");
    });

    addr
}

fn code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Percent-encode a string for safe inclusion in a URL query component.
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

type CallbackSender =
    Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<HashMap<String, String>>>>>;

/// Captures the loopback redirect's query params and hands them back over the
/// channel -- named (rather than an inline closure) so axum's `Handler` trait
/// resolves cleanly against a plain `State` + `Query` extractor pair.
async fn callback_handler(
    axum::extract::State(tx): axum::extract::State<CallbackSender>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> &'static str {
    if let Some(tx) = tx.lock().unwrap().take() {
        let _ = tx.send(params);
    }
    "you can close this window"
}

/// Bind a throwaway loopback listener and capture the first `GET /callback?...` it
/// receives -- the real RFC 8252 mechanic, not a stand-in for it. Returns the bound
/// `redirect_uri` and a receiver resolving to the captured query params.
async fn spawn_loopback_callback() -> (
    String,
    tokio::sync::oneshot::Receiver<HashMap<String, String>>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx: CallbackSender = Arc::new(std::sync::Mutex::new(Some(tx)));

    let router = axum::Router::new()
        .route("/callback", axum::routing::get(callback_handler))
        .with_state(tx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("should bind an ephemeral loopback port");
    let addr = listener
        .local_addr()
        .expect("bound listener has a local address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("loopback listener should serve");
    });

    (format!("http://127.0.0.1:{}/callback", addr.port()), rx)
}

/// The outcome of a full browser-hop authorization request: the code the Client can
/// now redeem, and the `redirect_uri`/`code_verifier` it needs to redeem it with.
struct AuthorizeOutcome {
    code: String,
    redirect_uri: String,
    code_verifier: String,
}

/// Drive the browser-hop half of the flow: GET the login form, POST the credentials,
/// follow the resulting redirect into a real loopback listener, and return the
/// captured authorization code plus what's needed to redeem it.
async fn complete_authorize(
    http: &reqwest::Client,
    server_addr: std::net::SocketAddr,
    code_verifier: &str,
) -> AuthorizeOutcome {
    let (redirect_uri, callback_rx) = spawn_loopback_callback().await;
    let challenge = code_challenge(code_verifier);

    let authorize_url = format!(
        "http://{server_addr}/authorize?response_type=code&code_challenge={challenge}&code_challenge_method=S256&redirect_uri={}&state=xyz",
        percent_encode(&redirect_uri)
    );

    let get_response = http
        .get(&authorize_url)
        .send()
        .await
        .expect("GET /authorize should succeed");
    assert_eq!(get_response.status(), reqwest::StatusCode::OK);
    let body = get_response
        .text()
        .await
        .expect("should read login form body");
    assert!(
        body.contains("<form"),
        "GET /authorize should render a login form"
    );

    let post_response = http
        .post(&authorize_url)
        .form(&[("username", TEST_USERNAME), ("password", TEST_PASSWORD)])
        .send()
        .await
        .expect("POST /authorize should succeed");
    // axum's `Redirect::to` issues 303 See Other (not 302 Found) -- functionally
    // equivalent here (the loopback listener's GET follows it the same way).
    assert_eq!(post_response.status(), reqwest::StatusCode::SEE_OTHER);
    let location = post_response
        .headers()
        .get(reqwest::header::LOCATION)
        .expect("POST /authorize should redirect")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        location.starts_with(&redirect_uri),
        "should redirect back to the Client's loopback redirect_uri"
    );

    // Actually request the redirect Location, so the loopback listener receives it
    // for real -- the same thing the system browser would do.
    http.get(&location)
        .send()
        .await
        .expect("loopback GET should succeed");

    let params = callback_rx
        .await
        .expect("loopback listener should have captured the callback");
    let code = params
        .get("code")
        .expect("callback should carry a code")
        .clone();

    AuthorizeOutcome {
        code,
        redirect_uri,
        code_verifier: code_verifier.to_string(),
    }
}

fn bearer_request<T>(message: T, access_token: &str) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    let value = format!("Bearer {access_token}")
        .parse()
        .expect("bearer header value should be valid ASCII");
    request.metadata_mut().insert("authorization", value);
    request
}

#[tokio::test]
async fn auth_flow_protects_sync_service_end_to_end() {
    let temp_dir = tempfile::tempdir().expect("should create a scratch tempdir");
    let server_addr = spawn_sync_server(&temp_dir.path().join("sync-server.db")).await;
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("should build an http client");

    let mut grpc_client = SyncServiceClient::connect(format!("http://{server_addr}"))
        .await
        .expect("should connect to the running Sync Server");

    // -- 1. Unauthenticated call rejected --
    let unauthenticated = grpc_client
        .pull(PullRequest {
            since_id: None,
            limit: 10,
        })
        .await;
    assert_eq!(
        unauthenticated.unwrap_err().code(),
        tonic::Code::Unauthenticated,
        "SyncService should reject a call with no bearer token"
    );

    // -- 2. redirect_uri mismatch is rejected at /token --
    let mismatch =
        complete_authorize(&http, server_addr, "verifier-for-redirect-mismatch-check").await;
    let mismatch_response = http
        .post(format!("http://{server_addr}/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &mismatch.code),
            ("code_verifier", &mismatch.code_verifier),
            (
                "redirect_uri",
                "http://127.0.0.1:1/not-the-real-redirect-uri",
            ),
        ])
        .send()
        .await
        .expect("POST /token should succeed as an HTTP call even on a rejected grant");
    assert_eq!(
        mismatch_response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "a redirect_uri that doesn't match the authorization request must be rejected"
    );

    // -- 3. Negative PKCE case: wrong verifier is rejected --
    let pkce_check = complete_authorize(&http, server_addr, "the-real-verifier").await;
    let wrong_verifier_response = http
        .post(format!("http://{server_addr}/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &pkce_check.code),
            (
                "code_verifier",
                "a-different-verifier-not-the-one-that-was-used",
            ),
            ("redirect_uri", &pkce_check.redirect_uri),
        ])
        .send()
        .await
        .expect("POST /token should succeed as an HTTP call even on a rejected grant");
    assert_eq!(
        wrong_verifier_response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "PKCE must actually be enforced, not just present"
    );

    // -- 4. Real end-to-end exchange --
    let real = complete_authorize(&http, server_addr, "the-real-final-verifier").await;
    let token_json: serde_json::Value = http
        .post(format!("http://{server_addr}/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &real.code),
            ("code_verifier", &real.code_verifier),
            ("redirect_uri", &real.redirect_uri),
        ])
        .send()
        .await
        .expect("POST /token should succeed")
        .json()
        .await
        .expect("token response should be JSON");

    let access_token = token_json["access_token"].as_str().unwrap().to_string();
    let refresh_token = token_json["refresh_token"].as_str().unwrap().to_string();

    // -- 5. Authenticated call succeeds --
    let authenticated = grpc_client
        .pull(bearer_request(
            PullRequest {
                since_id: None,
                limit: 10,
            },
            &access_token,
        ))
        .await;
    assert!(
        authenticated.is_ok(),
        "a valid access token should authorise SyncService"
    );

    // -- 6. Refresh rotates: old refresh token stops working, new one works --
    let refreshed_json: serde_json::Value = http
        .post(format!("http://{server_addr}/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ])
        .send()
        .await
        .expect("refresh should succeed")
        .json()
        .await
        .expect("refresh response should be JSON");
    let new_access_token = refreshed_json["access_token"].as_str().unwrap().to_string();
    let new_refresh_token = refreshed_json["refresh_token"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(
        new_refresh_token, refresh_token,
        "refresh should rotate to a new token"
    );

    let old_token_retry = http
        .post(format!("http://{server_addr}/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ])
        .send()
        .await
        .expect("retry should still be a valid HTTP call");
    assert_eq!(
        old_token_retry.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "a rotated-out refresh token must not be reusable"
    );

    let authenticated_with_new_token = grpc_client
        .pull(bearer_request(
            PullRequest {
                since_id: None,
                limit: 10,
            },
            &new_access_token,
        ))
        .await;
    assert!(
        authenticated_with_new_token.is_ok(),
        "the rotated access token should still authorise SyncService"
    );
}
