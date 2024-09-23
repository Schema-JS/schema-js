use crate::errors::ShardErrors;
use std::path::PathBuf;
use uuid::Uuid;
pub mod map_shard;
pub mod shard_collection;
pub mod shards;
pub mod temp_collection;
pub mod temp_map_shard;

pub trait ShardConfig: Clone {}

pub enum AvailableSpace {
    Fixed(usize),
    Unlimited,
}

pub trait Shard<Opts: ShardConfig> {
    fn new(path: PathBuf, opts: Opts, uuid: Option<Uuid>) -> Self;

    fn has_space(&self) -> bool;

    /// Determines the breaking point for this shard within the sharding mechanism.
    ///
    /// The "breaking point" is a critical threshold in the context of sharding,
    /// where the current shard has accumulated enough items to warrant splitting
    /// into multiple smaller shards. This method helps identify when such a
    /// condition is met and whether the shard is due for splitting.
    ///
    /// # Returns
    ///
    /// - `Some(u64)` - If the shard has a known finite breaking point, this method
    ///   returns `Some` containing the `u64` value that represents the threshold
    ///   number of items. When this number of items is reached or exceeded, the
    ///   shard is allowed to split into smaller pieces.
    /// - `None` - If the breaking point is considered infinite (i.e., there is no
    ///   practical limit to the number of items the shard can contain without
    ///   needing to split), the method returns `None`. This typically indicates
    ///   that the shard does not have a predefined threshold or the threshold is
    ///   so large that it is effectively infinite.
    ///
    /// # Context
    ///
    /// This method is part of the `Shard` trait, which is used in systems that
    /// employ sharding as a strategy to manage large datasets by dividing them
    /// into smaller, more manageable pieces. The `breaking_point` method plays a
    /// crucial role in determining when a shard should be split to maintain
    /// performance, balance load, or adhere to system constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// # trait Shard {
    /// #     fn breaking_point(&self) -> Option<u64>;
    /// # }
    /// # struct MyShard {
    /// #     breaking_point_value: Option<u64>,
    /// # }
    /// # impl Shard for MyShard {
    /// #     fn breaking_point(&self) -> Option<u64> {
    /// #         self.breaking_point_value
    /// #     }
    /// # }
    /// let shard = MyShard { breaking_point_value: Some(1000) };
    ///
    /// // This shard is allowed to split when it reaches 1000 items.
    /// assert_eq!(shard.breaking_point(), Some(1000));
    ///
    /// let infinite_shard = MyShard { breaking_point_value: None };
    ///
    /// // This shard has no practical limit and won't split.
    /// assert_eq!(infinite_shard.breaking_point(), None);
    /// ```
    ///
    /// # Note
    ///
    /// The breaking point is a pivotal aspect of the sharding strategy. Understanding
    /// when and why a shard should split is essential for maintaining the efficiency
    /// and scalability of the system. The exact threshold value and its implications
    /// should be well-defined and aligned with the overall design and requirements
    /// of the system.
    fn breaking_point(&self) -> Option<u64>;

    fn get_path(&self) -> PathBuf;

    fn get_last_index(&self) -> i64;

    fn read_item_from_index(&self, index: usize) -> Result<Vec<u8>, ShardErrors>;

    fn available_space(&self) -> AvailableSpace;

    fn insert_item(&self, data: &[&[u8]]) -> Result<u64, ShardErrors>;

    fn get_id(&self) -> String;
}

pub trait TempShardConfig<Opts: ShardConfig>: Clone {
    fn to_config(&self) -> Opts;
}
