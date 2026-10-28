use melba::{sleep, spawn};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
#[should_panic(expected = "No task is running?")]
fn sleep_outside_a_task_panics() {
    let _ = sleep(Duration::from_millis(1));
}

#[test]
fn task_does_cpu_work_without_yielding() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        let mut x = 0u64;
        for i in 0..1_000_000 {
            x = x.wrapping_add(i);
        }
        std::hint::black_box(x);
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn interleaved_work_and_sleep() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        c.fetch_add(1, Ordering::SeqCst);
        sleep(Duration::from_millis(5)).await;
        c.fetch_add(1, Ordering::SeqCst);
        sleep(Duration::from_millis(5)).await;
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[test]
fn many_tasks_many_sleeps() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        for _ in 0..50 {
            let c = c.clone();
            spawn(async move {
                sleep(Duration::from_millis(1)).await;
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(count.load(Ordering::SeqCst), 50);
}
