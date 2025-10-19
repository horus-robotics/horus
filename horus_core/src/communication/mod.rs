//! # Communication layer for HORUS
//!
//! This module provides multiple IPC implementations that can be used interchangeably:
//!
//! - **horus**: Native HORUS IPC with shared memory and cache optimizations
//!   - **Hub**: MPMC publisher-subscriber pattern (167-6994 ns/msg)
//!   - **Link**: SPSC point-to-point channels (85-167 ns/msg, ultra-low latency)
//!
//! - **iceoryx2**: Zero-copy IPC integration for large message scenarios
//!   - **Publisher/Subscriber**: Zero-copy loan-based messaging (624-6727 ns/msg)
//!   - **Service**: iceoryx2 service management
//!
//! - **zenoh**: Distributed IPC for network-based and cross-platform scenarios
//!   - **Publisher/Subscriber**: Network-aware messaging with automatic discovery
//!   - **Session**: Zenoh session management and configuration
//!
//! ## Usage Patterns
//!
//! **For ultra-low latency (real-time control loops):**
//! ```rust
//! use horus_core::communication::horus::Link;
//! let link = Link::new(1024);
//! ```
//!
//! **For general-purpose IPC:**
//! ```rust  
//! use horus_core::communication::horus::Hub;
//! let hub = Hub::new("topic_name").unwrap();
//! ```
//!
//! **For large message zero-copy:**
//! ```rust
//! use horus_core::communication::iceoryx2::Service;
//! let service = Service::new("service_name").unwrap();
//! let publisher = service.create_publisher().unwrap();
//! ```
//!
//! **For distributed/network communication:**
//! ```rust
//! use horus_core::communication::zenoh::Session;
//! let session = Session::new().await.unwrap();
//! let publisher = session.create_publisher("robot/sensors/lidar").unwrap();
//! ```
//!
//! **Backend-agnostic usage:**
//! ```rust
//! use horus_core::communication::traits::{Publisher, Subscriber};
//! // Works with both HORUS Hub and iceoryx2 Publisher
//! fn send_message<P: Publisher<String>>(pub: &P, msg: String) {
//!     pub.send(msg).unwrap();
//! }
//! ```

pub mod horus;
#[cfg(feature = "iceoryx2")]
pub mod iceoryx2;
pub mod traits;
#[cfg(feature = "zenoh")]
pub mod zenoh;

// Re-export commonly used types for convenience
pub use horus::{Hub, Link};
pub use traits::{Channel, IpcBackend, Publisher, Subscriber};

// Type aliases for backward compatibility
pub use horus::Hub as HorusHub;
pub use horus::Link as HorusLink;

#[cfg(feature = "iceoryx2")]
pub use iceoryx2::Publisher as IceoryxPublisher;
#[cfg(feature = "iceoryx2")]
pub use iceoryx2::Service as IceoryxService;
#[cfg(feature = "iceoryx2")]
pub use iceoryx2::Subscriber as IceoryxSubscriber;

#[cfg(feature = "zenoh")]
pub use zenoh::Publisher as ZenohPublisher;
#[cfg(feature = "zenoh")]
pub use zenoh::Session as ZenohSession;
#[cfg(feature = "zenoh")]
pub use zenoh::Subscriber as ZenohSubscriber;
