pub mod connection_service {
    tonic::include_proto!("sjs.connection");
}

use crate::{define_sjs_grpc_service, GrpcResponse};
use connection_service::{CheckConnectionRequest, CheckConnectionResponse};
use schemajs_internal::auth::types::VerifyUserArgs;
use tonic::Response;

define_sjs_grpc_service!(ConnectionService);

#[tonic::async_trait]
impl connection_service::proto_connection_service_server::ProtoConnectionService
    for ConnectionService
{
    async fn check_connection(
        &self,
        request: tonic::Request<CheckConnectionRequest>,
    ) -> GrpcResponse<CheckConnectionResponse> {
        let inner_req = request.into_inner();
        let auth_manager = self.db_manager.auth_manager();
        let valid_user = auth_manager.authenticate(VerifyUserArgs {
            scheme_name: inner_req.database,
            identifier: inner_req.username,
            password: inner_req.password,
        });

        if let Ok(token) = valid_user {
            Ok(Response::new(CheckConnectionResponse {
                is_connected: true,
                token: Some(token.to_string()),
            }))
        } else {
            Ok(Response::new(CheckConnectionResponse {
                is_connected: false,
                token: None,
            }))
        }
    }
}
