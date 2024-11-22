use crate::commit_log::error::CommitLogError;
use crate::cursor::Cursor;
use crate::shard::shards::UUID_BYTE_LEN;
use crate::U64_SIZE;
use enum_as_inner::EnumAsInner;
use sha2::digest::typenum::op;
use uuid::Uuid;

#[derive(EnumAsInner, Debug, PartialEq)]
pub enum CommitLogOperationType {
    Insert,
    Delete,
    Update,
    Raw,
}

impl CommitLogOperationType {
    pub fn get_le_identifier(&self) -> [u8; 1] {
        match self {
            CommitLogOperationType::Insert => [0u8],
            CommitLogOperationType::Delete => [1u8],
            CommitLogOperationType::Update => [2u8],
            CommitLogOperationType::Raw => [3u8],
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
    Insert { data: Vec<u8> },
    Delete { global_indx: u64 },
    Update { global_indx: u64, new_data: Vec<u8> },
    Raw { data: Vec<u8> },
}

#[derive(Debug)]
pub struct CommitLogEntry {
    operation_type: CommitLogOperationType,
    pub data: OperationData,
}

impl CommitLogEntry {
    pub fn raw(data: &[u8]) -> Self {
        Self {
            operation_type: CommitLogOperationType::Raw,
            data: OperationData::Raw {
                data: data.to_vec(),
            },
        }
    }

    pub fn insert(data: &[u8]) -> Self {
        Self {
            operation_type: CommitLogOperationType::Insert,
            data: OperationData::Insert {
                data: data.to_vec(),
            },
        }
    }

    pub fn delete(global_indx: u64) -> Self {
        Self {
            operation_type: CommitLogOperationType::Delete,
            data: OperationData::Delete { global_indx },
        }
    }

    pub fn update(data: &[u8], global_indx: u64) -> Self {
        Self {
            operation_type: CommitLogOperationType::Update,
            data: OperationData::Update {
                new_data: data.to_vec(),
                global_indx,
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
                OperationData::Delete { global_indx } => {
                    payload.extend_from_slice(&global_indx.to_le_bytes());
                }
                OperationData::Update {
                    new_data,
                    global_indx,
                } => {
                    payload.extend_from_slice(&global_indx.to_le_bytes());
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
        let item_type = CommitLogOperationType::from_bytes(item_type[0])
            .map_err(|_| CommitLogError::BrokenRecord)?;

        let payload_len = cursor
            .consume(U64_SIZE)
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let item_payload_le_bytes_input: [u8; 8] = payload_len
            .try_into()
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let item_payload_usize = u64::from_le_bytes(item_payload_le_bytes_input);

        let payload = cursor
            .consume(item_payload_usize as usize)
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let delimiter = cursor
            .consume(3)
            .map_err(|_| CommitLogError::BrokenRecord)?;

        if delimiter != b"/el" {
            return Err(CommitLogError::BrokenRecord);
        }

        let op_data = {
            match item_type {
                CommitLogOperationType::Insert => OperationData::Insert {
                    data: payload.to_vec(),
                },
                CommitLogOperationType::Raw => OperationData::Raw {
                    data: payload.to_owned(),
                },
                CommitLogOperationType::Delete => {
                    let mut delete_cursor = Cursor::new(&payload);

                    let global_indx = delete_cursor
                        .consume(U64_SIZE)
                        .map_err(|_| CommitLogError::BrokenRecord)?;

                    OperationData::Delete {
                        global_indx: u64::from_le_bytes(global_indx.to_vec().try_into().unwrap()),
                    }
                }
                CommitLogOperationType::Update => {
                    let mut update_cursor = Cursor::new(&payload);

                    let global_indx = update_cursor
                        .consume(U64_SIZE)
                        .map_err(|_| CommitLogError::BrokenRecord)?;

                    let data = update_cursor
                        .consume(payload.len() - U64_SIZE)
                        .map_err(|_| CommitLogError::BrokenRecord)?;

                    OperationData::Update {
                        global_indx: u64::from_le_bytes(global_indx.to_vec().try_into().unwrap()),
                        new_data: data.to_vec(),
                    }
                }
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
        let operation_item = CommitLogEntry::update(b"Hello World", 10);
        assert_eq!(operation_item.data.as_update().unwrap().0, &10);
        let a = operation_item.to_vec();
        let from_a = CommitLogEntry::try_from(a.clone()).unwrap();
        assert_eq!(a, from_a.to_vec());
        let updt = from_a.data.as_update().unwrap();
        assert_eq!(updt.1, b"Hello World");
        assert_eq!(updt.0, &10);
    }

    #[tokio::test]
    pub async fn test_delete() {
        let shard_id = Uuid::new_v4();
        let operation_item = CommitLogEntry::delete(1);
        let delete_item = operation_item.data.as_delete().unwrap();
        assert_eq!(delete_item, &1);
        let a = operation_item.to_vec();
        let from_a = CommitLogEntry::try_from(a.clone()).unwrap();
        let from_a = from_a.data.as_delete().unwrap();
        assert_eq!(from_a, &1);
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
