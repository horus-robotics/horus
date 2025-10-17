// Rust node using HORUS macros
use horus_macros::node;

node! {
    TestNode {
        counter: u64 = 0,

        tick(ctx) {
            self.counter += 1;
            println!("Macro node tick #{}", self.counter);

            if self.counter >= 5 {
                println!("Completed 5 ticks");
                std::process::exit(0);
            }
        }
    }
}

fn main() {
    println!("Testing macro-based node");
    let mut node = TestNode::new();

    for _ in 0..5 {
        node.tick();
    }
}
