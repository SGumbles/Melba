use std::{future::Future, pin::Pin, task::Context, time::Instant};

#[derive(Copy, Clone)]
enum TaskState {
    WaitingToBeRun,
    Running,
    Sleeping { wake_time: Instant },
}

pub struct Task {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    state: TaskState,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static + Send) -> Self {
        Task {
            future: Box::pin(future),
            state: TaskState::WaitingToBeRun,
        }
    }
    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }
    pub fn get_state(&self) -> TaskState {
        self.state
    }
    pub fn poll(&mut self, ctx: &mut Context) -> core::task::Poll<()> {
        self.future.as_mut().poll(ctx)
    }
}
