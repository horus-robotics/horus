//! Stress tests with many objects and robots

use bevy::prelude::*;
use std::time::{Duration, Instant};

/// Stress test configuration
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    pub name: String,
    pub object_count: usize,
    pub robot_count: usize,
    pub simulation_steps: usize,
    pub enable_physics: bool,
    pub enable_sensors: bool,
}

impl StressTestConfig {
    pub fn many_objects() -> Self {
        Self {
            name: "1000 Objects".to_string(),
            object_count: 1000,
            robot_count: 1,
            simulation_steps: 1000,
            enable_physics: true,
            enable_sensors: false,
        }
    }

    pub fn many_robots() -> Self {
        Self {
            name: "100 Robots".to_string(),
            object_count: 10,
            robot_count: 100,
            simulation_steps: 1000,
            enable_physics: true,
            enable_sensors: true,
        }
    }

    pub fn extreme_load() -> Self {
        Self {
            name: "Extreme Load (1000 objects + 100 robots)".to_string(),
            object_count: 1000,
            robot_count: 100,
            simulation_steps: 100,
            enable_physics: true,
            enable_sensors: true,
        }
    }

    pub fn physics_only() -> Self {
        Self {
            name: "Physics Stress (5000 objects)".to_string(),
            object_count: 5000,
            robot_count: 0,
            simulation_steps: 500,
            enable_physics: true,
            enable_sensors: false,
        }
    }
}

/// Stress test result
#[derive(Debug, Clone)]
pub struct StressTestResult {
    pub config: StressTestConfig,
    pub total_duration: Duration,
    pub avg_step_time: Duration,
    pub max_step_time: Duration,
    pub min_step_time: Duration,
    pub peak_memory_mb: f64,
    pub successful_steps: usize,
    pub failed_steps: usize,
}

impl StressTestResult {
    pub fn new(config: StressTestConfig) -> Self {
        Self {
            config,
            total_duration: Duration::ZERO,
            avg_step_time: Duration::ZERO,
            max_step_time: Duration::ZERO,
            min_step_time: Duration::MAX,
            peak_memory_mb: 0.0,
            successful_steps: 0,
            failed_steps: 0,
        }
    }

    /// Check if test passed performance criteria
    pub fn passed(&self, max_avg_step_ms: f64, max_memory_mb: f64) -> bool {
        let avg_ms = self.avg_step_time.as_secs_f64() * 1000.0;
        avg_ms <= max_avg_step_ms && self.peak_memory_mb <= max_memory_mb && self.failed_steps == 0
    }

    /// Generate report
    pub fn format_report(&self) -> String {
        let avg_ms = self.avg_step_time.as_secs_f64() * 1000.0;
        let max_ms = self.max_step_time.as_secs_f64() * 1000.0;
        let min_ms = self.min_step_time.as_secs_f64() * 1000.0;

        format!(
            "{}\n  Objects: {}, Robots: {}\n  Steps: {}/{}\n  Avg: {:.2}ms, Min: {:.2}ms, Max: {:.2}ms\n  Memory: {:.1} MB",
            self.config.name,
            self.config.object_count,
            self.config.robot_count,
            self.successful_steps,
            self.config.simulation_steps,
            avg_ms,
            min_ms,
            max_ms,
            self.peak_memory_mb
        )
    }
}

/// Memory usage tracker
pub struct MemoryTracker {
    peak_bytes: usize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self { peak_bytes: 0 }
    }

    /// Update peak memory (simplified - would use actual memory stats in production)
    pub fn update(&mut self, estimated_bytes: usize) {
        if estimated_bytes > self.peak_bytes {
            self.peak_bytes = estimated_bytes;
        }
    }

    /// Get peak memory in MB
    pub fn peak_mb(&self) -> f64 {
        self.peak_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Estimate memory for objects and robots
    pub fn estimate_memory(object_count: usize, robot_count: usize) -> usize {
        // Rough estimate: 1 KB per object, 10 KB per robot
        object_count * 1024 + robot_count * 10240
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_config_many_objects() {
        let config = StressTestConfig::many_objects();
        assert_eq!(config.object_count, 1000);
        assert_eq!(config.robot_count, 1);
    }

    #[test]
    fn test_stress_config_many_robots() {
        let config = StressTestConfig::many_robots();
        assert_eq!(config.robot_count, 100);
        assert!(config.enable_sensors);
    }

    #[test]
    fn test_stress_config_extreme() {
        let config = StressTestConfig::extreme_load();
        assert_eq!(config.object_count, 1000);
        assert_eq!(config.robot_count, 100);
    }

    #[test]
    fn test_stress_result_creation() {
        let config = StressTestConfig::many_objects();
        let result = StressTestResult::new(config);
        assert_eq!(result.successful_steps, 0);
        assert_eq!(result.failed_steps, 0);
    }

    #[test]
    fn test_stress_result_passed() {
        let mut result = StressTestResult::new(StressTestConfig::many_objects());
        result.avg_step_time = Duration::from_micros(5000); // 5ms
        result.peak_memory_mb = 100.0;
        result.successful_steps = 1000;

        assert!(result.passed(10.0, 200.0)); // Should pass
        assert!(!result.passed(2.0, 200.0)); // Should fail (too slow)
        assert!(!result.passed(10.0, 50.0)); // Should fail (too much memory)
    }

    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();
        tracker.update(1024 * 1024); // 1 MB
        tracker.update(512 * 1024); // 0.5 MB (lower, shouldn't update peak)

        assert!((tracker.peak_mb() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_memory() {
        let memory = MemoryTracker::estimate_memory(1000, 10);
        // 1000 objects * 1KB + 10 robots * 10KB = 1000KB + 100KB = 1100KB
        assert_eq!(memory, 1024 * 1000 + 10240 * 10);
    }

    #[test]
    fn test_stress_result_report() {
        let config = StressTestConfig::many_objects();
        let mut result = StressTestResult::new(config);
        result.avg_step_time = Duration::from_micros(5000);
        result.max_step_time = Duration::from_micros(10000);
        result.min_step_time = Duration::from_micros(3000);
        result.peak_memory_mb = 150.0;
        result.successful_steps = 1000;

        let report = result.format_report();
        assert!(report.contains("1000 Objects"));
        assert!(report.contains("5.00ms"));
    }
}
