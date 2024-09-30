use crate::define_sjs_grpc_service;
use crate::services::query::insert::insert_service::{InsertRowsRequest, InsertRowsResponse};
use tonic::{Request, Response, Status};

pub mod shared {
    tonic::include_proto!("sjs.shared");
}
pub mod insert_service {
    tonic::include_proto!("sjs.query");
}

define_sjs_grpc_service!(InsertService);

#[tonic::async_trait]
impl insert_service::proto_row_insert_service_server::ProtoRowInsertService for InsertService {
    async fn insert_rows(
        &self,
        request: Request<InsertRowsRequest>,
    ) -> Result<Response<InsertRowsResponse>, Status> {
        todo!()
    }
}
