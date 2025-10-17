struct SimpleNode {
    counter: u32,
}

trait Node {
    fn init() -> Self;
    fn tick(&mut self);
}

impl Node for SimpleNode {
    fn init() -> Self {
        Self { counter: 0 }
    }

    fn tick(&mut self) {
        self.counter += 1;
        println!("Tick #{}", self.counter);
    }
}

fn main() {
    let mut node = SimpleNode::init();
    for _ in 0..3 {
        node.tick();
    }
}
