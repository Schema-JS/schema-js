use std::time::Duration;

#[derive(Clone)]
pub enum TaskDuration {
    Defined(Duration),
    Once,
}
