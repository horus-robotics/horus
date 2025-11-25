//! # HORUS Scheduling System
//!
//! Simple, unified scheduling system that orchestrates node execution:
//!
//! - **Scheduler**: Unified scheduler with built-in monitoring integration
//! - **Simple Priorities**: Numeric priorities (0 = highest)
//! - **Optional Logging**: Per-node logging configuration
//!
//! ## Usage
//!
//! ```rust,ignore
//! use horus_core::Scheduler;
//!
//! let mut scheduler = Scheduler::new();
//! scheduler.add(Box::new(sensor_node), 10, Some(true));  // Enable logging
//! scheduler.add(Box::new(control_node), 20, Some(false)); // Disable logging
//! scheduler.add(Box::new(background_node), 200, None);    // Default logging (false)
//! scheduler.run(); // Handles initialization automatically
//! ```
//!
//! ## Priority Levels
//!
//! - **0-99**: High priority (real-time, sensors, control)
//! - **100-199**: Normal priority (processing, algorithms)
//! - **200+**: Background priority (logging, diagnostics)

pub mod config;
pub mod safety_monitor;
pub mod scheduler;

// Internal intelligence modules
mod executors;
mod fault_tolerance;
mod intelligence;
pub mod jit;

// Runtime OS-level features
pub mod runtime;

// Fault tolerance and monitoring
pub mod blackbox;
pub mod checkpoint;
pub mod redundancy;
pub mod telemetry;

// Record/Replay system
pub mod record_replay;

// Expose async_io module for AsyncNode
pub mod async_io {
    pub use super::executors::async_io::AsyncNode;
}

pub use config::{ConfigValue, ExecutionMode, RobotPreset, SchedulerConfig};
pub use safety_monitor::{SafetyMonitor, SafetyState, SafetyStats, WCETEnforcer, Watchdog};
pub use scheduler::Scheduler;

// Re-export runtime features
pub use runtime::{
    apply_rt_optimizations, get_core_count, get_max_rt_priority, get_numa_node_count,
    lock_all_memory, set_realtime_priority, set_thread_affinity,
};

// Re-export fault tolerance
pub use blackbox::{BlackBox, BlackBoxEvent};
pub use checkpoint::{Checkpoint, CheckpointManager};
pub use redundancy::{RedundancyManager, VoteResult, VotingStrategy};
pub use telemetry::{TelemetryEndpoint, TelemetryManager};

// Re-export record/replay
pub use record_replay::{
    NodeRecorder, NodeRecording, NodeReplayer, NodeTickSnapshot, RecordingConfig,
    RecordingManager, SchedulerRecording,
};
