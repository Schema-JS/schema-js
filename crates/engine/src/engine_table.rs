use crate::query_error::{InsertionError, QueryError};
use crate::serializer::borsh::BorshRowSerializer;
use crate::serializer::RowSerializer;
use crate::validation_error::ValidationError;
use deno_core::serde_json;
use schemajs_data::map_shard::MapShard;
use schemajs_data::temp_map_shard::TempMapShard;
use schemajs_dirs::create_schema_js_table;
use schemajs_primitives::table::Table;
use schemajs_primitives::types::DataTypes;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct EngineTable {
    pub tbl_folder: PathBuf,
    pub prim_table: Table,
    pub data: RwLock<MapShard>,
    pub temp_shards: TempMapShard,
    pub serializer: Arc<dyn RowSerializer>,
}

impl EngineTable {
    pub fn new(base_path: Option<PathBuf>, db: &str, table: Table) -> Self {
        let table_folder_path = create_schema_js_table(base_path, db, table.name.as_str());

        EngineTable {
            tbl_folder: table_folder_path.clone(),
            prim_table: table,
            data: RwLock::new(MapShard::new(table_folder_path.clone(), "data_", None)),
            temp_shards: TempMapShard::new(table_folder_path, Some(500_000), "datatemp-"),
            serializer: Arc::new(BorshRowSerializer::default()),
        }
    }

    fn validate_row_value(&self, item: &serde_json::Value) -> Result<(), ValidationError> {
        for (name, column) in self.prim_table.columns.iter() {
            let value = item.get(name);

            if column.required && value.is_none() {
                return Err(ValidationError::MissingColumn(name.clone()));
            }

            let value = value.unwrap();

            match column.data_type {
                DataTypes::String => {
                    if !value.is_string() {
                        return Err(ValidationError::ExpectedString(name.clone()));
                    }
                }
                DataTypes::Boolean => {
                    if !value.is_boolean() {
                        return Err(ValidationError::ExpectedBoolean(name.clone()));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn insert_row(&self, item: serde_json::Value) -> Result<(), QueryError> {
        // let validate = self.validate_row_value(&item);
        // validate.map_err(InsertionError::ValidationError)?;
        let val = self
            .serializer
            .serialize(&item)
            .map_err(InsertionError::SerializationError)?;
        self.temp_shards.insert_row(val);
        Ok(())
    }
}
