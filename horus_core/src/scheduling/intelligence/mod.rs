/// Dependency graph analysis for detecting parallelizable nodes
pub mod dependency_graph;

/// Runtime profiler for learning node execution characteristics
pub mod profiler;

/// Execution tier classifier for optimal backend selection
pub mod classifier;

pub use classifier::{ExecutionTier, TierClassifier, TierStats};
pub use dependency_graph::{DependencyGraph, GraphStats};
pub use profiler::{NodeStats, ProfilerSummary, RuntimeProfiler};
