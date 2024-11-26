use crate::commit_log::error::CommitLogError;
use crate::commit_log::operations::{
    CommitLogEntry, CommitLogOperationType, END_DELIMITER, START_DELIMITER,
};
use crate::cursor::Cursor;
use crate::utils::is_zero_aligned;
use enum_as_inner::EnumAsInner;
use std::time::Instant;

#[derive(Debug)]
pub struct CommitLogIterator<'a> {
    cursor: &'a mut Cursor<'a>,
}

impl<'a> CommitLogIterator<'a> {
    pub fn new(cursor: &'a mut Cursor<'a>) -> Self {
        Self { cursor }
    }

    fn extract_next_record(&mut self) -> Result<CommitLogEntry, CommitLogError> {
        if self.cursor.is_eof() {
            return Err(CommitLogError::Eof);
        }

        let validate_header = CommitLogEntry::validate_header(self.cursor);

        if let Err(e) = validate_header {
            if !e.is_eof() {
                // Full scan did not find valid entries
                let nearest_entry = self
                    .find_next_start_delimiter_chunked((1024 * 1024) * 4)
                    .ok_or_else(|| CommitLogError::Eof)?;
                self.cursor.move_to(nearest_entry as usize);
                return Err(CommitLogError::BrokenRecord);
            }

            return Err(CommitLogError::Eof);
        }

        let (_item_type, _uuid, payload_size) = validate_header.unwrap();

        let payload_size = payload_size as usize;

        if payload_size == 0 {
            // Find nearest
            return Err(CommitLogError::Eof);
        }

        let already_consumed = START_DELIMITER.len() + 1 + 16 + 8;

        let final_item_length = already_consumed + payload_size + END_DELIMITER.len();

        // In the case that the payload size was written but the full payload was not,
        // This line is necessary to inform the iterator that it has reached an EOF.
        // This would happen if the last item is broken.
        if self.cursor.position + (final_item_length - already_consumed) > self.cursor.len {
            return Err(CommitLogError::Eof);
        }

        if final_item_length <= self.cursor.len {
            self.cursor.set_back(already_consumed);

            // Could potentially include other records, it could extend to other records if the full content wasn't written.
            let full_item = self
                .cursor
                .consume(final_item_length)
                .map_err(|_| CommitLogError::Eof)?;

            let try_read_item = CommitLogEntry::try_from(full_item);
            return match try_read_item {
                Ok(entry) => Ok(entry),
                Err(err) => {
                    self.cursor.set_back(already_consumed);
                    let nearest_entry = self
                        .find_next_start_delimiter_chunked((1024 * 1024) * 4)
                        .ok_or_else(|| CommitLogError::Eof)?;
                    self.cursor.move_to(nearest_entry as usize);
                    Err(CommitLogError::BrokenRecord)
                }
            };
        }

        Err(CommitLogError::BrokenRecord)
    }

    fn find_next_start_delimiter_chunked(&mut self, chunk_size: usize) -> Option<u64> {
        let instant = Instant::now();
        let mut current_position = self.cursor.position;
        let total_length = self.cursor.len;

        while current_position < total_length {
            // Calculate the end of the current chunk
            let end_position = (current_position + chunk_size).min(total_length);

            let chunk = self.cursor.get_range(current_position..end_position);

            // TODO: This could be catastrophic consequences
            // If data was lost, or for some reason badly written with 0s
            // And Chunk=size ended up being 0s but after that there's records
            if is_zero_aligned(chunk) {
                return None;
            }

            // Search the current chunk for the delimiter
            if let Some(pos) = chunk
                .windows(START_DELIMITER.len())
                .position(|window| window == START_DELIMITER)
            {
                // Found the delimiter, return its absolute position
                return Some((current_position + pos) as u64);
            }

            // Move to the next chunk
            // Update current_position safely
            if end_position == total_length {
                // If at the end of the file, advance fully
                current_position = total_length;
            } else {
                // Otherwise, move to the next chunk with overlap
                current_position = end_position - (START_DELIMITER.len() - 1).min(end_position);
            }
        }

        None // No delimiter found
    }
}

#[derive(EnumAsInner, Debug)]
pub enum CommitLogIteratorItem {
    Valid(CommitLogEntry),
    Broken,
}

impl<'a> Iterator for CommitLogIterator<'a> {
    type Item = CommitLogIteratorItem;

    fn next(&mut self) -> Option<Self::Item> {
        let record = self.extract_next_record();
        match record {
            Ok(item) => Some(CommitLogIteratorItem::Valid(item)),
            Err(e) => {
                return if e.is_eof() {
                    None
                } else {
                    Some(CommitLogIteratorItem::Broken)
                }
            }
        }
    }
}

#[cfg(test)]
mod commit_log_iterator_tests {
    use crate::commit_log::iterator::CommitLogIterator;
    use crate::commit_log::operations::{CommitLogEntry, START_DELIMITER};
    use crate::cursor::Cursor;
    use uuid::Uuid;

    #[test]
    pub fn test_iterator_all_valid() {
        let item1 = CommitLogEntry::raw(b"Hello World");
        let item2 = CommitLogEntry::raw(b"Superman");
        let item3 = CommitLogEntry::raw(b"Elon Musk");

        let mut buff = vec![];
        buff.extend_from_slice(&item1.to_vec());
        buff.extend_from_slice(&item2.to_vec());
        buff.extend_from_slice(&item3.to_vec());

        let mut cursor = Cursor::new(&buff);
        let mut iterator = CommitLogIterator::new(&mut cursor);

        // Process first item
        let item = iterator.next().unwrap();
        assert!(item.is_valid());
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert_eq!(data, b"Hello World");

        // Process second item
        let item = iterator.next().unwrap();
        assert!(item.is_valid());
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert_eq!(data, b"Superman");

        // Process third item
        let item = iterator.next().unwrap();
        assert!(item.is_valid());
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert_eq!(data, b"Elon Musk");

        // Ensure no more items exist
        assert!(iterator.next().is_none());
    }

    #[test]
    pub fn test_iterator_first_invalid() {
        // This invalid payload wrote up to the moment of `type` and `payload` but did not write anything after it.
        let mut item1: Vec<u8> = vec![];
        item1.extend_from_slice(START_DELIMITER);
        item1.extend(vec![
            3, // Type "Raw",
            172, 123, 137, 130, 138, 122, 175, 64, 187, 243, 53, 224, 201, 21, 117,
            100, // 16 ITEM UUID,
            4, 0, 0, 0, 0, 0, 0, 0, // Payload length (4)
        ]);

        let item2 = CommitLogEntry::raw(b"Superman");
        let item3 = CommitLogEntry::raw(b"Elon Musk");

        let mut buff = vec![];
        buff.extend_from_slice(&item1.to_vec());
        buff.extend_from_slice(&item2.to_vec());
        buff.extend_from_slice(&item3.to_vec());

        let mut cursor = Cursor::new(&buff);
        let mut iterator = CommitLogIterator::new(&mut cursor);

        // Process first item (broken)
        let item = iterator.next().unwrap();
        assert!(item.is_broken());

        // Process second item
        let item = iterator.next().unwrap();
        assert!(item.is_valid());
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert_eq!(data, b"Superman");

        // Process third item
        let item = iterator.next().unwrap();
        assert!(item.is_valid());
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert_eq!(data, b"Elon Musk");

        // Ensure no more items exist
        assert!(iterator.next().is_none());
    }

    #[test]
    pub fn test_iterator_last_invalid() {
        let item1 = CommitLogEntry::raw(b"Hello World");
        let item2 = CommitLogEntry::raw(b"Superman");

        // This invalid payload wrote up to the moment of `type` and `payload` but did not write the full payload
        let item3: Vec<u8> = vec![
            3, // Type "Raw"
            172, 123, 137, 130, 138, 122, 175, 64, 187, 243, 53, 224, 201, 21, 117,
            100, // 16 ITEM UUID,
            9, 0, 0, 0, 0, 0, 0, 0, // Payload length (9)
            b'E', b'l', b'o', b'n', // Did not finish " Musk"
        ];

        let mut buff = vec![];
        buff.extend_from_slice(&item1.to_vec());
        buff.extend_from_slice(&item2.to_vec());
        buff.extend_from_slice(&item3.to_vec());

        let mut cursor = Cursor::new(&buff);
        let mut iterator = CommitLogIterator::new(&mut cursor);

        // Process first item
        let item = iterator.next().unwrap();
        assert!(item.is_valid());
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert_eq!(data, b"Hello World");

        // Process second item
        let item = iterator.next().unwrap();
        assert!(item.is_valid());
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert_eq!(data, b"Superman");

        // Process third item (broken)
        let item = iterator.next();
        assert!(item.is_none());
    }

    #[test]
    pub fn test_iterator_invalid_item() {
        // This invalid payload wrote up to the moment of `type` and `payload` but did not write the full payload
        let item1: Vec<u8> = vec![
            3, // Type "Raw"
            172, 123, 137, 130, 138, 122, 175, 64, 187, 243, 53, 224, 201, 21, 117,
            100, // 16 ITEM UUID,
            8, 0, 0, 0, 0, 0, 0, 0, // Payload length (8)
            b'S', b'u', b'p', b'e', b'r', b'm', b'a',
            b'n', // Superman
                  // Did not add magic trail
        ];

        let mut buff = vec![];
        buff.extend_from_slice(&item1.to_vec());

        let mut cursor = Cursor::new(&buff);
        let mut iterator = CommitLogIterator::new(&mut cursor);
        let next = iterator.next();
        assert!(next.is_none());
    }

    #[test]
    pub fn test_iterator_mid_invalid() {
        let item1 = CommitLogEntry::raw(b"Hello World");

        // This invalid payload wrote up to the moment of `type` and `payload` but did not write the full payload
        let mut item2 = vec![];
        item2.extend_from_slice(START_DELIMITER);
        item2.extend_from_slice(&vec![
            3, // Type "Raw"
            172, 123, 137, 130, 138, 122, 175, 64, 187, 243, 53, 224, 201, 21, 117,
            100, // 16 ITEM UUID,
            8, 0, 0, 0, 0, 0, 0, 0, // Payload length (8)
            b'S', b'u', b'p', b'e', b'r', b'm', b'a',
            b'n', // Superman
                  // Did not add magic trail
        ]);

        let item3 = CommitLogEntry::raw(b"Elon Musk");

        let mut buff = vec![];
        buff.extend_from_slice(&item1.to_vec());
        buff.extend_from_slice(&item2.to_vec());
        buff.extend_from_slice(&item3.to_vec());

        let mut cursor = Cursor::new(&buff);
        let mut iterator = CommitLogIterator::new(&mut cursor);

        // Process first item
        let item = iterator.next().unwrap();
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert!(item.is_valid());
        assert_eq!(data, b"Hello World");

        // Process second item (broken)
        let item = iterator.next().unwrap();
        assert!(item.is_broken());

        // Process third item
        let item = iterator.next().unwrap();
        let data = item.as_valid().unwrap();
        let data = data.data.as_raw().unwrap();
        assert!(item.is_valid());
        assert_eq!(data, b"Elon Musk");

        // Ensure no more items exist
        assert!(iterator.next().is_none());
    }
}
