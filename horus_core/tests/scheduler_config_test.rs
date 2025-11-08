// Test comprehensive scheduler configuration
use horus_core::core::{Node, NodeInfo};
use horus_core::error::HorusResult as Result;
use horus_core::scheduling::{ConfigValue, ExecutionMode, Scheduler, SchedulerConfig};

/// Simple test node for configuration testing
struct TestNode {
    name: String,
    tick_count: usize,
}

impl TestNode {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tick_count: 0,
        }
    }
}

impl Node for TestNode {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        ctx.log_info(&format!("{} initialized", self.name));
        Ok(())
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.tick_count += 1;
        if self.tick_count % 10 == 0 {
            ctx.log_info(&format!("{} tick #{}", self.name, self.tick_count));
        }
    }

    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        ctx.log_info(&format!(
            "{} shutdown after {} ticks",
            self.name, self.tick_count
        ));
        Ok(())
    }
}

#[test]
fn test_standard_config() {
    let mut scheduler = Scheduler::new();

    // Apply standard robot configuration
    scheduler.set_config(SchedulerConfig::standard());

    scheduler
        .add(Box::new(TestNode::new("sensor1")), 0, Some(true))
        .add(Box::new(TestNode::new("controller1")), 1, Some(true));

    // Run for a short duration to test
    let result = scheduler.run_for(std::time::Duration::from_millis(100));
    assert!(result.is_ok());
}

#[test]
fn test_safety_critical_config() {
    let mut scheduler = Scheduler::new();

    // Apply safety-critical robot configuration
    scheduler.set_config(SchedulerConfig::safety_critical());

    scheduler
        .add(Box::new(TestNode::new("safety_monitor")), 0, Some(true))
        .add(Box::new(TestNode::new("emergency_stop")), 0, Some(true));

    let result = scheduler.run_for(std::time::Duration::from_millis(50));
    assert!(result.is_ok());
}

#[test]
fn test_high_performance_config() {
    let mut scheduler = Scheduler::new();

    // Apply high-performance robot configuration
    scheduler.set_config(SchedulerConfig::high_performance());

    scheduler
        .add(Box::new(TestNode::new("fast_sensor")), 0, Some(false))
        .add(Box::new(TestNode::new("fast_control")), 1, Some(false));

    let result = scheduler.run_for(std::time::Duration::from_millis(50));
    assert!(result.is_ok());
}

#[test]
fn test_space_robot_config() {
    let mut scheduler = Scheduler::new();

    // Apply space robot configuration
    let config = SchedulerConfig::space();
    scheduler.set_config(config);

    scheduler
        .add(Box::new(TestNode::new("navigation")), 0, Some(true))
        .add(Box::new(TestNode::new("solar_panel")), 5, Some(true));

    let result = scheduler.run_for(std::time::Duration::from_millis(100));
    assert!(result.is_ok());
}

#[test]
fn test_custom_exotic_robot_config() {
    let mut scheduler = Scheduler::new();

    // Create fully custom configuration for an exotic robot type
    let mut config = SchedulerConfig::standard();
    config
        .custom
        .insert("bio_neural_network".to_string(), ConfigValue::Bool(true));
    config.custom.insert(
        "quantum_processor".to_string(),
        ConfigValue::String("entangled".to_string()),
    );
    config
        .custom
        .insert("organic_actuators".to_string(), ConfigValue::Integer(8));
    config.custom.insert(
        "photosynthesis_efficiency".to_string(),
        ConfigValue::Float(0.85),
    );

    scheduler.set_config(config);

    scheduler
        .add(Box::new(TestNode::new("bio_sensor")), 0, Some(true))
        .add(Box::new(TestNode::new("quantum_controller")), 1, Some(true));

    let result = scheduler.run_for(std::time::Duration::from_millis(100));
    assert!(result.is_ok());
}

#[test]
fn test_execution_modes() {
    // Test JIT optimized mode
    {
        let mut scheduler = Scheduler::new();
        let mut config = SchedulerConfig::standard();
        config.execution = ExecutionMode::JITOptimized;
        scheduler.set_config(config);

        scheduler.add(Box::new(TestNode::new("jit_node")), 0, Some(false));
        let result = scheduler.run_for(std::time::Duration::from_millis(50));
        assert!(result.is_ok());
    }

    // Test Sequential mode
    {
        let mut scheduler = Scheduler::new();
        let mut config = SchedulerConfig::standard();
        config.execution = ExecutionMode::Sequential;
        scheduler.set_config(config);

        scheduler.add(Box::new(TestNode::new("seq_node")), 0, Some(false));
        let result = scheduler.run_for(std::time::Duration::from_millis(50));
        assert!(result.is_ok());
    }

    // Test Parallel mode
    {
        let mut scheduler = Scheduler::new();
        let mut config = SchedulerConfig::standard();
        config.execution = ExecutionMode::Parallel;
        scheduler.set_config(config);

        scheduler.add(Box::new(TestNode::new("par_node1")), 0, Some(false));
        scheduler.add(Box::new(TestNode::new("par_node2")), 0, Some(false));
        let result = scheduler.run_for(std::time::Duration::from_millis(50));
        assert!(result.is_ok());
    }
}

#[test]
fn test_swarm_config() {
    let mut scheduler = Scheduler::new();

    // Apply swarm robotics configuration
    let config = SchedulerConfig::swarm();
    scheduler.set_config(config);

    scheduler
        .add(Box::new(TestNode::new("swarm_comm")), 0, Some(true))
        .add(Box::new(TestNode::new("swarm_behavior")), 1, Some(true));

    let result = scheduler.run_for(std::time::Duration::from_millis(100));
    assert!(result.is_ok());
}

#[test]
fn test_soft_robotics_config() {
    let mut scheduler = Scheduler::new();

    // Apply soft robotics configuration
    let config = SchedulerConfig::soft_robotics();
    scheduler.set_config(config);

    scheduler
        .add(Box::new(TestNode::new("pressure_sensor")), 0, Some(true))
        .add(Box::new(TestNode::new("soft_actuator")), 1, Some(true));

    let result = scheduler.run_for(std::time::Duration::from_millis(100));
    assert!(result.is_ok());
}

#[test]
fn test_quantum_config() {
    let mut scheduler = Scheduler::new();

    // Apply quantum-assisted robotics configuration
    let config = SchedulerConfig::quantum();
    scheduler.set_config(config);

    scheduler
        .add(Box::new(TestNode::new("quantum_sensor")), 0, Some(true))
        .add(Box::new(TestNode::new("quantum_optimizer")), 1, Some(true));

    let result = scheduler.run_for(std::time::Duration::from_millis(100));
    assert!(result.is_ok());
}
