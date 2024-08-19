use crate::manager::task_duration::TaskDuration;
use schemajs_engine::engine::SchemeJsEngine;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;

pub type TaskSignature = Box<dyn Fn(Arc<SchemeJsEngine>) -> Result<(), ()> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct TaskCallback {
    pub cb: Arc<TaskSignature>,
}

impl Debug for TaskCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Function pointer")
    }
}

#[derive(Clone)]
pub struct Task {
    pub id: String,
    pub func: TaskCallback,
    pub duration: TaskDuration,
    pub cancellation_token: CancellationToken,
}

impl Task {
    pub fn new(id: String, func: TaskSignature, task_duration: TaskDuration) -> Self {
        Self {
            id,
            func: TaskCallback { cb: Arc::new(func) },
            duration: task_duration,
            cancellation_token: CancellationToken::new(),
        }
    }
}
