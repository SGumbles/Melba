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
    next_task_id: Cell<u32>,
    currently_running_task: RefCell<Option<Weak<Task>>>,
}

impl Executor {
    pub(crate) fn new() -> Self {
        Executor {
            tasks: RefCell::new(VecDeque::new()),
            is_running: Cell::new(false),
            next_task_id: Cell::new(1),
            currently_running_task: RefCell::new(None),
        }
    }

    pub(crate) fn add_task(&self, future: impl Future<Output = ()> + 'static) {
        let id = self.next_task_id.get();
        self.next_task_id.set(id + 1);
        self.tasks
            .borrow_mut()
            .push_back(Rc::new(Task::new(id, future)));
        #[cfg(feature = "dev-logs")]
        log::debug!(
            "task {} added; queue size = {}",
            id,
            self.tasks.borrow().len()
        );
        if !self.is_running.get() {
            self.run();
        }
    }

    pub(crate) fn get_currently_running_task(&self) -> Option<Weak<Task>> {
        self.currently_running_task.borrow().clone()
    }

    fn run(&self) {
        self.is_running.set(true);
        #[cfg(feature = "dev-logs")]
        log::info!("executor starting run loop");
        let mut context = Context::from_waker(Waker::noop());
        loop {
            loop {
                let Some(next_task) = self
                    .tasks
                    .borrow_mut()
                    .pop_front_if(|t| t.get_state() == TaskState::WaitingToBeRun)
                else {
                    #[cfg(feature = "dev-logs")]
                    log::debug!("no more ready tasks");
                    break;
                };
                *self.currently_running_task.borrow_mut() = Some(Rc::downgrade(&next_task));
                #[cfg(feature = "dev-logs")]
                log::trace!("polling task {}", next_task.id());
                match next_task.poll(&mut context) {
                    std::task::Poll::Ready(_) => {
                        #[cfg(feature = "dev-logs")]
                        log::debug!("task {} completed", next_task.id());
                    }
                    std::task::Poll::Pending => {
                        #[cfg(feature = "dev-logs")]
                        log::debug!("task {} parked (Pending)", next_task.id());
                        self.tasks.borrow_mut().push_back(next_task);
                    }
                }
            }
            if self.tasks.borrow().is_empty() {
                #[cfg(feature = "dev-logs")]
                log::info!("executor idle; all tasks finished");
                break;
            }
            let wake_time = self
                .tasks
                .borrow()
                .iter()
                .filter_map(|t| t.wake_time())
                .min();
            #[cfg(feature = "dev-logs")]
            log::debug!("next wake time: {:?}", wake_time);
            std::thread::sleep(wake_time.unwrap() - std::time::Instant::now());
            #[cfg(feature = "dev-logs")]
            log::debug!("woke up; awakening expired tasks");
            for task in self.tasks.borrow().iter() {
                task.awaken();
            }
        }
        self.is_running.set(false);
    }
}
