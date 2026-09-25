pub(crate) mod proto;
mod service;

pub use proto::{PingRequest, PingResponse, UtilitiesServiceClient, UtilitiesServiceServer};
pub use service::UtilitiesService;
