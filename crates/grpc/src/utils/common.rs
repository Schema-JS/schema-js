use crate::services::query::query_data::query_service::query_ops::Operation;
use crate::services::query::query_data::query_service::{
    QueryOps as GrpcQueryOps, QueryVal as GrpcQueryVal,
};
use crate::services::shared::shared::data_value::ValueType;
use schemajs_engine::engine_db::EngineDb;
use schemajs_internal::auth::types::UserContext;
use schemajs_internal::manager::InternalManager;
use schemajs_primitives::column::types::DataValue;
use schemajs_query::ops::query_ops::{QueryOps, QueryVal};
use std::str::FromStr;
use std::sync::Arc;
use tonic::Status;
use uuid::Uuid;

pub fn convert_to_data_value(val: ValueType) -> DataValue {
    match val {
        ValueType::NullValue(_) => DataValue::Null,
        ValueType::UuidValue(u) => DataValue::Uuid(Uuid::from_str(&u).unwrap_or(Uuid::nil())),
        ValueType::StringValue(s) => DataValue::String(s),
        ValueType::BoolValue(b) => DataValue::Boolean(b),
        ValueType::NumberValue(n) => {
            DataValue::Number(serde_json::value::Number::from_f64(n as f64).unwrap())
        }
    }
}

pub fn convert_to_grpc_value(val: &DataValue) -> ValueType {
    match &val {
        &DataValue::Null => ValueType::NullValue(true),
        &DataValue::Uuid(u) => ValueType::UuidValue(u.to_string()),
        &DataValue::String(s) => ValueType::StringValue(s.clone()),
        &DataValue::Boolean(b) => ValueType::BoolValue(b.clone()),
        &DataValue::Number(n) => ValueType::NumberValue(n.as_f64().unwrap() as f32),
    }
}

pub fn find_database(
    internal_manager: &Arc<InternalManager>,
    user_context: Arc<UserContext>,
) -> Result<Arc<EngineDb>, Status> {
    let engine = internal_manager.clone().engine();
    let db_manager = engine
        .read()
        .map_err(|e| Status::internal(format!("Failed to read engine: {:?}", e)))?;
    let user = user_context.get_user();
    match db_manager.find_by_name_ref(&user.scheme) {
        Some(db) => Ok(db.clone()),
        None => return Err(Status::not_found("Database not found")),
    }
}

pub fn grpc_query_val_to_sjs_value(val: GrpcQueryVal) -> QueryVal {
    QueryVal {
        key: val.key,
        filter_type: val.filter_type,
        value: convert_to_data_value(
            val.value
                .map(|i| i.value_type.unwrap_or_else(|| ValueType::NullValue(true)))
                .unwrap(),
        ),
    }
}

pub fn grpc_operation_to_sjs_op(operation: Operation) -> Result<QueryOps, ()> {
    match operation {
        Operation::AndOp(val) => Ok(QueryOps::And(
            val.ops
                .into_iter()
                .map(|e| grpc_operation_to_sjs_op(e.operation.ok_or(())?))
                .collect::<Result<Vec<QueryOps>, ()>>()?,
        )),
        Operation::OrOp(val) => Ok(QueryOps::Or(
            val.ops
                .into_iter()
                .map(|e| grpc_operation_to_sjs_op(e.operation.ok_or(())?))
                .collect::<Result<Vec<QueryOps>, ()>>()?,
        )),
        Operation::Condition(val) => Ok(QueryOps::Condition(grpc_query_val_to_sjs_value(val))),
    }
}

pub fn from_grpc_ops_to_sjs_ops(query_ops: GrpcQueryOps) -> Result<QueryOps, ()> {
    println!("query_ops is some {}", query_ops.operation.is_some());
    match query_ops.operation {
        None => Err(()),
        Some(op) => {
            println!("grpc_operation_to_sjs_op");
            grpc_operation_to_sjs_op(op)
        }
    }
}
