use uuid::Uuid;

pub struct InsertItem<'a> {
    pub data: &'a [u8],
    pub uuid: Uuid,
}

impl<'a> InsertItem<'a> {
    pub fn new(data: &'a [u8], uuid: Uuid) -> Self {
        Self { data, uuid }
    }
}
