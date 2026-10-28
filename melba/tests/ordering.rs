use melba::{sleep, spawn};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn tasks_run_in_fifo_order() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let o = order.clone();
    spawn(async move {
        for i in 0..5 {
            let o = o.clone();
            spawn(async move {
                o.lock().unwrap().push(i);
            });
        }
    });
    assert_eq!(*order.lock().unwrap(), vec![0, 1, 2, 3, 4]);
}

#[test]
fn shorter_sleep_completes_first() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let o = order.clone();
    spawn(async move {
        let long = o.clone();
        spawn(async move {
            sleep(Duration::from_millis(100)).await;
            long.lock().unwrap().push("long");
        });
        let short = o.clone();
        spawn(async move {
            sleep(Duration::from_millis(10)).await;
            short.lock().unwrap().push("short");
        });
    });
    assert_eq!(*order.lock().unwrap(), vec!["short", "long"]);
}

#[test]
fn tasks_yield_to_each_other() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let e = events.clone();
    spawn(async move {
        let a = e.clone();
        spawn(async move {
            a.lock().unwrap().push("A start");
            sleep(Duration::from_millis(20)).await;
            a.lock().unwrap().push("A end");
        });
        let b = e.clone();
        spawn(async move {
            b.lock().unwrap().push("B start");
            sleep(Duration::from_millis(20)).await;
            b.lock().unwrap().push("B end");
        });
    });
    assert_eq!(
        *events.lock().unwrap(),
        vec!["A start", "B start", "A end", "B end"]
    );
}

#[test]
fn completion_order_matches_wake_order() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let o = order.clone();
    spawn(async move {
        for (label, ms) in [("a", 30u64), ("b", 10), ("c", 20)] {
            let o = o.clone();
            spawn(async move {
                sleep(Duration::from_millis(ms)).await;
                o.lock().unwrap().push(label);
            });
        }
    });
    assert_eq!(*order.lock().unwrap(), vec!["b", "c", "a"]);
}
