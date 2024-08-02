use schemajs_data::data_shard::DataShard;
use schemajs_dirs::create_schema_js_table;
use schemajs_primitives::table::Table;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Debug)]
pub struct EngineTable {
    pub tbl_folder: PathBuf,
    pub prim_table: Table,
    pub master_file: RwLock<DataShard>,
    pub shards: RwLock<Vec<DataShard>>,
}

impl EngineTable {
    pub fn new(db: &str, table: Table) -> Self {
        let path = create_schema_js_table(db, table.name.as_str());
        let master_file = path.join("0.data");
        let master_file_data_shard = DataShard::new(master_file, Some(1_000_000));

        EngineTable {
            tbl_folder: path,
            prim_table: table,
            master_file: RwLock::new(master_file_data_shard),
            shards: RwLock::new(vec![]),
        }
    }

    pub fn insert_row(&self, data: Vec<u8>) {
        self.master_file.write()
            .unwrap()
            .insert_item(data)
            .unwrap();
    }


}
