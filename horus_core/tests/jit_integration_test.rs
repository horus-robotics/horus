/// Integration test for JIT compilation system
/// Demonstrates how DataflowNode and JITCompiledNode work together
use horus_core::scheduling::Scheduler;

#[test]
fn test_jit_compiled_node() {
    use std::time::Duration;

    // Create a scheduler with JIT-capable nodes
    let mut scheduler = Scheduler::new();

    // Add a node with JIT-compatible name pattern
    // The scheduler will automatically detect and optimize JIT-capable nodes
    struct TestNode;
    impl horus_core::core::Node for TestNode {
        fn name(&self) -> &'static str {
            "jit_test_node" // Name pattern triggers JIT detection
        }

        fn tick(&mut self, _ctx: Option<&mut horus_core::core::NodeInfo>) {
            // Simple computation
        }

        fn init(
            &mut self,
            _ctx: &mut horus_core::core::NodeInfo,
        ) -> horus_core::error::HorusResult<()> {
            Ok(())
        }

        fn shutdown(
            &mut self,
            _ctx: &mut horus_core::core::NodeInfo,
        ) -> horus_core::error::HorusResult<()> {
            Ok(())
        }

        fn get_publishers(&self) -> Vec<horus_core::core::TopicMetadata> {
            Vec::new()
        }

        fn get_subscribers(&self) -> Vec<horus_core::core::TopicMetadata> {
            Vec::new()
        }
    }

    scheduler.add(Box::new(TestNode), 0, None);

    // Run for a short duration
    let result = scheduler.run_for(Duration::from_millis(100));
    assert!(result.is_ok());
}

#[test]
fn test_scheduler_with_scaling_node() {
    use std::time::Duration;

    let mut scheduler = Scheduler::new();

    // Add a node with scaling pattern (triggers JIT detection)
    struct ScalingTestNode;
    impl horus_core::core::Node for ScalingTestNode {
        fn name(&self) -> &'static str {
            "scaling_processor" // Name pattern triggers JIT detection
        }

        fn tick(&mut self, _ctx: Option<&mut horus_core::core::NodeInfo>) {
            // Scaling computation
        }

        fn init(
            &mut self,
            _ctx: &mut horus_core::core::NodeInfo,
        ) -> horus_core::error::HorusResult<()> {
            Ok(())
        }

        fn shutdown(
            &mut self,
            _ctx: &mut horus_core::core::NodeInfo,
        ) -> horus_core::error::HorusResult<()> {
            Ok(())
        }

        fn get_publishers(&self) -> Vec<horus_core::core::TopicMetadata> {
            Vec::new()
        }

        fn get_subscribers(&self) -> Vec<horus_core::core::TopicMetadata> {
            Vec::new()
        }
    }

    scheduler.add(Box::new(ScalingTestNode), 0, None);

    // Run for a short duration
    let result = scheduler.run_for(Duration::from_millis(100));
    assert!(result.is_ok());
}
