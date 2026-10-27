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
    CURRENT_EXECUTOR.with(|f| {
        f.borrow().add_task(future);
    });
}

pub fn sleep(duration: Duration) -> impl Future {
    Sleep::new(
        duration,
        CURRENT_EXECUTOR.with(|f| {
            f.borrow()
                .get_currently_running_task()
                .expect("No task is running?")
        }),
    )
}
