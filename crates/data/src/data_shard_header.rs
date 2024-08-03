use crate::errors::DataShardErrors;
use crate::U64_SIZE;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_MAX_OFFSETS: u64 = 100;
pub const UUID_BYTE_LEN: u64 = 16;

// TODO: Header version

#[derive(Debug, Serialize, Deserialize)]
pub struct DataShardHeader {
    max_offsets: u64,
    pub id: Uuid,
    pub offsets: Vec<u64>,
}

impl DataShardHeader {
    pub fn new(max_offsets: u64) -> Self {
        Self {
            offsets: vec![0; max_offsets as usize],
            max_offsets,
            id: Uuid::new_v4()
        }
    }

    pub fn get_max_offsets(&self) -> u64 {
        self.max_offsets
    }

    pub fn new_from_file(file: &mut File, max_offsets: Option<u64>) -> Self {
        let mut header = DataShardHeader::new(max_offsets.unwrap_or(DEFAULT_MAX_OFFSETS));

        // Check if the file is empty
        let metadata = file.metadata().expect("Failed to get file metadata");
        if metadata.len() == 0 {
            header.initialize_empty_file(file);
        } else {
            header.read_header(file);
        }

        header
    }

    /// Initializes an empty file with max_offsets and zeroed offsets
    fn initialize_empty_file(&self, file: &mut File) {
        file.seek(SeekFrom::Start(0))
            .expect("Failed to seek to start of file");

        // Header Items
        // Keep this in the order of the struct
        let max_offsets_size = U64_SIZE;
        let offsets_size = ((self.max_offsets as usize) * U64_SIZE);
        let id_len = UUID_BYTE_LEN as usize;

        // Calculate header size
        let header_size = max_offsets_size + offsets_size + id_len;

        // Create a buffer for the header
        let mut buffer = Vec::with_capacity(header_size);

        {
            // Write max_offsets to the buffer
            let max_offsets_bytes = (self.max_offsets as u64).to_le_bytes();
            buffer.extend_from_slice(&max_offsets_bytes);
        }

        {
            // Write shard id
            let id_bytes = self.id.to_bytes_le();
            buffer.extend_from_slice(&id_bytes);
        }

        {
            // Write offsets to the buffer
            for offset in &self.offsets {
                let offset_bytes = offset.to_le_bytes();
                buffer.extend_from_slice(&offset_bytes);
            }
        }

        // Write the buffer to the file
        file.write_all(&buffer).expect("Failed to write header");
    }

    /// Reads the header (max_offsets and offsets) from the file
    fn read_header(&mut self, file: &mut File) {
        file.seek(SeekFrom::Start(0))
            .expect("Failed to seek to start of file");

        {
            // Read max_offsets
            let mut max_offsets_bytes = [0u8; U64_SIZE];
            file.read_exact(&mut max_offsets_bytes)
                .expect("Failed to read max_offsets");
            self.max_offsets = u64::from_le_bytes(max_offsets_bytes);
        }

        {
            let mut max_offsets_bytes = [0u8; UUID_BYTE_LEN as usize];
            file.read_exact(&mut max_offsets_bytes)
                .expect("Failed to read max_offsets");
            self.id = Uuid::from_bytes_le(max_offsets_bytes);
        }

        {
            // Read offsets
            let mut header = vec![0u8; (self.max_offsets as usize) * U64_SIZE];
            file.read_exact(&mut header).expect("Failed to read header");

            self.offsets = header
                .chunks(U64_SIZE)
                .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
                .collect();
        }
    }

    pub fn add_next_offset(&mut self, file: &mut File, value: u64) -> Result<(), DataShardErrors> {
        if let Some(position) = self.get_next_available_pos() {
            self.offsets[position] = value;

            // Seek to the position in the file where the offset should be written
            let offset_position = self.calculate_offset_pos(position);

            // Write the new offset value to the file
            let offset_bytes = value.to_le_bytes();
            file.write_at(&offset_bytes, offset_position as u64)
                .expect("Failed to write offset to file");

            Ok(())
        } else {
            Err(DataShardErrors::OutOfPositions)
        }
    }

    pub fn get_next_available_pos(&self) -> Option<usize> {
        self.offsets.iter().position(|&offset| offset == 0)
    }

    pub fn get_offset_pos(&self, offset: u64) -> Result<usize, DataShardErrors> {
        match self.offsets.iter().position(|&i| i == offset) {
            None => Err(DataShardErrors::UnknownOffset),
            Some(pos) => Ok(pos),
        }
    }

    fn calculate_offset_pos(&self, position: usize) -> usize {
        let max_offsets = U64_SIZE;
        let id_len = UUID_BYTE_LEN as usize;
        let offsets_from_pos = position * U64_SIZE;

        max_offsets + id_len + offsets_from_pos
    }
}
