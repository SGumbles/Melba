use std::{
    collections::VecDeque,
    future::Future,
    hint::black_box,
    pin::Pin,
    process::exit,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Wake, Waker},
    thread::{self, JoinHandle, Thread},
    time::{Duration, Instant},
};

use rand_distr::{Distribution, LogNormal};

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

#[derive(PartialEq)]
#[repr(u8)]
enum TaskState {
    WaitingToBeRun = 0,
    Running = 1,
    WaitingForWake = 2,
}

struct Task {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    state: AtomicU8,
}

impl Task {
    fn new(future: impl Future<Output = ()> + 'static + Send) -> Self {
        Task {
            future: Box::pin(future),
            state: AtomicU8::new(TaskState::WaitingToBeRun as u8),
        }
    }
    fn set_state(&self, state: TaskState) {
        self.state.store(state as u8, Ordering::Release);
    }
    fn get_state(&self) -> TaskState {
        unsafe { std::mem::transmute::<u8, TaskState>(self.state.load(Ordering::Acquire)) }
    }
}

struct Executor {
    tasks: Arc<Mutex<VecDeque<Task>>>,
    max_worker_threads: u32,
    little_worker_bees: Mutex<Vec<JoinHandle<()>>>,
}

impl Executor {
    fn new() -> Self {
        Executor {
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            max_worker_threads: 4,
            little_worker_bees: Mutex::new(Vec::new()),
        }
    }

    fn add_task(&self, future: impl Future<Output = ()> + 'static + Send) {
        self.tasks.lock().unwrap().push_back(Task::new(future));
    }

    fn run(&self) {
        self.little_worker_bees
            .lock()
            .unwrap()
            .extend((0..self.max_worker_threads).map(|i| {
                let tasks = self.tasks.clone();
                thread::spawn(move || {
                    let next_task = tasks
                        .lock()
                        .unwrap()
                        .pop_front_if(|t| t.get_state() == TaskState::WaitingToBeRun);
                    match next_task {
                        Some(t) => {
                            todo!("Actuall run the damn")
                        }
                        None => {
                            println!("BEEP BEEP Parking a worker BEE #{}", i + 1);
                            std::thread::park();
                        }
                    }
                })
            }));
        self.little_worker_bees
            .lock()
            .unwrap()
            .drain(..)
            .for_each(|t| t.join().unwrap());
        // while let Some(task) = self.tasks.pop() {
        //     let future = task.future;
        //     let waker = task.waker;
        //     let mut context = Context::from_waker(&waker);
        //     let mut future = Box::pin(future);
        //     loop {
        //         match future.as_mut().poll(&mut context) {
        //             Poll::Pending => std::thread::park(),
        //             Poll::Ready(output) => return output,
        //         }
        //     }
        // }
    }
}

fn work(duration: Duration) {
    let deadline = Instant::now() + duration;
    let mut state = 0x853c_49e6_748f_ea9b_u64;

    while Instant::now() < deadline {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        black_box(z); // <-- forces the work to actually happen
    }

    black_box(state); // <-- keeps the final value "used"
}

fn sleep(duration: Duration) -> impl Future {
    Sleep::new(duration)
}

async fn spawn_more_work(exec: Arc<Executor>) {
    exec.add_task(async {
        println!("heh.. workin' hard or hardly workin'?");
        work(Duration::from_secs(1));
        sleep(Duration::from_secs(1)).await;
    });
    println!("Sleeping after adding more work to the executor!");
    sleep(Duration::from_secs(2)).await;
}

async fn big_work(exec: Arc<Executor>) {
    println!("OHH WE GOT THE BIG WORK AND SLEEP NOW!");
    println!("WORK WORK WORK!");
    work(Duration::from_secs(10));
    println!("SLEEP SLEEP SLEEP!");
    sleep(Duration::from_secs(15)).await;
    if rand::random_ratio(1, 5) {
        println!("GWAAAAAA!!! WE HAVE HIT THE BIG WORK AGAIN!!!");
        spawn_more_work(exec).await;
    }
}

fn main() {
    let executor = Arc::new(Executor::new());

    executor.run();
    println!("All done!");
    exit(0);

    let work_dist = LogNormal::from_mean_cv(3000.0, 0.60).unwrap();
    let sleep_dist = LogNormal::from_mean_cv(5000.0, 0.80).unwrap();

    for _ in 0..20 {
        let exec: Arc<Executor> = executor.clone();
        executor.add_task(async move {
            loop {
                let (work_ms, sleep_ms) = {
                    let mut rng = rand::rng();
                    (
                        work_dist.sample(&mut rng) as u64,
                        sleep_dist.sample(&mut rng) as u64,
                    )
                };
                println!("I work all day for {work_ms} milliseconds...");
                work(Duration::from_millis(work_ms));
                println!("Now I sleep all night for {sleep_ms} milliseconds...");
                sleep(Duration::from_millis(sleep_ms)).await;

                if rand::random_ratio(1, 20) {
                    big_work(exec.clone()).await;
                }
            }
        });
    }
    executor.run();
}
