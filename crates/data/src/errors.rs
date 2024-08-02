use enum_as_inner::EnumAsInner;

#[derive(Debug, Clone, EnumAsInner)]
pub enum DataShardErrors {
    OutOfPositions,
    UnknownOffset,
    FlushingError
}