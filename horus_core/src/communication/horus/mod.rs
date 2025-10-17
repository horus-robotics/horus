//! HORUS native IPC implementations
//! 
//! High-performance shared memory IPC with lock-free optimizations:
//! - Hub: MPMC publisher-subscriber with cache-aligned atomics
//! - Link: SPSC direct channel with ultra-low latency (85-167ns)

pub mod hub;
pub mod link;

pub use hub::Hub;
pub use link::Link;

use crate::communication::traits::{Publisher, Subscriber};

// Implement common traits for HORUS Hub
impl<T> Publisher<T> for Hub<T>
where
    T: Send + Sync + Clone + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned + 'static
{
    fn send(&self, msg: T) -> crate::error::HorusResult<()> {
        // Call the Hub's actual send method
        Hub::send(self, msg, None).map(|_| ()).map_err(|_| crate::error::HorusError::Communication("Failed to send message".to_string()))
    }
}

impl<T> Subscriber<T> for Hub<T>
where
    T: Send + Sync + Clone + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned + 'static
{
    fn recv(&self) -> Option<T> {
        Hub::recv(self, None)
    }
}

// Implement common traits for HORUS Link
impl<T> Publisher<T> for Link<T>
where
    T: Send + Sync + Clone + std::fmt::Debug + 'static
{
    fn send(&self, msg: T) -> crate::error::HorusResult<()> {
        Link::send(self, msg, None).map_err(|_| crate::error::HorusError::Communication("Failed to send message".to_string()))
    }

    fn try_send(&self, msg: T) -> bool {
        Link::send(self, msg, None).is_ok()
    }
}

impl<T> Subscriber<T> for Link<T>
where
    T: Send + Sync + Clone + std::fmt::Debug + 'static
{
    fn recv(&self) -> Option<T> {
        Link::recv(self, None)
    }

    fn has_messages(&self) -> bool {
        Link::has_messages(self)
    }
}