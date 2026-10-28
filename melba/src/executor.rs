use std::{
    cell::{Cell, RefCell},
    cmp::Reverse,
    collections::{BinaryHeap, VecDeque},
    future::Future,
    rc::{Rc, Weak},
    task::{Context, Waker},
    time::Instant,
};

use crate::task::{Task, TaskState};

pub struct Executor {
    running_tasks: RefCell<VecDeque<Rc<Task>>>,
    sleeping_tasks: RefCell<BinaryHeap<Reverse<Rc<Task>>>>,
    is_running: Cell<bool>,
    next_task_id: Cell<u32>,
    currently_running_task: RefCell<Option<Weak<Task>>>,
}

impl Executor {
    pub(crate) fn new() -> Self {
        Executor {
            sleeping_tasks: RefCell::new(BinaryHeap::new()),
            running_tasks: RefCell::new(VecDeque::new()),
            is_running: Cell::new(false),
            next_task_id: Cell::new(1),
            currently_running_task: RefCell::new(None),
        }
    }

    pub(crate) fn add_task(&self, future: impl Future<Output = ()> + 'static) {
        let id = self.next_task_id.get();
        self.next_task_id.set(id + 1);
        self.running_tasks
            .borrow_mut()
            .push_back(Rc::new(Task::new(id, future)));
        #[cfg(feature = "dev-logs")]
        log::debug!(
            "task {} added; queue size = {}",
            id,
            self.running_tasks.borrow().len()
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
        log::debug!("executor starting run loop");
        let mut context = Context::from_waker(Waker::noop());
        loop {
            loop {
                let Some(next_task) = self.running_tasks.borrow_mut().pop_front() else {
                    #[cfg(feature = "dev-logs")]
                    log::debug!("no more ready tasks");
                    break;
                };
                self.currently_running_task
                    .replace(Some(Rc::downgrade(&next_task)));
                #[cfg(feature = "dev-logs")]
                log::debug!("task {} : Polling", next_task.id());
                let run_result = next_task.poll(&mut context);
                let next_task_state = next_task.get_state();
                self.currently_running_task.replace(None);
                match run_result {
                    std::task::Poll::Ready(_) => {
                        #[cfg(feature = "dev-logs")]
                        log::debug!("task {} : returned Ready so dropping task", next_task.id());
                    }
                    std::task::Poll::Pending => {
                        #[cfg(feature = "dev-logs")]
                        log::debug!(
                            "task {} : returned Pending, so deciding how to reschedule",
                            next_task.id()
                        );
                        match next_task_state {
                            TaskState::WaitingToBeRun => {
                                #[cfg(feature = "dev-logs")]
                                log::debug!(
                                    "task {} : WaitingToBeRun, so pushing back on running_tasks",
                                    next_task.id()
                                );
                                self.running_tasks.borrow_mut().push_back(next_task);
                            }
                            TaskState::Sleeping {
                                wake_time: _wake_time,
                            } => {
                                #[cfg(feature = "dev-logs")]
                                log::debug!(
                                    "task {} : Sleeping until {:?}, so pushing back on sleeping_tasks",
                                    next_task.id(),
                                    _wake_time
                                );
                                self.sleeping_tasks.borrow_mut().push(Reverse(next_task));
                            }
                        }
                    }
                }
            }
            // At this point it's assumed that currently_running_task is empty
            assert!(self.running_tasks.borrow().is_empty());
            if self.sleeping_tasks.borrow().is_empty() {
                #[cfg(feature = "dev-logs")]
                log::debug!("No more sleeping tasks or running tasks, exiting run.");
                break;
            }
            let wake_time = self.next_wake_time().expect("No sleeping tasks!");
            #[cfg(feature = "dev-logs")]
            log::debug!("next wake time: {:?}", wake_time);
            std::thread::sleep(wake_time - std::time::Instant::now());
            #[cfg(feature = "dev-logs")]
            log::debug!(
                "woke up at {:?}; awakening expired tasks",
                std::time::Instant::now()
            );
            // Re-sort all our tasks after our sleep
            let drained: Vec<_> = self.sleeping_tasks.borrow_mut().drain().collect();
            for task in drained {
                let inner = task.0;
                inner.awaken();
                let new_state = inner.get_state();
                match new_state {
                    TaskState::Sleeping { .. } => {
                        self.sleeping_tasks.borrow_mut().push(Reverse(inner))
                    }
                    TaskState::WaitingToBeRun => self.running_tasks.borrow_mut().push_back(inner),
                }
            }
            #[cfg(feature = "dev-logs")]
            log::debug!(
                "Sleeping Tasks : {}\nRunning Tasks : {}",
                self.sleeping_tasks.borrow().len(),
                self.running_tasks.borrow().len()
            );
        }
        self.is_running.set(false);
    }

    fn next_wake_time(&self) -> Option<Instant> {
        self.sleeping_tasks
            .borrow()
            .peek()
            .and_then(|t| t.0.wake_time())
    }
}
