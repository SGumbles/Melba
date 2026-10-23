use std::{
    collections::VecDeque,
    future::Future,
    task::{Context, Waker},
};

use crate::task::Task;

pub struct Executor {
    tasks: VecDeque<Task>,
    is_running: bool,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: VecDeque::new(),
            is_running: false,
        }
    }

    pub fn add_task(&mut self, future: impl Future<Output = ()> + 'static + Send) {
        self.tasks.push_back(Task::new(future));
        if !self.is_running {
            self.run();
        }
    }

    fn run(&mut self) {
        self.is_running = true;
        let mut context = Context::from_waker(Waker::noop());
        while let Some(mut next_task) = self.tasks.pop_front() {
            match next_task.poll(&mut context) {
                std::task::Poll::Ready(_) => {}
                std::task::Poll::Pending => self.tasks.push_back(next_task),
            }
        }
        self.is_running = false;
    }
}
