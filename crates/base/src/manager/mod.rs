pub mod task;
pub mod task_duration;
pub mod tasks;

use crate::manager::task::Task;
use crate::manager::task_duration::TaskDuration;
use schemajs_engine::engine::SchemeJsEngine;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tokio::select;
use tokio_util::sync::CancellationToken;

pub struct SchemeJsManager {
    runtime: Arc<RwLock<SchemeJsEngine>>,
    running: Arc<AtomicBool>,
    tasks: Vec<Task>,
    cancellation_token: CancellationToken,
}

impl SchemeJsManager {
    pub fn new(runtime: Arc<RwLock<SchemeJsEngine>>) -> Self {
        Self {
            runtime,
            running: Arc::new(AtomicBool::new(true)),
            tasks: vec![],
            cancellation_token: CancellationToken::new(),
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn start_tasks(&self) {
        let engine = self.runtime.clone();
        let running = self.running.clone();

        for task in &self.tasks {
            let task_cancel_token = task.cancellation_token.clone();
            let task = task.clone();
            let engine = engine.clone();
            let running = running.clone();
            let cancel_token = self.cancellation_token.clone();
            tokio::spawn(async move {
                select! {
                    _ = cancel_token.cancelled() => {
                    }
                    _ = task_cancel_token.cancelled() => {
                    }
                    _ = Self::run_task(task, engine, running) => {
                    }
                }
            });
        }
    }

    async fn run_task(task: Task, engine: Arc<RwLock<SchemeJsEngine>>, running: Arc<AtomicBool>) {
        match task.duration {
            TaskDuration::Defined(dur) => {
                let mut interval = tokio::time::interval(dur);
                while running.load(Ordering::SeqCst) {
                    interval.tick().await;
                    let clone_rt_ref = engine.clone();
                    let cb = task.func.cb.clone();

                    cb(clone_rt_ref).unwrap_or_else(|_| println!("Error executing task"));
                }
            }
            TaskDuration::Once => {
                let clone_rt_ref = engine.clone();
                let cb = task.func.cb.clone();
                cb(clone_rt_ref).unwrap_or_else(|_| println!("Error executing task"));
            }
        }
    }

    pub fn stop_tasks(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.cancellation_token.cancel();
    }
}
