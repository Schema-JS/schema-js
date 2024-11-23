use crate::cursor::error::CursorError;
use memmap2::{Mmap, MmapMut};
use std::ops::Range;

mod error;

#[derive(Debug)]
pub enum CursorData<'a> {
    Raw(&'a [u8]),
    Mmap(&'a Mmap),
    MmapMut(&'a MmapMut),
}

#[derive(Debug)]
pub struct Cursor<'a> {
    data: CursorData<'a>,
    pub position: usize,
    pub last_consumed_size: usize,
    pub len: usize,
    pub starting_pos: Option<usize>,
}

impl<'a> Cursor<'a> {
    pub fn raw(data: &'a [u8]) -> Self {
        Cursor {
            data: CursorData::Raw(data),
            position: 0,
            len: data.len(),
            starting_pos: None,
            last_consumed_size: 0,
        }
    }

    pub fn mmap(data: &'a Mmap) -> Self {
        Cursor {
            data: CursorData::Mmap(data),
            position: 0,
            len: data.len(),
            starting_pos: None,
            last_consumed_size: 0,
        }
    }

    pub fn mmap_mut(data: &'a MmapMut) -> Self {
        Cursor {
            data: CursorData::MmapMut(data),
            position: 0,
            len: data.len(),
            starting_pos: None,
            last_consumed_size: 0,
        }
    }

    pub fn set_starting_pos(mut self, pos: usize) -> Self {
        self.starting_pos = Some(pos);
        self.position = pos;
        self
    }

    pub fn new(data: &'a [u8]) -> Self {
        Self::raw(data)
    }

    pub fn get_range(&self, range: Range<usize>) -> &'a [u8] {
        let data = match self.data {
            CursorData::Raw(data) => &data[range],
            CursorData::Mmap(data) => &data[range],
            CursorData::MmapMut(data) => &data[range],
        };

        data
    }

    pub fn peek(&self, size: usize) -> Result<&'a [u8], CursorError> {
        if self.position + size > self.len {
            return Err(CursorError::InvalidRange);
        }

        let range = self.position..(self.position + size);

        let data = match self.data {
            CursorData::Raw(data) => &data[range],
            CursorData::Mmap(data) => &data[range],
            CursorData::MmapMut(data) => &data[range],
        };

        Ok(data)
    }

    pub fn consume(&mut self, size: usize) -> Result<&'a [u8], CursorError> {
        let data = self.peek(size)?;
        self.position += size;
        self.last_consumed_size = size;

        Ok(data)
    }

    pub fn set_back(&mut self, steps: usize) {
        self.position = self.position - steps;
    }

    pub fn forward(&mut self, steps: usize) {
        self.position = self.position + steps;
    }

    pub fn move_to(&mut self, pos: usize) {
        self.position = pos;
    }

    pub fn reset(&mut self) {
        self.last_consumed_size = 0;
        self.position = self.starting_pos.unwrap_or(0);
    }

    pub fn is_eof(&self) -> bool {
        self.position >= self.len
    }
}

#[cfg(test)]
mod cursor_tests {
    use crate::cursor::Cursor;

    #[test]
    pub fn test() {
        let bytes = b"Hello World";
        let mut cursor = Cursor::new(bytes);
        let hello = cursor.consume(5).unwrap();
        assert_eq!(hello, b"Hello");
        let world = cursor.consume(6).unwrap();
        assert_eq!(world, b" World");
        let out_of_range = cursor.consume(1);
        assert!(out_of_range.err().unwrap().is_invalid_range());
    }
}
