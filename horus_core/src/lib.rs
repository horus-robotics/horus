//! # HORUS Core
//!
//! The core runtime system for the HORUS robotics framework.
//!
//! HORUS is a distributed real-time robotics system designed for high-performance
//! applications. This crate provides the fundamental building blocks:
//!
//! - **Nodes**: Independent computational units that process data
//! - **Communication**: Publisher-subscriber message passing between nodes
//! - **Memory**: High-performance shared memory and zero-copy messaging
//! - **Scheduling**: Real-time task scheduling and execution
//! - **Monitoring**: Cross-process system monitoring and diagnostics
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use horus_core::{Node, NodeInfo, Scheduler, Hub};
//!
//! struct ExampleNode {
//!     output: Hub<String>,
//! }
//!
//! impl Node for ExampleNode {
//!     fn name(&self) -> &'static str { "example" }
//!
//!     fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
//!         let _ = self.output.send("Hello HORUS!".into(), ctx);
//!     }
//! }
//! ```

pub mod backend;
pub mod communication;
pub mod core;
pub mod error;
pub mod memory;
pub mod params;
pub mod scheduling;

#[cfg(feature = "rtos")]
pub mod rtos;

// Re-export commonly used types for easy access
pub use communication::{Hub, Link, LinkMetrics};
pub use core::{
    HealthStatus, Node, NodeConfig, NodeHeartbeat, NodeInfo, NodeInfoExt, NodeMetrics, NodePriority, NodeState,
    TopicMetadata,
};
pub use error::{HorusError, HorusResult};
pub use params::RuntimeParams;
pub use scheduling::Scheduler;

// Re-export communication traits for backend-agnostic usage
pub use communication::traits::{Channel, Publisher, Subscriber};
