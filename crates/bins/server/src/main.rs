#![allow(dead_code)]
#![allow(unused_imports)]

use tonic::{transport::Server, Request, Response, Status};

use clap::{Arg, command};

use lib_rpc::{UtilitiesService, UtilitiesServiceServer, PingRequest, PingResponse};
use lib_telemetry as telemetry;
use lib_config as config;

#[derive(Default)]
pub struct MyUtilitiesService {}

#[tonic::async_trait]
impl UtilitiesService for MyUtilitiesService {
    async fn ping(
        &self,
        request: Request<PingRequest>,
    ) -> Result<Response<PingResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let reply: PingResponse = PingResponse {
            message: "Pong...".to_string(),
        };

        Ok(Response::new(reply)) // Send back ping response
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let config = config::LedgerConfig::parse(None)?;

    let telemetry_level = Some(&config.telemetry_config().telemetry_level());
    telemetry::init(telemetry_level)?;
    tracing::info!("Starting server with config: {:#?}", config);

    // let matched_results = command!().arg(
    //     Arg::new("firstname")
    // ).get_matches();

    // let addr = "0.0.0.0:50051".parse().unwrap();
    // let utility_server = MyUtilitiesService::default();

    // let tracing_level = Some(telemetry::TelemetryLevels::DEBUG);
    // telemetry::init(tracing_level.as_ref())?;

    // tracing::info!("UtilitiesServiceServer listening on {addr}");

    // Server::builder()
    //     .add_service(UtilitiesServiceServer::new(utility_server))
    //     .serve(addr)
    //     .await?;

    Ok(())
}