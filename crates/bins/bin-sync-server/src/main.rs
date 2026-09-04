mod auth;

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use secrecy::SecretString;
use tonic::service::Routes;

use lib_config as config;
use lib_rpc::{SyncService, SyncServiceServer, UtilitiesService, UtilitiesServiceServer};
use lib_telemetry as telemetry;

/// Bootstrap account credentials for this feasibility cycle -- ADR-0010 fixes the
/// Sync Server's user store at exactly one account; real credential provisioning is
/// deliberately deferred, the same "config surface still undecided" scope call this
/// file already made for the bind address.
const BOOTSTRAP_USERNAME: &str = "admin";
const BOOTSTRAP_PASSWORD: &str = "change-me";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::LedgerConfig::parse(None)?;

    let telemetry_level = Some(&config.telemetry_config().telemetry_level());
    telemetry::init(telemetry_level)?;
    tracing::info!("Starting Sync Server with config: {:#?}", config);

    // The Sync Server's own durable Change Set log (ADR-0009) -- reuses lib-database's
    // conventions and migrations directly, the same `[database]` config section a Client
    // uses for its local Ledger copy, just pointed at the Sync Server's own SQLite file.
    let database_connection =
        lib_database::DatabaseConnection::new(config.database_config().clone()).await?;
    let database_pool = database_connection.into_pool();
    sqlx::migrate!("../../libs/lib-database/migrations")
        .run(&database_pool)
        .await?;
    let database_pool = Arc::new(database_pool);

    bootstrap_account(&database_pool).await?;

    // The JWT signing key is ephemeral, generated fresh in memory on every start (not
    // config-driven this cycle -- see ADR-0010/this ticket's Non-goals). Access tokens
    // are short-lived by design; the durable "stay logged in" property comes from the
    // refresh token, which is persisted in `accounts.refresh_token_hash`, not this key.
    let signing_key_material = generate_signing_key();
    let interceptor_key = SecretString::from(signing_key_material.clone());
    let auth_state = auth::AuthState {
        pool: database_pool.clone(),
        codes: Arc::new(auth::CodeStore::new()),
        signing_key: SecretString::from(signing_key_material),
    };

    // Placeholder bind address -- the Sync Server's own config surface (host/port, TLS) is
    // still undecided.
    let addr: std::net::SocketAddr = "0.0.0.0:50051".parse()?;

    // `SyncService` (Push/Pull) is behind the auth interceptor -- FC-SYNC-007 protects
    // the sync endpoints; `UtilitiesService`/Ping stays open as a basic liveness check.
    let grpc_routes = Routes::new(UtilitiesServiceServer::new(UtilitiesService::default()))
        .add_service(SyncServiceServer::with_interceptor(
            SyncService::new(database_pool),
            auth::interceptor::AuthInterceptor::new(interceptor_key),
        ));

    // ADR-0010's "one listener, not two": merge the gRPC routes and the `/authorize`
    // + `/token` HTTP surface into a single axum `Router`, served from one Hyper
    // listener -- no second exposed port in the Docker image or Compose file.
    let router = grpc_routes
        .into_axum_router()
        .merge(auth::routes(auth_state));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Sync Server (gRPC + auth HTTP) listening on {addr}");

    axum::serve(listener, router).await?;

    Ok(())
}

/// Generate a fresh random JWT signing key, base64url-encoded for use as a `SecretString`.
fn generate_signing_key() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Create the single bootstrap account on first run, if the `accounts` table is empty.
async fn bootstrap_account(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    if lib_database::Account::find_only(pool).await?.is_some() {
        return Ok(());
    }

    tracing::warn!(
        "No account provisioned yet -- bootstrapping the demo account '{BOOTSTRAP_USERNAME}'. \
         This is a fixed feasibility-cycle default, not a real credential -- see main.rs's \
         bootstrap constants."
    );

    let password_hash = auth::hash_password(&SecretString::from(BOOTSTRAP_PASSWORD.to_string()))?;
    let account = lib_database::Account {
        id: lib_core::RowID::new(),
        username: BOOTSTRAP_USERNAME.to_string(),
        password_hash,
        refresh_token_hash: None,
        created_on: chrono::Utc::now(),
        updated_on: chrono::Utc::now(),
    };
    account.insert(pool).await?;

    Ok(())
}
