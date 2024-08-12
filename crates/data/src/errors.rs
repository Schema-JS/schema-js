use enum_as_inner::EnumAsInner;

#[derive(Debug, Clone, EnumAsInner)]
pub enum ShardErrors {
    OutOfPositions,
    UnknownOffset,
    FlushingError,
    ErrorReadingByteRange,
    ErrorAddingHeaderOffset,
}
