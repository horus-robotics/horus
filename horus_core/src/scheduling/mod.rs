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
//! ```rust
//! use horus_core::Scheduler;
//! 
//! let mut scheduler = Scheduler::new();
//! scheduler.register(Box::new(sensor_node), 10, Some(true));  // Enable logging
//! scheduler.register(Box::new(control_node), 20, Some(false)); // Disable logging
//! scheduler.register(Box::new(background_node), 200, None);    // Default logging (false)
//! scheduler.tick_all(); // Handles initialization automatically
//! ```
//!
//! ## Priority Levels
//!
//! - **0-99**: High priority (real-time, sensors, control)
//! - **100-199**: Normal priority (processing, algorithms)
//! - **200+**: Background priority (logging, diagnostics)

pub mod scheduler;

pub use scheduler::Scheduler;
