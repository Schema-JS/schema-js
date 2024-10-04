use crate::manager::task::Task;
use crate::manager::task_duration::TaskDuration;
use std::cell::LazyCell;
use std::time::Duration;

pub const RECONCILE_DB_TASK: LazyCell<Task> = LazyCell::new(|| {
    Task::new(
        "1".to_string(),
        Box::new(move |rt| {
            let engine = rt.write().unwrap();
            for db in engine.databases.iter() {
                let query_manager = &db.query_manager;
                for table in query_manager.table_names.read().unwrap().iter() {
                    let table = query_manager.tables.get(table).unwrap();
                    table.temps.reconcile_all();
                }
            }
            Ok(())
        }),
        TaskDuration::Defined(Duration::from_millis(250)),
    )
});
