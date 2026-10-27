use melba::{sleep, spawn};
use rand_distr::{Distribution, LogNormal};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

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

async fn spawn_more_work() {
    spawn(async {
        println!("heh.. workin' hard or hardly workin'?");
        work(Duration::from_secs(1));
        sleep(Duration::from_secs(1)).await;
    });
    println!("Sleeping after adding more work to the executor!");
    sleep(Duration::from_secs(2)).await;
}

async fn big_work() {
    println!("OHH WE GOT THE BIG WORK AND SLEEP NOW!");
    println!("WORK WORK WORK!");
    work(Duration::from_secs(10));
    println!("SLEEP SLEEP SLEEP!");
    sleep(Duration::from_secs(15)).await;
    if rand::random_ratio(1, 5) {
        println!("GWAAAAAA!!! WE HAVE HIT THE BIG WORK AGAIN!!!");
        spawn_more_work().await;
    }
}

fn main() {
    env_logger::init();

    let work_dist = LogNormal::from_mean_cv(3000.0, 0.60).unwrap();
    let sleep_dist = LogNormal::from_mean_cv(5000.0, 0.80).unwrap();

    for _ in 0..20 {
        spawn(async move {
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
                    big_work().await;
                }
            }
        });
    }
}
