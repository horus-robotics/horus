fn main() {
    if let Ok(val) = std::env::var("HORUS_TEST_VAR") {
        println!("Got env var: {}", val);
    } else {
        println!("No env var found");
    }
}
