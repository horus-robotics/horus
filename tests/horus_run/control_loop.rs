fn main() {
    println!("Control loop starting");

    let mut position = 0.0;
    let target = 10.0;

    for iteration in 1..=5 {
        let error = target - position;
        let control = error * 0.3;  // Simple P controller

        position += control;

        println!("Iteration {}: pos={:.2}, error={:.2}", iteration, position, error);

        if error.abs() < 0.01 {
            break;
        }
    }

    println!("Control loop completed");
}
