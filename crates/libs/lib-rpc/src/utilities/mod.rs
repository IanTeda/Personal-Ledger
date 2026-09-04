pub (crate) mod proto;
mod service;

pub use service::UtilitiesService;
pub use proto::{UtilitiesServiceClient, UtilitiesServiceServer, PingRequest, PingResponse};