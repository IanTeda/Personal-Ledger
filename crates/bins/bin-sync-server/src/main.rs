use std::sync::Arc;

use tonic::transport::Server;

use lib_config as config;
use lib_rpc::{SyncService, SyncServiceServer, UtilitiesService, UtilitiesServiceServer};
use lib_telemetry as telemetry;

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

    // Placeholder bind address -- the Sync Server's own config surface (host/port, TLS) is
    // still undecided; auth (ADR-0010) is separate work (issue #50).
    let addr = "0.0.0.0:50051".parse()?;
    let utility_server = UtilitiesService::default();
    let sync_server = SyncService::new(database_pool);

    tracing::info!("UtilitiesServiceServer and SyncServiceServer listening on {addr}");

    Server::builder()
        .add_service(UtilitiesServiceServer::new(utility_server))
        .add_service(SyncServiceServer::new(sync_server))
        .serve(addr)
        .await?;

    Ok(())
}
