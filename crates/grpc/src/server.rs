use crate::services::connection::connection_service::proto_connection_service_server::ProtoConnectionServiceServer;
use crate::services::connection::ConnectionService;
use schemajs_internal::manager::InternalManager;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tonic::transport::Server;

pub struct GrpcServer {
    db_manager: Arc<InternalManager>,
    ip: SocketAddr,
}

pub struct GrpcServerArgs {
    pub db_manager: Arc<InternalManager>,
    pub ip: Option<String>,
}

impl GrpcServer {
    pub fn new(args: GrpcServerArgs) -> Self {
        Self {
            db_manager: args.db_manager,
            ip: args
                .ip
                .unwrap_or_else(|| String::from("[::1]:34244"))
                .parse()
                .unwrap(),
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting GRPC at {}", self.ip);
        let _ = Server::builder()
            .add_service(ProtoConnectionServiceServer::new(ConnectionService::new(
                self.db_manager.clone(),
            )))
            .serve(self.ip.clone())
            .await?;

        Ok(())
    }
}
