//! # Communication layer for HORUS
//!
//! This module provides native HORUS IPC with shared memory and cache optimizations:
//!
//! - **Hub**: MPMC publisher-subscriber pattern (167-6994 ns/msg)
//! - **Link**: SPSC point-to-point channels (85-167 ns/msg, ultra-low latency)
//!
//! ## Usage Patterns
//!
//! **For ultra-low latency (real-time control loops):**
//! ```rust,ignore
//! use horus_core::communication::Link;
//! let link = Link::new("producer", "consumer", "topic");
//! ```
//!
//! **For general-purpose IPC:**
//! ```rust,no_run
//! use horus_core::communication::Hub;
//! let hub: Hub<String> = Hub::new("topic_name").unwrap();
//! ```
//!
//! **Backend-agnostic usage:**
//! ```rust,ignore
//! use horus_core::communication::traits::{Publisher, Subscriber};
//! fn send_message<P: Publisher<String>>(publisher: &P, msg: String) {
//!     publisher.send(msg, None).unwrap();
//! }
//! ```

pub mod horus;
pub mod traits;

// Re-export commonly used types for convenience
pub use horus::{Hub, Link};
pub use traits::{Channel, Publisher, Subscriber};

// Type aliases for backward compatibility
pub use horus::Hub as HorusHub;
pub use horus::Link as HorusLink;
