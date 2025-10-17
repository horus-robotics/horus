// Publisher node using HORUS IPC
use horus::prelude::*;

struct PublisherNode {
    counter: u64,
}

impl Node for PublisherNode {
    fn init() -> Self {
        Self { counter: 0 }
    }

    fn tick(&mut self, ctx: &NodeContext) {
        self.counter += 1;
        println!("Publishing message #{}", self.counter);

        let hub = Hub::new("test_topic");
        hub.publish(&format!("Message {}", self.counter));

        if self.counter >= 5 {
            println!("Published 5 messages, exiting");
            std::process::exit(0);
        }
    }
}

fn main() {
    let mut node = PublisherNode::init();
    let ctx = NodeContext::new("publisher");

    for _ in 0..5 {
        node.tick(&ctx);
    }
}
