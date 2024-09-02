use crate::keys::index_key_sha256::IndexKeySha256;
use crate::keys::string_index::StringIndexKey;
use enum_as_inner::EnumAsInner;

#[derive(Debug, Clone, EnumAsInner)]
pub enum IndexKeyType {
    Sha256(IndexKeySha256),
    String(StringIndexKey),
}
