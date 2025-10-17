fn main() {
    println!("Publisher starting...");

    for i in 1..=5 {
        println!("Publishing message {}", i);
        // Simulated publish
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    println!("Publisher completed");
}
