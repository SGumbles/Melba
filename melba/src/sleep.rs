use std::{
    rc::Weak,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use crate::task::Task;

pub struct Sleep {
    duration: Duration,
    end: Option<Instant>,
    started: bool,
    owning_task: Weak<Task>,
    // Only read by the `dev-logs` log statements, so it's dead when the feature is off.
    #[allow(dead_code)]
    task_id: u32,
}

impl Sleep {
    pub fn new(duration: Duration, owning_task: Weak<Task>, task_id: u32) -> Self {
        Sleep {
            duration,
            end: None,
            owning_task,
            started: false,
            task_id,
        }
    }
}

impl Future for Sleep {
    type Output = ();
    fn poll(mut self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.started && std::time::Instant::now() > self.end.unwrap() {
            #[cfg(feature = "dev-logs")]
            log::debug!("task {} sleep future complete", self.task_id);
            return Poll::Ready(());
        } else {
            if !self.started {
                self.as_mut().started = true;
                let end_time = std::time::Instant::now() + self.duration;
                self.end = Some(end_time);
                #[cfg(feature = "dev-logs")]
                log::debug!(
                    "task {} sleep future starting; waking at {:?}",
                    self.task_id,
                    end_time
                );
                let strong_ref = self.owning_task.upgrade();
                strong_ref
                    .unwrap()
                    .set_state(crate::task::TaskState::Sleeping {
                        wake_time: end_time,
                    });
            }
            Poll::Pending
        }
    }
}
