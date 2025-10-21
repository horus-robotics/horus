#[cfg(test)]
mod tests {
    use horus_core::core::{Node, NodeInfo, NodeState};
    use horus_core::error::HorusResult;
    use std::sync::{Arc, Mutex};

    // Simple test node implementation
    struct TestNode {
        name: &'static str,
        counter: Arc<Mutex<i32>>,
    }

    impl TestNode {
        fn new(name: &'static str, counter: Arc<Mutex<i32>>) -> Self {
            Self { name, counter }
        }
    }

    impl Node for TestNode {
        fn name(&self) -> &'static str {
            self.name
        }

        fn init(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
            ctx.log_info("Test node initializing");
            Ok(())
        }

        fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
            let mut count = self.counter.lock().unwrap();
            *count += 1;

            if let Some(ctx) = ctx {
                ctx.log_debug(&format!("Tick count: {}", *count));
            }
        }

        fn shutdown(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
            ctx.log_info("Test node shutting down");
            Ok(())
        }
    }

    #[test]
    fn test_node_creation() {
        let counter = Arc::new(Mutex::new(0));
        let node = TestNode::new("test_node", counter.clone());
        assert_eq!(node.name(), "test_node");
    }

    #[test]
    fn test_node_tick() {
        let counter = Arc::new(Mutex::new(0));
        let mut node = TestNode::new("tick_node", counter.clone());

        // Tick the node multiple times
        for _ in 0..5 {
            node.tick(None);
        }

        // Check counter was incremented
        let count = counter.lock().unwrap();
        assert_eq!(*count, 5);
    }

    #[test]
    fn test_node_lifecycle() {
        let counter = Arc::new(Mutex::new(0));
        let mut node = TestNode::new("lifecycle_node", counter.clone());
        let mut info = NodeInfo::new("lifecycle_node".to_string(), true);

        // Test full lifecycle
        assert!(node.init(&mut info).is_ok());

        // Tick a few times
        for _ in 0..3 {
            node.tick(Some(&mut info));
        }

        assert!(node.shutdown(&mut info).is_ok());

        // Counter should have been incremented
        let count = counter.lock().unwrap();
        assert_eq!(*count, 3);
    }

    #[test]
    fn test_node_state() {
        // Test NodeState enum
        let state = NodeState::Running;
        assert_eq!(format!("{}", state), "Running");

        let state = NodeState::Error("Test error".to_string());
        assert!(format!("{}", state).contains("Test error"));
    }

    #[test]
    fn test_multiple_nodes() {
        let counter1 = Arc::new(Mutex::new(0));
        let counter2 = Arc::new(Mutex::new(0));

        let mut node1 = TestNode::new("node1", counter1.clone());
        let mut node2 = TestNode::new("node2", counter2.clone());

        // Tick both nodes
        for _ in 0..10 {
            node1.tick(None);
            node2.tick(None);
        }

        // Both should have been ticked
        assert_eq!(*counter1.lock().unwrap(), 10);
        assert_eq!(*counter2.lock().unwrap(), 10);
    }

    #[test]
    fn test_node_info() {
        let mut info = NodeInfo::new("test_info_node".to_string(), true);

        // Test logging functions
        info.log_debug("Debug message");
        info.log_info("Info message");
        info.log_warning("Warning message");
        info.log_error("Error message");

        // NodeInfo basic test
        // (node_id method or similar would be tested here if available)
    }
}
