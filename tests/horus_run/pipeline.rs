fn filter_data(data: Vec<f64>) -> Vec<f64> {
    data.into_iter().filter(|x| *x > 0.0).collect()
}

fn transform_data(data: Vec<f64>) -> Vec<f64> {
    data.into_iter().map(|x| x * 2.0).collect()
}

fn main() {
    println!("Data pipeline starting");

    let raw_data = vec![-1.0, 2.0, -3.0, 4.0, 5.0];
    let filtered = filter_data(raw_data);
    let transformed = transform_data(filtered);

    println!("Processed data: {:?}", transformed);
    println!("Pipeline completed");
}
