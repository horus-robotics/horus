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

// Expose async_io module for AsyncNode
pub mod async_io {
    pub use super::executors::async_io::AsyncNode;
}

pub use config::{ConfigValue, ExecutionMode, RobotPreset, SchedulerConfig};
pub use safety_monitor::{SafetyMonitor, SafetyState, SafetyStats, WCETEnforcer, Watchdog};
pub use scheduler::Scheduler;
