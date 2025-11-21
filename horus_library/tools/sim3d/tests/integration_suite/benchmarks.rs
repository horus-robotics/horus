//! Benchmark suite for performance tracking

use bevy::prelude::*;
use sim3d::{
    physics::PhysicsPlugin,
    sensors::{camera::CameraPlugin, lidar::LidarPlugin},
};
use std::time::{Duration, Instant};

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration: Duration,
    pub iterations: usize,
    pub throughput: f64, // ops/sec
    pub memory_mb: f64,
}

impl BenchmarkResult {
    pub fn new(name: impl Into<String>, duration: Duration, iterations: usize) -> Self {
        let throughput = iterations as f64 / duration.as_secs_f64();
        Self {
            name: name.into(),
            duration,
            iterations,
            throughput,
            memory_mb: 0.0,
        }
    }

    pub fn with_memory(mut self, memory_mb: f64) -> Self {
        self.memory_mb = memory_mb;
        self
    }

    /// Check if performance regressed compared to baseline
    pub fn has_regressed(&self, baseline: &BenchmarkResult, threshold: f64) -> bool {
        let regression = (baseline.throughput - self.throughput) / baseline.throughput;
        regression > threshold
    }

    pub fn format_report(&self) -> String {
        format!(
            "{}: {:.2}ms, {:.0} ops/sec, {:.1} MB",
            self.name,
            self.duration.as_secs_f64() * 1000.0,
            self.throughput,
            self.memory_mb
        )
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    pub results: Vec<BenchmarkResult>,
    pub baselines: Vec<BenchmarkResult>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            baselines: Vec::new(),
        }
    }

    /// Run a benchmark
    pub fn run<F>(&mut self, name: impl Into<String>, iterations: usize, mut f: F)
    where
        F: FnMut(),
    {
        let name = name.into();

        // Warmup
        for _ in 0..10 {
            f();
        }

        // Actual benchmark
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        let duration = start.elapsed();

        let result = BenchmarkResult::new(name, duration, iterations);
        println!("{}", result.format_report());
        self.results.push(result);
    }

    /// Load baseline results
    pub fn load_baselines(&mut self, baselines: Vec<BenchmarkResult>) {
        self.baselines = baselines;
    }

    /// Check for regressions
    pub fn check_regressions(&self, threshold: f64) -> Vec<(String, f64)> {
        let mut regressions = Vec::new();

        for result in &self.results {
            if let Some(baseline) = self.baselines.iter().find(|b| b.name == result.name) {
                if result.has_regressed(baseline, threshold) {
                    let regression_pct =
                        (baseline.throughput - result.throughput) / baseline.throughput * 100.0;
                    regressions.push((result.name.clone(), regression_pct));
                }
            }
        }

        regressions
    }

    /// Generate report
    pub fn generate_report(&self) -> String {
        let mut report = String::from("=== Benchmark Report ===\n\n");

        for result in &self.results {
            report.push_str(&format!("{}\n", result.format_report()));

            // Compare with baseline
            if let Some(baseline) = self.baselines.iter().find(|b| b.name == result.name) {
                let diff_pct =
                    (result.throughput - baseline.throughput) / baseline.throughput * 100.0;
                report.push_str(&format!("  vs baseline: {:+.1}%\n", diff_pct));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result() {
        let result = BenchmarkResult::new("test", Duration::from_secs(1), 1000);
        assert_eq!(result.throughput, 1000.0);
    }

    #[test]
    fn test_regression_detection() {
        let baseline = BenchmarkResult::new("test", Duration::from_secs(1), 1000);
        let current = BenchmarkResult::new("test", Duration::from_secs(2), 1000);

        // Current is slower (500 ops/s vs 1000 ops/s) = 50% regression
        assert!(current.has_regressed(&baseline, 0.4));
        assert!(!current.has_regressed(&baseline, 0.6));
    }

    #[test]
    fn test_benchmark_runner() {
        let mut runner = BenchmarkRunner::new();

        runner.run("simple_op", 100, || {
            let _ = 1 + 1;
        });

        assert_eq!(runner.results.len(), 1);
        assert_eq!(runner.results[0].name, "simple_op");
    }

    #[test]
    fn test_check_regressions() {
        let mut runner = BenchmarkRunner::new();

        // Set baseline
        runner.load_baselines(vec![BenchmarkResult::new(
            "fast_op",
            Duration::from_millis(100),
            1000,
        )]);

        // Run current (slower)
        runner.results.push(BenchmarkResult::new(
            "fast_op",
            Duration::from_millis(200),
            1000,
        ));

        let regressions = runner.check_regressions(0.4);
        assert_eq!(regressions.len(), 1);
        assert_eq!(regressions[0].0, "fast_op");
    }
}
