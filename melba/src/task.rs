use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    task::Context,
    time::Instant,
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum TaskState {
    WaitingToBeRun,
    Sleeping { wake_time: Instant },
}

pub(crate) struct Task {
    id: u32,
    future: RefCell<Pin<Box<dyn Future<Output = ()>>>>,
    state: Cell<TaskState>,
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let state1 = self.state.get();
        let state2 = other.state.get();
        match (state1, state2) {
            (TaskState::WaitingToBeRun, TaskState::WaitingToBeRun) => std::cmp::Ordering::Equal,
            (TaskState::WaitingToBeRun, TaskState::Sleeping { .. }) => std::cmp::Ordering::Less,
            (TaskState::Sleeping { .. }, TaskState::WaitingToBeRun) => std::cmp::Ordering::Greater,
            (TaskState::Sleeping { wake_time: w1 }, TaskState::Sleeping { wake_time: w2 }) => {
                w1.cmp(&w2)
            }
        }
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(&other).is_eq()
    }
}
impl Eq for Task {}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Task {
    pub(crate) fn new(id: u32, future: impl Future<Output = ()> + 'static) -> Self {
        Task {
            id,
            future: RefCell::new(Box::pin(future)),
            state: Cell::new(TaskState::WaitingToBeRun),
        }
    }
    pub(crate) fn id(&self) -> u32 {
        self.id
    }
    pub(crate) fn set_state(&self, state: TaskState) {
        #[cfg(feature = "dev-logs")]
        log::debug!(
            "task {} : state: {:?} -> {:?}",
            self.id,
            self.get_state(),
            state
        );
        self.state.set(state);
    }
    pub(crate) fn get_state(&self) -> TaskState {
        self.state.get()
    }
    pub(crate) fn wake_time(&self) -> Option<Instant> {
        match self.get_state() {
            TaskState::Sleeping { wake_time } => Some(wake_time),
            _ => None,
        }
    }
    pub(crate) fn awaken(&self) {
        if let TaskState::Sleeping { wake_time } = self.get_state() {
            if std::time::Instant::now() >= wake_time {
                #[cfg(feature = "dev-logs")]
                log::debug!("task {} : waking (sleep expired)", self.id);
                self.set_state(TaskState::WaitingToBeRun);
            }
        }
    }
    pub(crate) fn poll(&self, ctx: &mut Context) -> core::task::Poll<()> {
        self.future.borrow_mut().as_mut().poll(ctx)
    }
}
