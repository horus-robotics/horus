use std::time::{Duration, Instant};

fn main() {
    println!("Timing test starting");

    let start = Instant::now();

    // Simulate work
    std::thread::sleep(Duration::from_millis(50));

    let elapsed = start.elapsed();

    println!("Elapsed time: {:?}", elapsed);
    println!("Timing test completed");
}
