use melba::{sleep, spawn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[test]
fn sleep_waits_for_requested_duration() {
    let duration = Duration::from_millis(50);
    let start = Instant::now();
    spawn(async move {
        sleep(duration).await;
    });
    assert!(
        start.elapsed() >= duration,
        "expected to sleep at least {duration:?}, but only {:?} elapsed",
        start.elapsed()
    );
}

#[test]
fn sleep_short_duration() {
    let duration = Duration::from_millis(1);
    let start = Instant::now();
    spawn(async move {
        sleep(duration).await;
    });
    assert!(start.elapsed() >= duration);
}

#[test]
fn sleep_zero_duration_completes() {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    spawn(async move {
        sleep(Duration::ZERO).await;
        flag.store(true, Ordering::SeqCst);
    });
    assert!(done.load(Ordering::SeqCst));
}

#[test]
fn multiple_sleeps_are_cumulative() {
    let start = Instant::now();
    spawn(async move {
        sleep(Duration::from_millis(20)).await;
        sleep(Duration::from_millis(20)).await;
        sleep(Duration::from_millis(20)).await;
    });
    assert!(start.elapsed() >= Duration::from_millis(60));
}

#[test]
fn sleep_resumes_after_waking() {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    spawn(async move {
        sleep(Duration::from_millis(10)).await;
        flag.store(true, Ordering::SeqCst);
    });
    assert!(done.load(Ordering::SeqCst));
}
