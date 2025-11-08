use std::collections::HashMap;
use std::time::Duration;

/// Runtime profiler that learns node characteristics during execution
/// Uses Welford's online algorithm for variance calculation
#[derive(Debug, Clone)]
pub struct RuntimeProfiler {
    pub node_stats: HashMap<String, NodeStats>,
    learning_phase: bool,
    learning_ticks: usize,
    target_learning_ticks: usize,
    enabled: bool,
    pub force_ultra_fast_classification: bool,
    pub force_async_io_classification: bool,
}

/// Statistics for a single node
#[derive(Debug, Clone)]
pub struct NodeStats {
    /// Average execution time in microseconds
    pub avg_us: f64,
    /// Standard deviation in microseconds
    pub stddev_us: f64,
    /// Number of samples collected
    pub count: usize,
    /// Is execution time deterministic? (low variance)
    pub is_deterministic: bool,
    /// Does this node perform I/O operations? (detected by long blocking times)
    pub is_io_heavy: bool,
    /// Is this node CPU-bound? (high execution time, low variance)
    pub is_cpu_bound: bool,
    /// Minimum execution time observed
    pub min_us: f64,
    /// Maximum execution time observed
    pub max_us: f64,
    /// Welford's algorithm internal state
    mean: f64,
    m2: f64,
}

impl Default for NodeStats {
    fn default() -> Self {
        Self {
            avg_us: 0.0,
            stddev_us: 0.0,
            count: 0,
            is_deterministic: false,
            is_io_heavy: false,
            is_cpu_bound: false,
            min_us: f64::MAX,
            max_us: 0.0,
            mean: 0.0,
            m2: 0.0,
        }
    }
}

impl NodeStats {
    /// Update statistics with new sample using Welford's online algorithm
    /// This allows us to compute variance without storing all samples
    pub fn update(&mut self, duration_us: f64) {
        self.count += 1;

        // Update min/max
        self.min_us = self.min_us.min(duration_us);
        self.max_us = self.max_us.max(duration_us);

        // Welford's online algorithm for mean and variance
        let delta = duration_us - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = duration_us - self.mean;
        self.m2 += delta * delta2;

        // Update public fields
        self.avg_us = self.mean;
        if self.count > 1 {
            self.stddev_us = (self.m2 / (self.count - 1) as f64).sqrt();
        }

        // Classify node characteristics (after sufficient samples)
        if self.count >= 10 {
            self.classify();
        }
    }

    /// Classify node based on execution patterns
    fn classify(&mut self) {
        let cv = if self.avg_us > 0.0 {
            self.stddev_us / self.avg_us // Coefficient of variation
        } else {
            0.0
        };

        // Deterministic: low coefficient of variation (<10%)
        self.is_deterministic = cv < 0.10;

        // I/O heavy: high variance AND occasional long spikes
        // Typical pattern: fast most of the time, occasional blocking
        self.is_io_heavy = cv > 0.30 && self.max_us > self.avg_us * 2.0;

        // CPU-bound: high average time, low variance
        self.is_cpu_bound = self.avg_us > 100.0 && cv < 0.20;
    }

    /// Get 95th percentile estimate (mean + 2*stddev)
    pub fn p95_us(&self) -> f64 {
        self.avg_us + 2.0 * self.stddev_us
    }

    /// Get 99th percentile estimate (mean + 3*stddev)
    pub fn p99_us(&self) -> f64 {
        self.avg_us + 3.0 * self.stddev_us
    }
}

impl RuntimeProfiler {
    /// Create new profiler with specified learning duration
    pub fn new(target_learning_ticks: usize) -> Self {
        Self {
            node_stats: HashMap::new(),
            learning_phase: true,
            learning_ticks: 0,
            target_learning_ticks,
            enabled: true,
            force_ultra_fast_classification: false,
            force_async_io_classification: false,
        }
    }

    /// Create profiler with default 100-tick learning phase
    pub fn new_default() -> Self {
        Self::new(100)
    }

    /// Enable profiling
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable profiling
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Record execution time for a node
    pub fn record(&mut self, node_name: &str, duration: Duration) {
        if !self.enabled {
            return;
        }

        let duration_us = duration.as_micros() as f64;

        self.node_stats
            .entry(node_name.to_string())
            .or_insert_with(NodeStats::default)
            .update(duration_us);
    }

    /// Advance learning phase tick counter
    pub fn tick(&mut self) {
        if self.learning_phase {
            self.learning_ticks += 1;
            if self.learning_ticks >= self.target_learning_ticks {
                self.learning_phase = false;
            }
        }
    }

    /// Check if learning phase is complete
    pub fn is_learning_complete(&self) -> bool {
        !self.learning_phase
    }

    /// Get progress through learning phase (0.0 to 1.0)
    pub fn learning_progress(&self) -> f64 {
        if self.target_learning_ticks == 0 {
            1.0
        } else {
            (self.learning_ticks as f64 / self.target_learning_ticks as f64).min(1.0)
        }
    }

    /// Get statistics for a specific node
    pub fn get_stats(&self, node_name: &str) -> Option<&NodeStats> {
        self.node_stats.get(node_name)
    }

    /// Get all ultra-fast nodes (<5μs average)
    pub fn get_ultra_fast_nodes(&self) -> Vec<String> {
        if self.force_ultra_fast_classification {
            // Force all nodes to be classified as ultra-fast
            self.node_stats.keys().cloned().collect()
        } else {
            self.node_stats
                .iter()
                .filter(|(_, stats)| stats.avg_us < 5.0 && stats.is_deterministic)
                .map(|(name, _)| name.clone())
                .collect()
        }
    }

    /// Get all I/O heavy nodes
    pub fn get_io_heavy_nodes(&self) -> Vec<String> {
        if self.force_async_io_classification {
            // Force all nodes to be classified as I/O heavy
            self.node_stats.keys().cloned().collect()
        } else {
            self.node_stats
                .iter()
                .filter(|(_, stats)| stats.is_io_heavy)
                .map(|(name, _)| name.clone())
                .collect()
        }
    }

    /// Get all CPU-bound nodes
    pub fn get_cpu_bound_nodes(&self) -> Vec<String> {
        self.node_stats
            .iter()
            .filter(|(_, stats)| stats.is_cpu_bound)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Generate summary report
    pub fn summary(&self) -> ProfilerSummary {
        let total_nodes = self.node_stats.len();
        let ultra_fast = self.get_ultra_fast_nodes().len();
        let io_heavy = self.get_io_heavy_nodes().len();
        let cpu_bound = self.get_cpu_bound_nodes().len();

        let mut total_avg_us: f64 = 0.0;
        let mut min_avg_us: f64 = f64::MAX;
        let mut max_avg_us: f64 = 0.0;

        for stats in self.node_stats.values() {
            total_avg_us += stats.avg_us;
            min_avg_us = min_avg_us.min(stats.avg_us);
            max_avg_us = max_avg_us.max(stats.avg_us);
        }

        let avg_avg_us = if total_nodes > 0 {
            total_avg_us / total_nodes as f64
        } else {
            0.0
        };

        ProfilerSummary {
            total_nodes,
            ultra_fast_nodes: ultra_fast,
            io_heavy_nodes: io_heavy,
            cpu_bound_nodes: cpu_bound,
            avg_execution_us: avg_avg_us,
            min_execution_us: if min_avg_us == f64::MAX {
                0.0
            } else {
                min_avg_us
            },
            max_execution_us: max_avg_us,
            learning_complete: self.is_learning_complete(),
            learning_progress: self.learning_progress(),
        }
    }

    /// Print detailed statistics for all nodes
    pub fn print_stats(&self) {
        println!("\n=== Runtime Profiler Statistics ===");
        println!(
            "Learning Phase: {} ({:.0}%)",
            if self.learning_phase {
                "Active"
            } else {
                "Complete"
            },
            self.learning_progress() * 100.0
        );
        println!("Total Nodes: {}", self.node_stats.len());
        println!("\nNode Statistics:");
        println!(
            "{:<30} {:>10} {:>10} {:>10} {:>10} {:>12}",
            "Node", "Avg (μs)", "StdDev", "Min", "Max", "Type"
        );
        println!("{}", "-".repeat(92));

        let mut nodes: Vec<_> = self.node_stats.iter().collect();
        nodes.sort_by(|a, b| a.1.avg_us.partial_cmp(&b.1.avg_us).unwrap());

        for (name, stats) in nodes {
            let node_type = if stats.is_deterministic && stats.avg_us < 5.0 {
                "UltraFast"
            } else if stats.is_io_heavy {
                "I/O Heavy"
            } else if stats.is_cpu_bound {
                "CPU Bound"
            } else if stats.is_deterministic {
                "Fast"
            } else {
                "Variable"
            };

            println!(
                "{:<30} {:>10.2} {:>10.2} {:>10.2} {:>10.2} {:>12}",
                name, stats.avg_us, stats.stddev_us, stats.min_us, stats.max_us, node_type
            );
        }

        let summary = self.summary();
        println!("\n=== Summary ===");
        println!(
            "Ultra-Fast Nodes: {} ({:.1}%)",
            summary.ultra_fast_nodes,
            (summary.ultra_fast_nodes as f64 / summary.total_nodes as f64) * 100.0
        );
        println!(
            "I/O Heavy Nodes: {} ({:.1}%)",
            summary.io_heavy_nodes,
            (summary.io_heavy_nodes as f64 / summary.total_nodes as f64) * 100.0
        );
        println!(
            "CPU Bound Nodes: {} ({:.1}%)",
            summary.cpu_bound_nodes,
            (summary.cpu_bound_nodes as f64 / summary.total_nodes as f64) * 100.0
        );
        println!();
    }
}

/// Summary of profiler statistics
#[derive(Debug, Clone)]
pub struct ProfilerSummary {
    pub total_nodes: usize,
    pub ultra_fast_nodes: usize,
    pub io_heavy_nodes: usize,
    pub cpu_bound_nodes: usize,
    pub avg_execution_us: f64,
    pub min_execution_us: f64,
    pub max_execution_us: f64,
    pub learning_complete: bool,
    pub learning_progress: f64,
}
