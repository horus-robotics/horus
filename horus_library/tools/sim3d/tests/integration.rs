//! Main integration test suite entry point

mod integration_suite;
mod physics_runtime;

use integration_suite::*;

#[test]
fn test_benchmark_runner() {
    let mut runner = benchmarks::BenchmarkRunner::new();

    // Run simple benchmarks
    runner.run("vec_creation", 1000, || {
        let _v: Vec<i32> = Vec::with_capacity(100);
    });

    runner.run("hashmap_insert", 100, || {
        let mut map = std::collections::HashMap::new();
        for i in 0..10 {
            map.insert(i, i * 2);
        }
    });

    // Check that benchmarks ran
    assert_eq!(runner.results.len(), 2);

    // Generate report
    let report = runner.generate_report();
    assert!(report.contains("vec_creation"));
    assert!(report.contains("hashmap_insert"));
}

#[test]
fn test_navigation_scenarios() {
    let simple = navigation::NavigationScenario::simple();
    assert_eq!(simple.obstacle_count, 10);

    let cluttered = navigation::NavigationScenario::cluttered();
    assert!(cluttered.obstacle_count > simple.obstacle_count);

    let maze = navigation::NavigationScenario::maze();
    assert!(maze.obstacle_count > cluttered.obstacle_count);
}

#[test]
fn test_manipulation_scenarios() {
    let simple = manipulation::ManipulationScenario::simple_pick();
    assert_eq!(simple.object_count, 1);

    let multi = manipulation::ManipulationScenario::multi_object();
    assert_eq!(multi.object_count, 10);

    let heavy = manipulation::ManipulationScenario::heavy_object();
    assert!(heavy.object_mass > simple.object_mass);
}

#[test]
fn test_multi_robot_scenarios() {
    let small = multi_robot::MultiRobotScenario::small_swarm();
    assert_eq!(small.robot_count, 5);

    let large = multi_robot::MultiRobotScenario::large_swarm();
    assert_eq!(large.robot_count, 20);

    let formation = multi_robot::MultiRobotScenario::formation_control();
    assert_eq!(
        formation.formation_type,
        multi_robot::FormationType::Formation
    );
}

#[test]
fn test_sensor_throughput() {
    let camera = sensors::SensorThroughputTest::camera_hd();
    let data_rate = camera.expected_data_rate();
    assert!(data_rate > 0.0);

    let lidar = sensors::SensorThroughputTest::lidar_64();
    assert_eq!(lidar.sensor_type, sensors::SensorType::Lidar);

    let depth = sensors::SensorThroughputTest::depth_camera();
    assert_eq!(depth.sensor_type, sensors::SensorType::DepthCamera);
}

#[test]
fn test_determinism_detection() {
    let mut test1 = determinism::DeterminismTest::new("physics", 42, 100);
    let mut test2 = determinism::DeterminismTest::new("physics", 42, 100);

    // Record same sequence
    for i in 0..100 {
        test1.record_state(&i);
        test2.record_state(&i);
    }

    assert!(test1.is_deterministic(&test2));
    assert_eq!(test1.find_divergence(&test2), None);
}

#[test]
fn test_determinism_divergence() {
    let mut test1 = determinism::DeterminismTest::new("physics", 42, 10);
    let mut test2 = determinism::DeterminismTest::new("physics", 42, 10);

    // Same for first 5 steps
    for i in 0..5 {
        test1.record_state(&i);
        test2.record_state(&i);
    }

    // Diverge at step 5
    for i in 5..10 {
        test1.record_state(&i);
        test2.record_state(&(i + 1000));
    }

    assert!(!test1.is_deterministic(&test2));
    assert_eq!(test1.find_divergence(&test2), Some(5));
}

#[test]
fn test_stress_configs() {
    let many_objects = stress::StressTestConfig::many_objects();
    assert_eq!(many_objects.object_count, 1000);

    let many_robots = stress::StressTestConfig::many_robots();
    assert_eq!(many_robots.robot_count, 100);

    let extreme = stress::StressTestConfig::extreme_load();
    assert_eq!(extreme.object_count, 1000);
    assert_eq!(extreme.robot_count, 100);
}

#[test]
fn test_stress_result_evaluation() {
    let config = stress::StressTestConfig::many_objects();
    let mut result = stress::StressTestResult::new(config);

    result.avg_step_time = std::time::Duration::from_micros(8000); // 8ms
    result.peak_memory_mb = 150.0;
    result.successful_steps = 1000;

    // Should pass with generous limits
    assert!(result.passed(10.0, 200.0));

    // Should fail with strict limits
    assert!(!result.passed(5.0, 200.0));
    assert!(!result.passed(10.0, 100.0));
}

#[test]
fn test_memory_estimation() {
    let memory = stress::MemoryTracker::estimate_memory(1000, 10);
    // Should be reasonable (1000 objects + 10 robots)
    assert!(memory > 100_000); // > 100 KB
    assert!(memory < 10_000_000); // < 10 MB
}
