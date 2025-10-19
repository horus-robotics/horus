//! # Core types and traits for the HORUS framework
//!
//! This module contains the fundamental building blocks of the HORUS system:
//!
//! - **Node**: The base trait for all computational units in HORUS
//! - **NodeContext**: Runtime context and utilities provided to nodes during execution
//! - **Contracts**: Message schemas and validation for type-safe communication
//! - **Data Fields**: Structured data types for robotics sensors and actuators
//!
//! ## Node Lifecycle
//!
//! All HORUS nodes follow a consistent lifecycle:
//! 1. **Construction** - Node is created with configuration
//! 2. **Initialization** - `init()` is called to set up resources
//! 3. **Execution** - `tick()` is called repeatedly by the scheduler
//! 4. **Shutdown** - `shutdown()` is called to clean up resources

pub mod log_buffer;
pub mod node;

pub use log_buffer::{LogEntry, LogType, SharedLogBuffer, GLOBAL_LOG_BUFFER};
pub use node::{
    HealthStatus, Node, NodeConfig, NodeHeartbeat, NodeInfo, NodeMetrics, NodePriority, NodeState,
};
