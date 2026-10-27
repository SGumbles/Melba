use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    future::Future,
    rc::{Rc, Weak},
    task::{Context, Waker},
};

use crate::task::{Task, TaskState};

pub struct Executor {
    tasks: RefCell<VecDeque<Rc<Task>>>,
    is_running: Cell<bool>,
    currently_running_task: RefCell<Option<Weak<Task>>>,
}

impl Executor {
    pub(crate) fn new() -> Self {
        Executor {
            tasks: RefCell::new(VecDeque::new()),
            is_running: Cell::new(false),
            currently_running_task: RefCell::new(None),
        }
    }

    pub(crate) fn add_task(&self, future: impl Future<Output = ()> + 'static) {
        self.tasks
            .borrow_mut()
            .push_back(Rc::new(Task::new(future)));
        if !self.is_running.get() {
            self.run();
        }
    }

    pub(crate) fn get_currently_running_task(&self) -> Option<Weak<Task>> {
        self.currently_running_task.borrow().clone()
    }

    fn run(&self) {
        self.is_running.set(true);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            loop {
                let Some(next_task) = self
                    .tasks
                    .borrow_mut()
                    .pop_front_if(|t| t.get_state() == TaskState::WaitingToBeRun)
                else {
                    break;
                };
                *self.currently_running_task.borrow_mut() = Some(Rc::downgrade(&next_task));
                match next_task.poll(&mut context) {
                    std::task::Poll::Ready(_) => {}
                    std::task::Poll::Pending => self.tasks.borrow_mut().push_back(next_task),
                }
            }
            if self.tasks.borrow().is_empty() {
                break;
            }
            let wake_time = self
                .tasks
                .borrow()
                .iter()
                .filter_map(|t| t.wake_time())
                .min();
            std::thread::sleep(wake_time.unwrap() - std::time::Instant::now());
            for task in self.tasks.borrow().iter() {
                task.awaken();
            }
        }
    }
}
