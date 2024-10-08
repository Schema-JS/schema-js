use crate::define_sjs_grpc_service;
use crate::services::query::insert::insert_service::{
    InsertRowsRequest, InsertRowsResponse, RowInsert,
};
use crate::services::shared::shared;
use crate::services::shared::shared::data_value::ValueType;
use crate::utils::common::convert_to_data_value;
use schemajs_internal::auth::types::UserContext;
use schemajs_primitives::column::types::DataValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub mod insert_service {
    tonic::include_proto!("sjs.query");
}

define_sjs_grpc_service!(InsertService, {
    pub fn insert_rows_into_db(
        &self,
        user_context: Arc<UserContext>,
        rows: Vec<RowInsert>,
    ) -> Result<bool, Status> {
        let engine = self.db_manager.engine();
        let db_manager = engine.read();
        let user = user_context.get_user();
        let db = match db_manager.find_by_name_ref(&user.scheme) {
            Some(db) => db,
            None => return Err(Status::not_found("Database not found")),
        };

        let new_rows: Vec<(String, HashMap<String, DataValue>)> = rows
            .into_iter()
            .map(|row| {
                let hrow: HashMap<String, DataValue> = row
                    .row_values
                    .into_iter()
                    .map(|(col_name, col_val)| {
                        let value = match col_val.value_type {
                            Some(vt) => convert_to_data_value(vt),
                            None => DataValue::Null, // Handle the case where value_type is None
                        };
                        (col_name, value)
                    })
                    .collect();
                (row.table_name, hrow)
            })
            .collect();

        let insert = db.query_manager.insert_from_value_map(new_rows, false);

        Ok(insert.is_ok())
    }
});

#[tonic::async_trait]
impl insert_service::proto_row_insert_service_server::ProtoRowInsertService for InsertService {
    async fn insert_rows(
        &self,
        request: Request<InsertRowsRequest>,
    ) -> Result<Response<InsertRowsResponse>, Status> {
        let ctx = match request.extensions().get::<Arc<UserContext>>() {
            Some(ctx) => ctx,
            None => return Err(Status::unauthenticated("Invalid session")),
        };

        let inserted = self.insert_rows_into_db(ctx.clone(), request.into_inner().rows)?;

        if !inserted {
            Err(Status::aborted("There was an issue inserting rows"))
        } else {
            Ok(Response::new(InsertRowsResponse {
                success: true,
                message: String::from("success"),
            }))
        }
    }
}
