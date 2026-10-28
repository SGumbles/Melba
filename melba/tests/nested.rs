use melba::{sleep, spawn};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
fn child_spawns_grandchild() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        let c2 = c.clone();
        spawn(async move {
            let c3 = c2.clone();
            spawn(async move {
                c3.fetch_add(1, Ordering::SeqCst);
            });
            c2.fetch_add(1, Ordering::SeqCst);
        });
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[test]
fn many_children_run() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        for _ in 0..100 {
            let c = c.clone();
            spawn(async move {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    assert_eq!(count.load(Ordering::SeqCst), 100);
}

#[test]
fn child_sleeps_while_parent_waits() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        let c2 = c.clone();
        spawn(async move {
            sleep(Duration::from_millis(20)).await;
            c2.fetch_add(1, Ordering::SeqCst);
        });
        sleep(Duration::from_millis(30)).await;
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn grandchild_with_sleep() {
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    spawn(async move {
        let c2 = c.clone();
        spawn(async move {
            let c3 = c2.clone();
            spawn(async move {
                sleep(Duration::from_millis(10)).await;
                c3.fetch_add(1, Ordering::SeqCst);
            });
            sleep(Duration::from_millis(10)).await;
            c2.fetch_add(1, Ordering::SeqCst);
        });
        sleep(Duration::from_millis(10)).await;
        c.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(count.load(Ordering::SeqCst), 3);
}
