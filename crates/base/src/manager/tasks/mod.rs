use crate::manager::task::Task;
use crate::manager::tasks::reconcile_task::RECONCILE_DB_TASK;

mod reconcile_task;

pub fn get_all_internal_tasks() -> Vec<Task> {
    vec![(*RECONCILE_DB_TASK).clone()]
}
