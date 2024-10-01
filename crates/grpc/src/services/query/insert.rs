use crate::define_sjs_grpc_service;
use crate::services::query::insert::insert_service::{InsertRowsRequest, InsertRowsResponse};
use schemajs_internal::auth::types::UserContext;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
        let ctx = request.extensions().get::<Arc<UserContext>>();
        if let Some(ctx) = ctx {
            let engine = self.db_manager.engine();
            let db_manager = engine.read().unwrap();
            let user = ctx.get_user();
            let db = db_manager.find_by_name_ref(&user.scheme);

            let mut err = false;

            if let Some(db) = db {
                for row in request.into_inner().rows {
                    let i = db
                        .query_manager
                        .insert_serializable(row.table_name.as_str(), row.row_values);
                    if i.is_err() {
                        err = true;
                        break;
                    }
                }
            }

            if err {
                Err(Status::aborted("There was an issue inserting rows"))
            } else {
                Ok(Response::new(InsertRowsResponse {
                    success: true,
                    message: String::from("success"),
                }))
            }
        } else {
            Err(Status::unauthenticated("Invalid session"))
        }
    }
}
