use crate::index::composite_key::CompositeKey;
use crate::index::index_type::IndexType;
use crate::map_shard::MapShard;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
pub struct IndexChild {
    // Name of the index
    pub name: String,

    // Fields in the index
    pub fields: Vec<String>,

    // Index Type (For now, there' just the Primary option)
    pub index_type: IndexType,

    // Index File Management
    pub data: Arc<RwLock<MapShard>>,

    // Pointers of CompositeKeys to the Data File Item
    pub pointers: BTreeMap<CompositeKey, u64>,
}

impl IndexChild {
    pub fn new(
        index_folder: PathBuf,
        index_name: Option<String>,
        fields: Vec<String>,
        index_type: IndexType,
    ) -> Self {
        let name = index_name.unwrap_or_else(|| {
            let uuid = Uuid::new_v4().to_string();
            let fields = fields.join("_");
            format!("indx_{}_{}", fields, uuid)
        });

        let map_shard = MapShard::new(index_folder.join(name.as_str()), "indx_", Some(5_000_000));
        Self {
            name,
            fields,
            index_type,
            data: Arc::new(RwLock::new(map_shard)),
            pointers: Default::default(),
        }
    }
}
