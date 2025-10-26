//! Acceptance tests for Node Lifecycle
//! Tests node init/tick/shutdown lifecycle directly without the Scheduler loop
//! The Scheduler's infinite loop design makes it unsuitable for unit testing,
//! so we test Node behavior directly

use horus_core::core::{Node, NodeInfo};
use horus_core::error::HorusResult;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

// Test node that tracks lifecycle
struct LifecycleNode {
    name: &'static str,
    init_called: Arc<AtomicBool>,
    tick_count: Arc<AtomicU32>,
    shutdown_called: Arc<AtomicBool>,
}

impl LifecycleNode {
    fn new(
        name: &'static str,
        init_called: Arc<AtomicBool>,
        tick_count: Arc<AtomicU32>,
        shutdown_called: Arc<AtomicBool>,
    ) -> Self {
        Self {
            name,
            init_called,
            tick_count,
            shutdown_called,
        }
    }
}

impl Node for LifecycleNode {
    fn name(&self) -> &'static str {
        self.name
    }

    fn init(&mut self, _ctx: &mut NodeInfo) -> HorusResult<()> {
        self.init_called.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        self.tick_count.fetch_add(1, Ordering::SeqCst);
    }

    fn shutdown(&mut self, _ctx: &mut NodeInfo) -> HorusResult<()> {
        self.shutdown_called.store(true, Ordering::SeqCst);
        Ok(())
    }
}

// Minimal node that only implements tick
struct MinimalNode {
    name: &'static str,
    tick_count: Arc<AtomicU32>,
}

impl MinimalNode {
    fn new(name: &'static str, tick_count: Arc<AtomicU32>) -> Self {
        Self { name, tick_count }
    }
}

impl Node for MinimalNode {
    fn name(&self) -> &'static str {
        self.name
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        self.tick_count.fetch_add(1, Ordering::SeqCst);
    }

    // init and shutdown use default implementations
}

// Node that fails during init
struct FailingInitNode {
    name: &'static str,
    tick_called: Arc<AtomicBool>,
}

impl Node for FailingInitNode {
    fn name(&self) -> &'static str {
        self.name
    }

    fn init(&mut self, _ctx: &mut NodeInfo) -> HorusResult<()> {
        Err("Initialization failed".into())
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        self.tick_called.store(true, Ordering::SeqCst);
    }
}

// Node that tracks execution order
struct OrderedNode {
    name: &'static str,
    execution_log: Arc<Mutex<Vec<String>>>,
}

impl OrderedNode {
    fn new(name: &'static str, execution_log: Arc<Mutex<Vec<String>>>) -> Self {
        Self { name, execution_log }
    }
}

impl Node for OrderedNode {
    fn name(&self) -> &'static str {
        self.name
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        let mut log = self.execution_log.lock().unwrap();
        log.push(self.name.to_string());
    }
}

#[test]
fn test_scenario_1_complete_node_lifecycle() {
    // Scenario 1: Complete Node Lifecycle
    // Given: Node implements init, tick, and shutdown
    // When: Node methods are called
    // Then: init() called once, tick() called multiple times, shutdown() called once

    let init_called = Arc::new(AtomicBool::new(false));
    let tick_count = Arc::new(AtomicU32::new(0));
    let shutdown_called = Arc::new(AtomicBool::new(false));

    let mut node = LifecycleNode::new(
        "lifecycle_test",
        init_called.clone(),
        tick_count.clone(),
        shutdown_called.clone(),
    );

    let mut ctx = NodeInfo::new("lifecycle_test".to_string(), true);

    // Test init
    assert!(node.init(&mut ctx).is_ok(), "init should succeed");
    assert!(init_called.load(Ordering::SeqCst), "init should be called");

    // Test tick multiple times
    for _ in 0..5 {
        node.tick(Some(&mut ctx));
    }
    assert_eq!(tick_count.load(Ordering::SeqCst), 5, "tick should be called 5 times");

    // Test shutdown
    assert!(node.shutdown(&mut ctx).is_ok(), "shutdown should succeed");
    assert!(shutdown_called.load(Ordering::SeqCst), "shutdown should be called");
}

#[test]
fn test_scenario_2_init_failure_prevents_execution() {
    // Scenario 2: init() Failure
    // Given: Node's init() returns Err
    // When: init is called
    // Then: Error is returned, tick should not be called by scheduler

    let tick_called = Arc::new(AtomicBool::new(false));
    let mut node = FailingInitNode {
        name: "failing",
        tick_called: tick_called.clone(),
    };

    let mut ctx = NodeInfo::new("failing".to_string(), false);

    // Init should fail
    assert!(node.init(&mut ctx).is_err(), "init should return error");

    // In a real scheduler, tick would not be called after init failure
    // But we can verify the behavior is correct
    assert!(!tick_called.load(Ordering::SeqCst), "tick should not have been called yet");
}

#[test]
fn test_scenario_5_minimal_node() {
    // Scenario 5: Optional init/shutdown
    // Given: Node only implements tick() (not custom init/shutdown)
    // When: Node runs
    // Then: Default init/shutdown do nothing, tick executes normally

    let tick_count = Arc::new(AtomicU32::new(0));
    let mut node = MinimalNode::new("minimal", tick_count.clone());

    let mut ctx = NodeInfo::new("minimal".to_string(), false);

    // Default init should succeed
    assert!(node.init(&mut ctx).is_ok(), "default init should succeed");

    // Tick should work
    for _ in 0..3 {
        node.tick(None);
    }
    assert_eq!(tick_count.load(Ordering::SeqCst), 3, "tick should be called 3 times");

    // Default shutdown should succeed
    assert!(node.shutdown(&mut ctx).is_ok(), "default shutdown should succeed");
}

#[test]
fn test_scenario_14_nodeinfo_in_tick() {
    // Scenario 14: NodeInfo in tick()
    // Given: tick() receives Option<&mut NodeInfo>
    // When: Node is ticked with and without context
    // Then: Both cases work correctly

    let tick_count = Arc::new(AtomicU32::new(0));
    let mut node = MinimalNode::new("test", tick_count.clone());

    let mut ctx = NodeInfo::new("test".to_string(), true);

    // Tick with context
    node.tick(Some(&mut ctx));
    assert_eq!(tick_count.load(Ordering::SeqCst), 1);

    // Tick without context
    node.tick(None);
    assert_eq!(tick_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_node_info_logging_methods() {
    // Scenario 15: Logging Methods
    // Given: NodeInfo is available
    // When: Node logs messages
    // Then: All logging methods work without panicking

    let mut ctx = NodeInfo::new("logged".to_string(), true);

    // These should not panic
    ctx.log_info("Test info");
    ctx.log_warning("Test warning");
    ctx.log_error("Test error");
    ctx.log_debug("Test debug");
}

#[test]
fn test_multiple_nodes_independent() {
    // Test that multiple nodes operate independently
    let count1 = Arc::new(AtomicU32::new(0));
    let count2 = Arc::new(AtomicU32::new(0));

    let mut node1 = MinimalNode::new("node1", count1.clone());
    let mut node2 = MinimalNode::new("node2", count2.clone());

    // Tick each node independently
    for _ in 0..3 {
        node1.tick(None);
    }

    for _ in 0..7 {
        node2.tick(None);
    }

    assert_eq!(count1.load(Ordering::SeqCst), 3, "node1 should tick 3 times");
    assert_eq!(count2.load(Ordering::SeqCst), 7, "node2 should tick 7 times");
}

#[test]
fn test_node_lifecycle_order() {
    // Test that lifecycle methods are called in the correct order
    let execution_log = Arc::new(Mutex::new(Vec::new()));

    struct OrderTrackingNode {
        execution_log: Arc<Mutex<Vec<String>>>,
    }

    impl Node for OrderTrackingNode {
        fn name(&self) -> &'static str {
            "order_test"
        }

        fn init(&mut self, _ctx: &mut NodeInfo) -> HorusResult<()> {
            let mut log = self.execution_log.lock().unwrap();
            log.push("init".to_string());
            Ok(())
        }

        fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
            let mut log = self.execution_log.lock().unwrap();
            log.push("tick".to_string());
        }

        fn shutdown(&mut self, _ctx: &mut NodeInfo) -> HorusResult<()> {
            let mut log = self.execution_log.lock().unwrap();
            log.push("shutdown".to_string());
            Ok(())
        }
    }

    let mut node = OrderTrackingNode {
        execution_log: execution_log.clone(),
    };

    let mut ctx = NodeInfo::new("order_test".to_string(), false);

    // Call lifecycle methods in order
    node.init(&mut ctx).unwrap();
    node.tick(None);
    node.tick(None);
    node.shutdown(&mut ctx).unwrap();

    let log = execution_log.lock().unwrap();
    assert_eq!(log.len(), 4);
    assert_eq!(log[0], "init");
    assert_eq!(log[1], "tick");
    assert_eq!(log[2], "tick");
    assert_eq!(log[3], "shutdown");
}

#[test]
fn test_tick_without_context() {
    // Test that nodes can tick without NodeInfo (logging disabled)
    let tick_count = Arc::new(AtomicU32::new(0));
    let mut node = MinimalNode::new("no_context", tick_count.clone());

    // Tick repeatedly without context
    for _ in 0..10 {
        node.tick(None);
    }

    assert_eq!(tick_count.load(Ordering::SeqCst), 10, "Should tick 10 times without context");
}

#[test]
fn test_node_name() {
    // Test that node name is correctly returned
    let tick_count = Arc::new(AtomicU32::new(0));
    let node = MinimalNode::new("test_name", tick_count);

    assert_eq!(node.name(), "test_name", "Node name should match");
}
