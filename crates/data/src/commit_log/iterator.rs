use crate::commit_log::error::CommitLogError;
use crate::commit_log::operations::{CommitLogEntry, CommitLogOperationType};
use crate::cursor::Cursor;
use enum_as_inner::EnumAsInner;

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

        let entry_type = self
            .cursor
            .consume(1)
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let _op_type = CommitLogOperationType::from_bytes(entry_type[0])
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let item_size = self
            .cursor
            .consume(8)
            .map_err(|_| CommitLogError::BrokenRecord)?;
        let item_payload_le_bytes_input: [u8; 8] = item_size
            .try_into()
            .map_err(|_| CommitLogError::BrokenRecord)?;

        let item_payload_usize = usize::from_le_bytes(item_payload_le_bytes_input);
        if item_payload_usize == 0 {
            return Err(CommitLogError::Eof);
        }

        let final_item_length = 1 + 8 + item_payload_usize + 3;

        // In the case that the payload size was written but the full payload was not,
        // This line is necessary to inform the iterator that it has reached an EOF.
        // This would happen if the last item is broken.
        if self.cursor.position + final_item_length > self.cursor.len {
            return Err(CommitLogError::Eof);
        }

        if final_item_length <= self.cursor.len {
            self.cursor.set_back(9); // 1 (Item type, previously consumed) + 8 ( Payload size, previously consumed )
            let full_item = self
                .cursor
                .consume(final_item_length)
                .map_err(|_| CommitLogError::BrokenRecord)?;
            return CommitLogEntry::try_from(full_item);
        }

        Err(CommitLogError::BrokenRecord)
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
    use crate::commit_log::operations::CommitLogEntry;
    use crate::cursor::Cursor;

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

        let iterator = CommitLogIterator::new(&mut cursor);

        let mut i = 0;

        for item in iterator {
            println!("{:?}", item);
            assert!(item.is_valid());
            let data = item.as_valid().unwrap();
            let data = data.data.as_raw().unwrap();
            if i == 0 {
                assert_eq!(data, b"Hello World");
            } else if i == 1 {
                assert_eq!(data, b"Superman");
            } else if i == 2 {
                assert_eq!(data, b"Elon Musk");
            }
            i += 1;
        }
    }

    #[test]
    pub fn test_iterator_first_invalid() {
        // This invalid payload wrote up to the moment of `type` and `payload` but did not write anything after it.
        let item1: Vec<u8> = vec![
            3, // Type "Raw"
            0, 0, 0, 0, 0, 0, 0, 4, // Payload length (4)
        ];

        let item2 = CommitLogEntry::raw(b"Superman");
        let item3 = CommitLogEntry::raw(b"Elon Musk");

        let mut buff = vec![];
        buff.extend_from_slice(&item1.to_vec());
        buff.extend_from_slice(&item2.to_vec());
        buff.extend_from_slice(&item3.to_vec());

        let mut cursor = Cursor::new(&buff);
        let iterator = CommitLogIterator::new(&mut cursor);

        let mut i = 0;

        for item in iterator {
            println!("{:?}", item);
            if i == 0 {
                assert!(item.is_broken());
            } else {
                assert!(item.is_valid());
                let data = item.as_valid().unwrap();
                let data = data.data.as_raw().unwrap();
                if i == 1 {
                    assert_eq!(data, b"Superman");
                } else if i == 2 {
                    assert_eq!(data, b"Elon Musk");
                }
            }
            i += 1;
        }
    }

    #[test]
    pub fn test_iterator_last_invalid() {
        let item1 = CommitLogEntry::raw(b"Hello World");
        let item2 = CommitLogEntry::raw(b"Superman");

        // This invalid payload wrote up to the moment of `type` and `payload` but did not write the full payload
        let item3: Vec<u8> = vec![
            3, // Type "Raw"
            9, 0, 0, 0, 0, 0, 0, 0, // Payload length (9)
            b'E', b'l', b'o', b'n', // Did not finish " Musk"
        ];

        let mut buff = vec![];
        buff.extend_from_slice(&item1.to_vec());
        buff.extend_from_slice(&item2.to_vec());
        buff.extend_from_slice(&item3.to_vec());

        let mut cursor = Cursor::new(&buff);
        let iterator = CommitLogIterator::new(&mut cursor);

        let mut i = 0;

        for item in iterator {
            println!("{:?}", item);
            if i < 2 {
                assert!(item.is_valid());
                let data = item.as_valid().unwrap();
                let data = data.data.as_raw().unwrap();
                if i == 0 {
                    assert_eq!(data, b"Hello World");
                } else if i == 1 {
                    assert_eq!(data, b"Superman");
                }
            } else {
                assert!(item.is_broken());
            }
            i += 1;
        }
    }

    #[test]
    pub fn test_iterator_mid_invalid() {
        let item1 = CommitLogEntry::raw(b"Hello World");

        // This invalid payload wrote up to the moment of `type` and `payload` but did not write the full payload
        let item2: Vec<u8> = vec![
            3, // Type "Raw"
            8, 0, 0, 0, 0, 0, 0, 0, // Payload length (8)
            b'S', b'u', b'p', b'e', b'r', b'm', b'a',
            b'n', // Superman
                  // Did not add magic trail
        ];

        let item3 = CommitLogEntry::raw(b"Elon Musk");

        let mut buff = vec![];
        buff.extend_from_slice(&item1.to_vec());
        buff.extend_from_slice(&item2.to_vec());
        buff.extend_from_slice(&item3.to_vec());

        let mut cursor = Cursor::new(&buff);
        let iterator = CommitLogIterator::new(&mut cursor);

        let mut i = 0;

        for item in iterator {
            if i == 0 || i == 2 {
                let data = item.as_valid().unwrap();
                let data = data.data.as_raw().unwrap();
                assert!(item.is_valid());
                if i == 0 {
                    assert_eq!(data, b"Hello World");
                } else if i == 2 {
                    assert_eq!(data, b"Elon Musk");
                }
            } else {
                assert!(item.is_broken());
            }

            i += 1;
        }
    }
}
