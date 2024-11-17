use crate::commit_log::error::CommitLogError;
use crate::cursor::Cursor;
use crate::shard::shards::UUID_BYTE_LEN;
use crate::U64_SIZE;
use enum_as_inner::EnumAsInner;
use sha2::digest::typenum::op;
use uuid::Uuid;

#[derive(EnumAsInner, Debug, PartialEq)]
pub enum OperationType {
    Insert,
    Delete,
    Update,
    Raw,
}

impl OperationType {
    pub fn get_le_identifier(&self) -> [u8; 1] {
        match self {
            OperationType::Insert => [0u8],
            OperationType::Delete => [1u8],
            OperationType::Update => [2u8],
            OperationType::Raw => [3u8],
        }
    }

    pub fn from_bytes(bytes: u8) -> Result<Self, ()> {
        match bytes {
            0u8 => Ok(Self::Insert),
            1u8 => Ok(Self::Delete),
            2u8 => Ok(Self::Update),
            3u8 => Ok(Self::Raw),
            _ => {
                // TODO: Add Logger
                Err(())
            }
        }
    }
}

#[derive(EnumAsInner, Debug, PartialEq)]
pub enum OperationData {
    Insert {
        data: Vec<u8>,
    },
    Delete {
        shard_id: Uuid,
        global_indx: u64,
        local_indx: u64,
    },
    Update {
        new_data: Vec<u8>,
    },
    Raw {
        data: Vec<u8>,
    },
}

#[derive(Debug)]
pub struct CommitLogEntry {
    operation_type: OperationType,
    pub data: OperationData,
}

impl CommitLogEntry {
    pub fn raw(data: &[u8]) -> Self {
        Self {
            operation_type: OperationType::Raw,
            data: OperationData::Raw {
                data: data.to_vec(),
            },
        }
    }

    pub fn insert(data: &[u8]) -> Self {
        Self {
            operation_type: OperationType::Insert,
            data: OperationData::Insert {
                data: data.to_vec(),
            },
        }
    }

    pub fn delete(shard_id: Uuid, global_indx: u64, local_indx: u64) -> Self {
        Self {
            operation_type: OperationType::Delete,
            data: OperationData::Delete {
                shard_id,
                global_indx,
                local_indx,
            },
        }
    }

    pub fn update(data: &[u8]) -> Self {
        Self {
            operation_type: OperationType::Update,
            data: OperationData::Update {
                new_data: data.to_vec(),
            },
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let item_type_bytes = self.operation_type.get_le_identifier();
        let mut operation_bytes: Vec<u8> = vec![];

        operation_bytes.extend_from_slice(&item_type_bytes);

        let data = {
            let mut payload: Vec<u8> = vec![];
            match &self.data {
                OperationData::Insert { data } => {
                    payload.extend_from_slice(data);
                }
                OperationData::Delete {
                    global_indx,
                    local_indx,
                    shard_id,
                } => {
                    payload.extend_from_slice(&shard_id.to_bytes_le());
                    payload.extend_from_slice(&global_indx.to_le_bytes());
                    payload.extend_from_slice(&local_indx.to_le_bytes());
                }
                OperationData::Update { new_data } => {
                    payload.extend_from_slice(new_data);
                }
                OperationData::Raw { data } => {
                    payload.extend_from_slice(data);
                }
            };

            payload
        };

        operation_bytes.extend_from_slice(&data.len().to_le_bytes());
        operation_bytes.extend_from_slice(&data);
        operation_bytes.extend_from_slice(b"/el"); // END log

        operation_bytes
    }
}

impl TryFrom<&[u8]> for CommitLogEntry {
    type Error = CommitLogError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut cursor = Cursor::new(value);
        let item_type = cursor
            .consume(1)
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let item_type =
            OperationType::from_bytes(item_type[0]).map_err(|_| CommitLogError::BrokenRecord)?;

        let payload_len = cursor
            .consume(U64_SIZE)
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let item_payload_le_bytes_input: [u8; 8] = payload_len
            .try_into()
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let item_payload_usize = usize::from_le_bytes(item_payload_le_bytes_input);

        let payload = cursor
            .consume(item_payload_usize)
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let delimiter = cursor
            .consume(3)
            .map_err(|_| CommitLogError::BrokenRecord)?;

        if delimiter != b"/el" {
            return Err(CommitLogError::BrokenRecord);
        }

        let op_data = {
            match item_type {
                OperationType::Insert => OperationData::Insert {
                    data: payload.to_vec(),
                },
                OperationType::Raw => OperationData::Raw {
                    data: payload.to_owned(),
                },
                OperationType::Delete => {
                    let mut delete_cursor = Cursor::new(&payload);

                    let uuid = delete_cursor
                        .consume(UUID_BYTE_LEN as usize)
                        .map_err(|_| CommitLogError::BrokenRecord)?;
                    let global_indx = delete_cursor
                        .consume(U64_SIZE)
                        .map_err(|_| CommitLogError::BrokenRecord)?;
                    let local_indx = delete_cursor
                        .consume(U64_SIZE)
                        .map_err(|_| CommitLogError::BrokenRecord)?;

                    OperationData::Delete {
                        shard_id: Uuid::from_bytes_le(uuid.to_vec().try_into().unwrap()),
                        global_indx: u64::from_le_bytes(global_indx.to_vec().try_into().unwrap()),
                        local_indx: u64::from_le_bytes(local_indx.to_vec().try_into().unwrap()),
                    }
                }
                OperationType::Update => OperationData::Update {
                    new_data: payload.to_vec(),
                },
            }
        };

        Ok(Self {
            operation_type: item_type,
            data: op_data,
        })
    }
}

impl TryFrom<Vec<u8>> for CommitLogEntry {
    type Error = CommitLogError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice()).map_err(|_| CommitLogError::BrokenRecord)
    }
}

#[cfg(test)]
mod test {
    use crate::commit_log::operations::CommitLogEntry;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_insert() {
        let operation_item = CommitLogEntry::insert(b"Hello World");
        let a = operation_item.to_vec();
        let from_a = CommitLogEntry::try_from(a.clone()).unwrap();
        assert_eq!(a, from_a.to_vec());
        assert_eq!(from_a.data.as_insert().unwrap(), b"Hello World");
    }

    #[tokio::test]
    pub async fn test_update() {
        let operation_item = CommitLogEntry::insert(b"Hello World");
        let a = operation_item.to_vec();
        let from_a = CommitLogEntry::try_from(a.clone()).unwrap();
        assert_eq!(a, from_a.to_vec());
        assert_eq!(from_a.data.as_insert().unwrap(), b"Hello World");
    }

    #[tokio::test]
    pub async fn test_delete() {
        let shard_id = Uuid::new_v4();
        let operation_item = CommitLogEntry::delete(shard_id.clone(), 0, 1);
        let delete_item = operation_item.data.as_delete().unwrap();
        assert_eq!(delete_item.0.to_string(), shard_id.to_string());
        assert_eq!(delete_item.1, &0);
        assert_eq!(delete_item.2, &1);
        let a = operation_item.to_vec();
        let from_a = CommitLogEntry::try_from(a.clone()).unwrap();
        let from_a = from_a.data.as_delete().unwrap();
        assert_eq!(from_a.0.to_string(), shard_id.to_string());
        assert_eq!(from_a.1, &0);
        assert_eq!(from_a.2, &1);
    }

    #[tokio::test]
    pub async fn test_invalid_delimiter() {
        let operation_item = CommitLogEntry::insert(b"Hello World");
        let a = operation_item.to_vec();

        let mut remove_delimiter = vec![];
        remove_delimiter.extend_from_slice(&a[0..(a.len() - 3)]);

        let from_a = CommitLogEntry::try_from(remove_delimiter);
        assert!(from_a.err().unwrap().is_broken_record());

        let mut remove_delimiter = vec![];
        remove_delimiter.extend_from_slice(&a[0..(a.len())]);
        let from_a = CommitLogEntry::try_from(remove_delimiter);
        let _ = from_a.unwrap();
    }
}
