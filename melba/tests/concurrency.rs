use melba::{sleep, spawn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[test]
fn concurrent_sleeps_overlap() {
    let start = Instant::now();
    spawn(async move {
        for _ in 0..10 {
            spawn(async move {
                sleep(Duration::from_millis(50)).await;
            });
        }
    });
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(50),
        "finished too early: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "sleeps did not overlap: {elapsed:?}"
    );
}

#[test]
fn two_tasks_sleep_then_finish() {
    let start = Instant::now();
    spawn(async move {
        spawn(async move {
            sleep(Duration::from_millis(50)).await;
        });
        spawn(async move {
            sleep(Duration::from_millis(50)).await;
        });
    });
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(50));
    assert!(elapsed < Duration::from_millis(150));
}

#[test]
fn child_runs_while_parent_sleeps() {
    let child_ran = Arc::new(AtomicBool::new(false));
    let c = child_ran.clone();
    spawn(async move {
        let c2 = c.clone();
        spawn(async move {
            c2.store(true, Ordering::SeqCst);
        });
        sleep(Duration::from_millis(50)).await;
    });
    assert!(child_ran.load(Ordering::SeqCst));
}
