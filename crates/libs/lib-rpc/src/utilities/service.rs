use crate::utilities::proto;

#[derive(Default)]
pub struct UtilitiesService {}

#[tonic::async_trait]
impl proto::UtilitiesService for UtilitiesService {
    async fn ping(
        &self,
        request: tonic::Request<proto::PingRequest>,
    ) -> Result<tonic::Response<proto::PingResponse>, tonic::Status> {
        tracing::debug!("Got a request from {:?}", request.remote_addr());

        let reply: proto::PingResponse = proto::PingResponse {
            message: "Pong...".to_string(),
        };

        Ok(tonic::Response::new(reply)) // Send back ping response
    }
}