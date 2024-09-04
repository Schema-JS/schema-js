use enum_as_inner::EnumAsInner;

#[derive(Debug, Clone, EnumAsInner)]
pub enum IndexError {
    UnrecognizedItemSize,
    InvalidItem,
}
