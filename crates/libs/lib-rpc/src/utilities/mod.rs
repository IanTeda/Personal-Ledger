pub (crate) mod proto;
mod service;
mod client;

pub use service::UtilitiesService;
pub use proto::{UtilitiesServiceClient, UtilitiesServiceServer, PingRequest, PingResponse};