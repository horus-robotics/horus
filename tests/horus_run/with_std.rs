use std::collections::HashMap;
use std::io::Write;

fn main() {
    let mut map = HashMap::new();
    map.insert("test", "success");

    println!("HashMap test: {}", map.get("test").unwrap());
    writeln!(std::io::stdout(), "IO test: success").unwrap();
}
