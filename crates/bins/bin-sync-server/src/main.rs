use tonic::transport::Server;

use lib_rpc::{UtilitiesService, UtilitiesServiceServer};
use lib_telemetry as telemetry;
use lib_config as config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::LedgerConfig::parse(None)?;

    let telemetry_level = Some(&config.telemetry_config().telemetry_level());
    telemetry::init(telemetry_level)?;
    tracing::info!("Starting Sync Server with config: {:#?}", config);

    // Placeholder bind address -- the Sync Server's own config surface (host/port,
    // TLS, Change Set store location) is undecided until the reconciliation-approach
    // and auth-mechanism ADRs land (see issues #44/#45); this is scaffolding only.
    let addr = "0.0.0.0:50051".parse()?;
    let utility_server = UtilitiesService::default();

    tracing::info!("UtilitiesServiceServer listening on {addr}");

    Server::builder()
        .add_service(UtilitiesServiceServer::new(utility_server))
        .serve(addr)
        .await?;

    Ok(())
}