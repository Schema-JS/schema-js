use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum QueryResult {
    Insert(InsertResult),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertResult {
    pub last_uuid: Option<Uuid>,
    pub succeeded: bool,
    pub failed_items: Vec<usize>, // Indexes of the items that were passed in Vec<InsertItem>
    pub duration: Duration,
}
