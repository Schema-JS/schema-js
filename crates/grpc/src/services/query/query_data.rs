use crate::define_sjs_grpc_service;
use crate::services::query::query_data::query_service::{
    DataMap, QueryDataRequest, QueryOps as GrpcQueryOps, QueryResponse,
};
use crate::services::shared::shared;
use crate::services::shared::shared::data_value::ValueType;
use crate::services::shared::shared::DataValue as GrpcDataValue;
use crate::utils::common::{convert_to_grpc_value, find_database, from_grpc_ops_to_sjs_ops};
use schemajs_internal::auth::types::UserContext;
use schemajs_primitives::column::types::DataValue;
use schemajs_query::row::Row;
use std::collections::HashMap;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub mod query_service {
    tonic::include_proto!("sjs.query");
}

define_sjs_grpc_service!(QueryService, {
    pub fn query_rows_from_db(
        &self,
        user_context: Arc<UserContext>,
        table_name: String,
        operation: Option<GrpcQueryOps>,
    ) -> Result<Vec<DataMap>, Status> {
        let db = find_database(&self.db_manager, user_context)?;
        if let Some(op) = operation {
            let query_ops = from_grpc_ops_to_sjs_ops(op);
            if let Ok(qops) = query_ops {
                let rows = db
                    .query_manager
                    .search_manager
                    .search(&table_name, &qops)
                    .map_err(|e| Status::internal("Query could not be completed"))?;
                // Refactor closure to handle errors
                let map_rows: Vec<HashMap<String, GrpcDataValue>> = rows
                    .into_iter()
                    .filter_map(|r| {
                        match r.to_map() {
                            Ok(val) => Some(
                                val.iter()
                                    .map(|(col, val)| {
                                        let grpc_val = convert_to_grpc_value(val);

                                        let data_val = GrpcDataValue {
                                            value_type: Some(grpc_val),
                                        };

                                        (col.clone(), data_val)
                                    })
                                    .collect::<HashMap<String, GrpcDataValue>>(),
                            ),
                            Err(_) => {
                                // Skip this row if it couldn't be deserialized
                                None
                            }
                        }
                    })
                    .collect();

                let map_rows = map_rows
                    .into_iter()
                    .map(|r| DataMap { values: r })
                    .collect();

                return Ok(map_rows);
            }
        }

        Ok(vec![])
    }
});

#[tonic::async_trait]
impl query_service::proto_query_service_server::ProtoQueryService for QueryService {
    async fn query_rows(
        &self,
        request: Request<QueryDataRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let ctx = (match request.extensions().get::<Arc<UserContext>>() {
            Some(ctx) => ctx,
            None => return Err(Status::unauthenticated("Invalid session")),
        })
        .clone();

        let inner = request.into_inner();

        let rows = self.query_rows_from_db(ctx, inner.table_name, inner.query)?;

        Ok(Response::new(QueryResponse { values: rows }))
    }
}
