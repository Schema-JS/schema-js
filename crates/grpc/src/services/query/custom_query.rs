use crate::define_sjs_grpc_service;
use crate::services::query::custom_query::custom_query_service::{
    CustomQueryRequest, CustomQueryResponse,
};
use crate::services::shared::shared;
use crate::utils::common::find_database;
use crate::utils::json::{serde_json_to_prost, to_prost_struct};
use prost_types::Any;
use schemajs_helpers::helper::HelperCall;
use schemajs_internal::auth::types::UserContext;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tonic::{Request, Response, Status};

pub mod custom_query_service {
    tonic::include_proto!("sjs.query");
}

define_sjs_grpc_service!(CustomQueryService, {
    pub async fn execute_custom_query(
        &self,
        user_context: Arc<UserContext>,
        req: CustomQueryRequest,
    ) -> Result<Value, Status> {
        let db = find_database(&self.db_manager, user_context)?;
        let (helper_response_tx, mut helper_response_rx) = self.create_response_handlers();
        let result = db
            .call_helper(HelperCall::CustomQuery {
                table: req.table_name,
                identifier: req.identifier,
                req: serde_json::from_str(&req.req)
                    .map_err(|_| Status::internal("Invalid Payload"))?,
                response: helper_response_tx,
            })
            .await;

        if result.is_err() {
            return Err(Status::internal("Error executing custom query call"));
        }

        let timeout = {
            let timeout_duration = Duration::from_secs(db.db_config.custom_query_timeout);
            tokio::time::sleep(timeout_duration)
        };

        let resp = select! {
            msg = helper_response_rx.recv() => {
                match msg {
                    None => Ok(Value::Null),
                    Some(val) => Ok(val)
                }
            }
            _ = timeout => Err(())
        };

        resp.map_err(|_| Status::aborted("Custom query timed out"))
    }

    fn create_response_handlers(&self) -> (UnboundedSender<Value>, UnboundedReceiver<Value>) {
        unbounded_channel()
    }
});

#[tonic::async_trait]
impl custom_query_service::proto_custom_query_service_server::ProtoCustomQueryService
    for CustomQueryService
{
    async fn custom_query(
        &self,
        request: Request<CustomQueryRequest>,
    ) -> Result<Response<CustomQueryResponse>, Status> {
        let ctx = match request.extensions().get::<Arc<UserContext>>() {
            Some(ctx) => ctx,
            None => return Err(Status::unauthenticated("Invalid session")),
        };

        let process_custom_query = self
            .execute_custom_query(ctx.clone(), request.into_inner())
            .await;
        match process_custom_query {
            Ok(val) => {
                let prost_val = serde_json_to_prost(val).map_err(|_| {
                    Status::internal("Could not serialize response from custom query")
                })?;
                Ok(Response::new(CustomQueryResponse {
                    value: Some(prost_val),
                }))
            }
            Err(s) => Err(s),
        }
    }
}
