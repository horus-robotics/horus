use std::thread;
use std::time::Duration;

fn main() {
    println!("Starting multi-threaded simulation");

    let handle1 = thread::spawn(|| {
        for i in 1..=3 {
            println!("Thread 1: tick {}", i);
            thread::sleep(Duration::from_millis(10));
        }
    });

    let handle2 = thread::spawn(|| {
        for i in 1..=3 {
            println!("Thread 2: tick {}", i);
            thread::sleep(Duration::from_millis(10));
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("Multi-threaded simulation completed");
}
