use melba::{sleep, spawn};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
fn spawn_runs_a_single_task() {
    let ran = Arc::new(AtomicUsize::new(0));
    let r = ran.clone();
    spawn(async move {
        r.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(ran.load(Ordering::SeqCst), 1);
}

#[test]
fn spawned_tasks_all_run() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        for _ in 0..10 {
            let c = c.clone();
            spawn(async move {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(count.load(Ordering::SeqCst), 10);
}

#[test]
fn sequential_top_level_spawns() {
    // Spawning several times at the top level, one after another. Each call
    // should enqueue a task and drive it to completion before returning.
    let count = Arc::new(AtomicUsize::new(0));
    for _ in 0..5 {
        let c = count.clone();
        spawn(async move {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }
    assert_eq!(count.load(Ordering::SeqCst), 5);
}

#[test]
fn spawn_from_within_a_task() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        let c2 = c.clone();
        spawn(async move {
            c2.fetch_add(1, Ordering::SeqCst);
        });
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn spawn_after_sleep() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        sleep(Duration::from_millis(10)).await;
        let c2 = c.clone();
        spawn(async move {
            c2.fetch_add(1, Ordering::SeqCst);
        });
    });
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
