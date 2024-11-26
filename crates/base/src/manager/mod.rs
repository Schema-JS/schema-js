pub mod task;
pub mod task_duration;
pub mod tasks;

use crate::manager::task::Task;
use crate::manager::task_duration::TaskDuration;
use parking_lot::RwLock;
use schemajs_engine::engine::SchemeJsEngine;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::select;
use tokio_util::sync::CancellationToken;

pub struct SchemeJsManager {
    runtime: Arc<RwLock<SchemeJsEngine>>,
    running: Arc<AtomicBool>,
    tasks: Vec<Task>,
    pub cancellation_token: Arc<CancellationToken>,
    tasks_state: HashMap<String, Arc<AtomicBool>>,
}

impl SchemeJsManager {
    pub fn new(runtime: Arc<RwLock<SchemeJsEngine>>) -> Self {
        Self {
            runtime,
            running: Arc::new(AtomicBool::new(true)),
            tasks: vec![],
            cancellation_token: Arc::new(CancellationToken::new()),
            tasks_state: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks_state
            .insert(task.id.clone(), Arc::new(AtomicBool::new(false)));
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
            let execution_state = self.tasks_state.get(&task.id).unwrap().clone();

            tokio::spawn(async move {
                select! {
                    _ = cancel_token.cancelled() => {
                    }
                    _ = task_cancel_token.cancelled() => {
                    }
                    _ = Self::run_task(task, engine, running, execution_state) => {
                    }
                }
            });
        }
    }

    async fn run_task(
        task: Task,
        engine: Arc<RwLock<SchemeJsEngine>>,
        running: Arc<AtomicBool>,
        task_currently_running: Arc<AtomicBool>,
    ) {
        match task.duration {
            TaskDuration::Defined(dur) => {
                let mut interval = tokio::time::interval(dur);
                while running.load(Ordering::SeqCst) {
                    interval.tick().await; // Wait for the next interval tick
                    let should_run = if task.loop_execution {
                        if task_currently_running.load(Ordering::Acquire) {
                            false // Task already running, skip
                        } else {
                            if task.loop_execution {
                                // Attempt to mark the task as running
                                task_currently_running
                                    .compare_exchange(
                                        false,
                                        true,
                                        Ordering::AcqRel,
                                        Ordering::Acquire,
                                    )
                                    .is_ok()
                            } else {
                                true
                            }
                        }
                    } else {
                        true // Non-looping tasks always run
                    };

                    if should_run {
                        // Execute the task
                        let clone_rt_ref = engine.clone();
                        let cb = task.func.cb.clone();
                        if let Err(err) = cb(clone_rt_ref) {
                            eprintln!("Error executing task: {:?}", err);
                        }

                        // Mark the task as not running after completion
                        if task.loop_execution {
                            task_currently_running.store(false, Ordering::Release);
                        }
                    }
                    tokio::task::yield_now().await
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

#[cfg(test)]
mod task_tests {
    use crate::manager::task::Task;
    use crate::manager::task_duration::TaskDuration;
    use crate::manager::SchemeJsManager;
    use parking_lot::RwLock;
    use schemajs_config::SchemeJsConfig;
    use schemajs_data::fdm::FileDescriptorManager;
    use schemajs_engine::engine::SchemeJsEngine;
    use schemajs_helpers::create_helper_channel;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[flaky_test::flaky_test(tokio)]
    pub async fn test_task_manager() {
        let create_helper = create_helper_channel(1);
        let config = SchemeJsConfig::default();
        let db_engine = Arc::new(RwLock::new(SchemeJsEngine::new(
            None,
            Arc::new(config),
            create_helper.0,
            Arc::new(FileDescriptorManager::new(2500)),
        )));

        let mut task_manager = SchemeJsManager::new(db_engine.clone());
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = counter.clone();

        let task = Task::new(
            "1".to_string(),
            Box::new(move |rt| {
                counter_clone.clone().fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
            TaskDuration::Defined(Duration::from_secs(2)),
            true,
        );

        task_manager.add_task(task);
        task_manager.start_tasks();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        task_manager.stop_tasks();
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        assert_eq!(counter.load(Ordering::Acquire), 3);
    }
}
