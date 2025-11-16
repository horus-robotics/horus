/// Dependency graph analysis for detecting parallelizable nodes
pub mod dependency_graph;

/// Runtime profiler for learning node execution characteristics
pub mod profiler;

/// Execution tier classifier for optimal backend selection
pub mod classifier;

pub use classifier::{ExecutionTier, TierClassifier};
pub use dependency_graph::DependencyGraph;
pub use profiler::RuntimeProfiler;
