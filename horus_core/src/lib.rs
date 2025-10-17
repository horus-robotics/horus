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
//! ```rust
//! use horus_core::{Node, NodeInfo, Scheduler, Hub};
//! 
//! struct ExampleNode {
//!     output: Hub<String>,
//! }
//! 
//! impl Node for ExampleNode {
//!     fn name(&self) -> &'static str { "example" }
//!     
//!     fn tick(&mut self, _ctx: &mut NodeInfo) {
//!         let _ = self.output.send("Hello HORUS!".into(), ctx.as_deref_mut());
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

// Re-export commonly used types for easy access
pub use core::{Node, NodeInfo, NodeState, NodePriority, NodeConfig};
pub use communication::{Hub, Link};
pub use scheduling::Scheduler;
pub use error::{HorusError, HorusResult};
pub use params::RuntimeParams;

// Re-export communication traits for backend-agnostic usage
pub use communication::traits::{Publisher, Subscriber, Channel, IpcBackend};
