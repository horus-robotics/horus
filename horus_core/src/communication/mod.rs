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

pub mod hub;
pub mod link;
pub mod traits;

// Re-export commonly used types for convenience
pub use hub::Hub;
pub use link::{Link, LinkMetrics};
pub use traits::{Channel, Publisher, Subscriber};

use crate::communication::traits::{Publisher as PublisherTrait, Subscriber as SubscriberTrait};

// Implement common traits for Hub
impl<T> PublisherTrait<T> for Hub<T>
where
    T: Send
        + Sync
        + Clone
        + std::fmt::Debug
        + serde::Serialize
        + serde::de::DeserializeOwned
        + 'static,
{
    fn send(&self, msg: T) -> crate::error::HorusResult<()> {
        // Call the Hub's actual send method
        Hub::send(self, msg, None).map(|_| ()).map_err(|_| {
            crate::error::HorusError::Communication("Failed to send message".to_string())
        })
    }
}

impl<T> SubscriberTrait<T> for Hub<T>
where
    T: Send
        + Sync
        + Clone
        + std::fmt::Debug
        + serde::Serialize
        + serde::de::DeserializeOwned
        + 'static,
{
    fn recv(&self) -> Option<T> {
        Hub::recv(self, None)
    }
}

// Implement common traits for Link
impl<T> PublisherTrait<T> for Link<T>
where
    T: Send + Sync + Clone + std::fmt::Debug + 'static,
{
    fn send(&self, msg: T) -> crate::error::HorusResult<()> {
        Link::send(self, msg, None).map_err(|_| {
            crate::error::HorusError::Communication("Failed to send message".to_string())
        })
    }

    fn try_send(&self, msg: T) -> bool {
        Link::send(self, msg, None).is_ok()
    }
}

impl<T> SubscriberTrait<T> for Link<T>
where
    T: Send + Sync + Clone + std::fmt::Debug + 'static,
{
    fn recv(&self) -> Option<T> {
        Link::recv(self, None)
    }

    fn has_messages(&self) -> bool {
        Link::has_messages(self)
    }
}
