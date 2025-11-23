//! # Shared Memory utilities for HORUS
//!
//! This module provides core shared memory functionality for robotics applications:
//!
//! - **ShmRegion**: Cross-process memory regions using HORUS absolute paths
//! - **ShmTopic**: Lock-free ring buffers in shared memory for high-performance messaging
//!
//! ## Performance Features
//!
//! HORUS shared memory is designed for low-latency robotics systems:
//! - **True shared memory**: Cross-process memory sharing via memory-mapped files
//! - **Lock-free ring buffers**: Atomic operations for high-concurrency scenarios
//! - **Zero-copy access**: Direct memory access without serialization overhead
//!
//! ## Memory Safety
//!
//! All memory operations maintain Rust's safety guarantees through careful
//! use of lifetime management and atomic operations.

pub mod platform;
pub mod shm_region;
pub mod shm_topic;

pub use platform::*;
pub use shm_region::ShmRegion;
pub use shm_topic::ShmTopic;

// Tests are in the tests/ directory
