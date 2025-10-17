use std::collections::HashMap;
use std::io::Write;
use std::fs::File;

fn main() {
    let mut map = HashMap::new();
    map.insert("test", "value");
    println!("Multiple use statements work");
}
