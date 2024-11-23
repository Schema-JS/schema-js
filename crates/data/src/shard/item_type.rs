use crate::cursor::Cursor;
use crate::shard::shards::UUID_BYTE_LEN;
use crate::U64_SIZE;
use enum_as_inner::EnumAsInner;
use uuid::Uuid;

const FACTOR_ITEM_THRESHOLD_BYTES: usize = 2048;

#[derive(EnumAsInner, Debug, PartialEq)]
pub enum ShardItemType {
    Raw,
    Deleted,
    Moved,
}

impl ShardItemType {
    pub fn get_le_identifier(&self) -> [u8; 1] {
        match self {
            ShardItemType::Raw => [0u8],
            ShardItemType::Deleted => [1u8],
            ShardItemType::Moved => [2u8],
        }
    }

    pub fn from_le_bytes(bytes: u8) -> Result<Self, ()> {
        match bytes {
            0u8 => Ok(Self::Raw),
            1u8 => Ok(Self::Deleted),
            2u8 => Ok(Self::Moved),
            _ => {
                // TODO: Add Logger
                Err(())
            }
        }
    }
}

#[derive(Debug)]
pub struct ShardItem {
    pub shard_item_type: ShardItemType,
    pub max_size: usize,
    pub used_size: usize,
    pub current_item_id: Uuid,
    pub current_shard_id: Uuid,
    pub current_offset: usize,
    pub moved_shard_id: Option<Uuid>,
    pub moved_offset: Option<usize>,
    pub data: Vec<u8>,
    internal_position: Option<usize>,
}

impl ShardItem {
    pub fn new_complete(
        item: &[u8],
        item_id: Uuid,
        shard_id: Uuid,
        offset: usize,
        threshold_extra_space: bool,
    ) -> Self {
        let item_type = ShardItemType::Raw;
        let item_len = item.len();

        let mut data = vec![];
        data.extend_from_slice(item);

        let mut max_size = item_len;

        if threshold_extra_space {
            let threshold_extra_space = {
                let item_len_multiply = item_len * 3;
                if item_len > FACTOR_ITEM_THRESHOLD_BYTES
                    || item_len_multiply > FACTOR_ITEM_THRESHOLD_BYTES
                {
                    FACTOR_ITEM_THRESHOLD_BYTES
                } else {
                    item_len_multiply
                }
            };
            data.extend_from_slice(&vec![0; threshold_extra_space]);
            max_size += threshold_extra_space;
        }

        Self {
            current_item_id: item_id,
            used_size: item_len,
            shard_item_type: item_type,
            max_size,
            moved_shard_id: None,
            moved_offset: None,
            data,
            internal_position: None,
            current_shard_id: shard_id,
            current_offset: offset,
        }
    }

    pub fn set_internal_pos(&mut self, pos: usize) {
        self.internal_position = Some(pos);
    }

    pub fn get_internal_pos(&self) -> Option<usize> {
        self.internal_position
    }

    pub fn new(item: &[u8], item_id: Uuid, shard_id: Uuid, offset: usize) -> Self {
        Self::new_complete(item, item_id, shard_id, offset, true)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let item_type_bytes = self.shard_item_type.get_le_identifier();
        let mut metadata: Vec<u8> = vec![];
        let item_len = self.used_size;

        let inner_metadata = {
            let mut metadata: Vec<u8> = vec![];
            metadata.extend_from_slice(&self.current_shard_id.to_bytes_le());
            metadata.extend_from_slice(&self.current_offset.to_le_bytes());
            metadata.extend_from_slice(&(item_len as u64).to_le_bytes());
            metadata.extend_from_slice(&(self.max_size as u64).to_le_bytes());

            // UUID + offset (If moved, UUID represents the shard that it has been moved into. offset represents the item offset in that shard)
            // ShardItemType::Moved needs to be present
            // UUID = 16b
            // Offset = 8b

            if let Some(uuid) = &self.moved_shard_id {
                metadata.extend_from_slice(&uuid.to_bytes_le());
                metadata.extend_from_slice(&(self.moved_offset.unwrap() as u64).to_le_bytes());
            } else {
                metadata.extend_from_slice(&[0u8; 24]);
            }

            metadata
        };

        metadata.extend_from_slice(&item_type_bytes);
        metadata.extend_from_slice(&self.current_item_id.to_bytes_le().as_slice());
        metadata.extend_from_slice(&(inner_metadata.len() as u64).to_le_bytes());
        metadata.extend(inner_metadata);

        let mut final_item = metadata;
        final_item.extend_from_slice(self.data.as_slice());

        final_item.to_vec()
    }

    pub fn update(&mut self, new_data: &[u8]) {
        let new_data_len = new_data.len();
        if new_data_len > self.max_size {
            self.shard_item_type = ShardItemType::Moved;
        } else {
            self.data[..new_data_len].copy_from_slice(new_data);
            self.used_size = new_data_len;
        }
    }

    pub fn get_used_data(&self) -> &[u8] {
        &self.data[0..self.used_size]
    }

    pub fn calculate_size(content: &[u8]) -> usize {
        // self.shard_item_type
        let mut initial_size: usize = 1;
        // Item Id
        initial_size += UUID_BYTE_LEN as usize;
        // Current Shard Id
        initial_size += UUID_BYTE_LEN as usize;
        // internal_position
        initial_size += U64_SIZE;
        // item_len = u64 = 8b
        initial_size += U64_SIZE;
        // self.max_size = u64 = 8b
        initial_size += U64_SIZE;
        // UUID
        initial_size += UUID_BYTE_LEN as usize;
        // OFFSET
        initial_size += U64_SIZE;
        // Metadata length (u64)
        initial_size += U64_SIZE;
        // Metadata
        initial_size += content.len();

        initial_size
    }
}

impl From<Vec<u8>> for ShardItem {
    fn from(value: Vec<u8>) -> Self {
        let mut cursor = Cursor::new(&value);
        let item_type = cursor.consume(1).unwrap();
        let item_type = ShardItemType::from_le_bytes(item_type[0]).unwrap();

        let _ = cursor.consume(U64_SIZE).unwrap(); // Consume metadata len before consuming the actual metadata

        let current_item_id = cursor.consume(UUID_BYTE_LEN as usize).unwrap();
        let curr_shard_id = cursor.consume(UUID_BYTE_LEN as usize).unwrap();
        let curr_offset = cursor.consume(U64_SIZE).unwrap();
        let item_len = cursor.consume(U64_SIZE).unwrap();
        let max_size = cursor.consume(U64_SIZE).unwrap();

        let item_len_le_bytes: [u8; 8] = item_len.try_into().unwrap();
        let max_size_le_bytes: [u8; 8] = max_size.try_into().unwrap();
        let uuid_and_offset = cursor.consume(UUID_BYTE_LEN as usize + U64_SIZE).unwrap();
        let uuid = &uuid_and_offset[0..(UUID_BYTE_LEN as usize)];
        let offset = &uuid_and_offset[(UUID_BYTE_LEN as usize)..];

        let uuid = if uuid == &[0u8; UUID_BYTE_LEN as usize] {
            None
        } else {
            Some(Uuid::from_bytes_le(uuid.to_vec().try_into().unwrap()))
        };

        // Offset will always be read if UUID is active. Because it means it has been moved.
        let offset = if uuid.is_none() {
            None
        } else {
            let moved_offset_le: [u8; 8] = offset.try_into().unwrap();
            Some(u64::from_le_bytes(moved_offset_le) as usize)
        };

        let max_size = u64::from_le_bytes(max_size_le_bytes);

        let data = &value[(value.len() - max_size as usize)..value.len()];

        Self {
            current_item_id: Uuid::from_bytes_le(current_item_id.to_vec().try_into().unwrap()),
            shard_item_type: item_type,
            max_size: max_size as usize,
            used_size: u64::from_le_bytes(item_len_le_bytes) as usize,
            current_shard_id: Uuid::from_bytes_le(curr_shard_id.to_vec().try_into().unwrap()),
            current_offset: u64::from_le_bytes(curr_offset.try_into().unwrap()) as usize,
            moved_shard_id: uuid,
            moved_offset: offset,
            data: data.to_vec(),
            internal_position: None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::cursor::Cursor;
    use crate::shard::item_type::{ShardItem, ShardItemType};
    use crate::shard::shards::UUID_BYTE_LEN;
    use crate::U64_SIZE;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn test_item_deserialization() {
        let shard_id = Uuid::new_v4();
        let offset = 20;
        let item = ShardItem::new(b"Hello", Uuid::new_v4(), shard_id, offset);
        assert_eq!(
            item.data,
            &[72, 101, 108, 108, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        let to_vec = item.to_vec();
        let from_vec = ShardItem::from(to_vec);
        assert_eq!(
            from_vec.data,
            &[72, 101, 108, 108, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(from_vec.used_size, item.used_size);
        assert_eq!(from_vec.max_size, item.max_size);
        assert_eq!(from_vec.moved_offset, item.moved_offset);
        assert_eq!(from_vec.moved_shard_id, item.moved_shard_id);
        assert_eq!(from_vec.shard_item_type, item.shard_item_type);
        assert_eq!(from_vec.current_shard_id, item.current_shard_id);
        assert_eq!(from_vec.current_offset, item.current_offset);

        // assert content
        assert_eq!(from_vec.get_used_data(), b"Hello");
    }

    #[tokio::test]
    pub async fn test_item_moved_with_offset_0() {
        let shard_id = Uuid::new_v4();
        let offset = 30;
        let mut item = ShardItem::new(b"Hello", Uuid::new_v4(), shard_id, offset);
        let id = Uuid::new_v4();
        item.moved_offset = Some(0);
        item.moved_shard_id = Some(id.clone());
        let vec = item.to_vec();
        let new_item = ShardItem::from(vec);
        assert_eq!(new_item.moved_offset.unwrap(), 0);
    }

    #[tokio::test]
    pub async fn test_item_moved() {
        let mut item = ShardItem::new(b"Hello", Uuid::new_v4(), Uuid::new_v4(), 27);
        let id = Uuid::new_v4();
        item.moved_offset = Some(1);
        item.moved_shard_id = Some(id.clone());
        let vec = item.to_vec();
        let new_item = ShardItem::from(vec);
        assert_eq!(new_item.moved_shard_id.unwrap().to_string(), id.to_string());
        assert_eq!(new_item.moved_offset.unwrap(), 1);

        // assert content
        assert_eq!(item.get_used_data(), b"Hello");
    }

    #[tokio::test]
    pub async fn test_item_serialization() {
        let item = ShardItem::new(b"Hello", Uuid::new_v4(), Uuid::new_v4(), 33);
        let original_uuid = item.current_item_id.clone();
        let to_vec = item.to_vec();

        let mut cursor = Cursor::new(&to_vec);

        let item_type = cursor.consume(1).unwrap();
        assert_eq!(item_type.len(), 1);
        let item_type = ShardItemType::from_le_bytes(item_type[0]).unwrap();
        assert!(item_type.is_raw());

        let uuid = cursor.consume(UUID_BYTE_LEN as usize).unwrap();
        assert_eq!(
            original_uuid,
            Uuid::from_bytes_le(uuid.to_vec().try_into().unwrap())
        );

        let item_metadata_len = cursor.consume(U64_SIZE).unwrap();

        let item_metadata_le_bytes_input: [u8; 8] = item_metadata_len.try_into().unwrap();
        let item_metadata_usize = usize::from_le_bytes(item_metadata_le_bytes_input);
        assert_eq!(item_metadata_usize, 64);

        let inner_metadata = cursor.consume(item_metadata_usize).unwrap();
        let mut metadata_cursor = Cursor::new(inner_metadata);
        let current_shard_id = metadata_cursor.consume(UUID_BYTE_LEN as usize).unwrap();
        let current_offset = metadata_cursor.consume(U64_SIZE).unwrap();
        let item_len = metadata_cursor.consume(U64_SIZE).unwrap();
        let max_size = metadata_cursor.consume(U64_SIZE).unwrap();
        let moved_id_and_offset = metadata_cursor.consume(24).unwrap();

        let item_len_le_bytes: [u8; 8] = item_len.try_into().unwrap();
        let max_size_le_bytes: [u8; 8] = max_size.try_into().unwrap();
        assert_eq!(u64::from_le_bytes(item_len_le_bytes), 5);
        assert_eq!(u64::from_le_bytes(max_size_le_bytes), 20);
        assert_eq!(moved_id_and_offset, &[0u8; 24]);
        let item = &to_vec[89..109];
        assert_eq!(
            item,
            &[72, 101, 108, 108, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert!(to_vec.get(110).is_none());
    }
}
