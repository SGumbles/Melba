use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Wake, Waker};
use std::thread::Thread;
use std::time::Duration;

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);

unsafe fn clone_waker(data: *const ()) -> RawWaker {
    let ptr = data.cast::<ThreadWaker>();
    unsafe { Arc::increment_strong_count(ptr) };
    RawWaker::new(data, &VTABLE)
}

unsafe fn wake(data: *const ()) {
    unsafe {
        let waker = Arc::<ThreadWaker>::from_raw(data.cast());
        waker.wake();
    }
}

unsafe fn wake_by_ref(data: *const ()) {
    let ptr = data.cast::<ThreadWaker>();
    unsafe {
        Arc::increment_strong_count(ptr);
    }
    unsafe {
        wake(data);
    }
}

unsafe fn drop_waker(data: *const ()) {
    let ptr = data.cast::<ThreadWaker>();
    unsafe {
        Arc::decrement_strong_count(ptr);
    }
}

struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

struct Sleep {
    duration: Duration,
    started: bool,
    done: Arc<AtomicBool>,
}

impl Sleep {
    fn new(duration: Duration) -> Self {
        Sleep {
            duration,
            started: false,
            done: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Future for Sleep {
    type Output = ();
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.done.load(Ordering::Acquire) {
            return Poll::Ready(());
        }

        if !self.started {
            self.started = true;

            let done = Arc::clone(&self.done);
            let waker = cx.waker().clone();
            let duration = self.duration;
            std::thread::spawn(move || {
                std::thread::sleep(duration);
                done.store(true, Ordering::Release);
                waker.wake();
            });
        }
        Poll::Pending
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let data = Arc::into_raw(Arc::new(ThreadWaker(std::thread::current())));
    let raw_waker = RawWaker::new(data.cast(), &VTABLE);
    let waker = unsafe { Waker::from_raw(raw_waker) };
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Pending => std::thread::park(),
            Poll::Ready(output) => return output,
        }
    }
}

fn main() {
    block_on(async {
        println!("Saying hi from before the yield!");
        Sleep::new(Duration::from_millis(2000)).await;
        println!("Saying hi from after the yield!");
    });
}
