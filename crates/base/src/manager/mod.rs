pub mod task;
pub mod task_duration;

use crate::manager::task::Task;
use crate::manager::task_duration::TaskDuration;
use schemajs_engine::engine::SchemeJsEngine;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

pub struct SchemeJsManager {
    runtime: Arc<RwLock<SchemeJsEngine>>,
    running: Arc<AtomicBool>,
    tasks: Vec<Task>,
}

impl SchemeJsManager {
    pub fn new(runtime: Arc<RwLock<SchemeJsEngine>>) -> Self {
        Self {
            runtime,
            running: Arc::new(AtomicBool::new(true)),
            tasks: vec![],
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn start_tasks(&self) {
        let engine = self.runtime.clone();
        let running = self.running.clone();

        for task in &self.tasks {
            let task = task.clone();
            let engine = engine.clone();
            let running = running.clone();

            tokio::spawn(async move {
                match task.duration {
                    TaskDuration::Defined(dur) => {
                        let mut interval = tokio::time::interval(dur);
                        while running.load(Ordering::Relaxed) {
                            interval.tick().await;
                            let clone_rt_ref = engine.clone();
                            let cb = task.func.cb.clone();

                            tokio::spawn(async move {
                                cb(clone_rt_ref)
                                    .unwrap_or_else(|_| println!("Error executing task"));
                            });
                        }
                    }
                    TaskDuration::Once => {
                        let clone_rt_ref = engine.clone();
                        let cb = task.func.cb.clone();
                        tokio::spawn(async move {
                            cb(clone_rt_ref).unwrap_or_else(|_| println!("Error executing task"));
                        });
                    }
                }
            });
        }
    }

    pub fn stop_tasks(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
