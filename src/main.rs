use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Wake, Waker};
use std::thread::Thread;

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
        println!("hey");
    });
    // for _ in 0..100 {
    //     let number = rand::random_range(2..=50);
    //     println!("{number}");
    // }
}
