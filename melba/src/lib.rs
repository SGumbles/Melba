use std::{cell::RefCell, future::Future, rc::Rc, time::Duration};

mod executor;
mod sleep;
mod task;

use executor::Executor;

use crate::sleep::Sleep;

thread_local! {
    static CURRENT_EXECUTOR: Rc<RefCell<Executor>> = Rc::new(RefCell::new(Executor::new()));
}

pub fn spawn(future: impl Future<Output = ()> + 'static) {
    #[cfg(feature = "dev-logs")]
    log::debug!("spawning a task");
    CURRENT_EXECUTOR.with(|f| {
        f.borrow().add_task(future);
    });
}

pub fn sleep(duration: Duration) -> impl Future {
    #[cfg(feature = "dev-logs")]
    log::trace!("creating sleep future for {:?}", duration);
    let owning_task = CURRENT_EXECUTOR.with(|f| {
        f.borrow()
            .get_currently_running_task()
            .expect("No task is running?")
    });
    let task_id = owning_task.upgrade().map(|t| t.id()).unwrap_or(0);
    Sleep::new(duration, owning_task, task_id)
}
