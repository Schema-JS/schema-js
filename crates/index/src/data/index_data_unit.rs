use crate::errors::IndexError;
use schemajs_data::data_handler::DataHandler;
use schemajs_data::U64_SIZE;

pub struct IndexDataUnit {
    pub item_size: u64,
    pub data: Vec<u8>,
}

impl IndexDataUnit {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            item_size: data.len() as u64,
            data,
        }
    }

    pub fn from_data_handler(offset: u64, data_handler: &DataHandler) -> Option<Self> {
        let mut index = IndexDataUnit::new(vec![]);
        let item_size = data_handler.read_pointer(offset, U64_SIZE);

        if let Some(get_item_size) = item_size {
            let item_size_bytes: [u8; 8] = get_item_size.try_into().unwrap();
            index.item_size = u64::from_le_bytes(item_size_bytes);

            let read_data = {
                data_handler
                    .read_pointer(offset + U64_SIZE as u64, index.item_size as usize)
                    .unwrap()
            };

            index.data = read_data;

            return Some(index);
        }

        None
    }

    pub fn header_size() -> usize {
        U64_SIZE // item_size
    }
}

impl TryFrom<&[u8]> for IndexDataUnit {
    type Error = IndexError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let mut unit = IndexDataUnit::new(vec![]);

        let item_size = data.get(0..U64_SIZE);
        return match item_size {
            None => Err(Self::Error::UnrecognizedItemSize),
            Some(size) => {
                let item_size_bytes: [u8; 8] = size.try_into().unwrap();
                unit.item_size = u64::from_le_bytes(item_size_bytes);

                if let Some(data) = data.get(U64_SIZE..(U64_SIZE + unit.item_size as usize)) {
                    unit.data = data.to_vec();
                    Ok(unit)
                } else {
                    Err(Self::Error::InvalidItem)
                }
            }
        };
    }
}

impl Into<Vec<u8>> for IndexDataUnit {
    fn into(self) -> Vec<u8> {
        let mut entry = vec![];

        entry.extend(self.item_size.to_le_bytes());
        entry.extend(self.data.as_slice());

        entry
    }
}
