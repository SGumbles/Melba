use std::future::Future;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Wake, Waker};
use std::thread::Thread;
use std::time::{Duration, Instant};

use libc::epoll_event;

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
    commands: Sender<TimerCommand>,
    id: u64,
    deadline: Instant,
    registered: bool,
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            return Poll::Ready(());
        }

        if !self.registered {
            if self
                .commands
                .send(TimerCommand::Register {
                    id: self.id,
                    deadline: self.deadline,
                    waker: cx.waker().clone(),
                })
                .is_err()
            {
                panic!("Timer thread disconnected");
            }
            self.registered = true;
        }
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if self.registered {
            let _ = self.commands.send(TimerCommand::Cancel { id: self.id });
        }
    }
}

enum TimerCommand {
    Register {
        id: u64,
        deadline: Instant,
        waker: Waker,
    },
    Cancel {
        id: u64,
    },
}

struct TimerService {
    commands: mpsc::Sender<TimerCommand>,
    next_id: AtomicU64,
}

impl TimerService {
    fn new() -> Self {
        let (commands, receiver) = mpsc::channel();
        std::thread::spawn(move || timer_thread(receiver));
        Self {
            commands,
            next_id: AtomicU64::new(0),
        }
    }
    fn sleep(&self, duration: Duration) -> Sleep {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        Sleep {
            commands: self.commands.clone(),
            id,
            deadline: Instant::now() + duration,
            registered: false,
        }
    }
}

fn timer_thread(receiver: mpsc::Receiver<TimerCommand>) {
    let mut timers = Vec::<TimerEntry>::new();
    loop {
        let command = match next_timeout(&timers) {
            Some(timeout) => match receiver.recv_timeout(timeout) {
                Ok(command) => Some(command),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => None,
            },
            None => match receiver.recv() {
                Ok(command) => Some(command),
                Err(_) => break,
            },
        };

        if let Some(command) = command {
            match command {
                TimerCommand::Register {
                    id,
                    deadline,
                    waker,
                } => {
                    timers.push(TimerEntry {
                        id,
                        deadline,
                        waker,
                    });
                }
                TimerCommand::Cancel { id } => {
                    timers.retain(|t| t.id != id);
                }
            }
        }
        wake_due_timers(&mut timers);
    }
}

fn wake_due_timers(timers: &mut Vec<TimerEntry>) {
    let now = Instant::now();
    let mut i = 0;

    while i < timers.len() {
        if timers[i].deadline <= now {
            let timer = timers.swap_remove(i);
            timer.waker.wake();
        } else {
            i += 1;
        }
    }
}

fn next_timeout(timers: &[TimerEntry]) -> Option<Duration> {
    let now = std::time::Instant::now();
    timers
        .iter()
        .map(|v| v.deadline.saturating_duration_since(now))
        .min()
}

struct TimerEntry {
    id: u64,
    deadline: Instant,
    waker: Waker,
}

struct Join2<A, B> {
    first: A,
    second: B,
    first_done: bool,
    second_done: bool,
}

impl<A, B> Future for Join2<A, B>
where
    A: Future<Output = ()> + Unpin,
    B: Future<Output = ()> + Unpin,
{
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if !this.first_done {
            if let Poll::Ready(()) = std::pin::Pin::new(&mut this.first).poll(cx) {
                this.first_done = true;
            }
        }
        if !this.second_done {
            if let Poll::Ready(()) = std::pin::Pin::new(&mut this.second).poll(cx) {
                this.second_done = true;
            }
        }

        if this.first_done && this.second_done {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

struct Epoll {
    fd: OwnedFd,
}

impl Epoll {
    fn new() -> io::Result<Self> {
        let raw_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if raw_fd == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            fd: unsafe { OwnedFd::from_raw_fd(raw_fd) },
        })
    }

    fn register(&self, socket_fd: RawFd, events: u32, token: u64) -> io::Result<()> {
        let mut event = libc::epoll_event { events, u64: token };
        let result = unsafe {
            libc::epoll_ctl(
                self.fd.as_raw_fd(),
                libc::EPOLL_CTL_ADD,
                socket_fd,
                &mut event,
            )
        };

        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn wait(&self, events: &mut [libc::epoll_event; 16]) -> io::Result<usize> {
        let count = unsafe {
            libc::epoll_wait(
                self.fd.as_raw_fd(),
                events.as_mut_ptr(),
                events.len() as i32,
                -1,
            )
        };

        if count == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(count as usize)
        }
    }
}

fn new_nonblocking_tcp_socket() -> io::Result<OwnedFd> {
    let raw_fd = unsafe {
        libc::socket(
            libc::AF_INET,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            libc::IPPROTO_TCP,
        )
    };

    if raw_fd == -1 {
        return Err(io::Error::last_os_error());
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(raw_fd) })
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

enum ConnectStart {
    Connected,
    InProgress,
}

enum IoCommand {
    Register {
        token: u64,
        socket: Arc<OwnedFd>,
        events: u32,
        waker: Waker,
    },
    Cancel {
        token: u64,
    },
}

struct IoReactor {
    commands: mpsc::Sender<IoCommand>,
    control_fd: Arc<OwnedFd>,
    next_token: AtomicU64,
}

fn start_connect(fd: RawFd, address: std::net::SocketAddrV4) -> io::Result<ConnectStart> {
    let address = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: address.port().to_be(),
        sin_addr: libc::in_addr {
            s_addr: u32::from_ne_bytes(address.ip().octets()),
        },
        sin_zero: [0; 8],
    };

    let result = unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_in).cast(),
            std::mem::size_of_val(&address) as libc::socklen_t,
        )
    };

    if result == 0 {
        Ok(ConnectStart::Connected)
    } else {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::EINPROGRESS) {
            Ok(ConnectStart::InProgress)
        } else {
            Err(error)
        }
    }
}

fn new_event_fd() -> io::Result<OwnedFd> {
    let raw_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };

    if raw_fd == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(unsafe { OwnedFd::from_raw_fd(raw_fd) })
}

fn drain_eventfd(fd: RawFd) -> io::Result<u64> {
    let mut value = 0u64;
    let size = std::mem::size_of_val(&value);
    let read = unsafe { libc::read(fd, (&mut value as *mut u64).cast(), size) };

    if read == -1 {
        Err(io::Error::last_os_error())
    } else if read != size as isize {
        Err(io::ErrorKind::UnexpectedEof.into())
    } else {
        Ok(value)
    }
}

fn signal_eventfd(fd: RawFd) -> io::Result<()> {
    let value: u64 = 1;
    let size = std::mem::size_of_val(&value);
    let written = unsafe { libc::write(fd, (&value as *const u64).cast(), size) };

    if written == -1 {
        Err(io::Error::last_os_error())
    } else if written != size as isize {
        Err(io::ErrorKind::WriteZero.into())
    } else {
        Ok(())
    }
}

fn socket_error(fd: RawFd) -> io::Result<i32> {
    let mut error: libc::c_int = 0;
    let mut len = std::mem::size_of_val(&error) as libc::socklen_t;

    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_ERROR,
            (&mut error as *mut libc::c_int).cast(),
            &mut len,
        )
    };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(error)
    }
}

fn main() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = match listener.local_addr().unwrap() {
        std::net::SocketAddr::V4(address) => address,
        std::net::SocketAddr::V6(_) => unreachable!(),
    };

    let socket = new_nonblocking_tcp_socket().unwrap();

    let epoll = Epoll::new().unwrap();

    const CONTROL_TOKEN: u64 = 0;

    let control_fd = new_event_fd().unwrap();

    epoll
        .register(control_fd.as_raw_fd(), libc::EPOLLIN as u32, CONTROL_TOKEN)
        .unwrap();

    signal_eventfd(control_fd.as_raw_fd()).unwrap();

    let mut control_events = std::array::from_fn(|_| libc::epoll_event { events: 0, u64: 0 });

    let count = epoll.wait(&mut control_events).unwrap();
    assert!(count > 0);
    let ev = control_events[0].u64;
    assert_eq!(ev, CONTROL_TOKEN);
    assert_eq!(drain_eventfd(control_fd.as_raw_fd()).unwrap(), 1);

    match start_connect(socket.as_raw_fd(), address) {
        Ok(ConnectStart::Connected) => println!("connected immediately"),
        Ok(ConnectStart::InProgress) => {
            epoll
                .register(socket.as_raw_fd(), libc::EPOLLOUT as u32, 42)
                .unwrap();
            let mut events = std::array::from_fn(|_| libc::epoll_event { events: 0, u64: 0 });
            let count = epoll.wait(&mut events).unwrap();
            assert!(count > 0);
            let token = events[0].u64;
            let flags = events[0].events;
            println!("epoll even: token={}, flags={:#x}", token, flags);
            let connection_error = socket_error(socket.as_raw_fd()).unwrap();
            if connection_error == 0 {
                println!("Connection completed successfully");
            } else {
                eprintln!(
                    "Connect failed : {}",
                    io::Error::from_raw_os_error(connection_error)
                )
            }
        }
        Err(error) => eprintln!("connect failed {error}"),
    }

    //     let timers = TimerService::new();
    //     let start = Instant::now();
    //
    //     block_on(Join2 {
    //         first: timers.sleep(Duration::from_millis(2000)),
    //         second: timers.sleep(Duration::from_millis(5000)),
    //         first_done: false,
    //         second_done: false,
    //     });
    //
    //     println!("Both finished in {:?}", start.elapsed());
    //
    // ------------------------------------------------------------------------------------------------
    //
    // let mut sleep = timers.sleep(Duration::from_millis(2000));
    // block_on(std::future::poll_fn(|cx| {
    //     assert!(std::pin::Pin::new(&mut sleep).poll(cx).is_pending());
    //     Poll::Ready(())
    // }));
    // drop(sleep);
    // block_on(async {
    //     println!("Saying hi from before the yield!");
    //     timers.sleep(Duration::from_millis(2000)).await;
    //     println!("Saying hi from after the yield!");
    //
    // });
}
