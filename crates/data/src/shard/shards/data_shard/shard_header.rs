use crate::data_handler::DataHandler;
use crate::errors::ShardErrors;
use crate::{I64_SIZE, U64_SIZE};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub const DEFAULT_MAX_OFFSETS: u64 = 100;
pub const UUID_BYTE_LEN: u64 = 16;

// TODO: Header version

#[derive(Debug)]
pub struct DataShardHeader {
    max_offsets: u64,
    last_offset_index: i64, // Even though this is realistically a u64, we use i64 because if everything is empty, it will be -1 which can't be with u64
    pub max_offset_positions: usize,
    pub id: Uuid,
    data: Arc<RwLock<DataHandler>>,
}

impl DataShardHeader {
    pub fn new(max_offsets: u64, uuid: Option<Uuid>, data: Arc<RwLock<DataHandler>>) -> Self {
        Self {
            max_offsets,
            last_offset_index: -1,
            id: uuid.unwrap_or_else(|| Uuid::new_v4()),
            max_offset_positions: Self::calculate_offset_pos(max_offsets as usize),
            data,
        }
    }

    pub fn get_max_offsets(&self) -> u64 {
        self.max_offsets
    }

    pub fn new_from_file(
        file: Arc<RwLock<DataHandler>>,
        max_offsets: Option<u64>,
        uuid: Option<Uuid>,
    ) -> Self {
        let mut header = DataShardHeader::new(
            max_offsets.unwrap_or(DEFAULT_MAX_OFFSETS),
            uuid,
            file.clone(),
        );

        // Check if the file is empty
        let metadata = file
            .read()
            .unwrap()
            .metadata()
            .expect("Failed to get file metadata");
        if metadata.len() == 0 {
            header.initialize_empty_file();
        } else {
            header.read_header();
        }

        header
    }

    /// Initializes an empty file with max_offsets and zeroed offsets
    fn initialize_empty_file(&mut self) {
        self.data
            .write()
            .unwrap()
            .operate(|file| {
                file.seek(SeekFrom::Start(0))
                    .expect("Failed to seek to start of file");

                // Header Items
                // Keep this in the order of the struct
                let max_offsets_size = U64_SIZE;
                let last_offset_index_size = I64_SIZE;
                let offsets_size = ((self.max_offsets as usize) * U64_SIZE);
                let id_len = UUID_BYTE_LEN as usize;

                // Calculate header size
                let header_size = max_offsets_size + last_offset_index_size + offsets_size + id_len;

                // Create a buffer for the header
                let mut buffer = Vec::with_capacity(header_size);

                {
                    // Write max_offsets to the buffer
                    let max_offsets_bytes = (self.max_offsets).to_le_bytes();
                    buffer.extend_from_slice(&max_offsets_bytes);
                }

                {
                    // Write last_used_offset to the buffer
                    let last_offset_index = (self.last_offset_index).to_le_bytes();
                    buffer.extend_from_slice(&last_offset_index);
                }

                {
                    // Write shard id
                    let id_bytes = self.id.to_bytes_le();
                    buffer.extend_from_slice(&id_bytes);
                }

                {
                    // Pre-allocate space for offsets by writing zeroed bytes
                    let zero_bytes = vec![0u8; offsets_size];
                    buffer.extend_from_slice(&zero_bytes);
                }

                // Write the buffer to the file
                file.write_all(&buffer).expect("Failed to write header");

                Ok(())
            })
            .unwrap()
    }

    /// Reads the header (max_offsets and offsets) from the file
    fn read_header(&mut self) {
        let reader = self.data.read().unwrap();
        {
            let max_offset_bytes = reader.get_bytes(0, U64_SIZE).unwrap();
            let max_offset_bytes: [u8; 8] = max_offset_bytes.try_into().unwrap();
            self.max_offsets = u64::from_le_bytes(max_offset_bytes);
        }

        self.max_offset_positions = Self::calculate_offset_pos(self.max_offsets as usize);

        {
            let last_offset_index_bytes = reader.read_pointer(U64_SIZE as u64, I64_SIZE).unwrap();
            let last_offset_index_bytes: [u8; 8] = last_offset_index_bytes.try_into().unwrap();
            self.last_offset_index = i64::from_le_bytes(last_offset_index_bytes);
        }

        {
            let id_bytes = reader
                .read_pointer((U64_SIZE + I64_SIZE) as u64, UUID_BYTE_LEN as usize)
                .unwrap();
            let id_bytes = id_bytes.try_into().unwrap();
            self.id = Uuid::from_bytes_le(id_bytes);
        }
    }

    fn calculate_offset_pos(index: usize) -> usize {
        let max_offsets = U64_SIZE;
        let last_used_offset = I64_SIZE;
        let id_len = UUID_BYTE_LEN as usize;
        let offsets_from_pos = index * U64_SIZE;

        max_offsets + last_used_offset + id_len + offsets_from_pos
    }

    pub fn add_next_offset(&mut self, value: u64, file: &mut File) -> Result<(), ShardErrors> {
        if let Some(available_index) = self.get_next_available_index() {
            // Write the new offset value to the file
            let offset_position = self.get_offset_pos_by_index(available_index);
            match offset_position {
                None => return Err(ShardErrors::OutOfPositions),
                Some(pos) => {
                    let offset_bytes = value.to_le_bytes();
                    file.write_at(&offset_bytes, pos as u64)
                        .expect("Failed to write offset to file");
                    self.last_offset_index = available_index as i64;
                    Ok(())
                }
            }
        } else {
            Err(ShardErrors::OutOfPositions)
        }
    }

    pub fn get_next_available_index(&self) -> Option<usize> {
        // Has not been initialized
        let has_space = self.has_space();

        if has_space {
            if self.last_offset_index == -1 {
                return Some(0);
            } else {
                return Some(self.last_offset_index as usize + 1);
            }
        }

        None
    }

    pub fn has_space(&self) -> bool {
        if self.last_offset_index == -1 {
            true
        } else {
            // Index count is not the same as .len()
            // Therefore max_offsets might be 10
            // While last_offset_index is 9
            // But theoretically, there's already 10 items
            self.max_offsets > (self.last_offset_index + 1) as u64
        }
    }

    pub fn get_offset_value_from_offset_header(&self, offset: usize) -> Option<u64> {
        // Read the pointer
        let bytes = match self
            .data
            .read()
            .unwrap()
            .read_pointer(offset as u64, U64_SIZE)
        {
            Some(bytes) => bytes,
            None => return None,
        };

        // Convert Vec<u8> to [u8; 8]
        let arr: [u8; 8] = match bytes.try_into() {
            Ok(arr) => arr,
            Err(_) => return None,
        };

        // Convert the byte array to u64
        let val = u64::from_le_bytes(arr);
        Some(val)
    }

    pub fn get_offset_pos_by_index(&self, index: usize) -> Option<usize> {
        let pos = Self::calculate_offset_pos(index);
        if self.max_offset_positions > pos {
            Some(pos)
        } else {
            None
        }
    }

    pub fn get_last_offset_index(&self) -> i64 {
        self.last_offset_index
    }
}
