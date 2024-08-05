use schemajs_data::map_shard::MapShard;
use schemajs_data::temp_map_shard::TempMapShard;
use schemajs_dirs::create_schema_js_table;
use schemajs_primitives::table::Table;
use std::path::PathBuf;

// TODO: Max shards
#[derive(Debug)]
pub struct EngineTable {
    pub tbl_folder: PathBuf,
    pub prim_table: Table,
    pub data: MapShard,
    pub temp_shards: TempMapShard,
}

impl EngineTable {
    pub fn new(db: &str, table: Table) -> Self {
        let table_folder_path = create_schema_js_table(db, table.name.as_str());

        EngineTable {
            tbl_folder: table_folder_path.clone(),
            prim_table: table,
            data: MapShard::new(table_folder_path.clone(), "data_"),
            temp_shards: TempMapShard::new(table_folder_path, Some(5000), "datatemp-"),
        }
    }
}
