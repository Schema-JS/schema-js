use crate::index::composite_key::composite_key::CompositeKey;
use crate::index::data::index_data_unit::IndexDataUnit;
use crate::index::types::IndexKey;
use crate::utils::hash::{sha256_to_string, to_sha256};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

#[derive(Debug)]
pub struct IndexKeySha256 {
    hash: String,
}

impl IndexKeySha256 {
    pub fn to_string(&self) -> String {
        self.hash.clone()
    }
}

impl From<IndexDataUnit> for IndexKeySha256 {
    fn from(value: IndexDataUnit) -> Self {
        IndexKeySha256 {
            hash: String::from_utf8(value.data).unwrap(),
        }
    }
}

impl From<CompositeKey> for IndexKeySha256 {
    fn from(value: CompositeKey) -> Self {
        let mut vec = Vec::new();

        for (key, val) in value.0 {
            vec.extend(key.into_bytes());
            vec.extend(val.into_bytes());
        }

        let hash = to_sha256(vec);

        IndexKeySha256 {
            hash: sha256_to_string(hash.to_vec()),
        }
    }
}

impl From<Vec<u8>> for IndexKeySha256 {
    fn from(value: Vec<u8>) -> Self {
        IndexKeySha256 {
            hash: sha256_to_string(value),
        }
    }
}

impl Into<Vec<u8>> for IndexKeySha256 {
    fn into(self) -> Vec<u8> {
        self.hash.into_bytes()
    }
}

impl Ord for IndexKeySha256 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl Eq for IndexKeySha256 {}

impl PartialEq<Self> for IndexKeySha256 {
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash)
    }
}

impl PartialOrd<Self> for IndexKeySha256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.hash.partial_cmp(&other.hash)
    }
}

impl Clone for IndexKeySha256 {
    fn clone(&self) -> Self {
        IndexKeySha256 {
            hash: self.hash.clone(),
        }
    }
}

impl Into<String> for IndexKeySha256 {
    fn into(self) -> String {
        self.hash
    }
}

impl IndexKey for IndexKeySha256 {}
