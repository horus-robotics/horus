//! # HORUS - Hybrid Optimized Robotics Unified System
//!
//! HORUS provides a comprehensive framework for building robotics applications in Rust,
//! with a focus on performance, safety, and developer experience.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use horus::prelude::*;
//! use horus::library::messages::cmd_vel::CmdVel;
//!
//! pub struct MyNode {
//!     publisher: Hub<CmdVel>,
//! }
//!
//! impl Node for MyNode {
//!     fn name(&self) -> &'static str { "MyNode" }
//!
//!     fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
//!         // Node logic here
//!     }
//! }
//! ```
//!
//! ## Features
//!
//! - **Zero-copy IPC** with multiple backend support
//! - **Type-safe message passing**
//! - **Built-in monitoring and debugging**
//! - **Standard library of components**
//! - **Comprehensive tooling**

// Re-export core components (avoiding conflicts)
pub use horus_core::{self, *};

// Re-export macros
#[cfg(feature = "macros")]
pub use horus_macros::*;

// Re-export standard library with alias
pub use horus_library as library;

/// The HORUS prelude - everything you need to get started
pub mod prelude {
    // Core node types
    pub use horus_core::core::node::NodeConfig;
    pub use horus_core::core::{Node, NodeInfo, NodeState};

    // Communication types
    pub use horus_core::communication::{Hub, Link};

    // Scheduling
    pub use horus_core::scheduling::Scheduler;

    // Error types
    pub use horus_core::error::{HorusError, HorusResult};
    pub type Result<T> = HorusResult<T>;

    // Common std types
    pub use std::sync::{Arc, Mutex};
    pub use std::time::{Duration, Instant};

    #[cfg(feature = "macros")]
    pub use horus_macros::*;

    // Common traits
    pub use serde::{Deserialize, Serialize};

    // Re-export anyhow for error handling
    pub use anyhow::{anyhow, bail, ensure, Context, Result as AnyResult};

    // Re-export all message types from horus_library for convenience
    pub use horus_library::messages::*;
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get HORUS version
pub fn version() -> &'static str {
    VERSION
}
