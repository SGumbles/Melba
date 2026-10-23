use std::{cell::RefCell, future::Future, time::Duration};

mod executor;
mod sleep;
mod task;

use executor::Executor;

use crate::sleep::Sleep;

thread_local! {
    static CURRENT_EXECUTOR: RefCell<Executor> = RefCell::new(Executor::new());
}

pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
    CURRENT_EXECUTOR.with(|f| {
        f.borrow_mut().add_task(future);
    });
}

pub fn sleep(duration: Duration) -> impl Future {
    Sleep::new(duration)
}
