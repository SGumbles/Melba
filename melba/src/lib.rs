use std::{future::Future, time::Duration};

mod executor;
mod sleep;
mod task;

use crate::sleep::Sleep;
use executor::Executor;

thread_local! {
    static CURRENT_EXECUTOR: Executor = Executor::new();
}

pub fn spawn(future: impl Future<Output = ()> + 'static) {
    #[cfg(feature = "dev-logs")]
    log::debug!("spawning a task");
    CURRENT_EXECUTOR.with(|f| {
        f.add_task(future);
    });
}

pub fn sleep(duration: Duration) -> impl Future {
    let owning_task =
        CURRENT_EXECUTOR.with(|f| f.get_currently_running_task().expect("No task is running?"));
    let task_id = owning_task.upgrade().map(|t| t.id()).unwrap_or(0);
    #[cfg(feature = "dev-logs")]
    log::debug!(
        "task {} : creating sleep future for {:?}",
        task_id,
        duration
    );
    Sleep::new(duration, owning_task, task_id)
}
