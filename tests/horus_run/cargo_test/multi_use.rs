use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

fn main() {
    let mut map = HashMap::new();
    map.insert("test", "value");
    println!("Multiple use statements work");
}
