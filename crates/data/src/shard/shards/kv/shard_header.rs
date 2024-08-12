use crate::data_handler::DataHandler;
use crate::shard::shards::UUID_BYTE_LEN;
use crate::{I64_SIZE, U64_SIZE};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct KvShardHeader {
    pub max_capacity: Option<u64>,
    pub items_len: u64,
    pub value_size: u64,
    pub id: Uuid,
    data: Arc<RwLock<DataHandler>>,
}

impl KvShardHeader {
    pub fn new(
        items_len: u64,
        max_capacity: Option<u64>,
        value_size: u64,
        uuid: Option<Uuid>,
        data: Arc<RwLock<DataHandler>>,
    ) -> Self {
        Self {
            items_len,
            max_capacity,
            data,
            id: uuid.unwrap_or_else(|| Uuid::new_v4()),
            value_size,
        }
    }

    pub fn new_from_file(
        file: Arc<RwLock<DataHandler>>,
        uuid: Option<Uuid>,
        items_len: Option<u64>,
        max_capacity: Option<u64>,
        value_size: u64,
    ) -> Self {
        let mut header = KvShardHeader::new(
            items_len.unwrap_or(0),
            max_capacity,
            value_size,
            uuid,
            file.clone(),
        );

        // Check if the file is empty
        let metadata = file
            .read()
            .unwrap()
            .metadata()
            .expect("Failed to get file metadata");

        let file_len = metadata.len();

        if file_len == 0 {
            header.initialize_empty_file();
        } else {
            header.read_header();
        }

        header
    }

    pub fn header_size() -> usize {
        let max_capacity_size = U64_SIZE;
        let items_len_size = U64_SIZE;
        let value_size = U64_SIZE;
        let id_len = UUID_BYTE_LEN as usize;
        let header_size = max_capacity_size + items_len_size + value_size + id_len;
        header_size
    }

    fn initialize_empty_file(&mut self) {
        self.data
            .write()
            .unwrap()
            .operate(|file| {
                file.seek(SeekFrom::Start(0))
                    .expect("Failed to seek to start of file");

                // Create a buffer for the header
                let mut buffer = Vec::with_capacity(Self::header_size());

                {
                    // Write max_offsets to the buffer
                    let max_capacity_bytes = (self.max_capacity).unwrap_or(0).to_le_bytes();
                    buffer.extend_from_slice(&max_capacity_bytes);
                }

                {
                    // Write max_offsets to the buffer
                    let items_len_bytes = (self.items_len).to_le_bytes();
                    buffer.extend_from_slice(&items_len_bytes);
                }

                {
                    // Write value_size to the buffer
                    let value_size_bytes = (self.value_size).to_le_bytes();
                    buffer.extend_from_slice(&value_size_bytes);
                }

                {
                    // Write shard id
                    let id_bytes = self.id.to_bytes_le();
                    buffer.extend_from_slice(&id_bytes);
                }

                // Write the buffer to the file
                file.write_all(&buffer)
                    .expect("Failed to write Index header");

                Ok(())
            })
            .unwrap();
    }

    fn read_header(&mut self) {
        let reader = self.data.read().unwrap();
        {
            let max_capacity_bytes = reader.get_bytes(0, U64_SIZE).unwrap();
            let max_capacity_bytes: [u8; 8] = max_capacity_bytes.try_into().unwrap();
            self.max_capacity = Some(u64::from_le_bytes(max_capacity_bytes));
        }

        {
            let items_len_bytes = reader.read_pointer(U64_SIZE as u64, U64_SIZE).unwrap();
            let items_len_bytes: [u8; 8] = items_len_bytes.try_into().unwrap();
            self.items_len = u64::from_le_bytes(items_len_bytes);
        }

        {
            let value_size_bytes = reader
                .read_pointer(
                    U64_SIZE as u64 + U64_SIZE as u64 + U64_SIZE as u64,
                    U64_SIZE,
                )
                .unwrap();
            let value_size_bytes: [u8; 8] = value_size_bytes.try_into().unwrap();
            self.value_size = u64::from_le_bytes(value_size_bytes);
        }

        {
            let id_bytes = reader
                .read_pointer(
                    (U64_SIZE + U64_SIZE + U64_SIZE + U64_SIZE) as u64,
                    UUID_BYTE_LEN as usize,
                )
                .unwrap();
            let id_bytes = id_bytes.try_into().unwrap();
            self.id = Uuid::from_bytes_le(id_bytes);
        }
    }

    pub fn increment_len(&mut self, file: &mut File) -> u64 {
        self.items_len += 1;
        file.write_at(&self.items_len.to_le_bytes(), U64_SIZE as u64)
            .unwrap();

        self.items_len
    }
}
