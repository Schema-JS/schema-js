use crate::data_handler::DataHandler;
use crate::U64_SIZE;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct IndexShardHeader {
    pub max_capacity: u64,
    pub items_len: u64,
    data: Arc<RwLock<DataHandler>>,
}

impl IndexShardHeader {
    pub fn new(items_len: u64, max_capacity: u64, data: Arc<RwLock<DataHandler>>) -> Self {
        Self {
            items_len,
            max_capacity,
            data,
        }
    }

    pub fn new_from_file(
        file: Arc<RwLock<DataHandler>>,
        items_len: Option<u64>,
        max_capacity: Option<u64>,
    ) -> Self {
        let mut header = IndexShardHeader::new(
            items_len.unwrap_or(0),
            max_capacity.unwrap_or(0),
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
        let header_size = max_capacity_size + items_len_size;
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
                    let max_capacity_bytes = (self.max_capacity).to_le_bytes();
                    buffer.extend_from_slice(&max_capacity_bytes);
                }

                {
                    // Write max_offsets to the buffer
                    let items_len_bytes = (self.items_len).to_le_bytes();
                    buffer.extend_from_slice(&items_len_bytes);
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
            self.max_capacity = u64::from_le_bytes(max_capacity_bytes);
        }

        {
            let items_len_bytes = reader.read_pointer(U64_SIZE as u64, U64_SIZE).unwrap();
            let items_len_bytes: [u8; 8] = items_len_bytes.try_into().unwrap();
            self.items_len = u64::from_le_bytes(items_len_bytes);
        }
    }

    pub fn increment_len(&mut self, file: &mut File) -> u64 {
        self.items_len += 1;
        file.write_at(&self.items_len.to_le_bytes(), U64_SIZE as u64)
            .unwrap();

        self.items_len
    }
}
