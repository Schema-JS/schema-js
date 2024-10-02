use crate::define_sjs_grpc_service;
use crate::services::query::insert::insert_service::{
    InsertRowsRequest, InsertRowsResponse, RowInsert,
};
use crate::services::query::insert::shared::data_value::ValueType;
use schemajs_internal::auth::types::UserContext;
use schemajs_primitives::column::types::DataValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub mod shared {
    tonic::include_proto!("sjs.shared");
}
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
        let db_manager = engine
            .read()
            .map_err(|e| Status::internal("SJS not available"))?;
        let user = user_context.get_user();
        let db = match db_manager.find_by_name_ref(&user.scheme) {
            Some(db) => db,
            None => return Err(Status::not_found("Database not found")),
        };

        let mut new_rows: Vec<HashMap<String, DataValue>> = vec![];

        for row in rows {
            let mut hrow: HashMap<String, DataValue> = HashMap::new();

            for cols in row.row_values.iter() {
                let val = cols
                    .1
                    .clone()
                    .value_type
                    .unwrap_or_else(|| ValueType::NullValue(true));
                let value_type = match val {
                    ValueType::NullValue(_) => DataValue::Null,
                    ValueType::UuidValue(u) => DataValue::Uuid(Uuid::from_str(&u).unwrap()),
                    ValueType::StringValue(s) => DataValue::String(s),
                    ValueType::BoolValue(b) => DataValue::Boolean(b),
                    ValueType::NumberValue(n) => {
                        DataValue::Number(serde_json::value::Number::from_f64(n as f64).unwrap())
                    }
                };

                hrow.insert(cols.0.clone(), value_type);
            }

            new_rows.push(hrow);
        }

        Ok(true)
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

        let err = self.insert_rows_into_db(ctx.clone(), request.into_inner().rows)?;

        if err {
            Err(Status::aborted("There was an issue inserting rows"))
        } else {
            Ok(Response::new(InsertRowsResponse {
                success: true,
                message: String::from("success"),
            }))
        }
    }
}
