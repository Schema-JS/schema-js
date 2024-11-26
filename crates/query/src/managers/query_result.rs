use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, EnumAsInner)]
pub enum QueryResult {
    Insert(InsertResult),
    Delete(DeleteResult),
    Update(UpdateResult),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertResult {
    pub last_uuid: Option<Uuid>,
    pub succeeded: bool,
    pub failed_items: Vec<usize>, // Indexes of the items that were passed in Vec<InsertItem>
    pub duration: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResult {
    pub succeeded: bool,
    pub failed_items: Vec<usize>,
    pub duration: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateResult {
    pub succeeded: bool,
    pub failed_items: Vec<usize>,
    pub duration: Duration,
}
