use schemajs_data::data_shard::DataShard;
use schemajs_dirs::create_schema_js_table;
use schemajs_primitives::table::Table;
use std::path::PathBuf;
use std::sync::RwLock;
use uuid::Uuid;

// TODO: Max shards
#[derive(Debug)]
pub struct EngineTable {
    tbl_folder: PathBuf,
    pub prim_table: Table,
    master_file: RwLock<DataShard>,
    shards: RwLock<Vec<DataShard>>,
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

    fn create_shard(&self) -> DataShard {
        let shard_path = self.tbl_folder.join(format!("shard_{}", Uuid::new_v4().to_string()));
        DataShard::new(shard_path, Some(100_000))
    }

    pub fn insert_row(&self, data: Vec<u8>) {
        let mut shards = self.shards.read().unwrap();
        let find_usable_shard = shards.iter().position(|i| i.has_space());
        let mut shard_index = match find_usable_shard {
            None => {
                let shard = self.create_shard();
                let mut shards = self.shards.write().unwrap();
                shards.push(shard);
                self.shards.read().unwrap().len() - 1 // Get a mutable reference to the newly created shard
            }
            Some(shard) => shard,
        };

        self.shards.write().unwrap()[shard_index].insert_item(data)
            .unwrap();
    }


}
