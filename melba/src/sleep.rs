use std::{
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub struct Sleep {
    wake_time: Instant,
}

impl Sleep {
    pub fn new(duration: Duration) -> Self {
        Sleep {
            wake_time: Instant::now() + duration,
        }
    }
}

impl Future for Sleep {
    type Output = ();
    fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() > self.wake_time {
            return Poll::Ready(());
        } else {
            Poll::Pending
        }
    }
}
