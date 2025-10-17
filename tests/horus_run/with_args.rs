fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("Program: {}", args[0]);

    for (i, arg) in args.iter().skip(1).enumerate() {
        println!("Arg {}: {}", i + 1, arg);
    }
}
