use crate::shard::{ShardConfig, TempShardConfig};
use crate::temp_offset_types::TempOffsetTypes;

#[derive(Clone, Debug)]
pub struct DataShardConfig {
    pub max_offsets: Option<u64>,
}

impl ShardConfig for DataShardConfig {}

#[derive(Debug)]
pub struct TempDataShardConfig {
    pub max_offsets: TempOffsetTypes,
}

impl TempShardConfig<DataShardConfig> for TempDataShardConfig {
    fn to_config(&self) -> DataShardConfig {
        DataShardConfig {
            max_offsets: self.max_offsets.get_real_offset(),
        }
    }
}
