use melba::{sleep, spawn};

fn main() {
    env_logger::init();

    spawn(async {
        let sleepy_time = std::time::Duration::from_millis(5000);

        let pre = std::time::Instant::now();
        println!("Before Sleep : {:?}", pre);
        sleep(sleepy_time).await;
        let post = std::time::Instant::now();
        println!(
            "After Sleep\t{:?}\nExpected\t{:?}\nDelta\t{:?}",
            post,
            pre + sleepy_time,
            post.saturating_duration_since(pre + sleepy_time)
        );
    });
}
