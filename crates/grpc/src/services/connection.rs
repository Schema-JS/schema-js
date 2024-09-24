pub mod connection_service {
    tonic::include_proto!("sjs.connection");
}

use crate::GrpcResponse;
use connection_service::{CheckConnectionRequest, CheckConnectionResponse};
use tonic::Response;

#[derive(Default)]
pub struct ConnectionService {}

#[tonic::async_trait]
impl connection_service::proto_connection_service_server::ProtoConnectionService
    for ConnectionService
{
    async fn check_connection(
        &self,
        request: tonic::Request<CheckConnectionRequest>,
    ) -> GrpcResponse<CheckConnectionResponse> {
        Ok(Response::new(CheckConnectionResponse {
            is_connected: true,
        }))
    }
}
