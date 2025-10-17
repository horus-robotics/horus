// Subscriber node using HORUS IPC
use horus::prelude::*;

struct SubscriberNode {
    count: u64,
}

impl Node for SubscriberNode {
    fn init() -> Self {
        Self { count: 0 }
    }

    fn tick(&mut self, ctx: &NodeContext) {
        let hub = Hub::new("test_topic");

        if let Some(msg) = hub.subscribe::<String>() {
            self.count += 1;
            println!("Received: {}", msg);

            if self.count >= 5 {
                println!("Received 5 messages, exiting");
                std::process::exit(0);
            }
        }
    }
}

fn main() {
    let mut node = SubscriberNode::init();
    let ctx = NodeContext::new("subscriber");

    loop {
        node.tick(&ctx);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
