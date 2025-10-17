fn sensor_read() -> Result<f64, String> {
    // Simulate successful read
    Ok(42.5)
}

fn main() {
    println!("Error handling test");

    match sensor_read() {
        Ok(value) => println!("Sensor value: {}", value),
        Err(e) => println!("Sensor error: {}", e),
    }

    println!("Error handling completed");
}
