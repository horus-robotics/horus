use super::profiler::{NodeStats, RuntimeProfiler};
use std::collections::HashMap;

/// Execution tier for a node based on characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionTier {
    /// Ultra-fast deterministic nodes (<5μs) - JIT dataflow
    UltraFast,
    /// Fast nodes (<1ms) - Inline execution
    Fast,
    /// I/O heavy nodes - Async/await
    AsyncIO,
    /// High failure rate - Process isolation
    Isolated,
    /// Everything else - Standard execution
    Background,
}

impl ExecutionTier {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            ExecutionTier::UltraFast => "UltraFast (JIT)",
            ExecutionTier::Fast => "Fast (Inline)",
            ExecutionTier::AsyncIO => "Async I/O",
            ExecutionTier::Isolated => "Isolated",
            ExecutionTier::Background => "Background",
        }
    }

    /// Get expected latency range
    pub fn latency_range(&self) -> &'static str {
        match self {
            ExecutionTier::UltraFast => "20-50ns",
            ExecutionTier::Fast => "50-100ns",
            ExecutionTier::AsyncIO => "10-100μs",
            ExecutionTier::Isolated => "1-10ms",
            ExecutionTier::Background => "1-100ms",
        }
    }
}

/// Classifier that assigns nodes to execution tiers
#[derive(Debug, Clone)]
pub struct TierClassifier {
    /// Tier assignments for each node
    pub assignments: HashMap<String, ExecutionTier>,
}

impl TierClassifier {
    /// Create new classifier and assign tiers based on profiler data
    pub fn from_profiler(profiler: &RuntimeProfiler) -> Self {
        let mut assignments = HashMap::new();

        for (node_name, stats) in &profiler.node_stats {
            let tier = Self::classify_node(stats);
            assignments.insert(node_name.clone(), tier);
        }

        Self { assignments }
    }

    /// Classify a single node based on its statistics
    fn classify_node(stats: &NodeStats) -> ExecutionTier {
        // Priority 1: Ultra-fast deterministic nodes  JIT tier
        if stats.avg_us < 5.0 && stats.is_deterministic {
            return ExecutionTier::UltraFast;
        }

        // Priority 2: I/O heavy nodes  Async tier
        if stats.is_io_heavy {
            return ExecutionTier::AsyncIO;
        }

        // Priority 3: Fast nodes (<1ms)  Inline tier
        if stats.avg_us < 1000.0 {
            return ExecutionTier::Fast;
        }

        // Priority 4: Default to background
        ExecutionTier::Background
    }

    /// Get tier for a specific node
    pub fn get_tier(&self, node_name: &str) -> Option<ExecutionTier> {
        self.assignments.get(node_name).copied()
    }

    /// Get all nodes in a specific tier
    pub fn get_nodes_in_tier(&self, tier: ExecutionTier) -> Vec<String> {
        self.assignments
            .iter()
            .filter(|(_, &t)| t == tier)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get statistics about tier distribution
    pub fn tier_stats(&self) -> TierStats {
        let mut stats = TierStats::default();
        stats.total_nodes = self.assignments.len();

        for tier in self.assignments.values() {
            match tier {
                ExecutionTier::UltraFast => stats.ultra_fast += 1,
                ExecutionTier::Fast => stats.fast += 1,
                ExecutionTier::AsyncIO => stats.async_io += 1,
                ExecutionTier::Isolated => stats.isolated += 1,
                ExecutionTier::Background => stats.background += 1,
            }
        }

        stats
    }

    /// Print classification results
    pub fn print_classification(&self) {
        println!("\n=== Execution Tier Classification ===");

        let stats = self.tier_stats();
        println!("Total Nodes: {}", stats.total_nodes);
        println!("\nTier Distribution:");
        println!(
            "{:<20} {:>8} {:>8} {:>15}",
            "Tier", "Count", "Percent", "Latency"
        );
        println!("{}", "-".repeat(60));

        let tiers = [
            (ExecutionTier::UltraFast, stats.ultra_fast),
            (ExecutionTier::Fast, stats.fast),
            (ExecutionTier::AsyncIO, stats.async_io),
            (ExecutionTier::Isolated, stats.isolated),
            (ExecutionTier::Background, stats.background),
        ];

        for (tier, count) in tiers {
            let percent = if stats.total_nodes > 0 {
                (count as f64 / stats.total_nodes as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "{:<20} {:>8} {:>7.1}% {:>15}",
                tier.name(),
                count,
                percent,
                tier.latency_range()
            );
        }

        // Print node assignments by tier
        println!("\n=== Nodes by Tier ===");
        for tier in &[
            ExecutionTier::UltraFast,
            ExecutionTier::Fast,
            ExecutionTier::AsyncIO,
            ExecutionTier::Isolated,
            ExecutionTier::Background,
        ] {
            let nodes = self.get_nodes_in_tier(*tier);
            if !nodes.is_empty() {
                println!("\n{}:", tier.name());
                for node in nodes {
                    println!("  - {}", node);
                }
            }
        }
        println!();
    }
}

/// Statistics about tier distribution
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct TierStats {
    pub total_nodes: usize,
    pub ultra_fast: usize,
    pub fast: usize,
    pub async_io: usize,
    pub isolated: usize,
    pub background: usize,
}

impl TierStats {
    /// Get percentage of nodes in ultra-fast tier
    pub fn ultra_fast_percent(&self) -> f64 {
        if self.total_nodes == 0 {
            0.0
        } else {
            (self.ultra_fast as f64 / self.total_nodes as f64) * 100.0
        }
    }

    /// Get percentage of nodes that can run in parallel (ultra-fast + fast)
    pub fn parallel_capable_percent(&self) -> f64 {
        if self.total_nodes == 0 {
            0.0
        } else {
            ((self.ultra_fast + self.fast) as f64 / self.total_nodes as f64) * 100.0
        }
    }
}
